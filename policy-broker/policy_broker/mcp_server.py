from fastapi import FastAPI, HTTPException
from policy_broker.models import PolicyDoc, CheckRequest, CheckResponse, AuditEntry, Rule
from policy_broker.interpreter import PolicyInterpreter
from policy_broker.state_store import StateStoreClient
from typing import Dict, Optional, List

app = FastAPI(title="L2Policy Broker")

state_store = StateStoreClient()
interpreter = PolicyInterpreter(state_store)


def load_default_policies() -> None:
    """Load default policies into the interpreter."""
    default_policies = PolicyDoc(
        rules=[
            Rule(
                path="ui.*",
                method="*",
                decision="ALLOW",
                capabilities=["CAP_STATE_READ"]
            ),
            Rule(
                path="policy.*",
                method="*",
                decision="DENY",
                capabilities=["CAP_STATE_WRITE"]
            ),
            Rule(
                path="*",
                method="*",
                decision="ALLOW",
                capabilities=["CAP_STATE_READ", "CAP_STATE_WRITE", "CAP_EVENT_PUBLISH", "CAP_TIMER", "CAP_EVENT_ADMIN"]
            )
        ]
    )
    interpreter.register(default_policies)


load_default_policies()


@app.post("/policy/check", response_model=CheckResponse)
async def policy_check(request: CheckRequest) -> CheckResponse:
    """Universal decision entrypoint for MCP calls."""
    return interpreter.check(request)


@app.post("/policy/register", status_code=204)
async def policy_register(policy_doc: PolicyDoc) -> None:
    """Load or update a policy document."""
    interpreter.register(policy_doc)


@app.get("/policy/confirm_result/{correlation_id}", response_model=CheckResponse)
async def policy_confirm_result(correlation_id: str) -> CheckResponse:
    """Poll or await the outcome of a CONFIRM/HOLD decision."""
    # TODO: Implement confirmation surface logic
    raise HTTPException(status_code=501, detail="Not implemented")


@app.post("/policy/audit_query", response_model=List[AuditEntry])
async def policy_audit_query(filter: Dict) -> List[AuditEntry]:
    """Query the audit log with a filter."""
    return interpreter.audit_logger.query(filter)