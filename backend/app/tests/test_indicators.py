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

def test_mfi_calculation():
    df = create_sample_data(100)
    result_df = IndicatorEngine.compute(df, 'mfi', length=14)
    assert 'MFI_14' in result_df.columns
    assert pd.isna(result_df['MFI_14'].iloc[0])
    last_mfi = result_df['MFI_14'].iloc[-1]
    assert 0 <= last_mfi <= 100

def test_stoch_calculation():
    df = create_sample_data(100)
    result_df = IndicatorEngine.compute(df, 'stoch', k=14, d=3, smooth_k=3)
    assert 'STOCHk_14_3_3' in result_df.columns
    assert 'STOCHd_14_3_3' in result_df.columns
    assert pd.isna(result_df['STOCHk_14_3_3'].iloc[0])
    last_k = result_df['STOCHk_14_3_3'].iloc[-1]
    last_d = result_df['STOCHd_14_3_3'].iloc[-1]
    assert 0 <= last_k <= 100
    assert 0 <= last_d <= 100

def test_adx_calculation():
    df = create_sample_data(100)
    result_df = IndicatorEngine.compute(df, 'adx', length=14)
    assert 'ADX_14' in result_df.columns
    assert 'DMP_14' in result_df.columns
    assert 'DMN_14' in result_df.columns
    assert pd.isna(result_df['ADX_14'].iloc[0])
    last_adx = result_df['ADX_14'].iloc[-1]
    assert last_adx >= 0

def test_relative_strength_calculation():
    dates = [datetime(2024, 1, 1) + timedelta(days=i) for i in range(30)]
    df = pd.DataFrame({
        'open': [100.0 + i for i in range(30)],
        'high': [105.0 + i for i in range(30)],
        'low': [95.0 + i for i in range(30)],
        'close': [100.0 + i for i in range(30)],
        'volume': [1000 + i for i in range(30)],
    }, index=dates)

    bench_df = pd.DataFrame({
        'close': [1000.0 + i * 5 for i in range(30)],
    }, index=dates)

    result_df = IndicatorEngine.compute(df, 'relative_strength', length=10, benchmark_df=bench_df)
    assert 'RS_VNINDEX_10' in result_df.columns
    # Warm up period: first 10 rows must be NaN
    for i in range(10):
        assert pd.isna(result_df['RS_VNINDEX_10'].iloc[i])

    # Day 10 calculation: symbol rose from 100 to 110 (+10%), bench rose from 1000 to 1050 (+5%)
    expected_day10 = (110.0 / 100.0) / (1050.0 / 1000.0) * 100.0
    assert result_df['RS_VNINDEX_10'].iloc[10] == pytest.approx(expected_day10)

def test_relative_strength_missing_benchmark_and_dates():
    dates = [datetime(2024, 1, 1) + timedelta(days=i) for i in range(30)]
    df = pd.DataFrame({
        'open': [100.0 + i for i in range(30)],
        'high': [105.0 + i for i in range(30)],
        'low': [95.0 + i for i in range(30)],
        'close': [100.0 + i for i in range(30)],
        'volume': [1000 + i for i in range(30)],
    }, index=dates)

    # Empty benchmark returns column filled with NaNs
    res_empty = IndicatorEngine.compute(df, 'relative_strength', length=10, benchmark_df=None)
    assert 'RS_VNINDEX_10' in res_empty.columns
    assert res_empty['RS_VNINDEX_10'].isna().all()

    # Benchmark missing some dates produces NaNs for missing dates without crashing
    bench_dates = [dates[i] for i in range(30) if i != 15]
    bench_df = pd.DataFrame({'close': [1000.0 + i * 5 for i in range(len(bench_dates))]}, index=bench_dates)
    res_gaps = IndicatorEngine.compute(df, 'relative_strength', length=10, benchmark_df=bench_df)
    assert 'RS_VNINDEX_10' in res_gaps.columns
    assert pd.isna(res_gaps['RS_VNINDEX_10'].iloc[15])
