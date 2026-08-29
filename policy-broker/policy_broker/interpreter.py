from typing import List, Optional, Dict
from policy_broker.models import PolicyDoc, Rule, CheckRequest, CheckResponse, DecisionType
from policy_broker.audit import AuditLogger
from policy_broker.state_store import StateStoreClient
import fnmatch
import time


class PolicyInterpreter:
    """
    Rule interpreter for policy evaluation.
    """

    def __init__(self, state_store_client: StateStoreClient):
        self.state_store = state_store_client
        self.audit_logger = AuditLogger(state_store_client)
        self.policies: Dict[str, PolicyDoc] = {}
        self.rate_limits: Dict[str, Dict[str, List[float]]] = {}

    def register(self, policy_doc: PolicyDoc, policy_key: str = "default") -> None:
        """Register or update a policy document."""
        self.policies[policy_key] = policy_doc
        self.state_store.put_policy(policy_key, policy_doc)

    def _match_rule(self, rule: Rule, capability: str, path: Optional[str] = None) -> bool:
        """Check if a rule matches the capability and path."""
        if capability not in rule.capabilities:
            return False
        if rule.path != "*" and path and not fnmatch.fnmatch(path, rule.path):
            return False
        return True

    def _check_rate_limit(self, rule: Rule, provenance: str) -> bool:
        """Check if the rate limit is exceeded for the given rule and provenance."""
        if rule.rate_limit is None:
            return True

        key = f"{rule.path}:{provenance}"
        now = time.time()
        window_start = now - rule.rate_limit.window

        if key not in self.rate_limits:
            self.rate_limits[key] = {"timestamps": []}

        timestamps = [t for t in self.rate_limits[key]["timestamps"] if t >= window_start]
        self.rate_limits[key]["timestamps"] = timestamps

        if len(timestamps) >= rule.rate_limit.count:
            return False

        self.rate_limits[key]["timestamps"].append(now)
        return True

    def _detect_anomaly(self, capability: str, path: Optional[str], principal: Optional[str]) -> bool:
        """Detect unusual capability combinations or other anomalies."""
        # TODO: Implement anomaly detection logic
        return False

    def check(self, request: CheckRequest) -> CheckResponse:
        """Evaluate a request against all registered policies."""
        capability = request.capability
        path = request.path
        principal = request.principal

        if self._detect_anomaly(capability, path, principal):
            decision = "DENY"
            self.audit_logger.log(capability, {"path": path, "principal": principal}, principal or "unknown", decision)
            return CheckResponse(decision=decision, message="Anomaly detected")

        for policy_key, policy_doc in self.policies.items():
            for rule in policy_doc.rules:
                if self._match_rule(rule, capability, path):
                    if not self._check_rate_limit(rule, principal or "unknown"):
                        decision = "DENY"
                        self.audit_logger.log(capability, {"path": path, "principal": principal}, principal or "unknown", decision)
                        return CheckResponse(decision=decision, message="Rate limit exceeded")

                    decision = rule.decision
                    correlation_id = None

                    if decision in ["CONFIRM", "HOLD"]:
                        correlation_id = f"{capability}:{principal or 'unknown'}:{time.time()}"

                    self.audit_logger.log(capability, {"path": path, "principal": principal}, principal or "unknown", decision, correlation_id)
                    return CheckResponse(decision=decision, correlation_id=correlation_id)

        # Default decision: DENY
        self.audit_logger.log(capability, {"path": path, "principal": principal}, principal or "unknown", "DENY")
        return CheckResponse(decision="DENY")