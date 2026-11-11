import React from 'react';
import { SignalCatalog } from './SignalCatalog';
import type { SignalDefinition } from '../../types/signals';

export interface SignalCatalogModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelectSignal?: (signal: SignalDefinition) => void;
  selectedSignalName?: string;
}

export const SignalCatalogModal: React.FC<SignalCatalogModalProps> = ({
  isOpen,
  onClose,
  onSelectSignal,
  selectedSignalName,
}) => {
  if (!isOpen) return null;

  return (
    <div
      data-testid="signal-catalog-modal"
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: 'rgba(0, 0, 0, 0.75)',
        backdropFilter: 'blur(6px)',
        zIndex: 1000,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '24px',
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="glass-panel"
        style={{
          width: '100%',
          maxWidth: '1100px',
          maxHeight: '90vh',
          display: 'flex',
          flexDirection: 'column',
          padding: '24px',
          borderRadius: '12px',
          background: 'var(--bg-panel)',
          border: '1px solid var(--border-color)',
          boxShadow: '0 20px 40px rgba(0, 0, 0, 0.6)',
          position: 'relative',
        }}
      >
        <button
          type="button"
          data-testid="close-signal-catalog-btn"
          onClick={onClose}
          style={{
            position: 'absolute',
            top: '16px',
            right: '16px',
            background: 'transparent',
            border: 'none',
            color: 'var(--text-muted)',
            fontSize: '20px',
            cursor: 'pointer',
            padding: '4px 8px',
          }}
          aria-label="Đóng modal danh mục tín hiệu"
        >
          ✕
        </button>

        <SignalCatalog
          selectedSignalName={selectedSignalName}
          onSelectSignal={(sig) => {
            onSelectSignal?.(sig);
            onClose();
          }}
        />
      </div>
    </div>
  );
};
