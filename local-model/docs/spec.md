# local-model/docs/spec.md
# L4Local Model Interface Specification

## Overview
The L4Local Model Interface provides a Tier A runtime with an always-on small model (3B-parameter quantized model like Phi-3). It integrates with the Policy Broker, Event Bus, and State Store for privacy tagging, health reporting, and embedding caching.

## Core Features
- **Tier A Runtime**: Always-on small model (e.g., Phi-3-mini-4k-instruct).
- **Privacy Tagging**: Stamp outputs with `privacy_tag` if input touched `CAP_MIC`/`CAP_CAMERA`/`CAP_FS_READ`.
- **Embedding Backend**: Power `lambda.search` semantic ranking.
- **Health Reporting**: Feed Event Bus (`category: health`).
- **MCP Interface**: Expose `localmodel.complete`, `localmodel.classify_intent`, `localmodel.embed`.

## Dependencies
- **Policy Broker**: Validate `CAP_IPC_CALL(targets=[localmodel])`.
- **Event Bus**: Publish `health` events.
- **State Store**: Cache embeddings for `lambda.search`.

## Implementation Details
- **Inference Engine**: `llama.cpp` (C++ backend with Python bindings).
- **Model**: Quantized 3B-parameter model (e.g., Phi-3).
- **Sandboxing**: Firecracker microVM or gVisor (stub for now).
- **Privacy Tagging**: Add `privacy_tag` to outputs if input touched sensitive capabilities.

## MCP Interface
- **Endpoints**:
  - `POST /mcp/localmodel.complete` → `{"text": "...", "privacy_tag": "..."}`
  - `POST /mcp/localmodel.classify_intent` → `{"intent": "media.play", "confidence": 0.95}`
  - `POST /mcp/localmodel.embed` → `{"embedding": [...], "privacy_tag": "..."}`
  - `GET /mcp/localmodel.health` → `{"status": "healthy", "load": 0.75}`
- **Protocol**: FastAPI + JSON.

## Example Usage
```python
from local_model.engine import LocalModelEngine
from local_model.models import CompletionRequest

engine = LocalModelEngine(model_path="/models/phi-3-q4.gguf")

# Complete text
response = engine.complete(CompletionRequest(
    prompt="Play a YouTube video of",
    max_tokens=50,
    privacy_tags=["CAP_MIC"]
))
print(response.text)  # Output: "rick astley never gonna give you up"
print(response.privacy_tag)  # Output: "CAP_MIC"

# Generate embeddings
embedding = engine.embed(EmbeddingRequest(text="Play YouTube"))
print(embedding.embedding)  # Output: [0.1, -0.3, ...]
```

## Project Structure
```
local-model/
├── pyproject.toml       # Poetry config + dependencies
├── local_model/
│   ├── __init__.py
│   ├── engine.py        # llama.cpp integration
│   ├── mcp_server.py    # FastAPI MCP interface
│   ├── models.py        # CompletionRequest, EmbeddingRequest
│   ├── privacy.py       # Privacy tagging logic
│   └── health.py        # Health reporting
├── docs/
│   └── spec.md          # Spec
└── tests/
    ├── test_engine.py
    └── test_mcp_server.py
```