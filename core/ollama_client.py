"""
Ollama Client

Wrapper for the Ollama API. Handles model creation, inference, and streaming.
"""

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import AsyncIterator, Iterator, Optional

import httpx


@dataclass
class OllamaConfig:
    """Configuration for Ollama connection."""
    host: str = "localhost"
    port: int = 11434
    timeout: float = 120.0  # Long timeout for inference

    @property
    def base_url(self) -> str:
        return f"http://{self.host}:{self.port}"


@dataclass
class ChatMessage:
    """A single message in a conversation."""
    role: str  # "system", "user", "assistant"
    content: str


@dataclass
class GenerateResponse:
    """Response from a generate/chat call."""
    model: str
    response: str
    done: bool
    total_duration: Optional[int] = None  # nanoseconds
    load_duration: Optional[int] = None
    prompt_eval_count: Optional[int] = None
    eval_count: Optional[int] = None  # tokens generated
    eval_duration: Optional[int] = None

    @property
    def tokens_per_second(self) -> float:
        """Calculate tokens per second."""
        if self.eval_count and self.eval_duration:
            return self.eval_count / (self.eval_duration / 1e9)
        return 0.0


class OllamaClient:
    """
    Client for interacting with Ollama API.

    Supports:
    - Model creation from modelfiles
    - Chat completions
    - Streaming responses
    - Model listing and management
    """

    def __init__(self, config: Optional[OllamaConfig] = None):
        self.config = config or OllamaConfig()
        self._client = httpx.Client(
            base_url=self.config.base_url,
            timeout=self.config.timeout,
        )
        self._async_client: Optional[httpx.AsyncClient] = None

    def _get_async_client(self) -> httpx.AsyncClient:
        """Lazy initialization of async client."""
        if self._async_client is None:
            self._async_client = httpx.AsyncClient(
                base_url=self.config.base_url,
                timeout=self.config.timeout,
            )
        return self._async_client

    def close(self):
        """Close HTTP clients."""
        self._client.close()
        if self._async_client:
            # Note: async client should be closed in async context
            pass

    # -------------------------------------------------------------------------
    # Model Management
    # -------------------------------------------------------------------------

    def list_models(self) -> list[dict]:
        """List all available models."""
        response = self._client.get("/api/tags")
        response.raise_for_status()
        return response.json().get("models", [])

    def model_exists(self, name: str) -> bool:
        """Check if a model exists."""
        models = self.list_models()
        return any(m.get("name", "").startswith(name) for m in models)

    def create_model(self, name: str, modelfile_content: str) -> bool:
        """
        Create a model from a modelfile.

        Args:
            name: Name for the new model
            modelfile_content: Content of the modelfile

        Returns:
            True if successful
        """
        response = self._client.post(
            "/api/create",
            json={"name": name, "modelfile": modelfile_content},
            timeout=300.0,  # Model creation can take a while
        )
        response.raise_for_status()
        return True

    def create_model_from_file(self, name: str, modelfile_path: Path) -> bool:
        """Create a model from a modelfile on disk."""
        content = modelfile_path.read_text(encoding="utf-8")
        return self.create_model(name, content)

    def delete_model(self, name: str) -> bool:
        """Delete a model."""
        response = self._client.delete("/api/delete", json={"name": name})
        response.raise_for_status()
        return True

    # -------------------------------------------------------------------------
    # Inference - Synchronous
    # -------------------------------------------------------------------------

    def generate(
        self,
        model: str,
        prompt: str,
        system: Optional[str] = None,
        temperature: Optional[float] = None,
        stream: bool = False,
    ) -> GenerateResponse | Iterator[str]:
        """
        Generate a completion.

        Args:
            model: Model name
            prompt: User prompt
            system: Optional system prompt override
            temperature: Optional temperature override
            stream: If True, return an iterator of tokens

        Returns:
            GenerateResponse or iterator of tokens if streaming
        """
        payload = {
            "model": model,
            "prompt": prompt,
            "stream": stream,
        }
        if system:
            payload["system"] = system
        if temperature is not None:
            payload["options"] = {"temperature": temperature}

        if stream:
            return self._stream_generate(payload)
        else:
            response = self._client.post("/api/generate", json=payload)
            response.raise_for_status()
            data = response.json()
            return GenerateResponse(
                model=data.get("model", model),
                response=data.get("response", ""),
                done=data.get("done", True),
                total_duration=data.get("total_duration"),
                load_duration=data.get("load_duration"),
                prompt_eval_count=data.get("prompt_eval_count"),
                eval_count=data.get("eval_count"),
                eval_duration=data.get("eval_duration"),
            )

    def _stream_generate(self, payload: dict) -> Iterator[str]:
        """Stream tokens from generate endpoint."""
        with self._client.stream("POST", "/api/generate", json=payload) as response:
            response.raise_for_status()
            for line in response.iter_lines():
                if line:
                    data = json.loads(line)
                    if token := data.get("response"):
                        yield token
                    if data.get("done"):
                        break

    def chat(
        self,
        model: str,
        messages: list[ChatMessage],
        temperature: Optional[float] = None,
        stream: bool = False,
    ) -> GenerateResponse | Iterator[str]:
        """
        Chat completion with message history.

        Args:
            model: Model name
            messages: List of ChatMessage objects
            temperature: Optional temperature override
            stream: If True, return an iterator of tokens

        Returns:
            GenerateResponse or iterator of tokens if streaming
        """
        payload = {
            "model": model,
            "messages": [{"role": m.role, "content": m.content} for m in messages],
            "stream": stream,
        }
        if temperature is not None:
            payload["options"] = {"temperature": temperature}

        if stream:
            return self._stream_chat(payload)
        else:
            response = self._client.post("/api/chat", json=payload)
            response.raise_for_status()
            data = response.json()
            return GenerateResponse(
                model=data.get("model", model),
                response=data.get("message", {}).get("content", ""),
                done=data.get("done", True),
                total_duration=data.get("total_duration"),
                load_duration=data.get("load_duration"),
                prompt_eval_count=data.get("prompt_eval_count"),
                eval_count=data.get("eval_count"),
                eval_duration=data.get("eval_duration"),
            )

    def _stream_chat(self, payload: dict) -> Iterator[str]:
        """Stream tokens from chat endpoint."""
        with self._client.stream("POST", "/api/chat", json=payload) as response:
            response.raise_for_status()
            for line in response.iter_lines():
                if line:
                    data = json.loads(line)
                    if msg := data.get("message"):
                        if token := msg.get("content"):
                            yield token
                    if data.get("done"):
                        break

    # -------------------------------------------------------------------------
    # Inference - Asynchronous
    # -------------------------------------------------------------------------

    async def agenerate(
        self,
        model: str,
        prompt: str,
        system: Optional[str] = None,
        temperature: Optional[float] = None,
        stream: bool = False,
    ) -> GenerateResponse | AsyncIterator[str]:
        """Async version of generate."""
        client = self._get_async_client()
        payload = {
            "model": model,
            "prompt": prompt,
            "stream": stream,
        }
        if system:
            payload["system"] = system
        if temperature is not None:
            payload["options"] = {"temperature": temperature}

        if stream:
            return self._astream_generate(payload)
        else:
            response = await client.post("/api/generate", json=payload)
            response.raise_for_status()
            data = response.json()
            return GenerateResponse(
                model=data.get("model", model),
                response=data.get("response", ""),
                done=data.get("done", True),
                total_duration=data.get("total_duration"),
                load_duration=data.get("load_duration"),
                prompt_eval_count=data.get("prompt_eval_count"),
                eval_count=data.get("eval_count"),
                eval_duration=data.get("eval_duration"),
            )

    async def _astream_generate(self, payload: dict) -> AsyncIterator[str]:
        """Async stream tokens from generate endpoint."""
        client = self._get_async_client()
        async with client.stream("POST", "/api/generate", json=payload) as response:
            response.raise_for_status()
            async for line in response.aiter_lines():
                if line:
                    data = json.loads(line)
                    if token := data.get("response"):
                        yield token
                    if data.get("done"):
                        break

    async def achat(
        self,
        model: str,
        messages: list[ChatMessage],
        temperature: Optional[float] = None,
        stream: bool = False,
    ) -> GenerateResponse | AsyncIterator[str]:
        """Async version of chat."""
        client = self._get_async_client()
        payload = {
            "model": model,
            "messages": [{"role": m.role, "content": m.content} for m in messages],
            "stream": stream,
        }
        if temperature is not None:
            payload["options"] = {"temperature": temperature}

        if stream:
            return self._astream_chat(payload)
        else:
            response = await client.post("/api/chat", json=payload)
            response.raise_for_status()
            data = response.json()
            return GenerateResponse(
                model=data.get("model", model),
                response=data.get("message", {}).get("content", ""),
                done=data.get("done", True),
                total_duration=data.get("total_duration"),
                load_duration=data.get("load_duration"),
                prompt_eval_count=data.get("prompt_eval_count"),
                eval_count=data.get("eval_count"),
                eval_duration=data.get("eval_duration"),
            )

    async def _astream_chat(self, payload: dict) -> AsyncIterator[str]:
        """Async stream tokens from chat endpoint."""
        client = self._get_async_client()
        async with client.stream("POST", "/api/chat", json=payload) as response:
            response.raise_for_status()
            async for line in response.aiter_lines():
                if line:
                    data = json.loads(line)
                    if msg := data.get("message"):
                        if token := msg.get("content"):
                            yield token
                    if data.get("done"):
                        break


# -------------------------------------------------------------------------
# Model Setup Utilities
# -------------------------------------------------------------------------

def setup_models(
    modelfiles_path: Path,
    ollama_config: Optional[OllamaConfig] = None,
    force: bool = False,
) -> list[str]:
    """
    Create all models from modelfiles directory.

    Args:
        modelfiles_path: Path to directory containing .modelfile files
        ollama_config: Ollama connection config
        force: If True, recreate existing models

    Returns:
        List of created model names
    """
    client = OllamaClient(ollama_config)
    created = []

    for modelfile in modelfiles_path.glob("*.modelfile"):
        model_name = f"ai-way-{modelfile.stem}"

        if not force and client.model_exists(model_name):
            print(f"Model exists: {model_name}")
            continue

        print(f"Creating model: {model_name}...")
        try:
            client.create_model_from_file(model_name, modelfile)
            created.append(model_name)
            print(f"  Created: {model_name}")
        except Exception as e:
            print(f"  Failed: {e}")

    client.close()
    return created


if __name__ == "__main__":
    import sys

    # Quick connectivity test
    config = OllamaConfig()
    client = OllamaClient(config)

    print(f"Connecting to Ollama at {config.base_url}...")

    try:
        models = client.list_models()
        print(f"Connected! Found {len(models)} models:")
        for model in models[:10]:
            print(f"  - {model.get('name')}")
        if len(models) > 10:
            print(f"  ... and {len(models) - 10} more")
    except httpx.ConnectError:
        print("Failed to connect to Ollama. Is it running?")
        sys.exit(1)

    # If modelfiles path provided, set up models
    if len(sys.argv) > 1:
        modelfiles_path = Path(sys.argv[1])
        if modelfiles_path.exists():
            print(f"\nSetting up models from {modelfiles_path}...")
            created = setup_models(modelfiles_path, config)
            print(f"\nCreated {len(created)} models")

    client.close()
