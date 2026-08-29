# local-model/local_model/engine.py
from llama_cpp import Llama
from typing import Optional
from local_model.models import (
    CompletionRequest,
    CompletionResponse,
    EmbeddingRequest,
    EmbeddingResponse,
    IntentRequest,
    IntentResponse,
)
from local_model.privacy import get_privacy_tag


class LocalModelEngine:
    def __init__(self, model_path: str):
        self.llm = Llama(model_path=model_path, n_ctx=4096, n_threads=4)

    def complete(self, request: CompletionRequest) -> CompletionResponse:
        output = self.llm(
            prompt=request.prompt,
            max_tokens=request.max_tokens,
            temperature=request.temperature,
        )
        privacy_tag = get_privacy_tag(request.privacy_tags)
        return CompletionResponse(
            text=output["choices"][0]["text"].strip(),
            privacy_tag=privacy_tag,
        )

    def embed(self, request: EmbeddingRequest) -> EmbeddingResponse:
        embedding = self.llm.create_embedding(input=request.text)["data"][0]["embedding"]
        privacy_tag = get_privacy_tag(request.privacy_tags)
        return EmbeddingResponse(
            embedding=embedding,
            privacy_tag=privacy_tag,
        )

    def classify_intent(self, request: IntentRequest) -> IntentResponse:
        # Stub for intent classification
        privacy_tag = get_privacy_tag(request.privacy_tags)
        return IntentResponse(
            intent="media.play",
            confidence=0.95,
            privacy_tag=privacy_tag,
        )