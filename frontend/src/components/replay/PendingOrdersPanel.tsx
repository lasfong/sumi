import React from 'react';
import type { Order } from '../../types';

interface PendingOrdersPanelProps {
  orders: Order[];
}

const statusColor = (status: Order['status']): string => {
  if (status === 'pending') return 'var(--color-primary)';
  if (status === 'executed') return 'var(--color-buy)';
  if (status === 'rejected' || status === 'cancelled') return 'var(--color-sell)';
  return 'var(--text-muted)';
};

export const PendingOrdersPanel: React.FC<PendingOrdersPanelProps> = ({ orders }) => {
  const visibleOrders = orders.filter((order) => order.status === 'pending' || order.status === 'rejected').slice(0, 6);

  return (
    <div className="panel" style={{ padding: '12px' }}>
      <h3 style={{ marginTop: 0, marginBottom: '10px', fontSize: '13px', textTransform: 'uppercase', letterSpacing: '0.5px', color: 'var(--text-muted)' }}>
        Pending Orders
      </h3>
      {visibleOrders.length === 0 ? (
        <p style={{ color: 'var(--text-muted)', fontSize: '13px', fontStyle: 'italic', margin: 0 }}>No pending orders.</p>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {visibleOrders.map((order) => (
            <div key={order.id} style={{ border: '1px solid var(--border-color)', borderRadius: '6px', padding: '8px', background: 'var(--bg-dark)' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '4px' }}>
                <strong style={{ color: order.side === 'BUY' ? 'var(--color-buy)' : 'var(--color-sell)', fontSize: '13px' }}>
                  {order.side} {order.order_type}
                </strong>
                <span style={{ color: statusColor(order.status), fontSize: '12px', textTransform: 'uppercase', fontWeight: 700 }}>
                  {order.status}
                </span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--text-muted)', fontSize: '12px' }}>
                <span>Qty {order.quantity.toLocaleString()}</span>
                <span>{order.requested_price ? order.requested_price.toLocaleString() : 'Market'}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
