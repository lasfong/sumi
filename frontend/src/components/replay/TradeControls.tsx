import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
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
const MARKET_REGIMES = ['Bull Trend', 'Bear Trend', 'Sideways / Range', 'High Volatility', 'Low Volatility'];
const EMOTIONS = ['Calm / Disciplined', 'FOMO / Impatient', 'Hesitant / Fearful', 'Overconfident', 'Revenge Trading'];
const MISTAKE_TAGS = ['None', 'Chased Price', 'Ignored Stop Loss', 'Oversized', 'Exited Too Early', 'Traded Against Trend', 'No Plan'];
const RULE_VIOLATIONS = ['None', 'Risk Limit Exceeded', 'Unconfirmed Setup', 'Traded Without Stop', 'Overtraded'];

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
  const [riskPercent, setRiskPercent] = useState('1.0');
  const [setupType, setSetupType] = useState('');
  const [marketRegime, setMarketRegime] = useState('Bull Trend');
  const [confidenceScore, setConfidenceScore] = useState('4');
  const [emotion, setEmotion] = useState('Calm / Disciplined');
  const [mistakeTag, setMistakeTag] = useState('None');
  const [ruleViolation, setRuleViolation] = useState('None');
  const [reason, setReason] = useState('');
  const [note, setNote] = useState('');
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
    setStopLoss(''); setTargetPrice(''); setRiskPercent('1.0'); setSetupType(''); setMarketRegime('Bull Trend');
    setConfidenceScore('4'); setEmotion('Calm / Disciplined'); setMistakeTag('None'); setRuleViolation('None');
    setReason(''); setNote(''); setFeedback(null);
  }, []);

  const close = useCallback(() => { if (!submitting) setPendingAction(null); }, [submitting]);
  const isTradeAction = pendingAction !== null && ['BUY', 'SELL', 'ADD', 'REDUCE'].includes(pendingAction);

  // Sync entry, stop loss, and target price from active/latest risk-reward chart drawing
  const syncFromDrawing = useCallback(() => {
    try {
      const stateElement = document.querySelector('[data-drawing-interaction-state]');
      const rawState = stateElement?.getAttribute('data-drawing-interaction-state');
      if (!rawState) return;
      const state = JSON.parse(rawState) as {
        drawings?: Array<{ tool: string; anchors?: Array<{ price: number }> }>;
        preview?: { tool: string; anchors?: Array<{ price: number }> } | null;
      };
      const rrDrawing = state.drawings?.find(d => d.tool === 'risk-reward')
        ?? (state.preview?.tool === 'risk-reward' ? state.preview : null);
      if (rrDrawing && rrDrawing.anchors && rrDrawing.anchors.length >= 3) {
        const entry = rrDrawing.anchors[0].price;
        const stop = rrDrawing.anchors[1].price;
        const target = rrDrawing.anchors[2].price;
        setStopLoss(String(stop));
        setTargetPrice(String(target));
        if (orderType === 'LIMIT') setLimitPrice(String(entry));
        setFeedback({ ok: true, message: `Synced from chart drawing: Entry ${entry}, Stop ${stop}, Target ${target}` });
      } else {
        setFeedback({ ok: false, message: 'No Risk-Reward drawing found on chart.' });
      }
    } catch {
      setFeedback({ ok: false, message: 'Unable to read drawing state from chart.' });
    }
  }, [orderType]);

  // Sizing and Risk calculations
  const sizingCalc = useMemo(() => {
    const entry = isTradeAction && orderType === 'LIMIT' && limitPrice !== '' ? Number(limitPrice) : snapshot.current_price;
    const stop = Number(stopLoss);
    const target = Number(targetPrice);
    const riskPct = Number(riskPercent) || 1.0;
    if (entry <= 0 || stop <= 0 || stop >= entry) {
      return null;
    }
    const riskPerShare = entry - stop;
    const rewardPerShare = target > entry ? target - entry : 0;
    const grossR = riskPerShare > 0 ? rewardPerShare / riskPerShare : 0;
    const maxRiskAmt = (snapshot.current_cash * riskPct) / 100;
    const rawQty = maxRiskAmt / riskPerShare;
    const lotRoundedQty = Math.floor(rawQty / 100) * 100;
    const maxCashQty = Math.floor(snapshot.current_cash / (entry * 1.0015) / 100) * 100;
    const plannedQty = Math.min(lotRoundedQty, maxCashQty);
    const plannedRiskAmt = plannedQty * riskPerShare;
    const grossBuy = plannedQty * entry;
    const buyFee = grossBuy * 0.0015;
    const grossSell = plannedQty * target;
    const sellFee = grossSell * 0.0015;
    const sellTax = grossSell * 0.0010;
    const netProfit = (grossSell - sellFee - sellTax) - (grossBuy + buyFee);
    const netR = (plannedRiskAmt + buyFee > 0) ? netProfit / (plannedRiskAmt + buyFee) : 0;

    return {
      entry, stop, target, riskPerShare, rewardPerShare, grossR, maxRiskAmt,
      plannedQty, plannedRiskAmt, grossBuy, netProfit, netR,
      affordable: plannedQty > 0,
    };
  }, [isTradeAction, orderType, limitPrice, snapshot.current_price, snapshot.current_cash, stopLoss, targetPrice, riskPercent]);

  const submit = useCallback(async () => {
    if (!pendingAction || submittingRef.current) return;
    const decision: DecisionCreate = {
      action: pendingAction,
      quantity: isTradeAction && quantity !== '' ? Number(quantity) : undefined,
      order_type: isTradeAction ? orderType : 'MARKET_AT_CLOSE',
      price: isTradeAction && orderType === 'LIMIT' && limitPrice !== '' ? Number(limitPrice) : undefined,
      stop_loss: isTradeAction && stopLoss !== '' ? Number(stopLoss) : undefined,
      target_price: isTradeAction && targetPrice !== '' ? Number(targetPrice) : undefined,
      planned_quantity: isTradeAction && quantity !== '' ? Number(quantity) : undefined,
      planned_risk: sizingCalc ? sizingCalc.plannedRiskAmt : undefined,
      planned_r: sizingCalc ? sizingCalc.grossR : undefined,
      setup_type: setupType || undefined,
      market_regime: marketRegime || undefined,
      confidence_score: confidenceScore ? Number(confidenceScore) : undefined,
      emotion: emotion || undefined,
      mistake_tag: mistakeTag || undefined,
      rule_violation: ruleViolation || undefined,
      reason: reason.trim() || undefined,
      note: note.trim() || undefined,
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
  }, [confidenceScore, emotion, isTradeAction, limitPrice, marketRegime, mistakeTag, note, onSubmitDecision, orderType, pendingAction, quantity, reason, ruleViolation, setupType, sizingCalc, snapshot, stopLoss, targetPrice]);

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
      <div className="glass-panel-solid" style={{ width: 'min(480px, 100%)', maxHeight: 'min(820px, 94vh)', overflowY: 'auto', padding: 20, display: 'grid', gap: 12 }}>
        <div>
          <h3 style={{ margin: 0 }}>Record {pendingAction}</h3>
          <small>{snapshot.symbol} · bar {snapshot.visible_bar}/{snapshot.total_bars} · {snapshot.current_date.slice(0, 10)}</small>
        </div>
        {isTradeAction && <>
          <label>Quantity
            <input aria-label="Quantity" type="number" min="1" value={quantity} onChange={event => setQuantity(event.target.value)} />
          </label>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
            <label>Order type
              <select aria-label="Order type" value={orderType} onChange={event => setOrderType(event.target.value)}>
                <option value="MARKET_AT_CLOSE">Market at close</option>
                <option value="LIMIT">Limit</option>
              </select>
            </label>
            {orderType === 'LIMIT' && (
              <label>Limit price
                <input aria-label="Limit price" type="number" min="0" step="100" value={limitPrice} onChange={event => setLimitPrice(event.target.value)} />
              </label>
            )}
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 8 }}>
            <label>Stop loss
              <input aria-label="Stop loss" type="number" step="100" value={stopLoss} onChange={event => setStopLoss(event.target.value)} />
            </label>
            <label>Target
              <input aria-label="Target price" type="number" step="100" value={targetPrice} onChange={event => setTargetPrice(event.target.value)} />
            </label>
            <label>Risk %
              <input aria-label="Risk percent" type="number" step="0.25" min="0.1" max="10" value={riskPercent} onChange={event => setRiskPercent(event.target.value)} />
            </label>
          </div>

          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', margin: '2px 0' }}>
            <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>Trade Sizing &amp; Risk</span>
            {pendingAction === 'BUY' && (
              <button type="button" data-testid="sync-from-drawing" onClick={syncFromDrawing} style={{ fontSize: 11, padding: '3px 8px' }}>
                📐 Sync from Drawing
              </button>
            )}
          </div>

          {sizingCalc && (
            <div data-testid="position-sizing-panel" style={{ padding: 10, background: 'rgba(41,98,255,.08)', border: '1px solid rgba(41,98,255,.3)', borderRadius: 6, display: 'grid', gap: 4, fontSize: 12 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span>Expected R: <strong style={{ color: '#00E676' }}>{sizingCalc.grossR.toFixed(2)}R</strong> (Net {sizingCalc.netR.toFixed(2)}R)</span>
                <span>Max Risk: <strong>{sizingCalc.maxRiskAmt.toLocaleString()} VND</strong></span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span>Recommended Size: <strong>{sizingCalc.plannedQty.toLocaleString()} shares</strong></span>
                <button
                  type="button"
                  data-testid="apply-sizing-qty"
                  onClick={() => setQuantity(String(sizingCalc.plannedQty))}
                  style={{ fontSize: 11, padding: '2px 8px' }}
                >
                  Use Size ({sizingCalc.plannedQty})
                </button>
              </div>
              {!sizingCalc.affordable && <div style={{ color: '#FF8A80' }}>Calculated size is below 100-share minimum lot or exceeds available cash.</div>}
            </div>
          )}
        </>}

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
          <label>Setup
            <select aria-label="Setup" value={setupType} onChange={event => setSetupType(event.target.value)}>
              <option value="">None</option>
              {SETUP_TYPES.map(value => <option key={value}>{value}</option>)}
            </select>
          </label>
          <label>Market Regime
            <select aria-label="Market Regime" value={marketRegime} onChange={event => setMarketRegime(event.target.value)}>
              {MARKET_REGIMES.map(value => <option key={value}>{value}</option>)}
            </select>
          </label>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
          <label>Emotion
            <select aria-label="Emotion" value={emotion} onChange={event => setEmotion(event.target.value)}>
              {EMOTIONS.map(value => <option key={value}>{value}</option>)}
            </select>
          </label>
          <label>Confidence (1-5)
            <select aria-label="Confidence" value={confidenceScore} onChange={event => setConfidenceScore(event.target.value)}>
              {[1, 2, 3, 4, 5].map(v => <option key={v} value={v}>{v} ★</option>)}
            </select>
          </label>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
          <label>Mistake Tag
            <select aria-label="Mistake Tag" value={mistakeTag} onChange={event => setMistakeTag(event.target.value)}>
              {MISTAKE_TAGS.map(value => <option key={value}>{value}</option>)}
            </select>
          </label>
          <label>Rule Violation
            <select aria-label="Rule Violation" value={ruleViolation} onChange={event => setRuleViolation(event.target.value)}>
              {RULE_VIOLATIONS.map(value => <option key={value}>{value}</option>)}
            </select>
          </label>
        </div>

        <label>Reason / Thesis
          <textarea aria-label="Reason" value={reason} onChange={event => setReason(event.target.value)} rows={3} placeholder="Technical catalysts, support/resistance context..." />
        </label>

        {feedback && <div role="alert" style={{ color: feedback.ok ? 'var(--color-buy)' : 'var(--color-sell)' }}>{feedback.message}</div>}
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
          <button onClick={close} disabled={submitting}>Cancel</button>
          <button className="btn-primary" onClick={submit} disabled={submitting}>{submitting ? 'Submitting…' : `Submit ${pendingAction}`}</button>
        </div>
      </div>
    </div>}
  </section>;
};
