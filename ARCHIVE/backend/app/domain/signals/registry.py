"""Signal Registry providing definitions, parameter resolution, and calculation dispatch.

Pure domain registry with no dependencies on FastAPI or database sessions.
"""

from typing import Any, Dict, List, Optional

from app.domain.signals.models import (
    CandleBar,
    SignalDefinition,
    SignalOutputType,
    SignalSeriesResult,
    SignalStatus,
    compute_canonical_params_hash,
)
from app.domain.signals.patterns import (
    calculate_any_bearish_pattern,
    calculate_any_bullish_pattern,
    calculate_bearish_engulfing,
    calculate_bullish_engulfing,
    calculate_dark_cloud_cover,
    calculate_evening_star,
    calculate_hammer,
    calculate_inside_bar_breakout_down,
    calculate_inside_bar_breakout_up,
    calculate_morning_star,
    calculate_piercing_line,
    calculate_shooting_star,
    calculate_tweezer_bottom,
    calculate_tweezer_top,
)
from app.domain.signals.regimes import (
    calculate_downtrend_regime,
    calculate_new_high,
    calculate_new_low,
    calculate_pullback_regime,
    calculate_recovery_regime,
    calculate_sideways_regime,
    calculate_uptrend_regime,
)
from app.domain.signals.support_resistance import (
    calculate_near_resistance,
    calculate_near_support,
)
from app.domain.signals.structure import calculate_candle_structure_score
from app.domain.signals.technical import (
    calculate_composite_technical_trigger,
    calculate_ema_cross,
    calculate_macd_signal_cross,
    calculate_macd_zero_cross,
    calculate_rsi_level_cross,
    calculate_swing_break,
)
from app.domain.signals.volume import (
    calculate_relative_volume,
    calculate_volume_climax_down,
    calculate_volume_climax_up,
    calculate_volume_spike,
    validate_relative_volume_params,
    validate_volume_spike_params,
)
from app.domain.signals.vsa import (
    calculate_spring,
    calculate_strong_demand,
    calculate_strong_demand_at_support,
    calculate_strong_supply_at_resistance,
    calculate_upthrust,
    calculate_weak_demand,
)
from app.domain.signals.ichimoku import (
    calculate_ichimoku_score,
    calculate_ichimoku_bullish,
    calculate_ichimoku_bearish,
    calculate_ichimoku_tk_cross_bullish,
    calculate_ichimoku_tk_cross_bearish,
    calculate_ichimoku_kumo_breakout_bullish,
    calculate_ichimoku_kumo_breakout_bearish,
)
from app.domain.signals.divergence import (
    calculate_confirmed_pivot_high,
    calculate_confirmed_pivot_low,
    calculate_rsi_regular_bullish,
    calculate_rsi_regular_bearish,
    calculate_rsi_hidden_bullish,
    calculate_rsi_hidden_bearish,
    calculate_macd_regular_bullish,
    calculate_macd_regular_bearish,
    calculate_macd_hidden_bullish,
    calculate_macd_hidden_bearish,
    calculate_stoch_regular_bullish,
    calculate_stoch_regular_bearish,
    calculate_stoch_hidden_bullish,
    calculate_stoch_hidden_bearish,
)
from app.domain.signals.health import (
    calculate_technical_health_score,
    calculate_technical_health_favorable,
    calculate_technical_health_unfavorable,
)
from app.domain.signals.flow_events import (
    calculate_bb_direction_rising,
    calculate_bb_direction_falling,
    calculate_bb_regime_positive,
    calculate_bb_regime_negative,
    calculate_bb_confluence_bullish,
    calculate_bb_confluence_bearish,
    calculate_bb_turn_up,
    calculate_bb_turn_down,
)




class SignalRegistry:
    """Authoritative in-memory registry of supported signals."""

    _DEFINITIONS: Dict[str, SignalDefinition] = {
        # Volume
        "volume.relative_volume": SignalDefinition(
            name="volume.relative_volume",
            version="1.0.0",
            category="volume",
            label_vi="Khối lượng tương đối (RVOL)",
            description="Tỷ lệ giữa khối lượng nến hiện tại so với trung bình N nến trước đó (loại trừ nến hiện tại).",
            output_type=SignalOutputType.FLOAT,
            parameters_schema={
                "period": {
                    "type": "int",
                    "default": 20,
                    "minimum": 1,
                    "maximum": 252,
                    "description": "Số phiên tính trung bình khối lượng",
                }
            },
            default_parameters={"period": 20},
            dependencies=[],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="volume__relative_volume",
        ),
        "volume.spike": SignalDefinition(
            name="volume.spike",
            version="1.0.0",
            category="volume",
            label_vi="Đột biến khối lượng (Volume Spike)",
            description="Xác định thanh nến có khối lượng tương đối RVOL lớn hơn hoặc bằng ngưỡng multiplier.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "period": {
                    "type": "int",
                    "default": 20,
                    "minimum": 1,
                    "maximum": 252,
                    "description": "Số phiên tính trung bình khối lượng",
                },
                "multiplier": {
                    "type": "float",
                    "default": 2.0,
                    "exclusiveMinimum": 0,
                    "maximum": 100,
                    "description": "Hệ số nhân ngưỡng đột biến",
                },
            },
            default_parameters={"period": 20, "multiplier": 2.0},
            dependencies=["volume.relative_volume"],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="volume__spike",
        ),
        "volume.climax_up": SignalDefinition(
            name="volume.climax_up",
            version="1.0.0",
            category="volume",
            label_vi="Cao trào khối lượng tăng (Volume Climax Up)",
            description="Thanh nến biên độ rộng với khối lượng tương đối cao đóng cửa ở vùng 25% cao nhất của phiên.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "period": {"type": "int", "default": 20, "minimum": 1, "maximum": 252, "description": "Chu kỳ tính RVOL"},
                "multiplier": {"type": "float", "default": 2.0, "minimum": 0.1, "maximum": 100.0, "description": "Ngưỡng RVOL tối thiểu"},
                "min_range_atr": {"type": "float", "default": 1.5, "minimum": 0.1, "maximum": 10.0, "description": "Ngưỡng Biên độ / ATR tối thiểu"},
                "min_close_location": {"type": "float", "default": 0.75, "minimum": 0.0, "maximum": 1.0, "description": "Vị trí đóng cửa tối thiểu"},
            },
            default_parameters={"period": 20, "multiplier": 2.0, "min_range_atr": 1.5, "min_close_location": 0.75},
            dependencies=["volume.relative_volume"],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="volume__climax_up",
        ),
        "volume.climax_down": SignalDefinition(
            name="volume.climax_down",
            version="1.0.0",
            category="volume",
            label_vi="Cao trào khối lượng giảm (Volume Climax Down)",
            description="Thanh nến biên độ rộng với khối lượng tương đối cao đóng cửa ở vùng 25% thấp nhất của phiên.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "period": {"type": "int", "default": 20, "minimum": 1, "maximum": 252, "description": "Chu kỳ tính RVOL"},
                "multiplier": {"type": "float", "default": 2.0, "minimum": 0.1, "maximum": 100.0, "description": "Ngưỡng RVOL tối thiểu"},
                "min_range_atr": {"type": "float", "default": 1.5, "minimum": 0.1, "maximum": 10.0, "description": "Ngưỡng Biên độ / ATR tối thiểu"},
                "max_close_location": {"type": "float", "default": 0.25, "minimum": 0.0, "maximum": 1.0, "description": "Vị trí đóng cửa tối đa"},
            },
            default_parameters={"period": 20, "multiplier": 2.0, "min_range_atr": 1.5, "max_close_location": 0.25},
            dependencies=["volume.relative_volume"],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="volume__climax_down",
        ),

        # Candlestick Patterns
        "pattern.bullish_engulfing": SignalDefinition(
            name="pattern.bullish_engulfing",
            version="1.0.0",
            category="pattern",
            label_vi="Nhấn chìm tăng (Bullish Engulfing)",
            description="Mô hình nến nhấn chìm tăng trưởng với bộ lọc chất lượng thân nến và vị trí đóng nến.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "min_body_ratio": {"type": "float", "default": 0.55, "minimum": 0.1, "maximum": 1.0, "description": "Tỷ lệ thân nến tối thiểu"},
                "min_close_location": {"type": "float", "default": 0.70, "minimum": 0.5, "maximum": 1.0, "description": "Vị trí đóng nến tối thiểu"},
            },
            default_parameters={"min_body_ratio": 0.55, "min_close_location": 0.70},
            dependencies=[],
            warmup_bars=1,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="pattern__bullish_engulfing",
        ),
        "pattern.bearish_engulfing": SignalDefinition(
            name="pattern.bearish_engulfing",
            version="1.0.0",
            category="pattern",
            label_vi="Nhấn chìm giảm (Bearish Engulfing)",
            description="Mô hình nến nhấn chìm giảm với bộ lọc chất lượng thân nến và vị trí đóng nến.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "min_body_ratio": {"type": "float", "default": 0.55, "minimum": 0.1, "maximum": 1.0, "description": "Tỷ lệ thân nến tối thiểu"},
                "min_close_location": {"type": "float", "default": 0.70, "minimum": 0.5, "maximum": 1.0, "description": "Ngưỡng vị trí đóng nến đối xứng"},
            },
            default_parameters={"min_body_ratio": 0.55, "min_close_location": 0.70},
            dependencies=[],
            warmup_bars=1,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="pattern__bearish_engulfing",
        ),
        "pattern.hammer": SignalDefinition(
            name="pattern.hammer",
            version="1.0.0",
            category="pattern",
            label_vi="Nến Búa / Pinbar Tăng (Hammer)",
            description="Nến búa có bóng dưới dài tối thiểu 2 lần thân nến và vị trí đóng nến cao.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "wick_multiplier": {"type": "float", "default": 2.0, "minimum": 1.0, "maximum": 5.0, "description": "Hệ số bóng nến dưới so với thân"},
            },
            default_parameters={"wick_multiplier": 2.0},
            dependencies=[],
            warmup_bars=0,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="pattern__hammer",
        ),
        "pattern.shooting_star": SignalDefinition(
            name="pattern.shooting_star",
            version="1.0.0",
            category="pattern",
            label_vi="Nến Sao Băng / Pinbar Giảm (Shooting Star)",
            description="Nến sao băng có bóng trên dài tối thiểu 2 lần thân nến và vị trí đóng nến thấp.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "wick_multiplier": {"type": "float", "default": 2.0, "minimum": 1.0, "maximum": 5.0, "description": "Hệ số bóng nến trên so với thân"},
            },
            default_parameters={"wick_multiplier": 2.0},
            dependencies=[],
            warmup_bars=0,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="pattern__shooting_star",
        ),
        "pattern.morning_star": SignalDefinition(
            name="pattern.morning_star",
            version="1.0.0",
            category="pattern",
            label_vi="Sao Mai (Morning Star)",
            description="Mô hình đảo chiều tăng 3 nến.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={},
            default_parameters={},
            dependencies=[],
            warmup_bars=2,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="pattern__morning_star",
        ),
        "pattern.evening_star": SignalDefinition(
            name="pattern.evening_star",
            version="1.0.0",
            category="pattern",
            label_vi="Sao Hôm (Evening Star)",
            description="Mô hình đảo chiều giảm 3 nến.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={},
            default_parameters={},
            dependencies=[],
            warmup_bars=2,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="pattern__evening_star",
        ),
        "pattern.piercing_line": SignalDefinition(
            name="pattern.piercing_line",
            version="1.0.0",
            category="pattern",
            label_vi="Đường Xuyên Phá (Piercing Line)",
            description="Mô hình nến phục hồi vượt 50% thân nến giảm trước đó.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={},
            default_parameters={},
            dependencies=[],
            warmup_bars=1,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="pattern__piercing_line",
        ),
        "pattern.dark_cloud_cover": SignalDefinition(
            name="pattern.dark_cloud_cover",
            version="1.0.0",
            category="pattern",
            label_vi="Mây Đen Bao Phủ (Dark Cloud Cover)",
            description="Mô hình nến giảm lấn quá 50% thân nến tăng trước đó.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={},
            default_parameters={},
            dependencies=[],
            warmup_bars=1,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="pattern__dark_cloud_cover",
        ),
        "pattern.tweezer_bottom": SignalDefinition(
            name="pattern.tweezer_bottom",
            version="1.0.0",
            category="pattern",
            label_vi="Đáy Nhíp (Tweezer Bottom)",
            description="Hai đáy nến liên tiếp bằng nhau trong biên độ sai số ATR.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "tolerance_atr": {"type": "float", "default": 0.15, "minimum": 0.01, "maximum": 1.0, "description": "Dung sai ATR"},
            },
            default_parameters={"tolerance_atr": 0.15},
            dependencies=[],
            warmup_bars=14,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="pattern__tweezer_bottom",
        ),
        "pattern.tweezer_top": SignalDefinition(
            name="pattern.tweezer_top",
            version="1.0.0",
            category="pattern",
            label_vi="Đỉnh Nhíp (Tweezer Top)",
            description="Hai đỉnh nến liên tiếp bằng nhau trong biên độ sai số ATR.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "tolerance_atr": {"type": "float", "default": 0.15, "minimum": 0.01, "maximum": 1.0, "description": "Dung sai ATR"},
            },
            default_parameters={"tolerance_atr": 0.15},
            dependencies=[],
            warmup_bars=14,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="pattern__tweezer_top",
        ),
        "pattern.inside_bar_breakout_up": SignalDefinition(
            name="pattern.inside_bar_breakout_up",
            version="1.0.0",
            category="pattern",
            label_vi="Phá vỡ Inside Bar Tăng",
            description="Phá vỡ nến mẹ lên phía trên sau một nến con nằm trong thân mẹ.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "breakout_buffer_atr": {"type": "float", "default": 0.05, "minimum": 0.0, "maximum": 0.5, "description": "Đệm ATR phá vỡ"},
            },
            default_parameters={"breakout_buffer_atr": 0.05},
            dependencies=[],
            warmup_bars=14,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="pattern__inside_bar_breakout_up",
        ),
        "pattern.inside_bar_breakout_down": SignalDefinition(
            name="pattern.inside_bar_breakout_down",
            version="1.0.0",
            category="pattern",
            label_vi="Phá vỡ Inside Bar Giảm",
            description="Phá vỡ nến mẹ xuống phía dưới sau một nến con nằm trong thân mẹ.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "breakout_buffer_atr": {"type": "float", "default": 0.05, "minimum": 0.0, "maximum": 0.5, "description": "Đệm ATR phá vỡ"},
            },
            default_parameters={"breakout_buffer_atr": 0.05},
            dependencies=[],
            warmup_bars=14,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="pattern__inside_bar_breakout_down",
        ),
        "pattern.any_bullish": SignalDefinition(
            name="pattern.any_bullish",
            version="1.0.0",
            category="pattern",
            label_vi="Bất kỳ mô hình nến tăng nào",
            description="Tập hợp mô hình tăng, trả về lý do tên mô hình khớp.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={},
            default_parameters={},
            dependencies=[],
            warmup_bars=14,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="pattern__any_bullish",
        ),
        "pattern.any_bearish": SignalDefinition(
            name="pattern.any_bearish",
            version="1.0.0",
            category="pattern",
            label_vi="Bất kỳ mô hình nến giảm nào",
            description="Tập hợp mô hình giảm, trả về lý do tên mô hình khớp.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={},
            default_parameters={},
            dependencies=[],
            warmup_bars=14,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="pattern__any_bearish",
        ),

        # Technical Triggers
        "tech.macd_signal_cross": SignalDefinition(
            name="tech.macd_signal_cross",
            version="1.0.0",
            category="technical",
            label_vi="Giao cắt MACD Signal",
            description="Đường MACD cắt qua đường Signal.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "fast": {"type": "int", "default": 12, "minimum": 2, "maximum": 100, "description": "Chu kỳ EMA nhanh"},
                "slow": {"type": "int", "default": 26, "minimum": 5, "maximum": 200, "description": "Chu kỳ EMA chậm"},
                "signal_period": {"type": "int", "default": 9, "minimum": 2, "maximum": 50, "description": "Chu kỳ đường Signal"},
            },
            default_parameters={"fast": 12, "slow": 26, "signal_period": 9},
            dependencies=[],
            warmup_bars=35,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="tech__macd_signal_cross",
        ),
        "tech.macd_zero_cross": SignalDefinition(
            name="tech.macd_zero_cross",
            version="1.0.0",
            category="technical",
            label_vi="Giao cắt MACD qua mức 0",
            description="Đường MACD cắt qua mức 0.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "fast": {"type": "int", "default": 12, "minimum": 2, "maximum": 100, "description": "Chu kỳ EMA nhanh"},
                "slow": {"type": "int", "default": 26, "minimum": 5, "maximum": 200, "description": "Chu kỳ EMA chậm"},
            },
            default_parameters={"fast": 12, "slow": 26},
            dependencies=[],
            warmup_bars=26,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="tech__macd_zero_cross",
        ),
        "tech.rsi_level_cross": SignalDefinition(
            name="tech.rsi_level_cross",
            version="1.0.0",
            category="technical",
            label_vi="Giao cắt mức RSI",
            description="RSI vượt qua ngưỡng cấu hình (30/50/70).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "length": {"type": "int", "default": 14, "minimum": 2, "maximum": 100, "description": "Chu kỳ RSI"},
                "level": {"type": "float", "default": 30.0, "minimum": 0.0, "maximum": 100.0, "description": "Mức ngưỡng RSI"},
            },
            default_parameters={"length": 14, "level": 30.0},
            dependencies=[],
            warmup_bars=15,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="tech__rsi_level_cross",
        ),
        "tech.ema_cross": SignalDefinition(
            name="tech.ema_cross",
            version="1.0.0",
            category="technical",
            label_vi="Giao cắt đường EMA",
            description="EMA nhanh cắt qua EMA chậm.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "fast_period": {"type": "int", "default": 20, "minimum": 2, "maximum": 100, "description": "Chu kỳ EMA nhanh"},
                "slow_period": {"type": "int", "default": 50, "minimum": 5, "maximum": 300, "description": "Chu kỳ EMA chậm"},
            },
            default_parameters={"fast_period": 20, "slow_period": 50},
            dependencies=[],
            warmup_bars=51,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="tech__ema_cross",
        ),
        "tech.swing_break": SignalDefinition(
            name="tech.swing_break",
            version="1.0.0",
            category="technical",
            label_vi="Phá vỡ đỉnh/đáy swing đã xác nhận",
            description="Phá vỡ đỉnh swing đã được xác nhận nhân quả.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "swing_window": {"type": "int", "default": 3, "minimum": 1, "maximum": 10, "description": "Biên độ nến swing"},
                "buffer_atr": {"type": "float", "default": 0.05, "minimum": 0.0, "maximum": 0.5, "description": "Đệm ATR phá vỡ"},
            },
            default_parameters={"swing_window": 3, "buffer_atr": 0.05},
            dependencies=[],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="tech__swing_break",
        ),
        "tech.composite": SignalDefinition(
            name="tech.composite",
            version="1.0.0",
            category="technical",
            label_vi="Bộ kích hoạt kỹ thuật tổng hợp",
            description="Kích hoạt khi có tối thiểu N tín hiệu kỹ thuật cùng xác nhận.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "minimum_confirmations": {"type": "int", "default": 2, "minimum": 1, "maximum": 5, "description": "Số xác nhận tối thiểu"},
            },
            default_parameters={"minimum_confirmations": 2},
            dependencies=[],
            warmup_bars=51,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="tech__composite",
        ),

        # Regimes
        "regime.uptrend": SignalDefinition(
            name="regime.uptrend",
            version="1.0.0",
            category="regime",
            label_vi="Chế độ Xu hướng Tăng",
            description="Thị trường trong trạng thái xu hướng tăng (Close > EMA20 > EMA50 và EMA20 dốc lên).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "fast_period": {"type": "int", "default": 20, "minimum": 5, "maximum": 100, "description": "EMA nhanh"},
                "slow_period": {"type": "int", "default": 50, "minimum": 10, "maximum": 200, "description": "EMA chậm"},
                "slope_lookback": {"type": "int", "default": 5, "minimum": 1, "maximum": 20, "description": "Độ trễ tính độ dốc"},
            },
            default_parameters={"fast_period": 20, "slow_period": 50, "slope_lookback": 5},
            dependencies=[],
            warmup_bars=55,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="regime__uptrend",
        ),
        "regime.downtrend": SignalDefinition(
            name="regime.downtrend",
            version="1.0.0",
            category="regime",
            label_vi="Chế độ Xu hướng Giảm",
            description="Thị trường trong trạng thái xu hướng giảm (Close < EMA20 < EMA50 và EMA20 dốc xuống).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "fast_period": {"type": "int", "default": 20, "minimum": 5, "maximum": 100, "description": "EMA nhanh"},
                "slow_period": {"type": "int", "default": 50, "minimum": 10, "maximum": 200, "description": "EMA chậm"},
                "slope_lookback": {"type": "int", "default": 5, "minimum": 1, "maximum": 20, "description": "Độ trễ tính độ dốc"},
            },
            default_parameters={"fast_period": 20, "slow_period": 50, "slope_lookback": 5},
            dependencies=[],
            warmup_bars=55,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="regime__downtrend",
        ),
        "regime.sideways": SignalDefinition(
            name="regime.sideways",
            version="1.0.0",
            category="regime",
            label_vi="Chế độ Đi ngang (Sideways)",
            description="Thị trường tích lũy đi ngang với độ dốc EMA gần như bằng 0.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "ema_period": {"type": "int", "default": 20, "minimum": 5, "maximum": 100, "description": "Chu kỳ EMA"},
                "slope_threshold": {"type": "float", "default": 0.008, "minimum": 0.001, "maximum": 0.05, "description": "Ngưỡng độ dốc tối đa"},
            },
            default_parameters={"ema_period": 20, "slope_threshold": 0.008},
            dependencies=[],
            warmup_bars=25,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="regime__sideways",
        ),
        "regime.pullback": SignalDefinition(
            name="regime.pullback",
            version="1.0.0",
            category="regime",
            label_vi="Chế độ Điều chỉnh (Pullback)",
            description="Nhịp điều chỉnh lành mạnh trong xu hướng tăng lớn.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "lookback": {"type": "int", "default": 20, "minimum": 5, "maximum": 100, "description": "Chu kỳ đỉnh trước"},
                "min_drawdown": {"type": "float", "default": 0.02, "minimum": 0.005, "maximum": 0.1, "description": "Độ sụt giảm tối thiểu"},
                "max_drawdown": {"type": "float", "default": 0.12, "minimum": 0.05, "maximum": 0.3, "description": "Độ sụt giảm tối đa"},
            },
            default_parameters={"lookback": 20, "min_drawdown": 0.02, "max_drawdown": 0.12},
            dependencies=[],
            warmup_bars=50,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="regime__pullback",
        ),
        "regime.recovery": SignalDefinition(
            name="regime.recovery",
            version="1.0.0",
            category="regime",
            label_vi="Chế độ Phục hồi Sớm",
            description="Trạng thái phục hồi sớm sau xu hướng giảm.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "lookback": {"type": "int", "default": 20, "minimum": 5, "maximum": 100, "description": "Chu kỳ đáy trước"},
            },
            default_parameters={"lookback": 20},
            dependencies=[],
            warmup_bars=50,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="regime__recovery",
        ),
        "regime.new_high": SignalDefinition(
            name="regime.new_high",
            version="1.0.0",
            category="regime",
            label_vi="Đỉnh cao mới (New High)",
            description="Giá vượt đỉnh cao nhất của N phiên trước đó (loại trừ phiên hiện tại theo TEST-CAUSAL-003).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "period": {"type": "int", "default": 20, "minimum": 1, "maximum": 252, "description": "Số phiên quá khứ"},
                "buffer": {"type": "float", "default": 0.0, "minimum": 0.0, "maximum": 100.0, "description": "Đệm giá vượt đỉnh"},
            },
            default_parameters={"period": 20, "buffer": 0.0},
            dependencies=[],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="regime__new_high",
        ),
        "regime.new_low": SignalDefinition(
            name="regime.new_low",
            version="1.0.0",
            category="regime",
            label_vi="Đáy thấp mới (New Low)",
            description="Giá xuyên thủng đáy thấp nhất của N phiên trước đó (loại trừ phiên hiện tại theo TEST-CAUSAL-003).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "period": {"type": "int", "default": 20, "minimum": 1, "maximum": 252, "description": "Số phiên quá khứ"},
                "buffer": {"type": "float", "default": 0.0, "minimum": 0.0, "maximum": 100.0, "description": "Đệm giá phá đáy"},
            },
            default_parameters={"period": 20, "buffer": 0.0},
            dependencies=[],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="regime__new_low",
        ),

        # Support & Resistance
        "sr.near_support": SignalDefinition(
            name="sr.near_support",
            version="1.0.0",
            category="sr",
            label_vi="Gần ngưỡng Hỗ trợ",
            description="Giá nằm gần ngưỡng hỗ trợ (Đáy N phiên, EMA20/50/200) trong khoảng dung sai ATR.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "period": {"type": "int", "default": 20, "minimum": 5, "maximum": 252, "description": "Chu kỳ đáy hỗ trợ"},
                "tolerance_atr": {"type": "float", "default": 0.5, "minimum": 0.1, "maximum": 2.0, "description": "Dung sai ATR"},
            },
            default_parameters={"period": 20, "tolerance_atr": 0.5},
            dependencies=[],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="sr__near_support",
        ),
        "sr.near_resistance": SignalDefinition(
            name="sr.near_resistance",
            version="1.0.0",
            category="sr",
            label_vi="Gần ngưỡng Kháng cự",
            description="Giá nằm gần ngưỡng kháng cự (Đỉnh N phiên, EMA20/50/200) trong khoảng dung sai ATR.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "period": {"type": "int", "default": 20, "minimum": 5, "maximum": 252, "description": "Chu kỳ đỉnh kháng cự"},
                "tolerance_atr": {"type": "float", "default": 0.5, "minimum": 0.1, "maximum": 2.0, "description": "Dung sai ATR"},
            },
            default_parameters={"period": 20, "tolerance_atr": 0.5},
            dependencies=[],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="sr__near_resistance",
        ),

        # Candle Structure
        "structure.candle_score": SignalDefinition(
            name="structure.candle_score",
            version="1.0.0",
            category="structure",
            label_vi="Điểm cấu trúc nến (Candle Structure Score)",
            description="Đánh giá cấu trúc thân nến, vị trí đóng cửa và cân bằng bóng nến theo thang điểm [-100, +100].",
            output_type=SignalOutputType.FLOAT,
            parameters_schema={
                "w_direction": {"type": "float", "default": 0.45, "minimum": 0.0, "maximum": 1.0, "description": "Trọng số hướng thân nến"},
                "w_close": {"type": "float", "default": 0.35, "minimum": 0.0, "maximum": 1.0, "description": "Trọng số vị trí đóng cửa"},
                "w_wick": {"type": "float", "default": 0.20, "minimum": 0.0, "maximum": 1.0, "description": "Trọng số cân bằng bóng nến"},
            },
            default_parameters={"w_direction": 0.45, "w_close": 0.35, "w_wick": 0.20},
            dependencies=[],
            warmup_bars=0,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="structure__candle_score",
        ),

        # VSA (Volume Spread Analysis)
        "vsa.strong_demand": SignalDefinition(
            name="vsa.strong_demand",
            version="1.0.0",
            category="vsa",
            label_vi="Cầu mạnh (Strong Demand)",
            description="Nến tăng biên độ rộng với khối lượng cao đóng cửa sát đỉnh (Spread/ATR >= 1.2, RVOL >= 1.5).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "period": {"type": "int", "default": 20, "minimum": 1, "maximum": 252, "description": "Chu kỳ tính RVOL"},
                "min_rvol": {"type": "float", "default": 1.5, "minimum": 0.1, "maximum": 100.0, "description": "Khối lượng tương đối tối thiểu"},
                "min_range_atr": {"type": "float", "default": 1.2, "minimum": 0.1, "maximum": 10.0, "description": "Biên độ / ATR tối thiểu"},
                "min_body_ratio": {"type": "float", "default": 0.50, "minimum": 0.0, "maximum": 1.0, "description": "Tỷ lệ thân nến tối thiểu"},
                "min_close_location": {"type": "float", "default": 0.70, "minimum": 0.0, "maximum": 1.0, "description": "Vị trí đóng cửa tối thiểu"},
            },
            default_parameters={
                "period": 20,
                "min_rvol": 1.5,
                "min_range_atr": 1.2,
                "min_body_ratio": 0.50,
                "min_close_location": 0.70,
            },
            dependencies=["volume.relative_volume"],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="vsa__strong_demand",
        ),
        "vsa.weak_demand": SignalDefinition(
            name="vsa.weak_demand",
            version="1.0.0",
            category="vsa",
            label_vi="Cầu yếu / Cạn cầu (Weak Demand)",
            description="Phiên nến tăng hoặc đi ngang với biên độ hẹp và khối lượng thấp dưới trung bình (RVOL <= 0.8, Spread/ATR <= 0.7).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "period": {"type": "int", "default": 20, "minimum": 1, "maximum": 252, "description": "Chu kỳ tính RVOL"},
                "max_rvol": {"type": "float", "default": 0.80, "minimum": 0.05, "maximum": 10.0, "description": "Khối lượng tương đối tối đa"},
                "max_range_atr": {"type": "float", "default": 0.70, "minimum": 0.05, "maximum": 5.0, "description": "Biên độ / ATR tối đa"},
            },
            default_parameters={"period": 20, "max_rvol": 0.80, "max_range_atr": 0.70},
            dependencies=["volume.relative_volume"],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="vsa__weak_demand",
        ),
        "vsa.upthrust": SignalDefinition(
            name="vsa.upthrust",
            version="1.0.0",
            category="vsa",
            label_vi="Upthrust / Kéo Xả kỹ thuật",
            description="Nến vượt đỉnh kháng cự trong phiên nhưng bị bán ép đóng cửa dưới kháng cự kèm khối lượng lớn.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "period": {"type": "int", "default": 20, "minimum": 5, "maximum": 252, "description": "Chu kỳ kháng cự và RVOL"},
                "min_rvol": {"type": "float", "default": 1.5, "minimum": 0.1, "maximum": 100.0, "description": "Khối lượng tương đối tối thiểu"},
                "min_upper_wick_ratio": {"type": "float", "default": 0.35, "minimum": 0.0, "maximum": 1.0, "description": "Tỷ lệ bóng trên tối thiểu"},
                "max_close_location": {"type": "float", "default": 0.45, "minimum": 0.0, "maximum": 1.0, "description": "Vị trí đóng cửa tối đa"},
                "breach_buffer_atr": {"type": "float", "default": 0.0, "minimum": 0.0, "maximum": 2.0, "description": "Đệm vượt ngưỡng (ATR)"},
            },
            default_parameters={
                "period": 20,
                "min_rvol": 1.5,
                "min_upper_wick_ratio": 0.35,
                "max_close_location": 0.45,
                "breach_buffer_atr": 0.0,
            },
            dependencies=["volume.relative_volume"],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="vsa__upthrust",
        ),
        "vsa.spring": SignalDefinition(
            name="vsa.spring",
            version="1.0.0",
            category="vsa",
            label_vi="Spring / Shakeout / Đạp Kéo",
            description="Nến đâm thủng hỗ trợ trong phiên nhưng rút chân đảo chiều đóng cửa trên hỗ trợ với khối lượng tương đối cao.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "period": {"type": "int", "default": 20, "minimum": 5, "maximum": 252, "description": "Chu kỳ hỗ trợ và RVOL"},
                "min_rvol": {"type": "float", "default": 1.30, "minimum": 0.1, "maximum": 100.0, "description": "Khối lượng tương đối tối thiểu"},
                "min_lower_wick_ratio": {"type": "float", "default": 0.35, "minimum": 0.0, "maximum": 1.0, "description": "Tỷ lệ bóng dưới tối thiểu"},
                "min_close_location": {"type": "float", "default": 0.70, "minimum": 0.0, "maximum": 1.0, "description": "Vị trí đóng cửa tối thiểu"},
                "breach_buffer_atr": {"type": "float", "default": 0.0, "minimum": 0.0, "maximum": 2.0, "description": "Đệm vượt ngưỡng (ATR)"},
            },
            default_parameters={
                "period": 20,
                "min_rvol": 1.30,
                "min_lower_wick_ratio": 0.35,
                "min_close_location": 0.70,
                "breach_buffer_atr": 0.0,
            },
            dependencies=["volume.relative_volume"],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="vsa__spring",
        ),
        "vsa.strong_demand_at_support": SignalDefinition(
            name="vsa.strong_demand_at_support",
            version="1.0.0",
            category="vsa",
            label_vi="Cầu mạnh tại hỗ trợ (Strong Demand at Support)",
            description="Tổ hợp tín hiệu giá gần ngưỡng hỗ trợ kết hợp xuất hiện Cầu mạnh hoặc Spring rút chân.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "period": {"type": "int", "default": 20, "minimum": 5, "maximum": 252, "description": "Chu kỳ hỗ trợ và RVOL"},
                "tolerance_atr": {"type": "float", "default": 0.5, "minimum": 0.1, "maximum": 2.0, "description": "Dung sai gần hỗ trợ (ATR)"},
                "min_rvol": {"type": "float", "default": 1.30, "minimum": 0.1, "maximum": 100.0, "description": "Khối lượng tương đối tối thiểu"},
            },
            default_parameters={"period": 20, "tolerance_atr": 0.5, "min_rvol": 1.30},
            dependencies=["sr.near_support", "vsa.strong_demand", "vsa.spring"],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="vsa__strong_demand_at_support",
        ),
        "vsa.strong_supply_at_resistance": SignalDefinition(
            name="vsa.strong_supply_at_resistance",
            version="1.0.0",
            category="vsa",
            label_vi="Cung mạnh tại kháng cự (Strong Supply at Resistance)",
            description="Tổ hợp tín hiệu giá gần kháng cự kết hợp xuất hiện Upthrust hoặc nến từ chối giảm mạnh kèm khối lượng lớn.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "period": {"type": "int", "default": 20, "minimum": 5, "maximum": 252, "description": "Chu kỳ kháng cự và RVOL"},
                "tolerance_atr": {"type": "float", "default": 0.5, "minimum": 0.1, "maximum": 2.0, "description": "Dung sai gần kháng cự (ATR)"},
                "min_rvol": {"type": "float", "default": 1.30, "minimum": 0.1, "maximum": 100.0, "description": "Khối lượng tương đối tối thiểu"},
            },
            default_parameters={"period": 20, "tolerance_atr": 0.5, "min_rvol": 1.30},
            dependencies=["sr.near_resistance", "vsa.upthrust"],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="vsa__strong_supply_at_resistance",
        ),

        # Ichimoku
        "ichimoku.score": SignalDefinition(
            name="ichimoku.score",
            version="1.0.0",
            category="ichimoku",
            label_vi="Điểm số Ichimoku (Ichimoku Score)",
            description="Điểm số kỹ thuật Ichimoku chuẩn hóa [-100, 100] dựa trên 4 yếu tố nhân quả: Giá vs Mây Kumo, Tenkan vs Kijun, Chikou vs Giá quá khứ, và Chiều hướng Mây tương lai.",
            output_type=SignalOutputType.FLOAT,
            parameters_schema={
                "tenkan_period": {"type": "int", "default": 9, "minimum": 1, "maximum": 200, "description": "Chu kỳ Tenkan-sen"},
                "kijun_period": {"type": "int", "default": 26, "minimum": 1, "maximum": 400, "description": "Chu kỳ Kijun-sen"},
                "senkou_period": {"type": "int", "default": 52, "minimum": 1, "maximum": 800, "description": "Chu kỳ Senkou Span B"},
                "displacement": {"type": "int", "default": 26, "minimum": 1, "maximum": 200, "description": "Độ dịch chuyển mây Kumo"},
                "include_kijun_slope": {"type": "int", "default": 0, "minimum": 0, "maximum": 1, "description": "Bao gồm độ dốc Kijun (0: tắt, 1: bật)"},
            },
            default_parameters={"tenkan_period": 9, "kijun_period": 26, "senkou_period": 52, "displacement": 26, "include_kijun_slope": 0},
            dependencies=[],
            warmup_bars=78,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="ichimoku__score",
        ),
        "ichimoku.bullish": SignalDefinition(
            name="ichimoku.bullish",
            version="1.0.0",
            category="ichimoku",
            label_vi="Trạng thái Tăng Ichimoku (Ichimoku Bullish)",
            description="Tín hiệu xu hướng tăng theo hệ thống Ichimoku khi điểm số đạt hoặc vượt ngưỡng tối thiểu (mặc định >= 50.0).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "tenkan_period": {"type": "int", "default": 9, "minimum": 1, "maximum": 200, "description": "Chu kỳ Tenkan-sen"},
                "kijun_period": {"type": "int", "default": 26, "minimum": 1, "maximum": 400, "description": "Chu kỳ Kijun-sen"},
                "senkou_period": {"type": "int", "default": 52, "minimum": 1, "maximum": 800, "description": "Chu kỳ Senkou Span B"},
                "displacement": {"type": "int", "default": 26, "minimum": 1, "maximum": 200, "description": "Độ dịch chuyển mây Kumo"},
                "include_kijun_slope": {"type": "int", "default": 0, "minimum": 0, "maximum": 1, "description": "Bao gồm độ dốc Kijun (0: tắt, 1: bật)"},
                "threshold": {"type": "float", "default": 50.0, "minimum": -100.0, "maximum": 100.0, "description": "Ngưỡng điểm số tối thiểu"},
            },
            default_parameters={"tenkan_period": 9, "kijun_period": 26, "senkou_period": 52, "displacement": 26, "include_kijun_slope": 0, "threshold": 50.0},
            dependencies=["ichimoku.score"],
            warmup_bars=78,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="ichimoku__bullish",
        ),
        "ichimoku.bearish": SignalDefinition(
            name="ichimoku.bearish",
            version="1.0.0",
            category="ichimoku",
            label_vi="Trạng thái Giảm Ichimoku (Ichimoku Bearish)",
            description="Tín hiệu xu hướng giảm theo hệ thống Ichimoku khi điểm số xuống dưới ngưỡng tối thiểu (mặc định <= -50.0).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "tenkan_period": {"type": "int", "default": 9, "minimum": 1, "maximum": 200, "description": "Chu kỳ Tenkan-sen"},
                "kijun_period": {"type": "int", "default": 26, "minimum": 1, "maximum": 400, "description": "Chu kỳ Kijun-sen"},
                "senkou_period": {"type": "int", "default": 52, "minimum": 1, "maximum": 800, "description": "Chu kỳ Senkou Span B"},
                "displacement": {"type": "int", "default": 26, "minimum": 1, "maximum": 200, "description": "Độ dịch chuyển mây Kumo"},
                "include_kijun_slope": {"type": "int", "default": 0, "minimum": 0, "maximum": 1, "description": "Bao gồm độ dốc Kijun (0: tắt, 1: bật)"},
                "threshold": {"type": "float", "default": -50.0, "minimum": -100.0, "maximum": 100.0, "description": "Ngưỡng điểm số tối đa"},
            },
            default_parameters={"tenkan_period": 9, "kijun_period": 26, "senkou_period": 52, "displacement": 26, "include_kijun_slope": 0, "threshold": -50.0},
            dependencies=["ichimoku.score"],
            warmup_bars=78,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="ichimoku__bearish",
        ),
        "ichimoku.tk_cross_bullish": SignalDefinition(
            name="ichimoku.tk_cross_bullish",
            version="1.0.0",
            category="ichimoku",
            label_vi="Giao cắt Vàng Tenkan/Kijun (Bullish TK Cross)",
            description="Tín hiệu khi Tenkan-sen cắt lên trên Kijun-sen tại phiên hiện tại.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "tenkan_period": {"type": "int", "default": 9, "minimum": 1, "maximum": 200, "description": "Chu kỳ Tenkan-sen"},
                "kijun_period": {"type": "int", "default": 26, "minimum": 1, "maximum": 400, "description": "Chu kỳ Kijun-sen"},
            },
            default_parameters={"tenkan_period": 9, "kijun_period": 26},
            dependencies=[],
            warmup_bars=27,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="ichimoku__tk_cross_bullish",
        ),
        "ichimoku.tk_cross_bearish": SignalDefinition(
            name="ichimoku.tk_cross_bearish",
            version="1.0.0",
            category="ichimoku",
            label_vi="Giao cắt Tử thần Tenkan/Kijun (Bearish TK Cross)",
            description="Tín hiệu khi Tenkan-sen cắt xuống dưới Kijun-sen tại phiên hiện tại.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "tenkan_period": {"type": "int", "default": 9, "minimum": 1, "maximum": 200, "description": "Chu kỳ Tenkan-sen"},
                "kijun_period": {"type": "int", "default": 26, "minimum": 1, "maximum": 400, "description": "Chu kỳ Kijun-sen"},
            },
            default_parameters={"tenkan_period": 9, "kijun_period": 26},
            dependencies=[],
            warmup_bars=27,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="ichimoku__tk_cross_bearish",
        ),
        "ichimoku.kumo_breakout_bullish": SignalDefinition(
            name="ichimoku.kumo_breakout_bullish",
            version="1.0.0",
            category="ichimoku",
            label_vi="Phá vỡ Mây Kumo Tăng (Bullish Kumo Breakout)",
            description="Giá đóng cửa vượt lên trên cạnh trên của mây Kumo hiển thị hiện tại.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "tenkan_period": {"type": "int", "default": 9, "minimum": 1, "maximum": 200, "description": "Chu kỳ Tenkan-sen"},
                "kijun_period": {"type": "int", "default": 26, "minimum": 1, "maximum": 400, "description": "Chu kỳ Kijun-sen"},
                "senkou_period": {"type": "int", "default": 52, "minimum": 1, "maximum": 800, "description": "Chu kỳ Senkou Span B"},
                "displacement": {"type": "int", "default": 26, "minimum": 1, "maximum": 200, "description": "Độ dịch chuyển mây Kumo"},
            },
            default_parameters={"tenkan_period": 9, "kijun_period": 26, "senkou_period": 52, "displacement": 26},
            dependencies=[],
            warmup_bars=79,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="ichimoku__kumo_breakout_bullish",
        ),
        "ichimoku.kumo_breakout_bearish": SignalDefinition(
            name="ichimoku.kumo_breakout_bearish",
            version="1.0.0",
            category="ichimoku",
            label_vi="Phá vỡ Mây Kumo Giảm (Bearish Kumo Breakout)",
            description="Giá đóng cửa xuyên thủng xuống dưới cạnh dưới của mây Kumo hiển thị hiện tại.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "tenkan_period": {"type": "int", "default": 9, "minimum": 1, "maximum": 200, "description": "Chu kỳ Tenkan-sen"},
                "kijun_period": {"type": "int", "default": 26, "minimum": 1, "maximum": 400, "description": "Chu kỳ Kijun-sen"},
                "senkou_period": {"type": "int", "default": 52, "minimum": 1, "maximum": 800, "description": "Chu kỳ Senkou Span B"},
                "displacement": {"type": "int", "default": 26, "minimum": 1, "maximum": 200, "description": "Độ dịch chuyển mây Kumo"},
            },
            default_parameters={"tenkan_period": 9, "kijun_period": 26, "senkou_period": 52, "displacement": 26},
            dependencies=[],
            warmup_bars=79,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="ichimoku__kumo_breakout_bearish",
        ),

        # Divergence - Confirmed Pivots
        "divergence.confirmed_pivot_high": SignalDefinition(
            name="divergence.confirmed_pivot_high",
            version="1.0.0",
            category="divergence",
            label_vi="Đỉnh Pivot Xác Nhận (Confirmed Pivot High)",
            description="Đỉnh giá cục bộ được xác nhận sau right_bars phiên (mặc định 3 phiên).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "left_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến kiểm tra bên trái"},
                "right_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến xác nhận bên phải"},
            },
            default_parameters={"left_bars": 3, "right_bars": 3},
            dependencies=[],
            warmup_bars=6,
            causal_delay_bars=3,
            status=SignalStatus.ACTIVE,
            ast_alias="divergence__confirmed_pivot_high",
        ),
        "divergence.confirmed_pivot_low": SignalDefinition(
            name="divergence.confirmed_pivot_low",
            version="1.0.0",
            category="divergence",
            label_vi="Đáy Pivot Xác Nhận (Confirmed Pivot Low)",
            description="Đáy giá cục bộ được xác nhận sau right_bars phiên (mặc định 3 phiên).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "left_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến kiểm tra bên trái"},
                "right_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến xác nhận bên phải"},
            },
            default_parameters={"left_bars": 3, "right_bars": 3},
            dependencies=[],
            warmup_bars=6,
            causal_delay_bars=3,
            status=SignalStatus.ACTIVE,
            ast_alias="divergence__confirmed_pivot_low",
        ),

        # Divergence - RSI
        "divergence.rsi_regular_bullish": SignalDefinition(
            name="divergence.rsi_regular_bullish",
            version="1.0.0",
            category="divergence",
            label_vi="Phân kỳ Thường Tăng RSI (RSI Regular Bullish Divergence)",
            description="Giá tạo Đáy Thấp Hơn (Lower Low) nhưng RSI tạo Đáy Cao Hơn (Higher Low).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "rsi_period": {"type": "int", "default": 14, "minimum": 2, "maximum": 100, "description": "Chu kỳ RSI"},
                "left_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến kiểm tra bên trái"},
                "right_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến xác nhận bên phải"},
                "min_separation": {"type": "int", "default": 5, "minimum": 1, "maximum": 50, "description": "Khoảng cách nến tối thiểu giữa 2 đỉnh/đáy"},
                "max_separation": {"type": "int", "default": 60, "minimum": 5, "maximum": 200, "description": "Khoảng cách nến tối đa giữa 2 đỉnh/đáy"},
            },
            default_parameters={"rsi_period": 14, "left_bars": 3, "right_bars": 3, "min_separation": 5, "max_separation": 60},
            dependencies=["divergence.confirmed_pivot_low"],
            warmup_bars=20,
            causal_delay_bars=3,
            status=SignalStatus.ACTIVE,
            ast_alias="divergence__rsi_regular_bullish",
        ),
        "divergence.rsi_regular_bearish": SignalDefinition(
            name="divergence.rsi_regular_bearish",
            version="1.0.0",
            category="divergence",
            label_vi="Phân kỳ Thường Giảm RSI (RSI Regular Bearish Divergence)",
            description="Giá tạo Đỉnh Cao Hơn (Higher High) nhưng RSI tạo Đỉnh Thấp Hơn (Lower High).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "rsi_period": {"type": "int", "default": 14, "minimum": 2, "maximum": 100, "description": "Chu kỳ RSI"},
                "left_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến kiểm tra bên trái"},
                "right_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến xác nhận bên phải"},
                "min_separation": {"type": "int", "default": 5, "minimum": 1, "maximum": 50, "description": "Khoảng cách nến tối thiểu"},
                "max_separation": {"type": "int", "default": 60, "minimum": 5, "maximum": 200, "description": "Khoảng cách nến tối đa"},
            },
            default_parameters={"rsi_period": 14, "left_bars": 3, "right_bars": 3, "min_separation": 5, "max_separation": 60},
            dependencies=["divergence.confirmed_pivot_high"],
            warmup_bars=20,
            causal_delay_bars=3,
            status=SignalStatus.ACTIVE,
            ast_alias="divergence__rsi_regular_bearish",
        ),
        "divergence.rsi_hidden_bullish": SignalDefinition(
            name="divergence.rsi_hidden_bullish",
            version="1.0.0",
            category="divergence",
            label_vi="Phân kỳ Ẩn Tăng RSI (RSI Hidden Bullish Divergence)",
            description="Giá tạo Đáy Cao Hơn (Higher Low) nhưng RSI tạo Đáy Thấp Hơn (Lower Low).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "rsi_period": {"type": "int", "default": 14, "minimum": 2, "maximum": 100, "description": "Chu kỳ RSI"},
                "left_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến kiểm tra bên trái"},
                "right_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến xác nhận bên phải"},
                "min_separation": {"type": "int", "default": 5, "minimum": 1, "maximum": 50, "description": "Khoảng cách nến tối thiểu"},
                "max_separation": {"type": "int", "default": 60, "minimum": 5, "maximum": 200, "description": "Khoảng cách nến tối đa"},
            },
            default_parameters={"rsi_period": 14, "left_bars": 3, "right_bars": 3, "min_separation": 5, "max_separation": 60},
            dependencies=["divergence.confirmed_pivot_low"],
            warmup_bars=20,
            causal_delay_bars=3,
            status=SignalStatus.ACTIVE,
            ast_alias="divergence__rsi_hidden_bullish",
        ),
        "divergence.rsi_hidden_bearish": SignalDefinition(
            name="divergence.rsi_hidden_bearish",
            version="1.0.0",
            category="divergence",
            label_vi="Phân kỳ Ẩn Giảm RSI (RSI Hidden Bearish Divergence)",
            description="Giá tạo Đỉnh Thấp Hơn (Lower High) nhưng RSI tạo Đỉnh Cao Hơn (Higher High).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "rsi_period": {"type": "int", "default": 14, "minimum": 2, "maximum": 100, "description": "Chu kỳ RSI"},
                "left_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến kiểm tra bên trái"},
                "right_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến xác nhận bên phải"},
                "min_separation": {"type": "int", "default": 5, "minimum": 1, "maximum": 50, "description": "Khoảng cách nến tối thiểu"},
                "max_separation": {"type": "int", "default": 60, "minimum": 5, "maximum": 200, "description": "Khoảng cách nến tối đa"},
            },
            default_parameters={"rsi_period": 14, "left_bars": 3, "right_bars": 3, "min_separation": 5, "max_separation": 60},
            dependencies=["divergence.confirmed_pivot_high"],
            warmup_bars=20,
            causal_delay_bars=3,
            status=SignalStatus.ACTIVE,
            ast_alias="divergence__rsi_hidden_bearish",
        ),

        # Divergence - MACD Histogram
        "divergence.macd_regular_bullish": SignalDefinition(
            name="divergence.macd_regular_bullish",
            version="1.0.0",
            category="divergence",
            label_vi="Phân kỳ Thường Tăng MACD (MACD Regular Bullish Divergence)",
            description="Giá tạo Đáy Thấp Hơn (Lower Low) nhưng MACD Histogram tạo Đáy Cao Hơn (Higher Low).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "fast": {"type": "int", "default": 12, "minimum": 1, "maximum": 100, "description": "Chu kỳ EMA nhanh"},
                "slow": {"type": "int", "default": 26, "minimum": 1, "maximum": 200, "description": "Chu kỳ EMA chậm"},
                "signal_period": {"type": "int", "default": 9, "minimum": 1, "maximum": 100, "description": "Chu kỳ đường tín hiệu"},
                "left_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến kiểm tra bên trái"},
                "right_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến xác nhận bên phải"},
                "min_separation": {"type": "int", "default": 5, "minimum": 1, "maximum": 50, "description": "Khoảng cách nến tối thiểu"},
                "max_separation": {"type": "int", "default": 60, "minimum": 5, "maximum": 200, "description": "Khoảng cách nến tối đa"},
            },
            default_parameters={"fast": 12, "slow": 26, "signal_period": 9, "left_bars": 3, "right_bars": 3, "min_separation": 5, "max_separation": 60},
            dependencies=["divergence.confirmed_pivot_low"],
            warmup_bars=35,
            causal_delay_bars=3,
            status=SignalStatus.ACTIVE,
            ast_alias="divergence__macd_regular_bullish",
        ),
        "divergence.macd_regular_bearish": SignalDefinition(
            name="divergence.macd_regular_bearish",
            version="1.0.0",
            category="divergence",
            label_vi="Phân kỳ Thường Giảm MACD (MACD Regular Bearish Divergence)",
            description="Giá tạo Đỉnh Cao Hơn (Higher High) nhưng MACD Histogram tạo Đỉnh Thấp Hơn (Lower High).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "fast": {"type": "int", "default": 12, "minimum": 1, "maximum": 100, "description": "Chu kỳ EMA nhanh"},
                "slow": {"type": "int", "default": 26, "minimum": 1, "maximum": 200, "description": "Chu kỳ EMA chậm"},
                "signal_period": {"type": "int", "default": 9, "minimum": 1, "maximum": 100, "description": "Chu kỳ đường tín hiệu"},
                "left_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến kiểm tra bên trái"},
                "right_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến xác nhận bên phải"},
                "min_separation": {"type": "int", "default": 5, "minimum": 1, "maximum": 50, "description": "Khoảng cách nến tối thiểu"},
                "max_separation": {"type": "int", "default": 60, "minimum": 5, "maximum": 200, "description": "Khoảng cách nến tối đa"},
            },
            default_parameters={"fast": 12, "slow": 26, "signal_period": 9, "left_bars": 3, "right_bars": 3, "min_separation": 5, "max_separation": 60},
            dependencies=["divergence.confirmed_pivot_high"],
            warmup_bars=35,
            causal_delay_bars=3,
            status=SignalStatus.ACTIVE,
            ast_alias="divergence__macd_regular_bearish",
        ),
        "divergence.macd_hidden_bullish": SignalDefinition(
            name="divergence.macd_hidden_bullish",
            version="1.0.0",
            category="divergence",
            label_vi="Phân kỳ Ẩn Tăng MACD (MACD Hidden Bullish Divergence)",
            description="Giá tạo Đáy Cao Hơn (Higher Low) nhưng MACD Histogram tạo Đáy Thấp Hơn (Lower Low).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "fast": {"type": "int", "default": 12, "minimum": 1, "maximum": 100, "description": "Chu kỳ EMA nhanh"},
                "slow": {"type": "int", "default": 26, "minimum": 1, "maximum": 200, "description": "Chu kỳ EMA chậm"},
                "signal_period": {"type": "int", "default": 9, "minimum": 1, "maximum": 100, "description": "Chu kỳ đường tín hiệu"},
                "left_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến kiểm tra bên trái"},
                "right_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến xác nhận bên phải"},
                "min_separation": {"type": "int", "default": 5, "minimum": 1, "maximum": 50, "description": "Khoảng cách nến tối thiểu"},
                "max_separation": {"type": "int", "default": 60, "minimum": 5, "maximum": 200, "description": "Khoảng cách nến tối đa"},
            },
            default_parameters={"fast": 12, "slow": 26, "signal_period": 9, "left_bars": 3, "right_bars": 3, "min_separation": 5, "max_separation": 60},
            dependencies=["divergence.confirmed_pivot_low"],
            warmup_bars=35,
            causal_delay_bars=3,
            status=SignalStatus.ACTIVE,
            ast_alias="divergence__macd_hidden_bullish",
        ),
        "divergence.macd_hidden_bearish": SignalDefinition(
            name="divergence.macd_hidden_bearish",
            version="1.0.0",
            category="divergence",
            label_vi="Phân kỳ Ẩn Giảm MACD (MACD Hidden Bearish Divergence)",
            description="Giá tạo Đỉnh Thấp Hơn (Lower High) nhưng MACD Histogram tạo Đỉnh Cao Hơn (Higher High).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "fast": {"type": "int", "default": 12, "minimum": 1, "maximum": 100, "description": "Chu kỳ EMA nhanh"},
                "slow": {"type": "int", "default": 26, "minimum": 1, "maximum": 200, "description": "Chu kỳ EMA chậm"},
                "signal_period": {"type": "int", "default": 9, "minimum": 1, "maximum": 100, "description": "Chu kỳ đường tín hiệu"},
                "left_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến kiểm tra bên trái"},
                "right_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến xác nhận bên phải"},
                "min_separation": {"type": "int", "default": 5, "minimum": 1, "maximum": 50, "description": "Khoảng cách nến tối thiểu"},
                "max_separation": {"type": "int", "default": 60, "minimum": 5, "maximum": 200, "description": "Khoảng cách nến tối đa"},
            },
            default_parameters={"fast": 12, "slow": 26, "signal_period": 9, "left_bars": 3, "right_bars": 3, "min_separation": 5, "max_separation": 60},
            dependencies=["divergence.confirmed_pivot_high"],
            warmup_bars=35,
            causal_delay_bars=3,
            status=SignalStatus.ACTIVE,
            ast_alias="divergence__macd_hidden_bearish",
        ),

        # Divergence - Stochastic
        "divergence.stoch_regular_bullish": SignalDefinition(
            name="divergence.stoch_regular_bullish",
            version="1.0.0",
            category="divergence",
            label_vi="Phân kỳ Thường Tăng Stochastic (Stochastic Regular Bullish)",
            description="Giá tạo Đáy Thấp Hơn (Lower Low) nhưng Stochastic %K tạo Đáy Cao Hơn (Higher Low).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "k_period": {"type": "int", "default": 14, "minimum": 1, "maximum": 100, "description": "Chu kỳ Stochastic %K"},
                "left_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến kiểm tra bên trái"},
                "right_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến xác nhận bên phải"},
                "min_separation": {"type": "int", "default": 5, "minimum": 1, "maximum": 50, "description": "Khoảng cách nến tối thiểu"},
                "max_separation": {"type": "int", "default": 60, "minimum": 5, "maximum": 200, "description": "Khoảng cách nến tối đa"},
            },
            default_parameters={"k_period": 14, "left_bars": 3, "right_bars": 3, "min_separation": 5, "max_separation": 60},
            dependencies=["divergence.confirmed_pivot_low"],
            warmup_bars=20,
            causal_delay_bars=3,
            status=SignalStatus.ACTIVE,
            ast_alias="divergence__stoch_regular_bullish",
        ),
        "divergence.stoch_regular_bearish": SignalDefinition(
            name="divergence.stoch_regular_bearish",
            version="1.0.0",
            category="divergence",
            label_vi="Phân kỳ Thường Giảm Stochastic (Stochastic Regular Bearish)",
            description="Giá tạo Đỉnh Cao Hơn (Higher High) nhưng Stochastic %K tạo Đỉnh Thấp Hơn (Lower High).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "k_period": {"type": "int", "default": 14, "minimum": 1, "maximum": 100, "description": "Chu kỳ Stochastic %K"},
                "left_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến kiểm tra bên trái"},
                "right_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến xác nhận bên phải"},
                "min_separation": {"type": "int", "default": 5, "minimum": 1, "maximum": 50, "description": "Khoảng cách nến tối thiểu"},
                "max_separation": {"type": "int", "default": 60, "minimum": 5, "maximum": 200, "description": "Khoảng cách nến tối đa"},
            },
            default_parameters={"k_period": 14, "left_bars": 3, "right_bars": 3, "min_separation": 5, "max_separation": 60},
            dependencies=["divergence.confirmed_pivot_high"],
            warmup_bars=20,
            causal_delay_bars=3,
            status=SignalStatus.ACTIVE,
            ast_alias="divergence__stoch_regular_bearish",
        ),
        "divergence.stoch_hidden_bullish": SignalDefinition(
            name="divergence.stoch_hidden_bullish",
            version="1.0.0",
            category="divergence",
            label_vi="Phân kỳ Ẩn Tăng Stochastic (Stochastic Hidden Bullish)",
            description="Giá tạo Đáy Cao Hơn (Higher Low) nhưng Stochastic %K tạo Đáy Thấp Hơn (Lower Low).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "k_period": {"type": "int", "default": 14, "minimum": 1, "maximum": 100, "description": "Chu kỳ Stochastic %K"},
                "left_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến kiểm tra bên trái"},
                "right_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến xác nhận bên phải"},
                "min_separation": {"type": "int", "default": 5, "minimum": 1, "maximum": 50, "description": "Khoảng cách nến tối thiểu"},
                "max_separation": {"type": "int", "default": 60, "minimum": 5, "maximum": 200, "description": "Khoảng cách nến tối đa"},
            },
            default_parameters={"k_period": 14, "left_bars": 3, "right_bars": 3, "min_separation": 5, "max_separation": 60},
            dependencies=["divergence.confirmed_pivot_low"],
            warmup_bars=20,
            causal_delay_bars=3,
            status=SignalStatus.ACTIVE,
            ast_alias="divergence__stoch_hidden_bullish",
        ),
        "divergence.stoch_hidden_bearish": SignalDefinition(
            name="divergence.stoch_hidden_bearish",
            version="1.0.0",
            category="divergence",
            label_vi="Phân kỳ Ẩn Giảm Stochastic (Stochastic Hidden Bearish)",
            description="Giá tạo Đỉnh Thấp Hơn (Lower High) nhưng Stochastic %K tạo Đỉnh Cao Hơn (Higher High).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "k_period": {"type": "int", "default": 14, "minimum": 1, "maximum": 100, "description": "Chu kỳ Stochastic %K"},
                "left_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến kiểm tra bên trái"},
                "right_bars": {"type": "int", "default": 3, "minimum": 1, "maximum": 20, "description": "Số nến xác nhận bên phải"},
                "min_separation": {"type": "int", "default": 5, "minimum": 1, "maximum": 50, "description": "Khoảng cách nến tối thiểu"},
                "max_separation": {"type": "int", "default": 60, "minimum": 5, "maximum": 200, "description": "Khoảng cách nến tối đa"},
            },
            default_parameters={"k_period": 14, "left_bars": 3, "right_bars": 3, "min_separation": 5, "max_separation": 60},
            dependencies=["divergence.confirmed_pivot_high"],
            warmup_bars=20,
            causal_delay_bars=3,
            status=SignalStatus.ACTIVE,
            ast_alias="divergence__stoch_hidden_bearish",
        ),
        # Technical Health (SIG-HEALTH-001)
        "health.score": SignalDefinition(
            name="health.score",
            version="1.0.0",
            category="health",
            label_vi="Điểm Sức Khỏe Kỹ Thuật Tổng Hợp (Technical Health Score)",
            description="Tổng hợp 4 họ chỉ báo độc lập (Xu hướng, Động lượng, Chuyển tiếp, Tham gia) về thang [-100, +100].",
            output_type=SignalOutputType.FLOAT,
            parameters_schema={
                "preset": {"type": "string", "default": "health_v1_balanced", "description": "Tên bộ trọng số chuẩn"},
            },
            default_parameters={"preset": "health_v1_balanced"},
            dependencies=["technical.ema_cross", "technical.rsi_level_cross", "technical.macd_zero_cross", "volume.relative_volume"],
            warmup_bars=50,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="health__score",
        ),
        "health.favorable": SignalDefinition(
            name="health.favorable",
            version="1.0.0",
            category="health",
            label_vi="Trạng Thái Kỹ Thuật Tích Cực (Technical Health Favorable)",
            description="Điểm sức khỏe kỹ thuật tổng hợp đạt ngưỡng tích cực (mặc định >= +35.0).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "favorable_threshold": {"type": "float", "default": 35.0, "description": "Ngưỡng điểm tích cực"},
            },
            default_parameters={"favorable_threshold": 35.0},
            dependencies=["health.score"],
            warmup_bars=50,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="health__favorable",
        ),
        "health.unfavorable": SignalDefinition(
            name="health.unfavorable",
            version="1.0.0",
            category="health",
            label_vi="Trạng Thái Kỹ Thuật Tiêu Cực (Technical Health Unfavorable)",
            description="Điểm sức khỏe kỹ thuật tổng hợp rơi vào vùng tiêu cực (mặc định <= -35.0).",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "unfavorable_threshold": {"type": "float", "default": -35.0, "description": "Ngưỡng điểm tiêu cực"},
            },
            default_parameters={"unfavorable_threshold": -35.0},
            dependencies=["health.score"],
            warmup_bars=50,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="health__unfavorable",
        ),
        # Money Flow BB Events (BBI-SIG-001/002/003)
        "bb.direction_rising": SignalDefinition(
            name="bb.direction_rising",
            version="1.0.0",
            category="flow",
            label_vi="Dòng Tiền BB Chuyển Động Tăng (BB Direction Rising)",
            description="Điểm Money Flow BB tăng so với phiên liền trước ở khung thời gian chỉ định.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "horizon": {"type": "string", "default": "T20", "description": "Khung thời gian Money Flow BB (T03, T05, T10, T20, T50, T200)"},
                "min_quality": {"type": "string", "default": "HIGH", "description": "Chuẩn chất lượng dữ liệu tối thiểu"},
            },
            default_parameters={"horizon": "T20", "min_quality": "HIGH"},
            dependencies=[],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="bb__direction_rising",
        ),
        "bb.direction_falling": SignalDefinition(
            name="bb.direction_falling",
            version="1.0.0",
            category="flow",
            label_vi="Dòng Tiền BB Chuyển Động Giảm (BB Direction Falling)",
            description="Điểm Money Flow BB giảm so với phiên liền trước ở khung thời gian chỉ định.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "horizon": {"type": "string", "default": "T20", "description": "Khung thời gian Money Flow BB (T03, T05, T10, T20, T50, T200)"},
                "min_quality": {"type": "string", "default": "HIGH", "description": "Chuẩn chất lượng dữ liệu tối thiểu"},
            },
            default_parameters={"horizon": "T20", "min_quality": "HIGH"},
            dependencies=[],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="bb__direction_falling",
        ),
        "bb.regime_positive": SignalDefinition(
            name="bb.regime_positive",
            version="1.0.0",
            category="flow",
            label_vi="Chế Độ Dòng Tiền BB Tích Cực (BB Regime Positive)",
            description="Điểm Money Flow BB nằm trên mức cân bằng 50.0 ở khung thời gian chỉ định.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "horizon": {"type": "string", "default": "T20", "description": "Khung thời gian Money Flow BB"},
                "min_quality": {"type": "string", "default": "HIGH", "description": "Chuẩn chất lượng dữ liệu tối thiểu"},
            },
            default_parameters={"horizon": "T20", "min_quality": "HIGH"},
            dependencies=[],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="bb__regime_positive",
        ),
        "bb.regime_negative": SignalDefinition(
            name="bb.regime_negative",
            version="1.0.0",
            category="flow",
            label_vi="Chế Độ Dòng Tiền BB Tiêu Cực (BB Regime Negative)",
            description="Điểm Money Flow BB nằm dưới mức cân bằng 50.0 ở khung thời gian chỉ định.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "horizon": {"type": "string", "default": "T20", "description": "Khung thời gian Money Flow BB"},
                "min_quality": {"type": "string", "default": "HIGH", "description": "Chuẩn chất lượng dữ liệu tối thiểu"},
            },
            default_parameters={"horizon": "T20", "min_quality": "HIGH"},
            dependencies=[],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="bb__regime_negative",
        ),
        "bb.confluence_bullish": SignalDefinition(
            name="bb.confluence_bullish",
            version="1.0.0",
            category="flow",
            label_vi="Đồng Thuận Dòng Tiền BB Tăng (BB Bullish Confluence)",
            description="Đồng thuận đa khung thời gian: cả hai khung ngắn và dài đều ở chế độ tích cực và đang tăng.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "short_horizon": {"type": "string", "default": "T05", "description": "Khung thời gian ngắn"},
                "long_horizon": {"type": "string", "default": "T20", "description": "Khung thời gian dài"},
                "min_quality": {"type": "string", "default": "HIGH", "description": "Chuẩn chất lượng dữ liệu tối thiểu"},
            },
            default_parameters={"short_horizon": "T05", "long_horizon": "T20", "min_quality": "HIGH"},
            dependencies=[],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="bb__confluence_bullish",
        ),
        "bb.confluence_bearish": SignalDefinition(
            name="bb.confluence_bearish",
            version="1.0.0",
            category="flow",
            label_vi="Đồng Thuận Dòng Tiền BB Giảm (BB Bearish Confluence)",
            description="Đồng thuận đa khung thời gian: cả hai khung ngắn và dài đều ở chế độ tiêu cực và đang giảm.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "short_horizon": {"type": "string", "default": "T05", "description": "Khung thời gian ngắn"},
                "long_horizon": {"type": "string", "default": "T20", "description": "Khung thời gian dài"},
                "min_quality": {"type": "string", "default": "HIGH", "description": "Chuẩn chất lượng dữ liệu tối thiểu"},
            },
            default_parameters={"short_horizon": "T05", "long_horizon": "T20", "min_quality": "HIGH"},
            dependencies=[],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="bb__confluence_bearish",
        ),
        "bb.turn_up": SignalDefinition(
            name="bb.turn_up",
            version="1.0.0",
            category="flow",
            label_vi="Điểm Đảo Chiều Tăng BB (BB Turn Up Event)",
            description="Sự kiện đảo chiều tăng có tham số: điểm BB phiên trước ở sát ngưỡng và phiên nay bứt phá đi lên.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "threshold": {"type": "float", "default": 40.0, "description": "Ngưỡng điểm BB chuyển hướng"},
                "tolerance": {"type": "float", "default": 2.0, "description": "Dung sai điểm"},
                "horizon": {"type": "string", "default": "T20", "description": "Khung thời gian Money Flow BB"},
                "min_quality": {"type": "string", "default": "HIGH", "description": "Chuẩn chất lượng dữ liệu tối thiểu"},
            },
            default_parameters={"threshold": 40.0, "tolerance": 2.0, "horizon": "T20", "min_quality": "HIGH"},
            dependencies=[],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="bb__turn_up",
        ),
        "bb.turn_down": SignalDefinition(
            name="bb.turn_down",
            version="1.0.0",
            category="flow",
            label_vi="Điểm Đảo Chiều Giảm BB (BB Turn Down Event)",
            description="Sự kiện đảo chiều giảm có tham số: điểm BB phiên trước ở sát ngưỡng và phiên nay đảo chiều đi xuống.",
            output_type=SignalOutputType.BOOL,
            parameters_schema={
                "threshold": {"type": "float", "default": 60.0, "description": "Ngưỡng điểm BB chuyển hướng"},
                "tolerance": {"type": "float", "default": 2.0, "description": "Dung sai điểm"},
                "horizon": {"type": "string", "default": "T20", "description": "Khung thời gian Money Flow BB"},
                "min_quality": {"type": "string", "default": "HIGH", "description": "Chuẩn chất lượng dữ liệu tối thiểu"},
            },
            default_parameters={"threshold": 60.0, "tolerance": 2.0, "horizon": "T20", "min_quality": "HIGH"},
            dependencies=[],
            warmup_bars=20,
            causal_delay_bars=0,
            status=SignalStatus.ACTIVE,
            ast_alias="bb__turn_down",
        ),
    }




    _ALIAS_TO_NAME: Dict[str, str] = {defn.ast_alias: defn.name for defn in _DEFINITIONS.values()}

    @classmethod
    def get_definition(cls, name: str) -> Optional[SignalDefinition]:
        """Retrieve signal definition by registry ID."""
        return cls._DEFINITIONS.get(name)

    @classmethod
    def list_definitions(cls) -> List[SignalDefinition]:
        """List all active signal definitions."""
        return list(cls._DEFINITIONS.values())

    @classmethod
    def resolve_name_from_alias(cls, alias: str) -> Optional[str]:
        """Map an AST alias (e.g. 'pattern__hammer') back to canonical ID ('pattern.hammer')."""
        return cls._ALIAS_TO_NAME.get(alias)

    @classmethod
    def get_alias_for_name(cls, name: str) -> Optional[str]:
        """Map canonical ID to AST alias."""
        defn = cls._DEFINITIONS.get(name)
        return defn.ast_alias if defn else None

    @classmethod
    def resolve_and_validate_params(cls, name: str, params: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """Validate and resolve configuration parameters, merging defaults."""
        defn = cls.get_definition(name)
        if not defn:
            raise ValueError(f"Unknown signal name: '{name}'")

        raw = dict(defn.default_parameters)
        if params:
            raw.update(params)

        # Validate schema constraints
        schema = defn.parameters_schema
        for k, val in raw.items():
            if k not in schema:
                raise ValueError(f"Unknown parameter '{k}' for signal '{name}'")
            expected_type = schema[k].get("type")
            if expected_type == "int":
                if isinstance(val, bool) or not isinstance(val, int):
                    raise ValueError(f"Parameter '{k}' must be an integer, got {type(val).__name__}")
                if "minimum" in schema[k] and val < schema[k]["minimum"]:
                    raise ValueError(f"Parameter '{k}' must be >= {schema[k]['minimum']}, got {val}")
                if "maximum" in schema[k] and val > schema[k]["maximum"]:
                    raise ValueError(f"Parameter '{k}' must be <= {schema[k]['maximum']}, got {val}")
            elif expected_type == "float":
                if isinstance(val, bool) or not isinstance(val, (int, float)):
                    raise ValueError(f"Parameter '{k}' must be a float, got {type(val).__name__}")
                f_val = float(val)
                if "minimum" in schema[k] and f_val < schema[k]["minimum"]:
                    raise ValueError(f"Parameter '{k}' must be >= {schema[k]['minimum']}, got {f_val}")
                if "maximum" in schema[k] and f_val > schema[k]["maximum"]:
                    raise ValueError(f"Parameter '{k}' must be <= {schema[k]['maximum']}, got {f_val}")
                if "exclusiveMinimum" in schema[k] and f_val <= schema[k]["exclusiveMinimum"]:
                    raise ValueError(f"Parameter '{k}' must be > {schema[k]['exclusiveMinimum']}, got {f_val}")
                raw[k] = f_val
            elif expected_type == "string":
                if not isinstance(val, str):
                    raise ValueError(f"Parameter '{k}' must be a string, got {type(val).__name__}")
                raw[k] = str(val)

        return raw

    @classmethod
    def calculate(
        cls,
        name: str,
        candles: List[CandleBar],
        params: Optional[Dict[str, Any]] = None,
        session_id: int = 0,
        observed_index: int = 0,
    ) -> SignalSeriesResult:
        """Calculate signal series for a list of daily candle bars."""
        defn = cls.get_definition(name)
        if not defn:
            raise ValueError(f"Unknown signal: '{name}'")

        resolved = cls.resolve_and_validate_params(name, params)
        params_hash = compute_canonical_params_hash(
            name, defn.version, resolved, allow_str=any(isinstance(v, str) for v in resolved.values())
        )

        # Dispatch
        if name == "volume.relative_volume":
            points = calculate_relative_volume(candles, period=resolved["period"])
        elif name == "volume.spike":
            points = calculate_volume_spike(candles, period=resolved["period"], multiplier=resolved["multiplier"])
        elif name == "volume.climax_up":
            points = calculate_volume_climax_up(
                candles,
                period=resolved["period"],
                multiplier=resolved["multiplier"],
                min_range_atr=resolved["min_range_atr"],
                min_close_location=resolved["min_close_location"],
            )
        elif name == "volume.climax_down":
            points = calculate_volume_climax_down(
                candles,
                period=resolved["period"],
                multiplier=resolved["multiplier"],
                min_range_atr=resolved["min_range_atr"],
                max_close_location=resolved["max_close_location"],
            )

        # Patterns
        elif name == "pattern.bullish_engulfing":
            points = calculate_bullish_engulfing(candles, min_body_ratio=resolved["min_body_ratio"], min_close_location=resolved["min_close_location"])
        elif name == "pattern.bearish_engulfing":
            points = calculate_bearish_engulfing(candles, min_body_ratio=resolved["min_body_ratio"], min_close_location=resolved["min_close_location"])
        elif name == "pattern.hammer":
            points = calculate_hammer(candles, wick_multiplier=resolved["wick_multiplier"])
        elif name == "pattern.shooting_star":
            points = calculate_shooting_star(candles, wick_multiplier=resolved["wick_multiplier"])
        elif name == "pattern.morning_star":
            points = calculate_morning_star(candles)
        elif name == "pattern.evening_star":
            points = calculate_evening_star(candles)
        elif name == "pattern.piercing_line":
            points = calculate_piercing_line(candles)
        elif name == "pattern.dark_cloud_cover":
            points = calculate_dark_cloud_cover(candles)
        elif name == "pattern.tweezer_bottom":
            points = calculate_tweezer_bottom(candles, tolerance_atr=resolved["tolerance_atr"])
        elif name == "pattern.tweezer_top":
            points = calculate_tweezer_top(candles, tolerance_atr=resolved["tolerance_atr"])
        elif name == "pattern.inside_bar_breakout_up":
            points = calculate_inside_bar_breakout_up(candles, breakout_buffer_atr=resolved["breakout_buffer_atr"])
        elif name == "pattern.inside_bar_breakout_down":
            points = calculate_inside_bar_breakout_down(candles, breakout_buffer_atr=resolved["breakout_buffer_atr"])
        elif name == "pattern.any_bullish":
            points = calculate_any_bullish_pattern(candles)
        elif name == "pattern.any_bearish":
            points = calculate_any_bearish_pattern(candles)

        # Technical
        elif name == "tech.macd_signal_cross":
            points = calculate_macd_signal_cross(candles, fast=resolved["fast"], slow=resolved["slow"], signal_period=resolved["signal_period"])
        elif name == "tech.macd_zero_cross":
            points = calculate_macd_zero_cross(candles, fast=resolved["fast"], slow=resolved["slow"])
        elif name == "tech.rsi_level_cross":
            points = calculate_rsi_level_cross(candles, length=resolved["length"], level=resolved["level"])
        elif name == "tech.ema_cross":
            points = calculate_ema_cross(candles, fast_period=resolved["fast_period"], slow_period=resolved["slow_period"])
        elif name == "tech.swing_break":
            points = calculate_swing_break(candles, swing_window=resolved["swing_window"], buffer_atr=resolved["buffer_atr"])
        elif name == "tech.composite":
            points = calculate_composite_technical_trigger(candles, minimum_confirmations=resolved["minimum_confirmations"])

        # Regimes
        elif name == "regime.uptrend":
            points = calculate_uptrend_regime(candles, fast_period=resolved["fast_period"], slow_period=resolved["slow_period"], slope_lookback=resolved["slope_lookback"])
        elif name == "regime.downtrend":
            points = calculate_downtrend_regime(candles, fast_period=resolved["fast_period"], slow_period=resolved["slow_period"], slope_lookback=resolved["slope_lookback"])
        elif name == "regime.sideways":
            points = calculate_sideways_regime(candles, ema_period=resolved["ema_period"], slope_threshold=resolved["slope_threshold"])
        elif name == "regime.pullback":
            points = calculate_pullback_regime(candles, lookback=resolved["lookback"], min_drawdown=resolved["min_drawdown"], max_drawdown=resolved["max_drawdown"])
        elif name == "regime.recovery":
            points = calculate_recovery_regime(candles, lookback=resolved["lookback"])
        elif name == "regime.new_high":
            points = calculate_new_high(candles, period=resolved["period"], buffer=resolved["buffer"])
        elif name == "regime.new_low":
            points = calculate_new_low(candles, period=resolved["period"], buffer=resolved["buffer"])

        # Support & Resistance
        elif name == "sr.near_support":
            points = calculate_near_support(candles, period=resolved["period"], tolerance_atr=resolved["tolerance_atr"])
        elif name == "sr.near_resistance":
            points = calculate_near_resistance(candles, period=resolved["period"], tolerance_atr=resolved["tolerance_atr"])

        # Candle Structure
        elif name == "structure.candle_score":
            points = calculate_candle_structure_score(
                candles,
                w_direction=resolved["w_direction"],
                w_close=resolved["w_close"],
                w_wick=resolved["w_wick"],
            )

        # VSA
        elif name == "vsa.strong_demand":
            points = calculate_strong_demand(
                candles,
                period=resolved["period"],
                min_rvol=resolved["min_rvol"],
                min_range_atr=resolved["min_range_atr"],
                min_body_ratio=resolved["min_body_ratio"],
                min_close_location=resolved["min_close_location"],
            )
        elif name == "vsa.weak_demand":
            points = calculate_weak_demand(
                candles,
                period=resolved["period"],
                max_rvol=resolved["max_rvol"],
                max_range_atr=resolved["max_range_atr"],
            )
        elif name == "vsa.upthrust":
            points = calculate_upthrust(
                candles,
                period=resolved["period"],
                min_rvol=resolved["min_rvol"],
                min_upper_wick_ratio=resolved["min_upper_wick_ratio"],
                max_close_location=resolved["max_close_location"],
                breach_buffer_atr=resolved["breach_buffer_atr"],
            )
        elif name == "vsa.spring":
            points = calculate_spring(
                candles,
                period=resolved["period"],
                min_rvol=resolved["min_rvol"],
                min_lower_wick_ratio=resolved["min_lower_wick_ratio"],
                min_close_location=resolved["min_close_location"],
                breach_buffer_atr=resolved["breach_buffer_atr"],
            )
        elif name == "vsa.strong_demand_at_support":
            points = calculate_strong_demand_at_support(
                candles,
                period=resolved["period"],
                tolerance_atr=resolved["tolerance_atr"],
                min_rvol=resolved["min_rvol"],
            )
        elif name == "vsa.strong_supply_at_resistance":
            points = calculate_strong_supply_at_resistance(
                candles,
                period=resolved["period"],
                tolerance_atr=resolved["tolerance_atr"],
                min_rvol=resolved["min_rvol"],
            )
        elif name == "ichimoku.score":
            points = calculate_ichimoku_score(
                candles,
                tenkan_period=resolved["tenkan_period"],
                kijun_period=resolved["kijun_period"],
                senkou_period=resolved["senkou_period"],
                displacement=resolved["displacement"],
                include_kijun_slope=bool(resolved["include_kijun_slope"]),
            )
        elif name == "ichimoku.bullish":
            points = calculate_ichimoku_bullish(
                candles,
                tenkan_period=resolved["tenkan_period"],
                kijun_period=resolved["kijun_period"],
                senkou_period=resolved["senkou_period"],
                displacement=resolved["displacement"],
                include_kijun_slope=bool(resolved["include_kijun_slope"]),
                threshold=resolved["threshold"],
            )
        elif name == "ichimoku.bearish":
            points = calculate_ichimoku_bearish(
                candles,
                tenkan_period=resolved["tenkan_period"],
                kijun_period=resolved["kijun_period"],
                senkou_period=resolved["senkou_period"],
                displacement=resolved["displacement"],
                include_kijun_slope=bool(resolved["include_kijun_slope"]),
                threshold=resolved["threshold"],
            )
        elif name == "ichimoku.tk_cross_bullish":
            points = calculate_ichimoku_tk_cross_bullish(
                candles,
                tenkan_period=resolved["tenkan_period"],
                kijun_period=resolved["kijun_period"],
            )
        elif name == "ichimoku.tk_cross_bearish":
            points = calculate_ichimoku_tk_cross_bearish(
                candles,
                tenkan_period=resolved["tenkan_period"],
                kijun_period=resolved["kijun_period"],
            )
        elif name == "ichimoku.kumo_breakout_bullish":
            points = calculate_ichimoku_kumo_breakout_bullish(
                candles,
                tenkan_period=resolved["tenkan_period"],
                kijun_period=resolved["kijun_period"],
                senkou_period=resolved["senkou_period"],
                displacement=resolved["displacement"],
            )
        elif name == "ichimoku.kumo_breakout_bearish":
            points = calculate_ichimoku_kumo_breakout_bearish(
                candles,
                tenkan_period=resolved["tenkan_period"],
                kijun_period=resolved["kijun_period"],
                senkou_period=resolved["senkou_period"],
                displacement=resolved["displacement"],
            )
        elif name == "divergence.confirmed_pivot_high":
            points = calculate_confirmed_pivot_high(
                candles,
                left_bars=resolved["left_bars"],
                right_bars=resolved["right_bars"],
            )
        elif name == "divergence.confirmed_pivot_low":
            points = calculate_confirmed_pivot_low(
                candles,
                left_bars=resolved["left_bars"],
                right_bars=resolved["right_bars"],
            )
        elif name == "divergence.rsi_regular_bullish":
            points = calculate_rsi_regular_bullish(
                candles,
                rsi_period=resolved["rsi_period"],
                left_bars=resolved["left_bars"],
                right_bars=resolved["right_bars"],
                min_separation=resolved["min_separation"],
                max_separation=resolved["max_separation"],
            )
        elif name == "divergence.rsi_regular_bearish":
            points = calculate_rsi_regular_bearish(
                candles,
                rsi_period=resolved["rsi_period"],
                left_bars=resolved["left_bars"],
                right_bars=resolved["right_bars"],
                min_separation=resolved["min_separation"],
                max_separation=resolved["max_separation"],
            )
        elif name == "divergence.rsi_hidden_bullish":
            points = calculate_rsi_hidden_bullish(
                candles,
                rsi_period=resolved["rsi_period"],
                left_bars=resolved["left_bars"],
                right_bars=resolved["right_bars"],
                min_separation=resolved["min_separation"],
                max_separation=resolved["max_separation"],
            )
        elif name == "divergence.rsi_hidden_bearish":
            points = calculate_rsi_hidden_bearish(
                candles,
                rsi_period=resolved["rsi_period"],
                left_bars=resolved["left_bars"],
                right_bars=resolved["right_bars"],
                min_separation=resolved["min_separation"],
                max_separation=resolved["max_separation"],
            )
        elif name == "divergence.macd_regular_bullish":
            points = calculate_macd_regular_bullish(
                candles,
                fast=resolved["fast"],
                slow=resolved["slow"],
                signal_period=resolved["signal_period"],
                left_bars=resolved["left_bars"],
                right_bars=resolved["right_bars"],
                min_separation=resolved["min_separation"],
                max_separation=resolved["max_separation"],
            )
        elif name == "divergence.macd_regular_bearish":
            points = calculate_macd_regular_bearish(
                candles,
                fast=resolved["fast"],
                slow=resolved["slow"],
                signal_period=resolved["signal_period"],
                left_bars=resolved["left_bars"],
                right_bars=resolved["right_bars"],
                min_separation=resolved["min_separation"],
                max_separation=resolved["max_separation"],
            )
        elif name == "divergence.macd_hidden_bullish":
            points = calculate_macd_hidden_bullish(
                candles,
                fast=resolved["fast"],
                slow=resolved["slow"],
                signal_period=resolved["signal_period"],
                left_bars=resolved["left_bars"],
                right_bars=resolved["right_bars"],
                min_separation=resolved["min_separation"],
                max_separation=resolved["max_separation"],
            )
        elif name == "divergence.macd_hidden_bearish":
            points = calculate_macd_hidden_bearish(
                candles,
                fast=resolved["fast"],
                slow=resolved["slow"],
                signal_period=resolved["signal_period"],
                left_bars=resolved["left_bars"],
                right_bars=resolved["right_bars"],
                min_separation=resolved["min_separation"],
                max_separation=resolved["max_separation"],
            )
        elif name == "divergence.stoch_regular_bullish":
            points = calculate_stoch_regular_bullish(
                candles,
                k_period=resolved["k_period"],
                left_bars=resolved["left_bars"],
                right_bars=resolved["right_bars"],
                min_separation=resolved["min_separation"],
                max_separation=resolved["max_separation"],
            )
        elif name == "divergence.stoch_regular_bearish":
            points = calculate_stoch_regular_bearish(
                candles,
                k_period=resolved["k_period"],
                left_bars=resolved["left_bars"],
                right_bars=resolved["right_bars"],
                min_separation=resolved["min_separation"],
                max_separation=resolved["max_separation"],
            )
        elif name == "divergence.stoch_hidden_bullish":
            points = calculate_stoch_hidden_bullish(
                candles,
                k_period=resolved["k_period"],
                left_bars=resolved["left_bars"],
                right_bars=resolved["right_bars"],
                min_separation=resolved["min_separation"],
                max_separation=resolved["max_separation"],
            )
        elif name == "divergence.stoch_hidden_bearish":
            points = calculate_stoch_hidden_bearish(
                candles,
                k_period=resolved["k_period"],
                left_bars=resolved["left_bars"],
                right_bars=resolved["right_bars"],
                min_separation=resolved["min_separation"],
                max_separation=resolved["max_separation"],
            )

        # Technical Health
        elif name == "health.score":
            points = calculate_technical_health_score(
                candles,
                preset=resolved["preset"],
            )
        elif name == "health.favorable":
            points = calculate_technical_health_favorable(
                candles,
                favorable_threshold=resolved["favorable_threshold"],
            )
        elif name == "health.unfavorable":
            points = calculate_technical_health_unfavorable(
                candles,
                unfavorable_threshold=resolved["unfavorable_threshold"],
            )

        # Money Flow BB Events
        elif name == "bb.direction_rising":
            points = calculate_bb_direction_rising(
                candles,
                horizon=resolved["horizon"],
                min_quality=resolved["min_quality"],
            )
        elif name == "bb.direction_falling":
            points = calculate_bb_direction_falling(
                candles,
                horizon=resolved["horizon"],
                min_quality=resolved["min_quality"],
            )
        elif name == "bb.regime_positive":
            points = calculate_bb_regime_positive(
                candles,
                horizon=resolved["horizon"],
                min_quality=resolved["min_quality"],
            )
        elif name == "bb.regime_negative":
            points = calculate_bb_regime_negative(
                candles,
                horizon=resolved["horizon"],
                min_quality=resolved["min_quality"],
            )
        elif name == "bb.confluence_bullish":
            points = calculate_bb_confluence_bullish(
                candles,
                short_horizon=resolved["short_horizon"],
                long_horizon=resolved["long_horizon"],
                min_quality=resolved["min_quality"],
            )
        elif name == "bb.confluence_bearish":
            points = calculate_bb_confluence_bearish(
                candles,
                short_horizon=resolved["short_horizon"],
                long_horizon=resolved["long_horizon"],
                min_quality=resolved["min_quality"],
            )
        elif name == "bb.turn_up":
            points = calculate_bb_turn_up(
                candles,
                threshold=resolved["threshold"],
                tolerance=resolved["tolerance"],
                horizon=resolved["horizon"],
                min_quality=resolved["min_quality"],
            )
        elif name == "bb.turn_down":
            points = calculate_bb_turn_down(
                candles,
                threshold=resolved["threshold"],
                tolerance=resolved["tolerance"],
                horizon=resolved["horizon"],
                min_quality=resolved["min_quality"],
            )

        else:
            raise ValueError(f"No calculation handler registered for '{name}'")


        return SignalSeriesResult(
            session_id=session_id,
            observed_index=observed_index,
            signal_name=name,
            signal_version=defn.version,
            resolved_params=resolved,
            params_hash=params_hash,
            points=points,
        )
