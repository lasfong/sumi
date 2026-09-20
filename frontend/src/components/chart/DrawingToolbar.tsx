import React, { useCallback, useState } from 'react';
import {
  MousePointer,
  Minus,
  TrendingUp,
  ArrowUpRight,
  Square,
  SlidersHorizontal,
  Scale,
  Type,
  Magnet,
  Undo2,
  Redo2,
  Trash2,
} from 'lucide-react';
import type { MagnetMode } from '../../features/drawings/drawingMagnet';
import { TEXT_MAX_LENGTH, type DrawingTool } from '../../features/drawings/drawingDomain';
import { useModalFocus } from '../../hooks/useModalFocus';

interface Props {
  activeTool: DrawingTool;
  onSelectTool: (tool: DrawingTool) => void;
  onClearAll: () => void;
  onUndo: () => void;
  onRedo: () => void;
  canUndo: boolean;
  canRedo: boolean;
  pendingText: boolean;
  onCommitText: (text: string) => boolean;
  onCancelText: () => void;
  magnetMode: MagnetMode;
  onMagnetMode: (mode: MagnetMode) => void;
  persistenceStatus: string;
}

const TOOL_DEFS: Array<{
  id: DrawingTool;
  label: string;
  icon: React.FC<{ size?: number }>;
  hint: string;
}> = [
  { id: 'select', label: 'Con trỏ / Chọn', icon: MousePointer, hint: 'Chọn hình vẽ (Esc)' },
  { id: 'horizontal', label: 'Đường ngang (Horizontal)', icon: Minus, hint: 'Vẽ đường ngang (Alt+H)' },
  { id: 'trendline', label: 'Đường xu hướng (Trendline)', icon: TrendingUp, hint: 'Vẽ đường xu hướng (Alt+T)' },
  { id: 'ray', label: 'Tia (Ray)', icon: ArrowUpRight, hint: 'Vẽ tia mở rộng' },
  { id: 'rectangle', label: 'Hình chữ nhật (Rectangle)', icon: Square, hint: 'Vẽ vùng giá / Khối nến' },
  { id: 'fibonacci-retracement', label: 'Thước Fibonacci', icon: SlidersHorizontal, hint: 'Thước thoái lui Fibonacci' },
  { id: 'risk-reward', label: 'Thước Risk-Reward', icon: Scale, hint: 'Đo tỷ lệ R:R Lãi/Lỗ (Alt+R)' },
  { id: 'text', label: 'Ghi chú (Text)', icon: Type, hint: 'Đặt chữ ghi chú trên chart' },
];

export const DrawingToolbar: React.FC<Props> = (props) => {
  const [confirmClear, setConfirmClear] = useState(false);
  const [textDraft, setTextDraft] = useState('');
  const [dialogElement, setDialogElement] = useState<HTMLDivElement | null>(null);
  const { onCancelText } = props;
  const cancelText = useCallback(() => {
    setTextDraft('');
    onCancelText();
  }, [onCancelText]);

  useModalFocus(props.pendingText, cancelText, dialogElement);
  const commitText = () => {
    if (props.onCommitText(textDraft)) setTextDraft('');
  };

  const isMagnetOn = props.magnetMode !== 'off';

  return (
    <>
      <div
        className="drawing-toolbar"
        data-testid="drawing-toolbar"
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 4,
          padding: '8px 4px',
          background: '#131722',
          borderRight: '1px solid #2a2e39',
          width: 44,
          alignItems: 'center',
          overflowY: 'auto',
          userSelect: 'none',
          flexShrink: 0,
        }}
      >
        {/* Drawing Tools */}
        {TOOL_DEFS.map((tool) => {
          const IconComp = tool.icon;
          const isActive = props.activeTool === tool.id;

          return (
            <button
              key={tool.id}
              type="button"
              title={`${tool.label} — ${tool.hint}`}
              aria-label={tool.label}
              aria-description={tool.hint}
              data-testid={`drawing-tool-${tool.id}`}
              aria-pressed={isActive}
              onClick={() => props.onSelectTool(tool.id)}
              style={{
                width: 32,
                height: 32,
                padding: 0,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                background: isActive ? 'rgba(41, 98, 255, 0.2)' : 'transparent',
                color: isActive ? '#2962ff' : '#787b86',
                border: `1px solid ${isActive ? '#2962ff' : 'transparent'}`,
                borderRadius: 6,
                cursor: 'pointer',
                transition: 'all 0.15s ease',
              }}
              onMouseEnter={(e) => {
                if (!isActive) {
                  e.currentTarget.style.color = '#f0f3fa';
                  e.currentTarget.style.background = 'rgba(255,255,255,0.06)';
                }
              }}
              onMouseLeave={(e) => {
                if (!isActive) {
                  e.currentTarget.style.color = '#787b86';
                  e.currentTarget.style.background = 'transparent';
                }
              }}
            >
              <IconComp size={16} />
            </button>
          );
        })}

        <div style={{ width: 24, height: 1, background: '#2a2e39', margin: '4px 0' }} />

        {/* Magnet Tool Button with hidden select for backward compatibility */}
        <div style={{ position: 'relative' }}>
          <button
            type="button"
            title={isMagnetOn ? 'Nam châm: BẬT (Bám đỉnh/đáy OHLC)' : 'Nam châm: TẮT'}
            onClick={() => props.onMagnetMode(isMagnetOn ? 'off' : 'ohlc')}
            style={{
              width: 32,
              height: 32,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              background: isMagnetOn ? 'rgba(8, 153, 129, 0.2)' : 'transparent',
              color: isMagnetOn ? '#089981' : '#787b86',
              border: `1px solid ${isMagnetOn ? '#089981' : 'transparent'}`,
              borderRadius: 6,
              cursor: 'pointer',
              position: 'relative',
            }}
          >
            <Magnet size={16} />
            {isMagnetOn && (
              <span
                style={{
                  position: 'absolute',
                  top: 4,
                  right: 4,
                  width: 5,
                  height: 5,
                  borderRadius: '50%',
                  background: '#089981',
                }}
              />
            )}
          </button>
          <select
            aria-label="Drawing magnet mode"
            data-testid="drawing-magnet-mode"
            value={props.magnetMode}
            onChange={(e) => props.onMagnetMode(e.target.value as MagnetMode)}
            style={{ display: 'none' }}
          >
            <option value="off">Off</option>
            <option value="ohlc">OHLC</option>
          </select>
        </div>

        {/* Undo / Redo */}
        <button
          type="button"
          title="Hoàn tác (Undo)"
          aria-label="Undo drawing"
          data-testid="undo-drawing"
          disabled={!props.canUndo}
          onClick={props.onUndo}
          style={{
            width: 32,
            height: 32,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'transparent',
            border: 'none',
            color: props.canUndo ? '#787b86' : '#363a45',
            cursor: props.canUndo ? 'pointer' : 'not-allowed',
            borderRadius: 6,
          }}
        >
          <Undo2 size={16} />
        </button>

        <button
          type="button"
          title="Làm lại (Redo)"
          aria-label="Redo drawing"
          data-testid="redo-drawing"
          disabled={!props.canRedo}
          onClick={props.onRedo}
          style={{
            width: 32,
            height: 32,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'transparent',
            border: 'none',
            color: props.canRedo ? '#787b86' : '#363a45',
            cursor: props.canRedo ? 'pointer' : 'not-allowed',
            borderRadius: 6,
          }}
        >
          <Redo2 size={16} />
        </button>

        <div style={{ flex: 1 }} />

        {/* Persistence Status */}
        <span
          data-testid="drawing-persistence-status"
          title={`Trạng thái lưu trữ: ${props.persistenceStatus}`}
          style={{
            fontSize: '9px',
            fontFamily: 'monospace',
            color: props.persistenceStatus === 'ready' ? '#089981' : '#ff9800',
            writingMode: 'vertical-rl',
            letterSpacing: '1px',
            opacity: 0.8,
            marginBottom: 6,
          }}
        >
          {props.persistenceStatus}
        </span>

        {/* Clear All */}
        {!confirmClear ? (
          <button
            type="button"
            title="Xóa tất cả hình vẽ"
            aria-label="Clear All Drawings"
            data-testid="clear-all-drawings"
            onClick={() => setConfirmClear(true)}
            style={{
              width: 32,
              height: 32,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              background: 'transparent',
              border: 'none',
              color: '#787b86',
              cursor: 'pointer',
              borderRadius: 6,
            }}
          >
            <Trash2 size={15} />
          </button>
        ) : (
          <button
            type="button"
            data-testid="confirm-clear-drawings"
            onClick={() => {
              props.onClearAll();
              setConfirmClear(false);
            }}
            style={{
              width: 36,
              height: 24,
              fontSize: '10px',
              padding: 0,
              background: '#f23645',
              color: '#fff',
              border: 'none',
              borderRadius: 4,
              cursor: 'pointer',
              fontWeight: 600,
            }}
          >
            Xóa
          </button>
        )}
      </div>

      {/* Text Modal */}
      {props.pendingText && (
        <div
          ref={setDialogElement}
          role="dialog"
          aria-modal="true"
          aria-label="Create Text / Note"
          data-testid="drawing-text-dialog"
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 1000,
            background: 'rgba(0,0,0,0.65)',
            backdropFilter: 'blur(4px)',
            display: 'grid',
            placeItems: 'center',
          }}
        >
          <div
            className="panel"
            style={{
              width: 360,
              padding: 16,
              display: 'grid',
              gap: 10,
              background: '#1e222d',
              border: '1px solid #2a2e39',
              borderRadius: 8,
              boxShadow: '0 8px 32px rgba(0,0,0,0.5)',
            }}
          >
            <strong style={{ color: '#f0f3fa' }}>Ghi chú trên biểu đồ (Text / Note)</strong>
            <textarea
              autoFocus
              aria-label="Text note content"
              data-testid="new-drawing-text"
              value={textDraft}
              maxLength={TEXT_MAX_LENGTH}
              placeholder="Nhập ghi chú tại đây..."
              onChange={(e) => setTextDraft(e.target.value)}
              onKeyDown={(e) => {
                if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') commitText();
              }}
              style={{
                width: '100%',
                height: 80,
                background: '#131722',
                border: '1px solid #2a2e39',
                color: '#f0f3fa',
                padding: 8,
                borderRadius: 4,
                outline: 'none',
                resize: 'none',
              }}
            />
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <small style={{ color: '#787b86' }}>
                {textDraft.length}/{TEXT_MAX_LENGTH}
              </small>
              <div style={{ display: 'flex', gap: 8 }}>
                <button type="button" onClick={cancelText} style={{ background: '#2a2e39', border: 'none', color: '#b2b5be', padding: '6px 12px', borderRadius: 4 }}>
                  Hủy
                </button>
                <button
                  type="button"
                  data-testid="commit-drawing-text"
                  disabled={!textDraft.trim()}
                  onClick={commitText}
                  style={{
                    background: '#2962ff',
                    border: 'none',
                    color: '#fff',
                    padding: '6px 12px',
                    borderRadius: 4,
                    cursor: textDraft.trim() ? 'pointer' : 'not-allowed',
                    opacity: textDraft.trim() ? 1 : 0.5,
                  }}
                >
                  Thêm chữ
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </>
  );
};
