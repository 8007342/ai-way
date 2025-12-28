"""
The Conductor (Yollayah)

The meta-agent that routes queries to specialists, aggregates responses,
and maintains the conversation's friendly face.
"""

import re
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

from .loader import AgentProfile, load_all_agents
from .ollama_client import ChatMessage, OllamaClient, OllamaConfig
from .session import RoutingDecision, Session


@dataclass
class ConductorConfig:
    """Configuration for the Conductor."""
    conductor_model: str = "ai-way-yollayah"
    default_agent_model_prefix: str = "ai-way-"
    max_context_turns: int = 10
    routing_temperature: float = 0.3  # Lower for more consistent routing
    response_temperature: float = 0.8  # Higher for personality


@dataclass
class ConductorResponse:
    """Response from the Conductor."""
    message: str
    routing: Optional[RoutingDecision] = None
    tokens_used: int = 0
    latency_ms: float = 0.0
    specialist_response: Optional[str] = None


class Conductor:
    """
    The Conductor orchestrates ai-way's multi-agent system.

    Responsibilities:
    1. Receive all queries from the user
    2. Determine routing (which specialist, if any)
    3. Hand off to specialists with appropriate context
    4. Aggregate and present responses as Yollayah
    5. Maintain conversation continuity
    """

    def __init__(
        self,
        agents: list[AgentProfile],
        ollama: OllamaClient,
        config: Optional[ConductorConfig] = None,
    ):
        self.agents = {a.name: a for a in agents}
        self.ollama = ollama
        self.config = config or ConductorConfig()

        # Build agent catalog for routing prompts
        self._agent_catalog = self._build_agent_catalog()

    def _build_agent_catalog(self) -> str:
        """Build a catalog of agents for routing decisions."""
        lines = []
        for name, agent in self.agents.items():
            # Truncate role to keep catalog concise
            role = agent.role[:150] + "..." if len(agent.role) > 150 else agent.role
            lines.append(f"- {name} ({agent.category}): {role}")
        return "\n".join(lines)

    def route(self, query: str, session: Session) -> RoutingDecision:
        """
        Determine which agent should handle this query.

        Returns:
            RoutingDecision with agent name, confidence, and reasoning
        """
        # Build routing prompt
        routing_prompt = f"""You are the routing component of Yollayah.

Analyze this query and decide which specialist agent should handle it.
If no specialist is needed (general conversation, simple questions), respond with "conductor".

Available specialists:
{self._agent_catalog}

Query: {query}

Context from conversation:
{session.get_context_summary() if session.turns else "No prior context."}

Respond in this exact format:
AGENT: <agent-name>
CONFIDENCE: <0.0-1.0>
REASON: <brief explanation>
"""

        # Get routing decision from Yollayah
        response = self.ollama.generate(
            model=self.config.conductor_model,
            prompt=routing_prompt,
            temperature=self.config.routing_temperature,
        )

        # Parse the response
        return self._parse_routing_response(response.response)

    def _parse_routing_response(self, response: str) -> RoutingDecision:
        """Parse the routing response into a RoutingDecision."""
        agent = "conductor"
        confidence = 0.5
        reasoning = "Default routing"

        # Parse AGENT line
        agent_match = re.search(r'AGENT:\s*(\S+)', response, re.IGNORECASE)
        if agent_match:
            agent = agent_match.group(1).strip().lower()

        # Parse CONFIDENCE line
        conf_match = re.search(r'CONFIDENCE:\s*([\d.]+)', response, re.IGNORECASE)
        if conf_match:
            try:
                confidence = float(conf_match.group(1))
                confidence = max(0.0, min(1.0, confidence))  # Clamp to [0, 1]
            except ValueError:
                pass

        # Parse REASON line
        reason_match = re.search(r'REASON:\s*(.+?)(?:\n|$)', response, re.IGNORECASE)
        if reason_match:
            reasoning = reason_match.group(1).strip()

        # Validate agent exists
        if agent != "conductor" and agent not in self.agents:
            # Try to find closest match
            for name in self.agents:
                if agent in name or name in agent:
                    agent = name
                    break
            else:
                agent = "conductor"
                reasoning = f"Agent '{agent}' not found, handling directly"

        return RoutingDecision(
            agent=agent,
            confidence=confidence,
            reasoning=reasoning,
        )

    def handoff(
        self,
        query: str,
        agent_name: str,
        session: Session,
    ) -> str:
        """
        Hand off a query to a specialist agent.

        Args:
            query: The user's query
            agent_name: Name of the specialist agent
            session: Current session for context

        Returns:
            The specialist's response
        """
        agent = self.agents.get(agent_name)
        if not agent:
            return f"Agent {agent_name} not found."

        # Build context for the specialist
        context = session.get_context_summary()

        # Create handoff prompt
        handoff_prompt = f"""Context from conversation:
{context}

User query: {query}

Please respond as the {agent_name.replace('-', ' ')} specialist.
Focus on your area of expertise. Be helpful and specific.
"""

        # Get model name
        model_name = f"{self.config.default_agent_model_prefix}{agent_name}"

        # Call the specialist
        response = self.ollama.generate(
            model=model_name,
            prompt=handoff_prompt,
            temperature=self.config.response_temperature,
        )

        return response.response

    def respond(
        self,
        query: str,
        session: Session,
        force_agent: Optional[str] = None,
    ) -> ConductorResponse:
        """
        Process a query and generate a response.

        This is the main entry point for handling user queries.

        Args:
            query: The user's query
            session: Current conversation session
            force_agent: If set, skip routing and use this agent

        Returns:
            ConductorResponse with message and metadata
        """
        start_time = time.time()
        specialist_response = None

        # Step 1: Route (unless forced)
        if force_agent:
            routing = RoutingDecision(
                agent=force_agent,
                confidence=1.0,
                reasoning="Agent forced by request",
            )
        else:
            routing = self.route(query, session)

        # Step 2: Handle based on routing
        if routing.agent == "conductor":
            # Yollayah handles directly
            response_text = self._respond_directly(query, session)
        else:
            # Hand off to specialist
            specialist_response = self.handoff(query, routing.agent, session)

            # Yollayah presents the specialist's response
            response_text = self._present_specialist_response(
                query, routing.agent, specialist_response, session
            )

        # Calculate metrics
        latency_ms = (time.time() - start_time) * 1000

        return ConductorResponse(
            message=response_text,
            routing=routing,
            latency_ms=latency_ms,
            specialist_response=specialist_response,
        )

    def _respond_directly(self, query: str, session: Session) -> str:
        """Generate a direct response from Yollayah (no specialist)."""
        # Build messages for chat
        messages = session.get_chat_history(self.config.max_context_turns)
        messages.append(ChatMessage(role="user", content=query))

        response = self.ollama.chat(
            model=self.config.conductor_model,
            messages=messages,
            temperature=self.config.response_temperature,
        )

        return response.response

    def _present_specialist_response(
        self,
        query: str,
        agent_name: str,
        specialist_response: str,
        session: Session,
    ) -> str:
        """
        Have Yollayah present a specialist's response.

        This maintains Yollayah's personality while delivering
        specialist information.
        """
        presentation_prompt = f"""You are Yollayah presenting information from a specialist.

The {agent_name.replace('-', ' ')} just provided this response to the user's question:

SPECIALIST RESPONSE:
{specialist_response}

USER'S ORIGINAL QUESTION:
{query}

Present this information in your own voice (warm, real, playfully opinionated).
You can:
- Summarize if the response is very technical
- Add encouraging comments
- Ask if they need clarification
- Celebrate if they're making progress

Keep your personality but make sure the specialist's key information comes through.
Don't say "the specialist said" - just present it naturally as if you're explaining it.
"""

        response = self.ollama.generate(
            model=self.config.conductor_model,
            prompt=presentation_prompt,
            temperature=self.config.response_temperature,
        )

        return response.response


def create_conductor(
    agents_path: Path,
    ollama_config: Optional[OllamaConfig] = None,
    conductor_config: Optional[ConductorConfig] = None,
) -> Conductor:
    """
    Factory function to create a Conductor with all dependencies.

    Args:
        agents_path: Path to agents repository
        ollama_config: Ollama connection settings
        conductor_config: Conductor behavior settings

    Returns:
        Configured Conductor instance
    """
    agents = load_all_agents(agents_path)
    ollama = OllamaClient(ollama_config)
    return Conductor(agents, ollama, conductor_config)


if __name__ == "__main__":
    import sys

    # Quick test
    agents_path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("../agents")

    print(f"Loading agents from: {agents_path}")
    agents = load_all_agents(agents_path)
    print(f"Loaded {len(agents)} agents")

    # Create conductor (won't actually call Ollama in this test)
    ollama = OllamaClient()
    conductor = Conductor(agents, ollama)

    print("\nAgent catalog:")
    print(conductor._agent_catalog)

    # Test routing parse
    test_responses = [
        "AGENT: ethical-hacker\nCONFIDENCE: 0.92\nREASON: Query about security review",
        "AGENT: conductor\nCONFIDENCE: 0.8\nREASON: General conversation",
        "AGENT: backend-engineer\nCONFIDENCE: 0.75\nREASON: API design question",
    ]

    print("\nTesting routing parse:")
    for resp in test_responses:
        decision = conductor._parse_routing_response(resp)
        print(f"  {decision.agent} @ {decision.confidence:.2f}: {decision.reasoning}")
