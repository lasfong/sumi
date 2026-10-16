import React, { useState } from 'react';
import type { PracticeOrder } from '../../types';

interface PendingOrdersPanelProps {
  orders: PracticeOrder[];
  onCancelOrder?: (orderId: number) => void | Promise<void>;
}

const statusColor = (status: PracticeOrder['status']): string => {
  if (status === 'pending') return 'var(--color-primary)';
  if (status === 'executed') return 'var(--color-buy)';
  if (status === 'rejected' || status === 'cancelled') return 'var(--color-sell)';
  return 'var(--text-muted)';
};

export const PendingOrdersPanel: React.FC<PendingOrdersPanelProps> = ({ orders, onCancelOrder }) => {
  const [cancellingId, setCancellingId] = useState<number | null>(null);
  const visibleOrders = orders.slice().reverse().slice(0, 8);

  const handleCancel = async (orderId: number) => {
    if (!onCancelOrder || cancellingId !== null) return;
    setCancellingId(orderId);
    try {
      await onCancelOrder(orderId);
    } finally {
      setCancellingId(null);
    }
  };

  return (
    <div className="panel" data-testid="pending-orders-panel" style={{ padding: '12px' }}>
      <h3 style={{ marginTop: 0, marginBottom: '10px', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '0.5px', color: 'var(--text-muted)' }}>
        Orders
      </h3>
      {visibleOrders.length === 0 ? (
        <p style={{ color: 'var(--text-muted)', fontSize: '13px', fontStyle: 'italic', margin: 0 }}>No orders at this replay bar.</p>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {visibleOrders.map((order) => (
            <div key={order.id} data-testid={`order-item-${order.id}`} style={{ border: '1px solid var(--border-color)', borderRadius: '6px', padding: '8px', background: 'var(--bg-dark)' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '4px' }}>
                <strong style={{ color: order.side === 'BUY' ? 'var(--color-buy)' : 'var(--color-sell)', fontSize: '13px' }}>
                  {order.side} {order.order_type}
                </strong>
                <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                  <span style={{ color: statusColor(order.status), fontSize: '12px', textTransform: 'uppercase', fontWeight: 700 }}>
                    {order.status}
                  </span>
                  {order.status === 'pending' && onCancelOrder && (
                    <button
                      type="button"
                      data-testid={`cancel-order-${order.id}`}
                      aria-label={`Hủy lệnh #${order.id}`}
                      disabled={cancellingId === order.id}
                      onClick={() => { void handleCancel(order.id); }}
                      style={{
                        fontSize: '11px',
                        padding: '2px 8px',
                        borderRadius: '4px',
                        background: 'rgba(255, 23, 68, 0.14)',
                        border: '1px solid rgba(255, 23, 68, 0.45)',
                        color: 'var(--color-sell)',
                        cursor: cancellingId === order.id ? 'wait' : 'pointer',
                        fontWeight: 600,
                      }}
                    >
                      {cancellingId === order.id ? 'Đang hủy…' : 'Hủy lệnh'}
                    </button>
                  )}
                </div>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--text-muted)', fontSize: '12px' }}>
                <span>Qty {order.quantity.toLocaleString()}</span>
                <span>{order.requested_price ? order.requested_price.toLocaleString() : 'Market'}</span>
              </div>
              <div style={{ color: 'var(--text-muted)', fontSize: 11, marginTop: 4 }}>{order.explanation}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
