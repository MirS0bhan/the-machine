from pydantic import BaseModel
from typing import List, Optional, Literal, Dict
from datetime import datetime

DecisionType = Literal["ALLOW", "DENY", "CONFIRM", "HOLD"]


class RateLimit(BaseModel):
    count: int
    window: int  # in seconds


class Rule(BaseModel):
    path: str
    method: str
    decision: DecisionType
    capabilities: List[str]
    rate_limit: Optional[RateLimit] = None


class PolicyDoc(BaseModel):
    rules: List[Rule]


class CheckRequest(BaseModel):
    capability: str
    path: Optional[str] = None
    principal: Optional[str] = None
    method: Optional[str] = None
    request: Optional[Dict] = None
    provenance: Optional[str] = None


class CheckResponse(BaseModel):
    decision: DecisionType
    correlation_id: Optional[str] = None
    message: Optional[str] = None


class AuditEntry(BaseModel):
    timestamp: datetime
    method: str
    request: Dict
    provenance: str
    decision: DecisionType
    correlation_id: Optional[str] = None