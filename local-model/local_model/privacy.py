# local-model/local_model/privacy.py
from typing import List, Optional

SENSITIVE_CAPS = {"CAP_MIC", "CAP_CAMERA", "CAP_FS_READ"}


def get_privacy_tag(privacy_tags: Optional[List[str]]) -> Optional[str]:
    """Stamp outputs with privacy_tag if input touched sensitive capabilities."""
    if not privacy_tags:
        return None
    for tag in privacy_tags:
        if tag in SENSITIVE_CAPS:
            return tag
    return None