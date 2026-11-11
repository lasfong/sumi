import os
from pathlib import Path
from pydantic import field_validator
from pydantic_settings import BaseSettings
from sqlalchemy.engine import make_url

BACKEND_DIR = Path(__file__).resolve().parent.parent
DEFAULT_SQLITE_PATH = (BACKEND_DIR / "sumi.db").resolve()
DEFAULT_DATABASE_URL = f"sqlite:///{DEFAULT_SQLITE_PATH.as_posix()}"


def verify_database_path_security(database_url: str | None = None) -> None:
    """Validate that the SQLite database path is secure, absolute, and not vulnerable to CWD drift.

    Raises RuntimeError if an unanchored relative SQLite database path is detected.
    """
    url_to_check = database_url if database_url is not None else settings.DATABASE_URL
    if not url_to_check:
        raise ValueError("Database URL cannot be empty")
    try:
        url = make_url(url_to_check)
    except Exception as e:
        raise ValueError(f"Invalid database URL: {url_to_check}") from e

    if url.drivername.startswith("sqlite") and url.database:
        # Ignore in-memory SQLite instances
        if url.database == ":memory:" or url.database == "":
            return
        db_path = Path(url.database)
        if not db_path.is_absolute():
            raise RuntimeError(
                f"Insecure relative SQLite database path detected: '{url.database}'. "
                f"Database path must be absolute to prevent rogue database file creation across CWDs."
            )


class Settings(BaseSettings):
    PROJECT_NAME: str = "Sumi - VN Replay Trading Lab"
    VERSION: str = "2.0.0-rc2"
    API_V1_STR: str = "/api/v1"

    # SQLite Connection URL anchored to backend/sumi.db
    DATABASE_URL: str = DEFAULT_DATABASE_URL
    DEBUG: bool = False
    AUTO_CREATE_TABLES: bool = False
    CORS_ALLOWED_ORIGINS: str = "http://localhost:5173,http://127.0.0.1:5173"

    # Synthetic provider policy (GAP-DB-04, ADR-PROV-001)
    SUMI_ALLOW_SYNTHETIC_PROVIDERS: bool = False

    # Provider credentials (ADR-PROV-001 centralized settings)
    SUMI_DATA_PROVIDER_KEY: str | None = None
    SUMI_SSI_CONSUMER_ID: str | None = None
    SUMI_SSI_CONSUMER_SECRET: str | None = None

    @field_validator("DATABASE_URL", mode="after")
    @classmethod
    def normalize_database_url(cls, v: str) -> str:
        if not v or not v.strip():
            return DEFAULT_DATABASE_URL
        try:
            url = make_url(v)
        except Exception:
            return v
        if url.drivername.startswith("sqlite") and url.database:
            if url.database == ":memory:" or url.database == "":
                return v
            db_path = Path(url.database)
            if not db_path.is_absolute():
                resolved = (BACKEND_DIR / db_path).resolve()
                return url.set(database=resolved.as_posix()).render_as_string(hide_password=False)
        return v

    @property
    def cors_allowed_origins_list(self) -> list[str]:
        return [
            origin.strip()
            for origin in self.CORS_ALLOWED_ORIGINS.split(",")
            if origin.strip()
        ]


settings = Settings()
