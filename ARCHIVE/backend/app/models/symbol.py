from sqlalchemy import Column, String, Boolean
from app.db import Base

class Symbol(Base):
    __tablename__ = "symbols"

    symbol = Column(String, primary_key=True, index=True)
    exchange = Column(String, nullable=True)
    company_name = Column(String, nullable=True)
    sector = Column(String, nullable=True)
    industry = Column(String, nullable=True)
    asset_type = Column(String, default="stock")
    is_active = Column(Boolean, default=True)
