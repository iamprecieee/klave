"""KLAVE Python SDK exceptions.

Hierarchy:
    KlaveError
    ├── KlaveApiError          — non-2xx response
    │   └── PolicyViolationError — HTTP 403 from policy engine
    └── KlaveConnectionError   — network / timeout
"""


class KlaveError(Exception):
    """Base exception for all KLAVE SDK errors."""


class KlaveApiError(KlaveError):
    def __init__(self, status_code: int, message: str) -> None:
        self.status_code = status_code
        self.message = message
        super().__init__(f"[{status_code}] {message}")


class PolicyViolationError(KlaveApiError):
    def __init__(self, message: str) -> None:
        super().__init__(status_code=403, message=message)


class KlaveConnectionError(KlaveError):
    def __init__(self, detail: str) -> None:
        self.detail = detail
        super().__init__(f"connection error: {detail}")
