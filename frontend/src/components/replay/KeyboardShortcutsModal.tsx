import React, { useEffect, useRef } from 'react';

interface KeyboardShortcutsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

interface ShortcutGroup {
  category: string;
  items: Array<{ keys: string[]; description: string }>;
}

const SHORTCUT_GROUPS: ShortcutGroup[] = [
  {
    category: 'Điều khiển tua nến (Replay)',
    items: [
      { keys: ['Space'], description: 'Tiến 1 nến (hoặc Play / Pause)' },
      { keys: ['→'], description: 'Tiến 1 nến' },
      { keys: ['←'], description: 'Lùi 1 nến' },
      { keys: ['Shift', '→'], description: 'Tua nhanh +5 nến' },
      { keys: ['Shift', '←'], description: 'Tua lùi -5 nến' },
    ],
  },
  {
    category: 'Giao dịch & Kỷ luật (Trading)',
    items: [
      { keys: ['B'], description: 'Mở nhanh vé mua (BUY)' },
      { keys: ['S'], description: 'Mở nhanh vé bán (SELL)' },
      { keys: ['H'], description: 'Ghi nhận quyết định giữ (HOLD)' },
      { keys: ['C'], description: 'Chuyển nhanh tab Checklist quan sát' },
      { keys: ['D'], description: 'Bảng tổng kết phiên (Session Debrief)' },
    ],
  },
  {
    category: 'Công cụ vẽ kỹ thuật (Drawings)',
    items: [
      { keys: ['Alt', 'T'], description: 'Đường xu hướng (Trendline)' },
      { keys: ['Alt', 'H'], description: 'Đường ngang (Horizontal Line)' },
      { keys: ['Alt', 'R'], description: 'Công cụ Risk-Reward' },
      { keys: ['Delete'], description: 'Xóa hình vẽ đang chọn' },
      { keys: ['Esc'], description: 'Hủy vẽ / Đóng cửa sổ lệnh' },
    ],
  },
];

export const KeyboardShortcutsModal: React.FC<KeyboardShortcutsModalProps> = ({ isOpen, onClose }) => {
  const modalRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Bảng phím tắt Trading Lab"
      data-testid="keyboard-shortcuts-dialog"
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 2000,
        background: 'rgba(0, 0, 0, 0.75)',
        display: 'grid',
        placeItems: 'center',
        padding: 16,
        backdropFilter: 'blur(4px)',
      }}
      onClick={e => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={modalRef}
        className="glass-panel-solid"
        style={{
          width: 'min(640px, 95vw)',
          maxHeight: '85vh',
          overflowY: 'auto',
          padding: 24,
          background: 'rgba(19, 23, 34, 0.98)',
          border: '1px solid rgba(41, 98, 255, 0.4)',
          borderRadius: 12,
          boxShadow: '0 16px 48px rgba(0, 0, 0, 0.8)',
          display: 'grid',
          gap: 20,
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <span style={{ fontSize: 22 }}>⌨️</span>
            <div>
              <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>Phím tắt Trading Lab (Pro Hotkeys)</h2>
              <span style={{ fontSize: 12, color: 'var(--text-muted)' }}>
                Thao tác luyện tập phản xạ nhanh mà không cần rời tay khỏi bàn phím
              </span>
            </div>
          </div>
          <button
            onClick={onClose}
            style={{
              background: 'transparent',
              border: 'none',
              color: 'var(--text-muted)',
              fontSize: 20,
              cursor: 'pointer',
              padding: '4px 8px',
            }}
          >
            ✕
          </button>
        </div>

        <div style={{ display: 'grid', gap: 16 }}>
          {SHORTCUT_GROUPS.map(group => (
            <div key={group.category} style={{ display: 'grid', gap: 8 }}>
              <h3 style={{ margin: 0, fontSize: 13, color: '#58a6ff', textTransform: 'uppercase', letterSpacing: '0.5px' }}>
                {group.category}
              </h3>
              <div
                style={{
                  background: 'rgba(255, 255, 255, 0.02)',
                  border: '1px solid rgba(255, 255, 255, 0.06)',
                  borderRadius: 8,
                  overflow: 'hidden',
                }}
              >
                {group.items.map((item, idx) => (
                  <div
                    key={idx}
                    style={{
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'center',
                      padding: '8px 12px',
                      borderBottom: idx < group.items.length - 1 ? '1px solid rgba(255, 255, 255, 0.04)' : 'none',
                    }}
                  >
                    <span style={{ fontSize: 13, color: 'var(--text-main)' }}>{item.description}</span>
                    <div style={{ display: 'flex', gap: 4 }}>
                      {item.keys.map((k, kidx) => (
                        <kbd
                          key={kidx}
                          style={{
                            padding: '3px 7px',
                            fontSize: 11,
                            fontWeight: 600,
                            fontFamily: 'monospace',
                            background: 'rgba(255, 255, 255, 0.08)',
                            color: 'white',
                            borderRadius: 4,
                            border: '1px solid rgba(255, 255, 255, 0.2)',
                            boxShadow: '0 2px 0 rgba(0, 0, 0, 0.4)',
                          }}
                        >
                          {k}
                        </kbd>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>

        <div style={{ display: 'flex', justifyContent: 'flex-end', paddingTop: 8, borderTop: '1px solid var(--border-color)' }}>
          <button className="btn-primary" onClick={onClose} style={{ padding: '6px 16px' }}>
            Đã hiểu (Esc)
          </button>
        </div>
      </div>
    </div>
  );
};
