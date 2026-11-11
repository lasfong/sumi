import React, { useEffect, useRef, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Search, TrendingUp, X } from 'lucide-react';
import { getSymbols } from '../../api/symbolsApi';
import type { StockSymbol } from '../../types';

interface SymbolSwitcherModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelectSymbol: (symbol: string) => void;
  currentSymbol?: string;
}

const POPULAR_VN30 = ['VNINDEX', 'FPT', 'SSI', 'HPG', 'VNM', 'VIC', 'MWG', 'TCB', 'MBB'];

const SymbolSwitcherContent: React.FC<Omit<SymbolSwitcherModalProps, 'isOpen'>> = ({
  onClose,
  onSelectSymbol,
  currentSymbol,
}) => {
  const [search, setSearch] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const { data: symbols = [], isLoading } = useQuery({
    queryKey: ['symbols', search],
    queryFn: () => getSymbols({ search: search.trim() || undefined }),
    staleTime: 60000,
  });

  useEffect(() => {
    const timer = setTimeout(() => inputRef.current?.focus(), 50);
    return () => clearTimeout(timer);
  }, []);

  const displayList = symbols.slice(0, 15);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex(prev => Math.min(prev + 1, displayList.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex(prev => Math.max(prev - 1, 0));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (displayList[selectedIndex]) {
        onSelectSymbol(displayList[selectedIndex].symbol);
        onClose();
      } else if (search.trim()) {
        onSelectSymbol(search.trim().toUpperCase());
        onClose();
      }
    }
  };

  const handleSelect = (sym: string) => {
    onSelectSymbol(sym.toUpperCase());
    onClose();
  };

  return (
    <div
      role="presentation"
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 300,
        background: 'rgba(0, 0, 0, 0.75)',
        backdropFilter: 'blur(4px)',
        display: 'grid',
        placeItems: 'center',
      }}
      onMouseDown={e => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Symbol Switcher"
        data-testid="symbol-switcher-modal"
        style={{
          width: 'min(540px, calc(100vw - 32px))',
          background: '#161B22',
          border: '1px solid #30363D',
          borderRadius: '12px',
          boxShadow: '0 20px 40px rgba(0,0,0,0.8)',
          overflow: 'hidden',
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        {/* Search Header */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            padding: '14px 18px',
            borderBottom: '1px solid #30363D',
            gap: 12,
            background: '#0D1117',
          }}
        >
          <Search size={18} color="#58A6FF" />
          <input
            ref={inputRef}
            data-testid="symbol-switcher-input"
            value={search}
            onChange={e => {
              setSearch(e.target.value);
              setSelectedIndex(0);
            }}
            onKeyDown={handleKeyDown}
            placeholder="Tìm mã cổ phiếu (FPT, SSI, HPG, VNINDEX...)"
            style={{
              flex: 1,
              background: 'transparent',
              border: 'none',
              outline: 'none',
              color: '#F0F6FC',
              fontSize: '15px',
              fontWeight: 500,
            }}
          />
          <button
            type="button"
            data-testid="close-symbol-switcher"
            onClick={onClose}
            aria-label="Close"
            title="Đóng"
            style={{
              background: 'transparent',
              border: 'none',
              color: '#8B949E',
              cursor: 'pointer',
              padding: 4,
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <X size={18} />
          </button>
        </div>

        {/* Quick Popular VN30 Chips */}
        <div
          style={{
            padding: '10px 18px',
            borderBottom: '1px solid rgba(255,255,255,0.06)',
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            flexWrap: 'wrap',
            background: 'rgba(255,255,255,0.02)',
          }}
        >
          <span style={{ fontSize: '11px', color: '#8B949E', display: 'flex', alignItems: 'center', gap: 4 }}>
            <TrendingUp size={12} /> Phổ biến:
          </span>
          {POPULAR_VN30.map(chip => (
            <button
              key={chip}
              type="button"
              data-testid={`symbol-chip-${chip}`}
              onClick={() => handleSelect(chip)}
              style={{
                background: currentSymbol === chip ? 'rgba(41, 98, 255, 0.25)' : 'rgba(255,255,255,0.05)',
                color: currentSymbol === chip ? '#58A6FF' : '#C9D1D9',
                border: currentSymbol === chip ? '1px solid #2962FF' : '1px solid rgba(255,255,255,0.08)',
                borderRadius: '4px',
                padding: '2px 7px',
                fontSize: '11px',
                fontWeight: 600,
                cursor: 'pointer',
              }}
            >
              {chip}
            </button>
          ))}
        </div>

        {/* Results List */}
        <div
          data-testid="symbol-switcher-results"
          style={{
            maxHeight: '340px',
            overflowY: 'auto',
            padding: '6px 0',
          }}
        >
          {isLoading && (
            <div style={{ padding: '20px', textAlign: 'center', color: '#8B949E', fontSize: '13px' }}>
              Đang tìm kiếm mã...
            </div>
          )}

          {!isLoading && displayList.length === 0 && (
            <div style={{ padding: '24px', textAlign: 'center', color: '#8B949E', fontSize: '13px' }}>
              {search.trim() ? (
                <>
                  Không tìm thấy kết quả cho &ldquo;{search}&rdquo;.
                  <br />
                  <button
                    type="button"
                    onClick={() => handleSelect(search)}
                    style={{
                      marginTop: 8,
                      background: '#238636',
                      color: '#fff',
                      border: 'none',
                      padding: '4px 12px',
                      borderRadius: 4,
                      cursor: 'pointer',
                      fontSize: '12px',
                    }}
                  >
                    Vẫn thử mở &ldquo;{search.toUpperCase()}&rdquo;
                  </button>
                </>
              ) : (
                'Gõ ký tự để bắt đầu tìm kiếm mã'
              )}
            </div>
          )}

          {!isLoading &&
            displayList.map((s: StockSymbol, index: number) => {
              const isSelected = index === selectedIndex;
              const isCurrent = s.symbol === currentSymbol;

              return (
                <div
                  key={s.symbol}
                  data-testid={`symbol-item-${s.symbol}`}
                  onClick={() => handleSelect(s.symbol)}
                  onMouseEnter={() => setSelectedIndex(index)}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '8px 18px',
                    cursor: 'pointer',
                    background: isSelected ? 'rgba(56, 139, 253, 0.15)' : 'transparent',
                    borderLeft: isSelected ? '3px solid #58A6FF' : '3px solid transparent',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                    <span style={{ fontWeight: 700, fontSize: '14px', color: '#F0F6FC' }}>
                      {s.symbol}
                    </span>
                    {s.company_name && (
                      <span style={{ fontSize: '12px', color: '#8B949E', maxWidth: '300px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                        {s.company_name}
                      </span>
                    )}
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                    {isCurrent && (
                      <span style={{ fontSize: '10px', background: 'rgba(35, 134, 54, 0.2)', color: '#3FB950', padding: '1px 5px', borderRadius: 3 }}>
                        Đang xem
                      </span>
                    )}
                    {s.exchange && (
                      <span style={{ fontSize: '11px', color: '#6E7681', textTransform: 'uppercase' }}>
                        {s.exchange}
                      </span>
                    )}
                  </div>
                </div>
              );
            })}
        </div>

        {/* Footer info */}
        <div
          style={{
            padding: '8px 18px',
            borderTop: '1px solid #30363D',
            background: '#0D1117',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            fontSize: '11px',
            color: '#6E7681',
          }}
        >
          <span>↑↓ Di chuyển · Enter Chọn mã · Esc Đóng</span>
          <span>Phím tắt: / để mở nhanh</span>
        </div>
      </div>
    </div>
  );
};

export const SymbolSwitcherModal: React.FC<SymbolSwitcherModalProps> = ({ isOpen, ...props }) => {
  if (!isOpen) return null;
  return <SymbolSwitcherContent {...props} />;
};

