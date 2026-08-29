# local-model/local_model/mcp_server.py
from fastapi import FastAPI, HTTPException
from local_model.engine import LocalModelEngine
from local_model.models import (
    CompletionRequest,
    CompletionResponse,
    EmbeddingRequest,
    EmbeddingResponse,
    IntentRequest,
    IntentResponse,
    HealthResponse,
)
from local_model.health import get_health_status

app = FastAPI()
engine = LocalModelEngine(model_path="/models/phi-3-q4.gguf")


@app.post("/mcp/localmodel.complete", response_model=CompletionResponse)
async def complete(request: CompletionRequest) -> CompletionResponse:
    return engine.complete(request)


@app.post("/mcp/localmodel.embed", response_model=EmbeddingResponse)
async def embed(request: EmbeddingRequest) -> EmbeddingResponse:
    return engine.embed(request)


@app.post("/mcp/localmodel.classify_intent", response_model=IntentResponse)
async def classify_intent(request: IntentRequest) -> IntentResponse:
    return engine.classify_intent(request)


@app.get("/mcp/localmodel.health", response_model=HealthResponse)
async def health() -> HealthResponse:
    return get_health_status()