import React, { useState, useCallback } from 'react';
import type { DecisionAction, DecisionCreate } from '../../types';

interface TradeControlsProps {
  sessionId: number | null;
  onSubmitDecision: (decision: DecisionCreate) => Promise<void>;
  disabled?: boolean;
}

const SETUP_TYPES = ['Breakout', 'Pullback', 'Reversal', 'Trend Follow', 'Range Play', 'Gap Play', 'Accumulation', 'Distribution'];
const MISTAKE_TAGS = ['FOMO', 'Early Entry', 'Late Entry', 'Early Exit', 'Late Exit', 'Ignored Stop', 'Oversized', 'No Plan'];

export const TradeControls: React.FC<TradeControlsProps> = ({ sessionId, onSubmitDecision, disabled }) => {
  const [showModal, setShowModal] = useState(false);
  const [pendingAction, setPendingAction] = useState<DecisionAction | null>(null);
  const [setupType, setSetupType] = useState('');
  const [confidence, setConfidence] = useState(3);
  const [reason, setReason] = useState('');
  const [marketContext, setMarketContext] = useState('');
  const [mistakeTag, setMistakeTag] = useState('');
  const [note, setNote] = useState('');
  const [quantity, setQuantity] = useState<string>('100');
  const [orderType, setOrderType] = useState<string>('MARKET_AT_CLOSE');
  const [limitPrice, setLimitPrice] = useState<string>('');
  const [stopLoss, setStopLoss] = useState<string>('');
  const [targetPrice, setTargetPrice] = useState<string>('');

  const handleDecisionClick = useCallback((action: DecisionAction) => {
    if (!sessionId) return;
    setPendingAction(action);
    setSetupType('');
    setConfidence(3);
    setReason('');
    setMarketContext('');
    setMistakeTag('');
    setNote('');
    setQuantity('100');
    setOrderType('MARKET_AT_CLOSE');
    setLimitPrice('');
    setStopLoss('');
    setTargetPrice('');
    setShowModal(true);
  }, [sessionId]);

  const confirmDecision = useCallback(async () => {
    if (!sessionId || !pendingAction) return;
    const decision: DecisionCreate = {
      action: pendingAction,
      setup_type: setupType || undefined,
      confidence_score: confidence,
      market_context: marketContext || undefined,
      reason: reason || undefined,
      note: note || undefined,
      mistake_tag: mistakeTag || undefined,
      quantity: quantity ? parseFloat(quantity) : undefined,
      order_type: orderType,
      price: orderType === 'LIMIT' && limitPrice ? parseFloat(limitPrice) : undefined,
      stop_loss: stopLoss ? parseFloat(stopLoss) : undefined,
      target_price: targetPrice ? parseFloat(targetPrice) : undefined,
    };
    await onSubmitDecision(decision);
    setShowModal(false);
    setPendingAction(null);
  }, [sessionId, pendingAction, setupType, confidence, marketContext, reason, note, mistakeTag, quantity, orderType, limitPrice, stopLoss, targetPrice, onSubmitDecision]);

  const actionColor = (action: string): string => {
    if (action === 'BUY' || action === 'ADD') return 'var(--color-buy)';
    if (action === 'SELL' || action === 'REDUCE' || action === 'CUT_LOSS') return 'var(--color-sell)';
    return 'var(--color-close)';
  };

  const isTradeAction = pendingAction === 'BUY' || pendingAction === 'ADD' || pendingAction === 'SELL' || pendingAction === 'REDUCE';

  return (
    <>
      <div className="glass-panel" style={{ padding: '20px' }}>
        <h3 style={{ marginTop: 0, fontSize: '13px', textTransform: 'uppercase', letterSpacing: '1px', color: 'var(--text-muted)' }}>
          Order Panel
        </h3>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px', marginBottom: '10px' }}>
          <button className="btn-buy" disabled={disabled} onClick={() => handleDecisionClick('BUY')}>
            BUY
          </button>
          <button className="btn-sell" disabled={disabled} onClick={() => handleDecisionClick('SELL')}>
            SELL
          </button>
        </div>
        <button disabled={disabled} onClick={() => handleDecisionClick('CLOSE')} style={{ width: '100%', background: 'transparent', border: '1px solid var(--color-close)', color: 'var(--color-close)', marginBottom: '10px' }}>
          CLOSE ALL
        </button>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
          <button disabled={disabled} onClick={() => handleDecisionClick('HOLD')} style={{ background: 'transparent', border: '1px solid var(--border-color)', color: 'var(--text-main)' }}>
            HOLD
          </button>
          <button disabled={disabled} onClick={() => handleDecisionClick('SKIP')} style={{ background: 'transparent', border: '1px solid var(--border-color)', color: 'var(--text-main)' }}>
            SKIP
          </button>
        </div>
      </div>

      {showModal && (
        <div
          className="animate-fade-in"
          style={{
            position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
            background: 'rgba(0,0,0,0.7)', backdropFilter: 'blur(4px)',
            display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 1000,
          }}
          onClick={() => setShowModal(false)}
        >
          <div className="glass-panel-solid" style={{ width: '420px', padding: '24px', display: 'flex', flexDirection: 'column', gap: '16px' }} onClick={(e) => e.stopPropagation()}>
            <h3 style={{ margin: 0, color: pendingAction ? actionColor(pendingAction) : 'white', fontSize: '20px' }}>
              Confirm {pendingAction}
            </h3>

            {isTradeAction && (
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
                <div>
                  <label style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>Quantity</label>
                  <input
                    type="number"
                    value={quantity}
                    onChange={(e) => setQuantity(e.target.value)}
                    style={{ width: '100%' }}
                    min="1"
                  />
                </div>
                <div>
                  <label style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>Order Type</label>
                  <select style={{ width: '100%' }} value={orderType} onChange={(e) => setOrderType(e.target.value)}>
                    <option value="MARKET_AT_CLOSE">Market At Close</option>
                    <option value="LIMIT">Limit</option>
                  </select>
                </div>
              </div>
            )}

            {isTradeAction && orderType === 'LIMIT' && (
              <div>
                <label style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>Limit Price</label>
                <input
                  type="number"
                  value={limitPrice}
                  onChange={(e) => setLimitPrice(e.target.value)}
                  style={{ width: '100%' }}
                  step="0.1"
                  min="0"
                  required
                />
              </div>
            )}

            <div>
              <label style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>Setup Type</label>
              <select style={{ width: '100%' }} value={setupType} onChange={(e) => setSetupType(e.target.value)}>
                <option value="">None</option>
                {SETUP_TYPES.map((t) => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
              <div>
                <label style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>Market Context</label>
                <input
                  value={marketContext}
                  onChange={(e) => setMarketContext(e.target.value)}
                  style={{ width: '100%' }}
                  placeholder="VNINDEX, sector, regime"
                />
              </div>
              <div>
                <label style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>Mistake Tag</label>
                <select style={{ width: '100%' }} value={mistakeTag} onChange={(e) => setMistakeTag(e.target.value)}>
                  <option value="">None</option>
                  {MISTAKE_TAGS.map((tag) => (
                    <option key={tag} value={tag}>{tag}</option>
                  ))}
                </select>
              </div>
            </div>

            <div>
              <label style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>Confidence (1-5)</label>
              <input type="range" min="1" max="5" value={confidence} onChange={(e) => setConfidence(parseInt(e.target.value))} style={{ width: '100%', accentColor: 'var(--color-primary)' }} />
              <div style={{ textAlign: 'center', fontSize: '13px', color: 'var(--color-primary)', fontWeight: 'bold' }}>{confidence} / 5</div>
            </div>

            {isTradeAction && (
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
                <div>
                  <label style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>Stop Loss</label>
                  <input type="number" value={stopLoss} onChange={(e) => setStopLoss(e.target.value)} style={{ width: '100%' }} placeholder="Optional" />
                </div>
                <div>
                  <label style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>Target Price</label>
                  <input type="number" value={targetPrice} onChange={(e) => setTargetPrice(e.target.value)} style={{ width: '100%' }} placeholder="Optional" />
                </div>
              </div>
            )}

            <div>
              <label style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>Reason / Notes</label>
              <textarea
                style={{ width: '100%', background: 'rgba(0,0,0,0.2)', color: 'white', border: '1px solid var(--border-color)', borderRadius: '6px', padding: '10px', minHeight: '80px', resize: 'vertical', fontFamily: 'Inter' }}
                value={reason} onChange={(e) => setReason(e.target.value)} placeholder="Why are you taking this action?"
              />
            </div>

            <div>
              <label style={{ display: 'block', marginBottom: '6px', color: 'var(--text-muted)', fontSize: '13px' }}>Review Note</label>
              <textarea
                style={{ width: '100%', background: 'rgba(0,0,0,0.2)', color: 'white', border: '1px solid var(--border-color)', borderRadius: '6px', padding: '10px', minHeight: '64px', resize: 'vertical', fontFamily: 'Inter' }}
                value={note}
                onChange={(e) => setNote(e.target.value)}
                placeholder="Observation, emotion, or rule checklist"
              />
            </div>

            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '12px', marginTop: '8px' }}>
              <button onClick={() => setShowModal(false)} style={{ background: 'transparent', border: '1px solid var(--border-color)', color: 'var(--text-main)' }}>Cancel</button>
              <button className="btn-primary" onClick={confirmDecision}>Submit</button>
            </div>
          </div>
        </div>
      )}
    </>
  );
};
