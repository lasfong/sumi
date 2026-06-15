import os
from pydantic_settings import BaseSettings

class Settings(BaseSettings):
    PROJECT_NAME: str = "Sumi - VN Replay Trading Lab"
    VERSION: str = "0.1.0"
    API_V1_STR: str = "/api/v1"
    
    # Use absolute path for SQLite to avoid issues when running from different directories
    BASE_DIR: str = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    DATA_DIR: str = os.path.join(BASE_DIR, "data")
    SQLITE_URL: str = f"sqlite:///{os.path.join(DATA_DIR, 'market.db')}"

settings = Settings()

# Ensure data directory exists
os.makedirs(settings.DATA_DIR, exist_ok=True)
