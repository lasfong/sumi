import pandas as pd
import pandas_ta as ta

class IndicatorEngine:
    """
    Wrapper for pandas-ta to compute technical indicators.
    """
    @staticmethod
    def compute(df: pd.DataFrame, indicator: str, **kwargs) -> pd.DataFrame:
        """
        Compute an indicator and append it to the DataFrame.
        Returns a new DataFrame with the computed columns.
        """
        # Ensure DataFrame has lowercase columns required by pandas-ta
        # and a DatetimeIndex
        df_copy = df.copy()
        
        # If there's a timestamp column, we might want to set it as index, 
        # but pandas_ta usually works with columns 'open', 'high', 'low', 'close', 'volume'
        
        # Convert indicator to lowercase
        indicator = indicator.lower()
        
        try:
            # Dynamically call the pandas-ta method on the dataframe
            # Strategy or direct method call
            result = getattr(df_copy.ta, indicator)(**kwargs)
            
            if isinstance(result, tuple):
                for res_df in result:
                    if isinstance(res_df, pd.DataFrame):
                        for col in res_df.columns:
                            df_copy[col] = res_df[col]
                    elif isinstance(res_df, pd.Series):
                        df_copy[res_df.name] = res_df
            elif isinstance(result, pd.DataFrame):
                for col in result.columns:
                    df_copy[col] = result[col]
            elif isinstance(result, pd.Series):
                df_copy[result.name] = result
            else:
                # Fallback, try to see if append=True worked silently
                pass
                
            return df_copy
        except AttributeError:
            raise ValueError(f"Indicator '{indicator}' is not supported or not found in pandas-ta.")
        except Exception as e:
            raise RuntimeError(f"Error computing indicator '{indicator}': {e}")
