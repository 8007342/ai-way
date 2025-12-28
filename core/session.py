"""
Session Management

Manages conversation state across interactions and surfaces.
Sessions are ephemeral by default (privacy-first) but can be
persisted if the user explicitly opts in.
"""

import json
import uuid
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any, Optional

from .ollama_client import ChatMessage


@dataclass
class RoutingDecision:
    """Record of a routing decision made by the Conductor."""
    agent: str
    confidence: float
    reasoning: str
    timestamp: datetime = field(default_factory=datetime.now)


@dataclass
class Turn:
    """A single turn in the conversation (user message + response)."""
    user_message: str
    assistant_response: str
    routing: Optional[RoutingDecision] = None
    tokens_used: int = 0
    latency_ms: float = 0.0
    timestamp: datetime = field(default_factory=datetime.now)


@dataclass
class Session:
    """
    A conversation session.

    Sessions maintain:
    - Conversation history (for context)
    - Accumulated context (for agent handoffs)
    - Active surface tracking (which UI is currently active)
    - Developer mode metrics
    """
    id: str = field(default_factory=lambda: str(uuid.uuid4())[:8])
    created_at: datetime = field(default_factory=datetime.now)

    # Conversation history
    turns: list[Turn] = field(default_factory=list)

    # Accumulated context for agent handoffs
    context: dict[str, Any] = field(default_factory=dict)

    # Which surface is currently active
    active_surface: Optional[str] = None

    # User preferences discovered during session
    preferences: dict[str, Any] = field(default_factory=dict)

    # Session-level mood detection
    detected_mood: str = "neutral"

    def add_turn(
        self,
        user_message: str,
        assistant_response: str,
        routing: Optional[RoutingDecision] = None,
        tokens_used: int = 0,
        latency_ms: float = 0.0,
    ) -> Turn:
        """Add a new turn to the conversation."""
        turn = Turn(
            user_message=user_message,
            assistant_response=assistant_response,
            routing=routing,
            tokens_used=tokens_used,
            latency_ms=latency_ms,
        )
        self.turns.append(turn)
        return turn

    def get_chat_history(self, max_turns: int = 10) -> list[ChatMessage]:
        """
        Get conversation history as ChatMessage list for Ollama.

        Args:
            max_turns: Maximum number of turns to include

        Returns:
            List of ChatMessage objects
        """
        messages = []
        recent_turns = self.turns[-max_turns:] if max_turns else self.turns

        for turn in recent_turns:
            messages.append(ChatMessage(role="user", content=turn.user_message))
            messages.append(ChatMessage(role="assistant", content=turn.assistant_response))

        return messages

    def get_context_summary(self) -> str:
        """
        Generate a summary of accumulated context for agent handoffs.

        This is what gets passed to specialist agents so they have
        relevant background without seeing the full conversation.
        """
        parts = []

        # Recent conversation summary
        if self.turns:
            recent = self.turns[-3:]  # Last 3 turns
            parts.append("Recent conversation:")
            for turn in recent:
                parts.append(f"  User: {turn.user_message[:100]}...")
                parts.append(f"  Assistant: {turn.assistant_response[:100]}...")

        # Explicit context items
        if self.context:
            parts.append("\nContext:")
            for key, value in self.context.items():
                parts.append(f"  {key}: {value}")

        # Detected preferences
        if self.preferences:
            parts.append("\nUser preferences:")
            for key, value in self.preferences.items():
                parts.append(f"  {key}: {value}")

        # Current mood
        if self.detected_mood != "neutral":
            parts.append(f"\nDetected mood: {self.detected_mood}")

        return "\n".join(parts)

    def update_context(self, key: str, value: Any):
        """Add or update a context item."""
        self.context[key] = value

    def set_mood(self, mood: str):
        """Update detected mood."""
        self.detected_mood = mood

    @property
    def total_tokens(self) -> int:
        """Total tokens used in this session."""
        return sum(turn.tokens_used for turn in self.turns)

    @property
    def total_turns(self) -> int:
        """Number of conversation turns."""
        return len(self.turns)

    def to_dict(self) -> dict:
        """Serialize session to dictionary."""
        return {
            "id": self.id,
            "created_at": self.created_at.isoformat(),
            "turns": [
                {
                    "user_message": t.user_message,
                    "assistant_response": t.assistant_response,
                    "routing": {
                        "agent": t.routing.agent,
                        "confidence": t.routing.confidence,
                        "reasoning": t.routing.reasoning,
                    } if t.routing else None,
                    "tokens_used": t.tokens_used,
                    "latency_ms": t.latency_ms,
                    "timestamp": t.timestamp.isoformat(),
                }
                for t in self.turns
            ],
            "context": self.context,
            "preferences": self.preferences,
            "detected_mood": self.detected_mood,
            "active_surface": self.active_surface,
        }

    @classmethod
    def from_dict(cls, data: dict) -> "Session":
        """Deserialize session from dictionary."""
        session = cls(
            id=data["id"],
            created_at=datetime.fromisoformat(data["created_at"]),
            context=data.get("context", {}),
            preferences=data.get("preferences", {}),
            detected_mood=data.get("detected_mood", "neutral"),
            active_surface=data.get("active_surface"),
        )

        for turn_data in data.get("turns", []):
            routing = None
            if turn_data.get("routing"):
                routing = RoutingDecision(
                    agent=turn_data["routing"]["agent"],
                    confidence=turn_data["routing"]["confidence"],
                    reasoning=turn_data["routing"]["reasoning"],
                )

            session.turns.append(Turn(
                user_message=turn_data["user_message"],
                assistant_response=turn_data["assistant_response"],
                routing=routing,
                tokens_used=turn_data.get("tokens_used", 0),
                latency_ms=turn_data.get("latency_ms", 0.0),
                timestamp=datetime.fromisoformat(turn_data["timestamp"]),
            ))

        return session


class SessionManager:
    """
    Manages multiple sessions.

    Sessions are ephemeral by default. Persistence is opt-in.
    """

    def __init__(self, persist_path: Optional[Path] = None):
        self._sessions: dict[str, Session] = {}
        self._persist_path = persist_path

        if persist_path and persist_path.exists():
            self._load_sessions()

    def create_session(self) -> Session:
        """Create a new session."""
        session = Session()
        self._sessions[session.id] = session
        return session

    def get_session(self, session_id: str) -> Optional[Session]:
        """Get a session by ID."""
        return self._sessions.get(session_id)

    def get_or_create_session(self, session_id: Optional[str] = None) -> Session:
        """Get existing session or create new one."""
        if session_id and session_id in self._sessions:
            return self._sessions[session_id]
        return self.create_session()

    def delete_session(self, session_id: str) -> bool:
        """Delete a session (privacy: user can clear history)."""
        if session_id in self._sessions:
            del self._sessions[session_id]
            if self._persist_path:
                self._save_sessions()
            return True
        return False

    def list_sessions(self) -> list[Session]:
        """List all active sessions."""
        return list(self._sessions.values())

    def persist(self):
        """Persist sessions to disk (opt-in only)."""
        if self._persist_path:
            self._save_sessions()

    def _save_sessions(self):
        """Save sessions to disk."""
        if not self._persist_path:
            return

        self._persist_path.parent.mkdir(parents=True, exist_ok=True)
        data = {
            session_id: session.to_dict()
            for session_id, session in self._sessions.items()
        }
        self._persist_path.write_text(json.dumps(data, indent=2))

    def _load_sessions(self):
        """Load sessions from disk."""
        if not self._persist_path or not self._persist_path.exists():
            return

        try:
            data = json.loads(self._persist_path.read_text())
            for session_id, session_data in data.items():
                self._sessions[session_id] = Session.from_dict(session_data)
        except (json.JSONDecodeError, KeyError) as e:
            print(f"Warning: Failed to load sessions: {e}")


if __name__ == "__main__":
    # Quick test
    manager = SessionManager()

    # Create a session
    session = manager.create_session()
    print(f"Created session: {session.id}")

    # Simulate some turns
    session.add_turn(
        user_message="Help me review this Python code for security issues",
        assistant_response="I'll get my security expert on this...",
        routing=RoutingDecision(
            agent="ethical-hacker",
            confidence=0.92,
            reasoning="Query mentions 'security' and 'review code'",
        ),
        tokens_used=150,
        latency_ms=1234.5,
    )

    session.add_turn(
        user_message="What about SQL injection specifically?",
        assistant_response="Good question! Let me check for SQL injection patterns...",
        routing=RoutingDecision(
            agent="ethical-hacker",
            confidence=0.95,
            reasoning="Follow-up on security topic, same agent",
        ),
        tokens_used=200,
        latency_ms=987.3,
    )

    # Update context
    session.update_context("code_language", "Python")
    session.update_context("focus_area", "security")

    print(f"\nSession stats:")
    print(f"  Turns: {session.total_turns}")
    print(f"  Tokens: {session.total_tokens}")

    print(f"\nContext summary:")
    print(session.get_context_summary())

    print(f"\nSerialized:")
    print(json.dumps(session.to_dict(), indent=2, default=str))
