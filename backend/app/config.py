import os
from pydantic_settings import BaseSettings

class Settings(BaseSettings):
    PROJECT_NAME: str = "Sumi - VN Replay Trading Lab"
    VERSION: str = "0.1.0"
    API_V1_STR: str = "/api/v1"
    
    # SQLite Connection URL for local testing without installing Postgres
    DATABASE_URL: str = "sqlite:///./sumi.db"
    DEBUG: bool = False
    AUTO_CREATE_TABLES: bool = False
    CORS_ALLOWED_ORIGINS: str = "http://localhost:5173,http://127.0.0.1:5173"

    @property
    def cors_allowed_origins_list(self) -> list[str]:
        return [
            origin.strip()
            for origin in self.CORS_ALLOWED_ORIGINS.split(",")
            if origin.strip()
        ]

settings = Settings()
