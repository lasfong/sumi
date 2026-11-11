/** Stock symbol / index from the backend */
export interface StockSymbol {
  symbol: string;
  exchange?: string;
  company_name?: string;
  sector?: string;
  industry?: string;
  asset_type: string;
  is_active: boolean;
}
