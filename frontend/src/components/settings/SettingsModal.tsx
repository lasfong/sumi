import React, { useEffect, useRef } from 'react';
import { Database, ShieldCheck, Keyboard, Info, X, Server, HardDrive, Cpu } from 'lucide-react';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({ isOpen, onClose }) => {
  const modalContentRef = useRef<HTMLDivElement>(null);

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

  const envMode = typeof import.meta !== 'undefined' && import.meta.env?.MODE ? import.meta.env.MODE : 'development';

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Cài đặt hệ thống"
      data-testid="settings-modal"
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 2000,
        backgroundColor: 'rgba(0, 0, 0, 0.75)',
        display: 'grid',
        placeItems: 'center',
        padding: '16px',
        backdropFilter: 'blur(6px)',
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={modalContentRef}
        className="glass-panel"
        style={{
          width: 'min(720px, 95vw)',
          maxHeight: '90vh',
          overflowY: 'auto',
          background: 'rgba(19, 23, 34, 0.98)',
          border: '1px solid rgba(41, 98, 255, 0.4)',
          borderRadius: '12px',
          boxShadow: '0 20px 50px rgba(0, 0, 0, 0.8)',
          display: 'flex',
          flexDirection: 'column',
          gap: '20px',
          padding: '24px',
          color: 'var(--text-main, #e6edf3)',
        }}
      >
        {/* Header */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderBottom: '1px solid rgba(255, 255, 255, 0.1)', paddingBottom: '16px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <div
              style={{
                width: '36px',
                height: '36px',
                borderRadius: '8px',
                background: 'rgba(41, 98, 255, 0.2)',
                border: '1px solid rgba(41, 98, 255, 0.5)',
                display: 'grid',
                placeItems: 'center',
                color: '#58a6ff',
              }}
            >
              ⚙️
            </div>
            <div>
              <h2 style={{ margin: 0, fontSize: '18px', fontWeight: 600 }}>Cài Đặt Hệ Thống (Settings)</h2>
              <span style={{ fontSize: '12px', color: 'var(--text-muted, #8b949e)' }}>
                Cấu hình hệ thống, phím tắt thao tác và thông tin môi trường Sumi V3
              </span>
            </div>
          </div>
          <button
            type="button"
            data-testid="close-settings-btn"
            onClick={onClose}
            aria-label="Đóng cửa sổ cài đặt"
            style={{
              background: 'transparent',
              border: 'none',
              color: 'var(--text-muted, #8b949e)',
              cursor: 'pointer',
              padding: '6px',
              borderRadius: '6px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              transition: 'all 0.2s',
            }}
          >
            <X size={20} />
          </button>
        </div>

        {/* Section 1: Cấu hình hệ thống */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', color: '#58a6ff', fontSize: '13px', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.5px' }}>
            <Server size={16} />
            <span>Cấu Hình Hệ Thống (System Configuration)</span>
          </div>

          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))',
              gap: '12px',
            }}
          >
            <div
              style={{
                background: 'rgba(255, 255, 255, 0.03)',
                border: '1px solid rgba(255, 255, 255, 0.08)',
                borderRadius: '8px',
                padding: '12px 14px',
                display: 'flex',
                flexDirection: 'column',
                gap: '4px',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12px', color: 'var(--text-muted, #8b949e)' }}>
                <HardDrive size={14} />
                <span>Cơ Sở Dữ Liệu (Database)</span>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginTop: '2px' }}>
                <span style={{ fontSize: '13px', fontWeight: 600, fontFamily: 'monospace' }}>SQLite (backend/sumi.db)</span>
                <span
                  style={{
                    fontSize: '11px',
                    fontWeight: 600,
                    color: '#3fb950',
                    background: 'rgba(63, 185, 80, 0.15)',
                    padding: '2px 8px',
                    borderRadius: '12px',
                    border: '1px solid rgba(63, 185, 80, 0.3)',
                  }}
                >
                  Sẵn sàng
                </span>
              </div>
            </div>

            <div
              style={{
                background: 'rgba(255, 255, 255, 0.03)',
                border: '1px solid rgba(255, 255, 255, 0.08)',
                borderRadius: '8px',
                padding: '12px 14px',
                display: 'flex',
                flexDirection: 'column',
                gap: '4px',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12px', color: 'var(--text-muted, #8b949e)' }}>
                <ShieldCheck size={14} />
                <span>Chế Độ Hoạt Động (Operation Mode)</span>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginTop: '2px' }}>
                <span style={{ fontSize: '13px', fontWeight: 600 }}>Local-First (Offline Ready)</span>
                <span
                  style={{
                    fontSize: '11px',
                    fontWeight: 600,
                    color: '#58a6ff',
                    background: 'rgba(41, 98, 255, 0.15)',
                    padding: '2px 8px',
                    borderRadius: '12px',
                    border: '1px solid rgba(41, 98, 255, 0.3)',
                  }}
                >
                  Zero Telemetry
                </span>
              </div>
            </div>

            <div
              style={{
                background: 'rgba(255, 255, 255, 0.03)',
                border: '1px solid rgba(255, 255, 255, 0.08)',
                borderRadius: '8px',
                padding: '12px 14px',
                display: 'flex',
                flexDirection: 'column',
                gap: '4px',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12px', color: 'var(--text-muted, #8b949e)' }}>
                <Cpu size={14} />
                <span>Backend API URL</span>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginTop: '2px' }}>
                <span style={{ fontSize: '13px', fontWeight: 600, fontFamily: 'monospace' }}>http://localhost:8000</span>
                <span style={{ fontSize: '11px', color: '#3fb950' }}>● Connected</span>
              </div>
            </div>

            <div
              style={{
                background: 'rgba(255, 255, 255, 0.03)',
                border: '1px solid rgba(255, 255, 255, 0.08)',
                borderRadius: '8px',
                padding: '12px 14px',
                display: 'flex',
                flexDirection: 'column',
                gap: '4px',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12px', color: 'var(--text-muted, #8b949e)' }}>
                <Database size={14} />
                <span>Trạng Thái Bộ Đệm (Data Cache)</span>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginTop: '2px' }}>
                <span style={{ fontSize: '13px', fontWeight: 600 }}>Active (In-Memory & IndexedDB)</span>
                <span style={{ fontSize: '11px', color: '#8b949e' }}>Tự động đồng bộ</span>
              </div>
            </div>
          </div>
        </div>

        {/* Section 2: Phím tắt thao tác */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', color: '#58a6ff', fontSize: '13px', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.5px' }}>
            <Keyboard size={16} />
            <span>Bảng Phím Tắt Hệ Thống (Keyboard Shortcuts)</span>
          </div>

          <div
            style={{
              background: 'rgba(255, 255, 255, 0.02)',
              border: '1px solid rgba(255, 255, 255, 0.06)',
              borderRadius: '8px',
              overflow: 'hidden',
            }}
          >
            {[
              { keys: ['Space'], desc: 'Tiến 1 nến (Step next candle / Play / Pause)' },
              { keys: ['→'], desc: 'Tiến 1 nến kế tiếp' },
              { keys: ['←'], desc: 'Lùi 1 nến trước đó' },
              { keys: ['Shift', '→'], desc: 'Tua nhanh +5 nến' },
              { keys: ['B'], desc: 'Mở nhanh lệnh Mua (BUY ticket)' },
              { keys: ['S'], desc: 'Mở nhanh lệnh Bán (SELL ticket)' },
              { keys: ['C'], desc: 'Đóng lệnh / Mở Checklist quan sát (Close / Checklist)' },
              { keys: ['1 - 9'], desc: 'Chuyển nhanh khung thời gian (1D, 1W, 1M,...)' },
              { keys: ['Esc'], desc: 'Hủy vẽ / Đóng cửa sổ lệnh hoặc popup' },
            ].map((shortcut, index, arr) => (
              <div
                key={shortcut.desc}
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  padding: '8px 14px',
                  borderBottom: index < arr.length - 1 ? '1px solid rgba(255, 255, 255, 0.04)' : 'none',
                }}
              >
                <span style={{ fontSize: '13px', color: 'var(--text-main, #e6edf3)' }}>{shortcut.desc}</span>
                <div style={{ display: 'flex', gap: '4px' }}>
                  {shortcut.keys.map((k) => (
                    <kbd
                      key={k}
                      style={{
                        padding: '2px 8px',
                        fontSize: '11px',
                        fontWeight: 600,
                        fontFamily: 'monospace',
                        background: 'rgba(255, 255, 255, 0.08)',
                        color: '#fff',
                        borderRadius: '4px',
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

        {/* Section 3: Phiên bản & Môi trường */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', color: '#58a6ff', fontSize: '13px', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.5px' }}>
            <Info size={16} />
            <span>Phiên Bản & Môi Trường (Version & Environment)</span>
          </div>

          <div
            style={{
              background: 'rgba(255, 255, 255, 0.02)',
              border: '1px solid rgba(255, 255, 255, 0.06)',
              borderRadius: '8px',
              padding: '12px 16px',
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
              gap: '12px',
            }}
          >
            <div>
              <div style={{ fontSize: '11px', color: 'var(--text-muted, #8b949e)' }}>Phiên bản Sumi</div>
              <div style={{ fontSize: '14px', fontWeight: 700, color: '#58a6ff', marginTop: '2px' }}>v3.0.0</div>
            </div>
            <div>
              <div style={{ fontSize: '11px', color: 'var(--text-muted, #8b949e)' }}>Môi trường chạy</div>
              <div style={{ fontSize: '13px', fontWeight: 600, textTransform: 'capitalize', marginTop: '2px' }}>{envMode}</div>
            </div>
            <div>
              <div style={{ fontSize: '11px', color: 'var(--text-muted, #8b949e)' }}>Nền tảng</div>
              <div style={{ fontSize: '13px', fontWeight: 500, marginTop: '2px' }}>Sumi Quantitative Replay Platform</div>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div
          style={{
            display: 'flex',
            justifyContent: 'flex-end',
            borderTop: '1px solid rgba(255, 255, 255, 0.1)',
            paddingTop: '14px',
          }}
        >
          <button
            type="button"
            onClick={onClose}
            style={{
              padding: '8px 20px',
              background: 'rgba(41, 98, 255, 0.25)',
              border: '1px solid var(--color-primary, #2962ff)',
              color: '#fff',
              borderRadius: '6px',
              cursor: 'pointer',
              fontWeight: 500,
              fontSize: '13px',
              transition: 'all 0.2s',
            }}
          >
            Đóng (Esc)
          </button>
        </div>
      </div>
    </div>
  );
};
