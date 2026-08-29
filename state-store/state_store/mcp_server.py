import asyncio
import json
import os
from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import StreamingResponse
from typing import AsyncGenerator, List, Optional
from .models import PatchOp, WatchRequest, StateResponse
from .policy import policy_check, CAP_STATE_READ, CAP_STATE_WRITE


def _create_backend():
    backend = os.environ.get("STATE_STORE_BACKEND", "auto")
    db_path = os.environ.get("STATE_STORE_PATH", "/var/lib/state-store")
    if backend == "memory":
        from .memory_backend import MemoryBackend
        return MemoryBackend(db_path)
    try:
        from .rocksdb_backend import RocksDBBackend
        return RocksDBBackend(db_path)
    except Exception:
        from .memory_backend import MemoryBackend
        return MemoryBackend(db_path)


app = FastAPI(title="L1State Store MCP Server")
db = _create_backend()


@app.get("/state.get")
@policy_check(CAP_STATE_READ)
async def state_get(path: str) -> StateResponse:
    """Retrieve the value at `path`."""
    value = db.get(path)
    if value is None:
        raise HTTPException(status_code=404, detail="Path not found")
    return value


@app.post("/state.patch")
@policy_check(CAP_STATE_WRITE)
async def state_patch(ops: List[PatchOp]) -> dict:
    """Apply a list of patch operations."""
    results = db.patch(ops)
    return {"results": results}


@app.get("/state.watch")
@policy_check(CAP_STATE_READ)
async def state_watch(path_prefix: str, since_revision: Optional[int] = None) -> StreamingResponse:
    """Subscribe to changes for paths matching `path_prefix` using SSE."""
    async def event_stream() -> AsyncGenerator[str, None]:
        last_revision = since_revision or 0
        while True:
            # Simulate watching for changes (in a real implementation, this would use RocksDB's change feed)
            current = db.get(path_prefix)
            if current and current.revision > last_revision:
                yield f"data: {json.dumps(current.model_dump())}\n\n"
                last_revision = current.revision
            await asyncio.sleep(0.1)  # Polling interval

    return StreamingResponse(event_stream(), media_type="text/event-stream")


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)