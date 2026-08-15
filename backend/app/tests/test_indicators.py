import pytest
import pandas as pd
import numpy as np
from datetime import datetime, timedelta
from app.domain.engine.indicator_engine import IndicatorEngine

def create_sample_data(num_rows=100):
    np.random.seed(42)
    dates = [datetime.now() - timedelta(days=x) for x in range(num_rows)]
    dates.reverse()
    
    # Generate random walk for close
    close_prices = 100 + np.random.randn(num_rows).cumsum()
    
    df = pd.DataFrame({
        'timestamp': dates,
        'open': close_prices + np.random.randn(num_rows) * 0.5,
        'high': close_prices + np.abs(np.random.randn(num_rows)),
        'low': close_prices - np.abs(np.random.randn(num_rows)),
        'close': close_prices,
        'volume': np.random.randint(1000, 10000, num_rows)
    })
    df.set_index('timestamp', inplace=True)
    return df

def test_rsi_calculation():
    df = create_sample_data(100)
    
    # Calculate RSI
    result_df = IndicatorEngine.compute(df, 'rsi', length=14)
    
    # Check if RSI column was added
    assert 'RSI_14' in result_df.columns
    
    # The first 14 values should be NaN
    assert pd.isna(result_df['RSI_14'].iloc[0])
    
    # The last value should be a valid float between 0 and 100
    last_rsi = result_df['RSI_14'].iloc[-1]
    assert 0 <= last_rsi <= 100

def test_macd_calculation():
    df = create_sample_data(100)
    
    # Calculate MACD
    result_df = IndicatorEngine.compute(df, 'macd')
    
    # Check if MACD columns were added (MACD, MACDh, MACDs)
    macd_cols = [col for col in result_df.columns if 'MACD' in col]
    assert len(macd_cols) == 3

def test_indicator_registry_contains_v2_core_indicators():
    definitions = IndicatorEngine.list_definitions()
    ids = {definition["id"] for definition in definitions}

    assert {"macd", "rsi", "ichimoku", "bbands", "atr", "adx", "stoch", "volume_sma"}.issubset(ids)

def test_sma_calculation():
    df = create_sample_data(100)
    result_df = IndicatorEngine.compute(df, 'sma', length=20)
    assert 'SMA_20' in result_df.columns
    assert pd.isna(result_df['SMA_20'].iloc[18])
    assert result_df['SMA_20'].iloc[19] == pytest.approx(df['close'].iloc[:20].mean())
    assert not pd.isna(result_df['SMA_20'].iloc[-1])

def test_bbands_calculation():
    df = create_sample_data(100)
    result_df = IndicatorEngine.compute(df, 'bbands', length=20, std=2.0)
    assert 'BBU_20_2.0_2.0' in result_df.columns
    assert 'BBM_20_2.0_2.0' in result_df.columns
    assert 'BBL_20_2.0_2.0' in result_df.columns
    assert pd.isna(result_df['BBU_20_2.0_2.0'].iloc[18])
    last_upper = result_df['BBU_20_2.0_2.0'].iloc[-1]
    last_mid = result_df['BBM_20_2.0_2.0'].iloc[-1]
    last_lower = result_df['BBL_20_2.0_2.0'].iloc[-1]
    assert last_upper > last_mid > last_lower

def test_bbands_non_default_std_calculation():
    df = create_sample_data(100)
    res_default = IndicatorEngine.compute(df, 'bbands', length=20, std=2.0)
    res_225 = IndicatorEngine.compute(df, 'bbands', length=20, std=2.25)
    res_115 = IndicatorEngine.compute(df, 'bbands', length=20, std=1.15)

    assert 'BBU_20_2.25_2.25' in res_225.columns
    assert 'BBM_20_2.25_2.25' in res_225.columns
    assert 'BBL_20_2.25_2.25' in res_225.columns
    assert 'BBU_20_1.15_1.15' in res_115.columns
    assert 'BBM_20_1.15_1.15' in res_115.columns
    assert 'BBL_20_1.15_1.15' in res_115.columns

    # Values must be materially distinct from default std=2.0
    upper_default = res_default['BBU_20_2.0_2.0'].iloc[-1]
    lower_default = res_default['BBL_20_2.0_2.0'].iloc[-1]
    mid_default = res_default['BBM_20_2.0_2.0'].iloc[-1]

    upper_225 = res_225['BBU_20_2.25_2.25'].iloc[-1]
    lower_225 = res_225['BBL_20_2.25_2.25'].iloc[-1]
    mid_225 = res_225['BBM_20_2.25_2.25'].iloc[-1]

    upper_115 = res_115['BBU_20_1.15_1.15'].iloc[-1]
    lower_115 = res_115['BBL_20_1.15_1.15'].iloc[-1]
    mid_115 = res_115['BBM_20_1.15_1.15'].iloc[-1]

    assert upper_225 > upper_default > upper_115
    assert lower_225 < lower_default < lower_115
    assert mid_225 == pytest.approx(mid_default)
    assert mid_115 == pytest.approx(mid_default)

def test_atr_calculation():
    df = create_sample_data(100)
    result_df = IndicatorEngine.compute(df, 'atr', length=14)
    atr_col = [c for c in result_df.columns if c.startswith('ATRr_') or c.startswith('ATR_')][0]
    assert atr_col
    assert pd.isna(result_df[atr_col].iloc[12])
    last_atr = result_df[atr_col].iloc[-1]
    assert last_atr > 0

def test_volume_sma_calculation():
    df = create_sample_data(30)

    result_df = IndicatorEngine.compute(df, 'vma', length=5)

    assert 'VOLUME_SMA_5' in result_df.columns
    assert result_df['VOLUME_SMA_5'].iloc[-1] == pytest.approx(df['volume'].tail(5).mean())

def test_cci_uses_canonical_centered_formula():
    df = create_sample_data(60)
    result_df = IndicatorEngine.compute(df, 'cci', length=20)

    typical_price = (df['high'] + df['low'] + df['close']) / 3
    mean = typical_price.rolling(20).mean()
    deviation = typical_price.rolling(20).apply(
        lambda values: abs(values - values.mean()).mean(),
        raw=False,
    )
    expected = (typical_price - mean) / (0.015 * deviation)

    assert result_df['CCI_20_0.015'].iloc[-1] == pytest.approx(expected.iloc[-1])
    assert abs(result_df['CCI_20_0.015'].iloc[-1]) < 1000

def test_reject_unknown_indicator_param():
    df = create_sample_data()

    with pytest.raises(ValueError, match="Unsupported parameter"):
        IndicatorEngine.compute(df, 'rsi', length=14, unsafe=True)
    
def test_invalid_indicator():
    df = create_sample_data()
    
    with pytest.raises(ValueError, match="Indicator 'invalid_ind' is not supported"):
        IndicatorEngine.compute(df, 'invalid_ind')
