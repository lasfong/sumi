class MarketConstraints:
    EXCHANGE_LIMITS = {
        "HOSE": 0.07,   # ±7%
        "HNX": 0.10,    # ±10%
        "UPCOM": 0.15,  # ±15%
    }

    @classmethod
    def get_price_limits(cls, reference_price: float, exchange: str) -> tuple:
        limit_pct = cls.EXCHANGE_LIMITS.get(exchange, 0.07)
        ceiling = reference_price * (1 + limit_pct)
        floor = reference_price * (1 - limit_pct)
        return (floor, ceiling)

    @classmethod
    def is_price_within_limits(cls, price: float, exchange: str, reference_price: float) -> bool:
        floor, ceiling = cls.get_price_limits(reference_price, exchange)
        return floor <= price <= ceiling
