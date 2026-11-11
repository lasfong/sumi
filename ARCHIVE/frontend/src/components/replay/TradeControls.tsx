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
      ? window.matchMedia('(max-width: 1279px)').matches
      : false,
  );
  const [pendingAction, setPendingAction] = useState<DecisionAction | null>(null);
  const [quantity, setQuantity] = useState('100');
  const [orderType, setOrderType] = useState('MARKET_NEXT_OPEN');
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
    const query = window.matchMedia('(max-width: 1279px)');
    const update = () => setLimitedViewport(query.matches);
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }, []);

  const open = useCallback((action: DecisionAction) => {
    setPendingAction(action); setQuantity('100'); setOrderType('MARKET_NEXT_OPEN'); setLimitPrice('');
    setStopLoss(''); setTargetPrice(''); setRiskPercent('1.0'); setSetupType(''); setMarketRegime('Bull Trend');
    setConfidenceScore('4'); setEmotion('Calm / Disciplined'); setMistakeTag('None'); setRuleViolation('None');
    setReason(''); setNote(''); setFeedback(null);
  }, []);

  const close = useCallback(() => { if (!submitting) setPendingAction(null); }, [submitting]);
  const openLongPosition = snapshot.positions.find(p => p.quantity > 0);
  const openShortPosition = !openLongPosition ? snapshot.positions.find(p => p.quantity < 0) : undefined;
  const openPosition = openLongPosition ?? openShortPosition;
  const availableQty = openPosition?.available_quantity ?? snapshot.available_quantity ?? 0;
  const openPositionValue = useMemo(
    () => snapshot.positions.reduce((sum, p) => sum + p.current_price * p.quantity, 0),
    [snapshot.positions],
  );
  const accountEquity = snapshot.current_cash + openPositionValue;

  const isTradeAction = pendingAction !== null && ['BUY', 'SELL', 'ADD', 'REDUCE'].includes(pendingAction);
  const isClosingLong = Boolean(
    openLongPosition && pendingAction !== null && ['SELL', 'REDUCE', 'CLOSE'].includes(pendingAction),
  );
  const isShortAction = Boolean(
    !openLongPosition &&
      pendingAction !== null &&
      ((pendingAction as string) === 'SHORT' ||
        (openShortPosition && ['SELL', 'REDUCE'].includes(pendingAction))),
  );
  const directionLabel = isShortAction ? 'SHORT' : isClosingLong ? 'LONG EXIT' : 'LONG';

  // Sync entry, stop loss, and target price from active/latest risk-reward chart drawing (both LONG and SHORT)
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
        const drawingDir = stop > entry ? 'SHORT' : 'LONG';
        setStopLoss(String(stop));
        setTargetPrice(String(target));
        if (orderType === 'LIMIT') setLimitPrice(String(entry));
        setFeedback({
          ok: true,
          message: `Synced ${drawingDir} from chart drawing: Entry ${entry}, Stop ${stop}, Target ${target}`,
        });
      } else {
        setFeedback({ ok: false, message: 'No Risk-Reward drawing found on chart.' });
      }
    } catch {
      setFeedback({ ok: false, message: 'Unable to read drawing state from chart.' });
    }
  }, [orderType]);

  const entryPrice = useMemo(
    () => (isTradeAction && orderType === 'LIMIT' && limitPrice !== '' ? Number(limitPrice) : snapshot.current_price),
    [isTradeAction, orderType, limitPrice, snapshot.current_price],
  );

  // Inline SL/TP validation message (keeps sizing panel visible instead of disappearing silently)
  const slTpValidationError = useMemo(() => {
    if (!isTradeAction || entryPrice <= 0) return null;
    const hasStop = stopLoss.trim() !== '';
    const hasTarget = targetPrice.trim() !== '';
    const stop = Number(stopLoss);
    const target = Number(targetPrice);

    if (hasStop && (!Number.isFinite(stop) || stop <= 0)) {
      return 'Giá cắt lỗ (SL) phải lớn hơn 0.';
    }
    if (hasTarget && (!Number.isFinite(target) || target <= 0)) {
      return 'Giá chốt lời (TP) phải lớn hơn 0.';
    }

    if (isShortAction) {
      if (hasStop && stop <= entryPrice) {
        return `Giá cắt lỗ (SL) phải cao hơn giá bán (${entryPrice.toLocaleString()}) cho vị thế Bán/Short (SHORT).`;
      }
      if (hasTarget && target >= entryPrice) {
        return `Giá chốt lời (TP) phải thấp hơn giá bán (${entryPrice.toLocaleString()}) cho vị thế Bán/Short (SHORT).`;
      }
    } else {
      if (hasStop && stop >= entryPrice) {
        return `Giá cắt lỗ (SL) phải thấp hơn giá mua (${entryPrice.toLocaleString()}) cho vị thế Mua (LONG).`;
      }
      if (hasTarget && target <= entryPrice) {
        return `Giá chốt lời (TP) phải cao hơn giá mua (${entryPrice.toLocaleString()}) cho vị thế Mua (LONG).`;
      }
    }
    return null;
  }, [entryPrice, isShortAction, isTradeAction, stopLoss, targetPrice]);

  // Sizing and Risk calculations (symmetric for LONG and SHORT, aligned with backend /plan-sizing equity formula)
  const sizingCalc = useMemo(() => {
    const entry = entryPrice;
    const stop = Number(stopLoss);
    const target = Number(targetPrice);
    const riskPct = Number(riskPercent) || 1.0;
    if (entry <= 0 || stopLoss.trim() === '' || !Number.isFinite(stop) || stop <= 0 || slTpValidationError) {
      return null;
    }
    const riskPerShare = isShortAction ? stop - entry : entry - stop;
    if (riskPerShare <= 0) return null;
    const hasValidTarget =
      targetPrice.trim() !== '' &&
      Number.isFinite(target) &&
      (isShortAction ? target > 0 && target < entry : target > entry);
    const rewardPerShare = hasValidTarget ? (isShortAction ? entry - target : target - entry) : 0;
    const grossR = riskPerShare > 0 ? rewardPerShare / riskPerShare : 0;
    const maxRiskAmt = Math.max(0, (accountEquity * riskPct) / 100);
    const rawQty = maxRiskAmt / riskPerShare;
    const lotRoundedQty = Math.floor(rawQty / 100) * 100;
    const maxCashQty = Math.floor(Math.max(0, snapshot.current_cash) / (entry * 1.0015) / 100) * 100;
    const maxSellableQty = Math.floor(Math.max(0, availableQty) / 100) * 100;
    const capacityQty = isClosingLong ? maxSellableQty : maxCashQty;
    const plannedQty = Math.max(0, Math.min(lotRoundedQty, capacityQty));
    const plannedRiskAmt = plannedQty * riskPerShare;

    let grossBuy = 0;
    let netProfit = 0;
    let netR = 0;
    if (isShortAction) {
      const grossSell = plannedQty * entry;
      const sellFee = grossSell * 0.0015;
      const sellTax = grossSell * 0.0010;
      grossBuy = hasValidTarget ? plannedQty * target : plannedQty * entry;
      const buyFee = grossBuy * 0.0015;
      netProfit = hasValidTarget ? (grossSell - sellFee - sellTax) - (grossBuy + buyFee) : 0;
      const riskOutlay = plannedRiskAmt + sellFee + sellTax;
      netR = riskOutlay > 0 ? netProfit / riskOutlay : 0;
    } else {
      grossBuy = plannedQty * entry;
      const buyFee = grossBuy * 0.0015;
      const grossSell = hasValidTarget ? plannedQty * target : plannedQty * entry;
      const sellFee = grossSell * 0.0015;
      const sellTax = grossSell * 0.0010;
      netProfit = hasValidTarget ? (grossSell - sellFee - sellTax) - (grossBuy + buyFee) : 0;
      const riskOutlay = plannedRiskAmt + buyFee;
      netR = riskOutlay > 0 ? netProfit / riskOutlay : 0;
    }

    return {
      entry, stop, target, riskPerShare, rewardPerShare, grossR, maxRiskAmt,
      accountEquity, availableCash: snapshot.current_cash,
      lotRoundedQty, maxCashQty, capacityQty,
      plannedQty, plannedRiskAmt, grossBuy, netProfit, netR,
      affordable: plannedQty >= 100,
    };
  }, [accountEquity, availableQty, entryPrice, isClosingLong, isShortAction, riskPercent, slTpValidationError, snapshot.current_cash, stopLoss, targetPrice]);

  const slPctPreview = useMemo(() => {
    const stop = Number(stopLoss);
    if (stopLoss.trim() === '' || !Number.isFinite(stop) || stop <= 0 || entryPrice <= 0) return null;
    return ((stop - entryPrice) / entryPrice) * 100;
  }, [entryPrice, stopLoss]);

  const tpPctPreview = useMemo(() => {
    const target = Number(targetPrice);
    if (targetPrice.trim() === '' || !Number.isFinite(target) || target <= 0 || entryPrice <= 0) return null;
    return ((target - entryPrice) / entryPrice) * 100;
  }, [entryPrice, targetPrice]);

  const submit = useCallback(async () => {
    if (!pendingAction || submittingRef.current) return;
    const decision: DecisionCreate = {
      action: pendingAction,
      quantity: isTradeAction && quantity !== '' ? Number(quantity) : undefined,
      order_type: isTradeAction ? orderType : 'MARKET_NEXT_OPEN',
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

  const actionsDisabled = !snapshot.can_trade || submitting || disabled || limitedViewport;
  const unsettledQty = openPosition
    ? Math.max(0, openPosition.blocked_quantity ?? (Math.abs(openPosition.quantity) - availableQty))
    : Math.max(0, snapshot.blocked_quantity ?? 0);
  const releaseDateStr = (openPosition?.earliest_release_date ?? snapshot.earliest_release_date)?.slice(0, 10) || null;
  const sellDisabled = actionsDisabled || !openPosition || availableQty <= 0;
  const settlementLabel = snapshot.settlement_label || 'T+1 · về sau 1 phiên';
  const sellTitle = !openPosition
    ? 'No open position'
    : availableQty <= 0
      ? `${settlementLabel}: Cổ phiếu chưa về tài khoản để bán`
      : 'Bán một phần hoặc toàn bộ cổ phiếu khả dụng';
  const closeTitle = !openPosition
    ? 'No open position'
    : availableQty <= 0
      ? `${settlementLabel}: Cổ phiếu chưa về tài khoản để đóng vị thế`
      : 'Đóng toàn bộ vị thế khả dụng hiện tại';

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
      <button className="btn-sell" disabled={sellDisabled} title={sellTitle} onClick={() => open('SELL')}>SELL</button>
      <button disabled={sellDisabled} title={closeTitle} onClick={() => open('CLOSE')} style={{ color: 'var(--color-close)' }}>CLOSE</button>
      <button disabled={actionsDisabled} onClick={() => open('HOLD')}>HOLD</button>
      <button disabled={actionsDisabled} onClick={() => open('SKIP')}>SKIP</button>
    </div>
    {openPosition && unsettledQty > 0 && (
      <div
        role="status"
        data-testid="settlement-inline-notice"
        style={{
          fontSize: 11,
          padding: '6px 8px',
          borderRadius: 4,
          background: 'rgba(255, 209, 102, 0.12)',
          border: '1px solid rgba(255, 209, 102, 0.35)',
          color: '#FFD166',
        }}
      >
        ⏳ Đang chờ cổ phiếu về (<strong>{settlementLabel}</strong>) · Khả dụng:{' '}
        <strong>{availableQty.toLocaleString()}</strong> / Chờ về:{' '}
        <strong>{unsettledQty.toLocaleString()}</strong> CP
        {releaseDateStr ? ` · Về ngày ${releaseDateStr}` : ''}
      </div>
    )}
    {feedback && !pendingAction && <div role="status" style={{ color: feedback.ok ? 'var(--color-buy)' : 'var(--color-sell)', fontSize: 12 }}>{feedback.message}</div>}

    {pendingAction && <div ref={setDialogElement} role="dialog" aria-modal="true" aria-label={`Record ${pendingAction}`} data-testid="trade-decision-dialog"
      style={{ position: 'fixed', inset: 0, zIndex: 1200, background: 'rgba(0,0,0,.72)', display: 'grid', placeItems: 'center', padding: 16 }}>
      <div className="glass-panel-solid" style={{ width: 'min(480px, 100%)', maxHeight: 'min(820px, 94vh)', overflowY: 'auto', padding: 20, display: 'grid', gap: 12 }}>
        <div>
          <h3 style={{ margin: 0 }}>Record {pendingAction}</h3>
          <small>{snapshot.symbol} · bar {snapshot.visible_bar}/{snapshot.total_bars} · {snapshot.current_date.slice(0, 10)}</small>
          {isClosingLong && openLongPosition && (
            <div
              data-testid="closing-long-context"
              style={{
                marginTop: 6,
                padding: '4px 8px',
                borderRadius: 4,
                background: 'rgba(0, 230, 118, 0.1)',
                border: '1px solid rgba(0, 230, 118, 0.3)',
                color: '#00E676',
                fontSize: 11,
              }}
            >
              Đóng/Giảm vị thế Mua (LONG) · Khả dụng: <strong>{availableQty.toLocaleString()}</strong> /{' '}
              <strong>{openLongPosition.quantity.toLocaleString()}</strong> CP (Giá vốn TB:{' '}
              {openLongPosition.average_price.toLocaleString()})
            </div>
          )}
        </div>
        {isTradeAction && <>
          <label>Quantity
            <input aria-label="Quantity" type="number" step="100" min="100" value={quantity} onChange={event => setQuantity(event.target.value)} />
          </label>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
            <label>Order type
              <select aria-label="Order type" value={orderType} onChange={event => setOrderType(event.target.value)}>
                <option value="MARKET_NEXT_OPEN">Market at next open</option>
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
            <div>
              <label htmlFor="order-sl">
                Stop loss {slPctPreview !== null ? <small style={{ color: 'var(--text-muted)' }}>({slPctPreview >= 0 ? '+' : ''}{slPctPreview.toFixed(1)}%)</small> : null}
              </label>
              <input id="order-sl" aria-label="Stop loss" type="number" step="100" value={stopLoss} onChange={event => setStopLoss(event.target.value)} />
              <div style={{ display: 'flex', gap: 4, marginTop: 4 }}>
                <button type="button" onClick={() => {
                  const entry = entryPrice;
                  if (entry > 0) {
                    setStopLoss(String(Math.round(entry * (isShortAction ? 1.05 : 0.95))));
                  }
                }} style={{ fontSize: 10, padding: '2px 6px' }}>{isShortAction ? '+5%' : '-5%'}</button>
                <button type="button" onClick={() => {
                  const entry = entryPrice;
                  if (entry > 0) {
                    setStopLoss(String(Math.round(entry * (isShortAction ? 1.07 : 0.93))));
                  }
                }} style={{ fontSize: 10, padding: '2px 6px' }}>{isShortAction ? '+7%' : '-7%'}</button>
              </div>
            </div>
            <div>
              <label htmlFor="order-tp">
                Target {tpPctPreview !== null ? <small style={{ color: 'var(--text-muted)' }}>({tpPctPreview >= 0 ? '+' : ''}{tpPctPreview.toFixed(1)}%)</small> : null}
              </label>
              <input id="order-tp" aria-label="Target price" type="number" step="100" value={targetPrice} onChange={event => setTargetPrice(event.target.value)} />
              <div style={{ display: 'flex', gap: 4, marginTop: 4 }}>
                <button type="button" onClick={() => {
                  const entry = entryPrice;
                  const sl = Number(stopLoss) || Math.round(entry * (isShortAction ? 1.05 : 0.95));
                  if (entry > 0 && sl > 0) {
                    if (!stopLoss) setStopLoss(String(sl));
                    const riskDist = Math.abs(entry - sl);
                    setTargetPrice(String(Math.round(isShortAction ? Math.max(1, entry - 2.0 * riskDist) : entry + 2.0 * riskDist)));
                  }
                }} style={{ fontSize: 10, padding: '2px 6px' }}>2R</button>
                <button type="button" onClick={() => {
                  const entry = entryPrice;
                  const sl = Number(stopLoss) || Math.round(entry * (isShortAction ? 1.05 : 0.95));
                  if (entry > 0 && sl > 0) {
                    if (!stopLoss) setStopLoss(String(sl));
                    const riskDist = Math.abs(entry - sl);
                    setTargetPrice(String(Math.round(isShortAction ? Math.max(1, entry - 3.0 * riskDist) : entry + 3.0 * riskDist)));
                  }
                }} style={{ fontSize: 10, padding: '2px 6px' }}>3R</button>
              </div>
            </div>
            <div>
              <label>Risk %
                <input aria-label="Risk percent" type="number" step="0.25" min="0.1" max="10" value={riskPercent} onChange={event => setRiskPercent(event.target.value)} />
              </label>
            </div>
          </div>

          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', margin: '2px 0' }}>
            <span data-testid="trade-sizing-direction-label" style={{ fontSize: 11, color: 'var(--text-muted)' }}>
              Trade Sizing &amp; Risk ({directionLabel})
            </span>
            <button type="button" data-testid="sync-from-drawing" onClick={syncFromDrawing} style={{ fontSize: 11, padding: '3px 8px' }}>
              📐 Sync from Drawing
            </button>
          </div>

          {slTpValidationError && (
            <div
              role="alert"
              data-testid="sl-tp-validation-alert"
              style={{
                padding: '8px 10px',
                borderRadius: 6,
                background: 'rgba(255, 23, 68, 0.12)',
                border: '1px solid rgba(255, 23, 68, 0.45)',
                color: '#FF8A80',
                fontSize: 12,
              }}
            >
              ⚠️ {slTpValidationError}
            </div>
          )}

          <div data-testid="position-sizing-panel" style={{ padding: 10, background: 'rgba(41,98,255,.08)', border: '1px solid rgba(41,98,255,.3)', borderRadius: 6, display: 'grid', gap: 4, fontSize: 12 }}>
            {sizingCalc ? (
              <>
                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <span>Expected R: <strong style={{ color: '#00E676' }}>1 : {sizingCalc.grossR.toFixed(1)}</strong> ({sizingCalc.grossR.toFixed(2)}R, Net {sizingCalc.netR.toFixed(2)}R)</span>
                  <span data-testid="sizing-max-risk">Max Risk: <strong>{sizingCalc.maxRiskAmt.toLocaleString()} VND</strong></span>
                </div>
                <div data-testid="sizing-equity-basis" style={{ fontSize: 11, color: 'var(--text-muted)', display: 'flex', justifyContent: 'space-between' }}>
                  <span>Equity: {sizingCalc.accountEquity.toLocaleString()} VND</span>
                  <span>Cash: {sizingCalc.availableCash.toLocaleString()} VND</span>
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
                {!sizingCalc.affordable && <div style={{ color: '#FF8A80' }}>Calculated size is below 100-share minimum lot or exceeds available {isClosingLong ? 'position' : 'cash'}.</div>}
              </>
            ) : (
              <div style={{ color: 'var(--text-muted)' }}>
                {slTpValidationError
                  ? 'Điều chỉnh lại SL/TP hợp lệ để tính khối lượng khuyến nghị theo rủi ro.'
                  : 'Nhập giá Cắt lỗ (SL) và Chốt lời (TP) để xem tỷ lệ R:R và khối lượng khuyến nghị (lô chẵn 100 CP).'}
              </div>
            )}
            <div data-testid="trade-market-assumptions" style={{ fontSize: 10, color: 'var(--text-muted)', borderTop: '1px solid rgba(255,255,255,0.08)', paddingTop: 4, marginTop: 2 }}>
              Giả định thị trường: Phí mua 0.15% · Phí bán 0.15% · Thuế bán 0.10% · Lô chẵn 100 CP · {settlementLabel}
            </div>
          </div>
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
          <button className="btn-primary" onClick={submit} disabled={submitting || (isTradeAction && Number(quantity) % 100 !== 0)}>{submitting ? 'Submitting…' : `Submit ${pendingAction}`}</button>
        </div>
      </div>
    </div>}
  </section>;
};
