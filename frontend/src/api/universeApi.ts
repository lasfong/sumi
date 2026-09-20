import { apiClient } from './client';

export interface UniverseMember {
  symbol: string;
  exchange: string;
  company_name?: string | null;
  sector?: string | null;
  effective_from?: string | null;
  effective_to?: string | null;
  status: string;
  inclusion_reason?: string;
  weight?: number;
  metadata?: Record<string, unknown>;
}

export interface UniverseSummary {
  universe_id: string;
  version: string;
  name: string;
  status: string;
  default_mode: string;
  effective_from?: string | null;
  effective_to?: string | null;
  selection_purpose: string;
  source_evidence: string;
  total_member_count: number;
  active_member_count: number;
  content_hash: string;
}

export interface UniverseDetail extends UniverseSummary {
  selection_rules: string[];
  members: UniverseMember[];
  metadata: Record<string, unknown>;
}

export interface UniverseResolution {
  universe_id: string;
  version: string;
  as_of: string;
  mode: string;
  is_point_in_time: boolean;
  active_symbols: string[];
  excluded_symbols: string[];
  active_count: number;
  total_target_count: number;
  content_hash: string;
  survivor_bias_caveat?: string | null;
}

/**
 * List all registered universe definitions.
 */
export async function listUniverses(): Promise<UniverseSummary[]> {
  const response = await apiClient.get('/universe/list');
  return response.data;
}

/**
 * Get full specification and constituent members for a universe.
 */
export async function getUniverseDefinition(universeId: string, version?: string): Promise<UniverseDetail> {
  const params = version ? { version } : {};
  const response = await apiClient.get(`/universe/${universeId}/definition`, { params });
  return response.data;
}

/**
 * Resolve active constituents of a universe as of a specific date.
 */
export async function resolveUniverse(
  universeId: string,
  asOf: string,
  version?: string,
  mode?: string
): Promise<UniverseResolution> {
  const params: Record<string, string> = { as_of: asOf };
  if (version) params.version = version;
  if (mode) params.mode = mode;
  const response = await apiClient.get(`/universe/${universeId}/resolve`, { params });
  return response.data;
}
