"""Backend root pytest conftest.

Installs the fail-closed database safeguard for all backend test suites and
locks the immutable session-level baseline SHA-256 integrity before executing
any tests, asserting zero delta upon session completion.
"""

from __future__ import annotations

import sys
from pathlib import Path
import pytest

BACKEND_DIR = Path(__file__).resolve().parent
if str(BACKEND_DIR) not in sys.path:
    sys.path.insert(0, str(BACKEND_DIR))

from tests_safety import (
    PROD_DB_BASELINE_SHA256,
    PROD_DB_BASELINE_SIZE,
    capture_production_database_baseline,
    install_database_safeguard,
    is_session_baseline_locked,
    verify_production_database_integrity,
)

# Install immediately upon module load
install_database_safeguard()


def pytest_configure(config: pytest.Config) -> None:
    """Ensure safeguard is installed as early as possible during pytest configuration."""
    install_database_safeguard()


def pytest_sessionstart(session: pytest.Session) -> None:
    """Capture dynamic immutable session-level baseline of backend/sumi.db prior to executing any test."""
    install_database_safeguard()
    capture_production_database_baseline()


def pytest_sessionfinish(session: pytest.Session, exitstatus: int) -> None:
    """Verify zero-delta database integrity against locked session baseline upon test session completion."""
    is_valid, sha256, size = verify_production_database_integrity()
    if not is_valid:
        raise RuntimeError(
            f"FAIL-CLOSED SAFEGUARD: Production database integrity violation detected at session finish! "
            f"Database was modified during test execution! "
            f"Final SHA-256: {sha256}, Size: {size} bytes."
        )


@pytest.fixture(scope="session", autouse=True)
def enforce_database_isolation() -> None:
    """Session-level autouse fixture ensuring safeguard is active and verifying zero delta after all tests."""
    install_database_safeguard()
    capture_production_database_baseline()
    yield
    is_valid, sha256, size = verify_production_database_integrity()
    assert is_valid, (
        f"Production database integrity violated after test execution! "
        f"Database was modified during test execution: Final SHA-256={sha256}, Size={size}"
    )
