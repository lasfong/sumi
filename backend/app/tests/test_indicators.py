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

def test_volume_sma_calculation():
    df = create_sample_data(30)

    result_df = IndicatorEngine.compute(df, 'vma', length=5)

    assert 'VOLUME_SMA_5' in result_df.columns
    assert result_df['VOLUME_SMA_5'].iloc[-1] == pytest.approx(df['volume'].tail(5).mean())

def test_reject_unknown_indicator_param():
    df = create_sample_data()

    with pytest.raises(ValueError, match="Unsupported parameter"):
        IndicatorEngine.compute(df, 'rsi', length=14, unsafe=True)
    
def test_invalid_indicator():
    df = create_sample_data()
    
    with pytest.raises(ValueError, match="Indicator 'invalid_ind' is not supported"):
        IndicatorEngine.compute(df, 'invalid_ind')
