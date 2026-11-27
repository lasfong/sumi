-- =============================================================================
-- PROPOSAL ONLY — Doraemi Rust (`doraemi-db`) Migration 0006: Sumi v3.0 Sync
-- =============================================================================
-- NOTE: Per architectural boundary Decision #1, `/Users/mizuhara/workspace/Doraemi`
-- remains 100% Read-Only during Sumi v3.0 implementation. When you choose to enable
-- direct cloud sync of Sumi workspaces & KT01 verdicts through Doraemi `/sync/v1`,
-- apply this additive migration to `doraemon_ai` and add the two new `entity_kind`
-- values (`sumi_workspace_bundle`, `sumi_kt01_verdict`) to `doraemi-db/src/sync.rs`.
--
-- Strictly isolated inside `doraemon_ai`. Zero mutations to `public`, `market`,
-- `personal`, or existing 9 MCP tool schemas (`tool_schema_sha256` unchanged).
-- =============================================================================

ALTER TABLE yumeko_sync_revisions
    DROP CONSTRAINT IF EXISTS chk_yumeko_sync_entity_kind;

ALTER TABLE yumeko_sync_revisions
    ADD CONSTRAINT chk_yumeko_sync_entity_kind CHECK (
        entity_kind IN (
            -- Existing 18 Yumeko v3.0 entity kinds (preserved 100%):
            'accepted_report',
            'regime_journal',
            'symbol_thesis',
            'thesis_outcome',
            'distilled_lesson',
            'qmv_daily_digest',
            'qmv_market_radar',
            'qmv_symbol_blackbox_batch',
            'qmv_compact_schema',
            'qmv_compact_batch',
            'qmv_screener_run',
            'symbol_valuation_batch',
            'watchlist',
            'macro_policy_state',
            'macro_observations',
            'chat_session_bundle',
            'parity_ledger',
            'cutover_settings',
            -- New additive Sumi v3.0 KT01 workstation entity kinds:
            'sumi_workspace_bundle',
            'sumi_kt01_verdict'
        )
    );
