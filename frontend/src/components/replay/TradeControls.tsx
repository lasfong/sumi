import React, { useCallback, useEffect, useRef, useState } from 'react';
import type { DecisionAction, DecisionCreate, PracticeWorkflowSnapshot } from '../../types';
import { validateDecision } from '../../features/practice/practiceDomain';
import { useModalFocus } from '../../hooks/useModalFocus';

export interface SubmissionResult { ok: boolean; message: string }

interface TradeControlsProps {
  snapshot: PracticeWorkflowSnapshot;
  onSubmitDecision: (decision: DecisionCreate) => Promise<SubmissionResult>;
  disabled?: boolean;
}

const SETUP_TYPES = ['Breakout', 'Pullback', 'Reversal', 'Trend Follow', 'Range Play', 'Accumulation', 'Distribution'];

export const TradeControls: React.FC<TradeControlsProps> = ({ snapshot, onSubmitDecision, disabled }) => {
  const [limitedViewport, setLimitedViewport] = useState(() =>
    typeof window !== 'undefined' && typeof window.matchMedia === 'function'
      ? window.matchMedia('(max-width: 1179px)').matches
      : false,
  );
  const [pendingAction, setPendingAction] = useState<DecisionAction | null>(null);
  const [quantity, setQuantity] = useState('100');
  const [orderType, setOrderType] = useState('MARKET_AT_CLOSE');
  const [limitPrice, setLimitPrice] = useState('');
  const [stopLoss, setStopLoss] = useState('');
  const [targetPrice, setTargetPrice] = useState('');
  const [setupType, setSetupType] = useState('');
  const [reason, setReason] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const submittingRef = useRef(false);
  const [feedback, setFeedback] = useState<SubmissionResult | null>(null);
  const [dialogElement, setDialogElement] = useState<HTMLDivElement | null>(null);

  useEffect(() => {
    if (typeof window.matchMedia !== 'function') return undefined;
    const query = window.matchMedia('(max-width: 1179px)');
    const update = () => setLimitedViewport(query.matches);
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }, []);

  const open = useCallback((action: DecisionAction) => {
    setPendingAction(action); setQuantity('100'); setOrderType('MARKET_AT_CLOSE'); setLimitPrice('');
    setStopLoss(''); setTargetPrice(''); setSetupType(''); setReason(''); setFeedback(null);
  }, []);

  const close = useCallback(() => { if (!submitting) setPendingAction(null); }, [submitting]);
  const isTradeAction = pendingAction !== null && ['BUY', 'SELL', 'ADD', 'REDUCE'].includes(pendingAction);

  const submit = useCallback(async () => {
    if (!pendingAction || submittingRef.current) return;
    const decision: DecisionCreate = {
      action: pendingAction,
      quantity: isTradeAction && quantity !== '' ? Number(quantity) : undefined,
      order_type: isTradeAction ? orderType : 'MARKET_AT_CLOSE',
      price: isTradeAction && orderType === 'LIMIT' && limitPrice !== '' ? Number(limitPrice) : undefined,
      stop_loss: isTradeAction && stopLoss !== '' ? Number(stopLoss) : undefined,
      target_price: isTradeAction && targetPrice !== '' ? Number(targetPrice) : undefined,
      setup_type: setupType || undefined,
      reason: reason.trim() || undefined,
    };
    const validation = validateDecision(decision, snapshot);
    if (validation) { setFeedback({ ok: false, message: validation }); return; }
    submittingRef.current = true;
    setSubmitting(true); setFeedback(null);
    let result: SubmissionResult;
    try {
      result = await onSubmitDecision(decision);
    } finally {
      submittingRef.current = false;
      setSubmitting(false);
    }
    setFeedback(result);
    if (result.ok) setPendingAction(null);
  }, [isTradeAction, limitPrice, onSubmitDecision, orderType, pendingAction, quantity, reason, setupType, snapshot, stopLoss, targetPrice]);

  const actionsDisabled = disabled || limitedViewport || submitting || !snapshot.can_trade;
  useModalFocus(pendingAction !== null, close, dialogElement);
  return <section className="panel" data-testid="trade-controls" style={{ padding: 12, display: 'grid', gap: 10 }}>
    <div>
      <strong style={{ fontSize: 13 }}>Trade · {snapshot.symbol}</strong>
      <div data-testid="trade-context" style={{ color: 'var(--text-muted)', fontSize: 11 }}>
        Bar {snapshot.visible_bar}/{snapshot.total_bars} · {snapshot.current_date.slice(0, 10)} · Close {snapshot.current_price.toLocaleString()}
      </div>
    </div>
    {snapshot.trade_block_reason && <div role="status" data-testid="historical-trade-block" style={{ color: '#FFD166', fontSize: 12 }}>{snapshot.trade_block_reason}</div>}
    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
      <button className="btn-buy" disabled={actionsDisabled} onClick={() => open('BUY')}>BUY</button>
      <button className="btn-sell" disabled={actionsDisabled} onClick={() => open('SELL')}>SELL</button>
      <button disabled={actionsDisabled || !snapshot.positions.length} onClick={() => open('CLOSE')} style={{ color: 'var(--color-close)' }}>CLOSE</button>
      <button disabled={actionsDisabled} onClick={() => open('HOLD')}>HOLD</button>
      <button disabled={actionsDisabled} onClick={() => open('SKIP')}>SKIP</button>
    </div>
    {feedback && !pendingAction && <div role="status" style={{ color: feedback.ok ? 'var(--color-buy)' : 'var(--color-sell)', fontSize: 12 }}>{feedback.message}</div>}

    {pendingAction && <div ref={setDialogElement} role="dialog" aria-modal="true" aria-label={`Record ${pendingAction}`} data-testid="trade-decision-dialog"
      style={{ position: 'fixed', inset: 0, zIndex: 1200, background: 'rgba(0,0,0,.72)', display: 'grid', placeItems: 'center', padding: 16 }}>
      <div className="glass-panel-solid" style={{ width: 'min(440px, 100%)', maxHeight: 'min(760px, 92vh)', overflowY: 'auto', padding: 20, display: 'grid', gap: 12 }}>
        <div><h3 style={{ margin: 0 }}>Record {pendingAction}</h3><small>{snapshot.symbol} · bar {snapshot.visible_bar}/{snapshot.total_bars} · {snapshot.current_date.slice(0, 10)}</small></div>
        {isTradeAction && <>
          <label>Quantity<input aria-label="Quantity" type="number" min="1" value={quantity} onChange={event => setQuantity(event.target.value)} /></label>
          <label>Order type<select aria-label="Order type" value={orderType} onChange={event => setOrderType(event.target.value)}><option value="MARKET_AT_CLOSE">Market at close</option><option value="LIMIT">Limit</option></select></label>
          {orderType === 'LIMIT' && <label>Limit price<input aria-label="Limit price" type="number" min="0" value={limitPrice} onChange={event => setLimitPrice(event.target.value)} /></label>}
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
            <label>Stop loss<input aria-label="Stop loss" type="number" value={stopLoss} onChange={event => setStopLoss(event.target.value)} /></label>
            <label>Target<input aria-label="Target price" type="number" value={targetPrice} onChange={event => setTargetPrice(event.target.value)} /></label>
          </div>
        </>}
        <label>Setup<select aria-label="Setup" value={setupType} onChange={event => setSetupType(event.target.value)}><option value="">None</option>{SETUP_TYPES.map(value => <option key={value}>{value}</option>)}</select></label>
        <label>Reason / observation<textarea aria-label="Reason" value={reason} onChange={event => setReason(event.target.value)} rows={4} /></label>
        {feedback && <div role="alert" style={{ color: feedback.ok ? 'var(--color-buy)' : 'var(--color-sell)' }}>{feedback.message}</div>}
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}><button onClick={close} disabled={submitting}>Cancel</button><button className="btn-primary" onClick={submit} disabled={submitting}>{submitting ? 'Submitting…' : `Submit ${pendingAction}`}</button></div>
      </div>
    </div>}
  </section>;
};
