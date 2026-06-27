import os
from pydantic_settings import BaseSettings

class Settings(BaseSettings):
    PROJECT_NAME: str = "Sumi - VN Replay Trading Lab"
    VERSION: str = "0.1.0"
    API_V1_STR: str = "/api/v1"
    
    # SQLite Connection URL for local testing without installing Postgres
    DATABASE_URL: str = "sqlite:///./sumi.db"

settings = Settings()
