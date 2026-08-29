# local-model/local_model/models.py
from pydantic import BaseModel
from typing import List, Optional


class CompletionRequest(BaseModel):
    prompt: str
    max_tokens: int = 50
    temperature: float = 0.7
    privacy_tags: Optional[List[str]] = None


class CompletionResponse(BaseModel):
    text: str
    privacy_tag: Optional[str] = None


class EmbeddingRequest(BaseModel):
    text: str
    privacy_tags: Optional[List[str]] = None


class EmbeddingResponse(BaseModel):
    embedding: List[float]
    privacy_tag: Optional[str] = None


class IntentRequest(BaseModel):
    text: str
    privacy_tags: Optional[List[str]] = None


class IntentResponse(BaseModel):
    intent: str
    confidence: float
    privacy_tag: Optional[str] = None


class HealthResponse(BaseModel):
    status: str
    load: float