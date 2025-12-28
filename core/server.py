"""
ai-way Core Server

FastAPI server exposing the Core API for surfaces to connect.
Includes REST endpoints and WebSocket for streaming.
"""

import asyncio
import json
from contextlib import asynccontextmanager
from pathlib import Path
from typing import Optional

import yaml
from fastapi import FastAPI, HTTPException, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

from .conductor import Conductor, ConductorConfig, create_conductor
from .loader import load_all_agents
from .ollama_client import OllamaClient, OllamaConfig
from .session import Session, SessionManager


# -------------------------------------------------------------------------
# Configuration
# -------------------------------------------------------------------------

def load_config(config_path: Path = Path("config.yaml")) -> dict:
    """Load configuration from YAML file."""
    if config_path.exists():
        return yaml.safe_load(config_path.read_text())
    return {}


# -------------------------------------------------------------------------
# Request/Response Models
# -------------------------------------------------------------------------

class ChatRequest(BaseModel):
    """Request for chat endpoint."""
    message: str
    session_id: Optional[str] = None
    force_agent: Optional[str] = None  # Skip routing, use this agent


class ChatResponse(BaseModel):
    """Response from chat endpoint."""
    message: str
    session_id: str
    routing: Optional[dict] = None
    tokens_used: int = 0
    latency_ms: float = 0.0

    # Developer mode extras
    specialist_response: Optional[str] = None


class AgentInfo(BaseModel):
    """Information about an available agent."""
    name: str
    category: str
    role: str


class SessionInfo(BaseModel):
    """Information about a session."""
    id: str
    turns: int
    total_tokens: int
    detected_mood: str


# -------------------------------------------------------------------------
# Application State
# -------------------------------------------------------------------------

class AppState:
    """Global application state."""
    config: dict
    conductor: Optional[Conductor] = None
    session_manager: Optional[SessionManager] = None
    dev_mode: bool = True

    def __init__(self):
        self.config = {}


state = AppState()


# -------------------------------------------------------------------------
# Lifespan Management
# -------------------------------------------------------------------------

@asynccontextmanager
async def lifespan(app: FastAPI):
    """Manage application startup and shutdown."""
    # Startup
    print("Starting ai-way Core...")

    # Load config
    state.config = load_config()
    state.dev_mode = state.config.get("dev_mode", {}).get("show_routing", True)

    # Initialize Ollama client
    ollama_config = OllamaConfig(
        host=state.config.get("ollama", {}).get("host", "localhost"),
        port=state.config.get("ollama", {}).get("port", 11434),
    )

    # Initialize session manager
    persist_path = None
    if state.config.get("dev_mode", {}).get("log_conversations"):
        persist_path = Path("logs/sessions.json")
    state.session_manager = SessionManager(persist_path)

    # Initialize conductor
    agents_path = Path(state.config.get("agents", {}).get("path", "../agents"))
    conductor_config = ConductorConfig(
        conductor_model=f"ai-way-yollayah",
    )

    try:
        state.conductor = create_conductor(agents_path, ollama_config, conductor_config)
        print(f"Loaded {len(state.conductor.agents)} agents")
    except Exception as e:
        print(f"Warning: Failed to initialize conductor: {e}")
        print("Server will start but chat functionality may be limited.")

    print(f"ai-way Core running on port {state.config.get('core', {}).get('port', 8420)}")

    yield

    # Shutdown
    print("Shutting down ai-way Core...")
    if state.session_manager:
        state.session_manager.persist()


# -------------------------------------------------------------------------
# FastAPI Application
# -------------------------------------------------------------------------

app = FastAPI(
    title="ai-way Core",
    description="Privacy-first local AI runtime",
    version="0.1.0",
    lifespan=lifespan,
)

# CORS for browser surfaces
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # In production, restrict to known surfaces
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


# -------------------------------------------------------------------------
# REST Endpoints
# -------------------------------------------------------------------------

@app.get("/")
async def root():
    """Health check and info."""
    return {
        "name": "ai-way Core",
        "version": "0.1.0",
        "status": "running",
        "conductor": state.conductor is not None,
        "agents": len(state.conductor.agents) if state.conductor else 0,
    }


@app.post("/chat", response_model=ChatResponse)
async def chat(request: ChatRequest):
    """
    Send a message and get a response.

    This is the main endpoint for conversing with Yollayah.
    """
    if not state.conductor:
        raise HTTPException(status_code=503, detail="Conductor not initialized")

    # Get or create session
    session = state.session_manager.get_or_create_session(request.session_id)

    # Process the query
    try:
        response = state.conductor.respond(
            query=request.message,
            session=session,
            force_agent=request.force_agent,
        )
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))

    # Record the turn
    session.add_turn(
        user_message=request.message,
        assistant_response=response.message,
        routing=response.routing,
        tokens_used=response.tokens_used,
        latency_ms=response.latency_ms,
    )

    # Build response
    chat_response = ChatResponse(
        message=response.message,
        session_id=session.id,
        tokens_used=response.tokens_used,
        latency_ms=response.latency_ms,
    )

    # Include dev mode info
    if state.dev_mode:
        if response.routing:
            chat_response.routing = {
                "agent": response.routing.agent,
                "confidence": response.routing.confidence,
                "reasoning": response.routing.reasoning,
            }
        chat_response.specialist_response = response.specialist_response

    return chat_response


@app.get("/agents", response_model=list[AgentInfo])
async def list_agents():
    """List all available specialist agents."""
    if not state.conductor:
        return []

    return [
        AgentInfo(
            name=agent.name,
            category=agent.category,
            role=agent.role[:200] + "..." if len(agent.role) > 200 else agent.role,
        )
        for agent in state.conductor.agents.values()
    ]


@app.get("/sessions", response_model=list[SessionInfo])
async def list_sessions():
    """List all active sessions."""
    if not state.session_manager:
        return []

    return [
        SessionInfo(
            id=session.id,
            turns=session.total_turns,
            total_tokens=session.total_tokens,
            detected_mood=session.detected_mood,
        )
        for session in state.session_manager.list_sessions()
    ]


@app.get("/sessions/{session_id}")
async def get_session(session_id: str):
    """Get full session details."""
    if not state.session_manager:
        raise HTTPException(status_code=503, detail="Session manager not initialized")

    session = state.session_manager.get_session(session_id)
    if not session:
        raise HTTPException(status_code=404, detail="Session not found")

    return session.to_dict()


@app.delete("/sessions/{session_id}")
async def delete_session(session_id: str):
    """Delete a session (privacy: user can clear history)."""
    if not state.session_manager:
        raise HTTPException(status_code=503, detail="Session manager not initialized")

    if state.session_manager.delete_session(session_id):
        return {"status": "deleted", "session_id": session_id}
    else:
        raise HTTPException(status_code=404, detail="Session not found")


# -------------------------------------------------------------------------
# WebSocket for Streaming
# -------------------------------------------------------------------------

@app.websocket("/stream")
async def websocket_stream(websocket: WebSocket):
    """
    WebSocket endpoint for streaming responses.

    Protocol:
    - Client sends: {"message": "...", "session_id": "..."}
    - Server streams: {"token": "..."} for each token
    - Server sends: {"done": true, "routing": {...}} when complete
    """
    await websocket.accept()

    try:
        while True:
            # Receive message
            data = await websocket.receive_json()
            message = data.get("message", "")
            session_id = data.get("session_id")

            if not state.conductor:
                await websocket.send_json({"error": "Conductor not initialized"})
                continue

            # Get or create session
            session = state.session_manager.get_or_create_session(session_id)

            # Send session ID if new
            if session_id != session.id:
                await websocket.send_json({"session_id": session.id})

            # Process and stream response
            # Note: For now, we'll send the full response
            # TODO: Implement true streaming with Ollama streaming API
            try:
                response = state.conductor.respond(
                    query=message,
                    session=session,
                )

                # Send the response
                await websocket.send_json({
                    "message": response.message,
                    "done": True,
                    "routing": {
                        "agent": response.routing.agent,
                        "confidence": response.routing.confidence,
                        "reasoning": response.routing.reasoning,
                    } if response.routing else None,
                    "latency_ms": response.latency_ms,
                })

                # Record the turn
                session.add_turn(
                    user_message=message,
                    assistant_response=response.message,
                    routing=response.routing,
                    latency_ms=response.latency_ms,
                )

            except Exception as e:
                await websocket.send_json({"error": str(e)})

    except WebSocketDisconnect:
        print("WebSocket client disconnected")


# -------------------------------------------------------------------------
# Main Entry Point
# -------------------------------------------------------------------------

def main():
    """Run the server."""
    import uvicorn

    config = load_config()
    host = config.get("core", {}).get("host", "0.0.0.0")
    port = config.get("core", {}).get("port", 8420)

    print(f"Starting ai-way Core on {host}:{port}")
    uvicorn.run(app, host=host, port=port)


if __name__ == "__main__":
    main()
