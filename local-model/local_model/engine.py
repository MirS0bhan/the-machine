# local-model/local_model/engine.py
import os
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
    """Tier-A local inference engine. Uses llama.cpp when a model is present, stub otherwise."""

    def __init__(self, model_path: Optional[str] = None):
        self.model_path = model_path or os.environ.get(
            "LOCAL_MODEL_PATH", "/models/phi-3-q4.gguf"
        )
        self._stub = not os.path.isfile(self.model_path)
        self.llm = None
        if not self._stub:
            from llama_cpp import Llama
            self.llm = Llama(model_path=self.model_path, n_ctx=4096, n_threads=4)

    def complete(self, request: CompletionRequest) -> CompletionResponse:
        privacy_tag = get_privacy_tag(request.privacy_tags)
        if self._stub:
            return CompletionResponse(
                text=f"[stub] {request.prompt[:80]}",
                privacy_tag=privacy_tag,
            )
        output = self.llm(
            prompt=request.prompt,
            max_tokens=request.max_tokens,
            temperature=request.temperature,
        )
        return CompletionResponse(
            text=output["choices"][0]["text"].strip(),
            privacy_tag=privacy_tag,
        )

    def embed(self, request: EmbeddingRequest) -> EmbeddingResponse:
        privacy_tag = get_privacy_tag(request.privacy_tags)
        if self._stub:
            # Deterministic pseudo-embedding for tests.
            vec = [float(ord(c) % 97) / 97.0 for c in request.text[:16]]
            return EmbeddingResponse(embedding=vec, privacy_tag=privacy_tag)
        embedding = self.llm.create_embedding(input=request.text)["data"][0]["embedding"]
        return EmbeddingResponse(embedding=embedding, privacy_tag=privacy_tag)

    def classify_intent(self, request: IntentRequest) -> IntentResponse:
        privacy_tag = get_privacy_tag(request.privacy_tags)
        text = request.text.lower()
        if "video" in text or "play" in text or "watch" in text:
            intent = "media.play"
        elif "calc" in text or "math" in text:
            intent = "calc.eval"
        else:
            intent = "general.query"
        return IntentResponse(intent=intent, confidence=0.85, privacy_tag=privacy_tag)
