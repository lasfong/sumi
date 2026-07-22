import type { DecisionAction, DecisionCreate, JournalEntry, JournalEntryCreate, PracticeWorkflowSnapshot } from '../../types';

export const CHECKLIST_FIELDS = [
  'trendIdentified', 'setupConfirmed', 'entryTriggerDefined',
  'riskDefined', 'exitPlanDefined', 'emotionChecked',
] as const;

export type ChecklistField = typeof CHECKLIST_FIELDS[number];
export type ChecklistChecks = Record<ChecklistField, boolean>;

export interface PracticeChecklistV1 {
  schemaVersion: 1;
  context: { sessionId: number; symbol: string; candleIndex: number; date: string };
  checks: ChecklistChecks;
  observation: string;
}

export const emptyChecklistChecks = (): ChecklistChecks => ({
  trendIdentified: false,
  setupConfirmed: false,
  entryTriggerDefined: false,
  riskDefined: false,
  exitPlanDefined: false,
  emotionChecked: false,
});

export const createChecklistEntry = (
  snapshot: PracticeWorkflowSnapshot,
  checks: ChecklistChecks,
  observation: string,
  association?: { decisionId?: number; tradeId?: number },
): JournalEntryCreate => ({
  note_type: 'practice_checklist',
  content: JSON.stringify({
    schemaVersion: 1,
    context: {
      sessionId: snapshot.session_id,
      symbol: snapshot.symbol,
      candleIndex: snapshot.current_index,
      date: snapshot.current_date.slice(0, 10),
    },
    checks,
    observation: observation.trim(),
  } satisfies PracticeChecklistV1),
  decision_id: association?.decisionId,
  trade_id: association?.tradeId,
});

export const parseChecklist = (entry: JournalEntry): PracticeChecklistV1 | null => {
  if (entry.note_type !== 'practice_checklist') return null;
  try {
    const value = JSON.parse(entry.content) as Partial<PracticeChecklistV1>;
    if (value.schemaVersion !== 1 || !value.context || !value.checks || typeof value.observation !== 'string') return null;
    if (CHECKLIST_FIELDS.some(field => typeof value.checks?.[field] !== 'boolean')) return null;
    return value as PracticeChecklistV1;
  } catch {
    return null;
  }
};

const tradingActions = new Set<DecisionAction>(['BUY', 'SELL', 'ADD', 'REDUCE']);

export const validateDecision = (
  decision: DecisionCreate,
  snapshot: PracticeWorkflowSnapshot,
): string | null => {
  if (!snapshot.can_trade) return snapshot.trade_block_reason || 'Historical replay view is read-only.';
  if (tradingActions.has(decision.action)) {
    if (!Number.isFinite(decision.quantity) || (decision.quantity ?? 0) <= 0) return 'Quantity must be greater than zero.';
    if (decision.order_type === 'LIMIT' && (!Number.isFinite(decision.price) || (decision.price ?? 0) <= 0)) return 'Limit price must be greater than zero.';
  }
  if (['SELL', 'REDUCE'].includes(decision.action) && (decision.quantity ?? 0) > (snapshot.positions[0]?.quantity ?? 0)) {
    return 'Sell quantity exceeds the projected open position.';
  }
  if (decision.action === 'CLOSE' && !snapshot.positions.length) return 'There is no open position to close.';
  if (decision.stop_loss !== undefined && (!Number.isFinite(decision.stop_loss) || decision.stop_loss <= 0)) return 'Stop loss must be greater than zero.';
  if (decision.target_price !== undefined && (!Number.isFinite(decision.target_price) || decision.target_price <= 0)) return 'Target price must be greater than zero.';
  const reference = decision.price ?? snapshot.current_price;
  if (['BUY', 'ADD'].includes(decision.action)) {
    if (decision.stop_loss !== undefined && decision.stop_loss >= reference) return 'A long stop loss must be below the entry price.';
    if (decision.target_price !== undefined && decision.target_price <= reference) return 'A long target must be above the entry price.';
  }
  return null;
};
