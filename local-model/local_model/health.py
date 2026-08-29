# local-model/local_model/health.py
from local_model.models import HealthResponse


def get_health_status() -> HealthResponse:
    """Stub for health reporting. Feed Event Bus (category: health)."""
    return HealthResponse(status="healthy", load=0.75)