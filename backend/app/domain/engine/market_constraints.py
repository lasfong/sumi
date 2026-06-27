from typing import Dict

class MarketConstraints:
    # Biên độ giá theo sàn (Price Limits)
    EXCHANGE_LIMITS = {
        'HOSE': 0.07,   # 7%
        'HNX': 0.10,    # 10%
        'UPCOM': 0.15   # 15%
    }

    @staticmethod
    def get_price_limits(exchange: str, reference_price: float) -> tuple[float, float]:
        """
        Tính toán giá trần, sàn dựa trên giá tham chiếu và biên độ sàn.
        """
        limit_pct = MarketConstraints.EXCHANGE_LIMITS.get(exchange.upper(), 0.07) # Mặc định HOSE
        
        ceiling_price = reference_price * (1 + limit_pct)
        floor_price = reference_price * (1 - limit_pct)
        
        # Ở Việt Nam giá làm tròn theo tick size, ta dùng round đơn giản ở đây
        return round(floor_price, 2), round(ceiling_price, 2)

    @staticmethod
    def is_price_within_limits(price: float, exchange: str, reference_price: float) -> bool:
        """Kiểm tra xem giá đặt có nằm trong biên độ trần sàn hay không"""
        if reference_price <= 0:
            return True
        floor, ceiling = MarketConstraints.get_price_limits(exchange, reference_price)
        return floor <= price <= ceiling

    @staticmethod
    def calculate_t_plus_2_ready(current_index: int, entry_index: int) -> bool:
        """
        Kiểm tra xem cổ phiếu đã về tài khoản theo luật T+2 chưa.
        Với dữ liệu nến (candles) đã loại bỏ ngày nghỉ lễ, ta chỉ cần đếm số phiên (index).
        """
        return current_index >= entry_index + 2
