from enum import Enum
from typing import Any, List, Optional
from pydantic import BaseModel


class PatchOpType(str, Enum):
    SET = "="
    INCREMENT = "+"
    DECREMENT = "-"
    TOGGLE = "~"


class PatchOp(BaseModel):
    path: str
    op: PatchOpType
    value: Any


class WatchRequest(BaseModel):
    path_prefix: str
    since_revision: Optional[int] = None


class StateResponse(BaseModel):
    value: Any
    revision: int