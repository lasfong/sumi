/**
 * Signal domain and API contract types for Sumi.
 */

export type SignalQuality =
  | 'VALID'
  | 'INSUFFICIENT_HISTORY'
  | 'INVALID_VOLUME'
  | 'ZERO_BASELINE';

export type SignalOutputType = 'bool' | 'float' | 'enum';

export type SignalStatus = 'ACTIVE' | 'EXPERIMENTAL' | 'DEPRECATED';

export interface SignalParameterSchema {
  type: 'int' | 'float';
  default: number;
  minimum?: number;
  exclusiveMinimum?: number;
  maximum?: number;
  description: string;
}

export interface SignalDefinition {
  name: string;
  version: string;
  category: string;
  label_vi: string;
  description: string;
  output_type: SignalOutputType;
  parameters_schema: Record<string, SignalParameterSchema>;
  default_parameters: Record<string, number>;
  dependencies: string[];
  warmup_bars: number;
  causal_delay_bars: number;
  status: SignalStatus;
  ast_alias: string;
}

export interface SignalRegistryResponse {
  signals: SignalDefinition[];
}

export interface SignalCalculationItemRequest {
  name: string;
  version?: string;
  params?: Record<string, number>;
}

export interface SignalCalculationRequest {
  signals: SignalCalculationItemRequest[];
}

export interface SignalOutputPoint {
  bar_index: number;
  timestamp: string;
  output_type: SignalOutputType;
  value: boolean | number | string | null;
  quality: SignalQuality;
  reasons: string[];
  baseline: number | null;
  current_volume: number | null;
  relative_volume: number | null;
  threshold: number | null;
  availability_event: string;
  available_at_index: number;
  available_at_timestamp: string;
}

export interface SignalCalculationSeriesResponse {
  signal_name: string;
  signal_version: string;
  resolved_params: Record<string, number>;
  params_hash: string;
  points: SignalOutputPoint[];
}

export interface SignalCalculationResponse {
  session_id: number;
  observed_current_index: number;
  timeframe: string;
  results: SignalCalculationSeriesResponse[];
}

export type CalculateSignalsResponse = SignalCalculationResponse;
