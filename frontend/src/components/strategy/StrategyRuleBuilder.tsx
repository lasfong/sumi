import React, { useState, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getSignalRegistry } from '../../api/signalsApi';
import type { SignalDefinition } from '../../types/signals';

export interface StrategyRuleBuilderProps {
  onRuleGenerated?: (ruleExpression: string) => void;
  className?: string;
}

interface RuleClause {
  signalAlias: string;
  operator: string;
  compareValue: string;
  logicalOp: 'and' | 'or';
}

const COMPARISON_OPERATORS = [
  { value: '==', label: 'Bằng (==)' },
  { value: '!=', label: 'Khác (!=)' },
  { value: '>', label: 'Lớn hơn (>)' },
  { value: '>=', label: 'Lớn hơn hoặc bằng (>=)' },
  { value: '<', label: 'Nhỏ hơn (<)' },
  { value: '<=', label: 'Nhỏ hơn hoặc bằng (<=)' },
];

export const StrategyRuleBuilder: React.FC<StrategyRuleBuilderProps> = ({
  onRuleGenerated,
  className = '',
}) => {
  const { data: registryData } = useQuery({
    queryKey: ['signal-registry'],
    queryFn: getSignalRegistry,
  });

  const allSignals = useMemo(() => registryData?.signals || [], [registryData]);

  const [clauses, setClauses] = useState<RuleClause[]>([
    {
      signalAlias: 'health__favorable',
      operator: '==',
      compareValue: 'True',
      logicalOp: 'and',
    },
    {
      signalAlias: 'bb__direction_rising',
      operator: '==',
      compareValue: 'True',
      logicalOp: 'and',
    },
  ]);

  const [ruleName, setRuleName] = useState<string>('Bo_Dieu_Kien_Mau');
  const [copied, setCopied] = useState<boolean>(false);

  // Group signals by category for convenient dropdown grouping
  const signalsByCategory = useMemo(() => {
    const map: Record<string, SignalDefinition[]> = {};
    allSignals.forEach((sig) => {
      const cat = sig.category || 'other';
      if (!map[cat]) map[cat] = [];
      map[cat].push(sig);
    });
    return map;
  }, [allSignals]);

  const handleClauseChange = (index: number, field: keyof RuleClause, value: string) => {
    setClauses((prev) => {
      const next = [...prev];
      next[index] = { ...next[index], [field]: value };
      return next;
    });
  };

  const handleAddClause = () => {
    setClauses((prev) => [
      ...prev,
      {
        signalAlias: allSignals[0]?.ast_alias || 'health__score',
        operator: '>',
        compareValue: '0',
        logicalOp: 'and',
      },
    ]);
  };

  const handleRemoveClause = (index: number) => {
    if (clauses.length <= 1) return;
    setClauses((prev) => prev.filter((_, i) => i !== index));
  };

  // Build the pure AST boolean expression
  const generatedExpression = useMemo(() => {
    if (clauses.length === 0) return '';
    return clauses
      .map((c, i) => {
        const clauseStr = `${c.signalAlias} ${c.operator} ${c.compareValue}`;
        if (i === 0) return clauseStr;
        return `${c.logicalOp} ${clauseStr}`;
      })
      .join(' ');
  }, [clauses]);

  // Build YAML representation
  const generatedYaml = useMemo(() => {
    return `# Biểu thức quy tắc sinh tự động từ Signal Registry
name: ${ruleName}
condition: "${generatedExpression}"
signals_required:
${Array.from(new Set(clauses.map((c) => `  - ${c.signalAlias}`))).join('\n')}
`;
  }, [ruleName, generatedExpression, clauses]);

  const handleCopyYaml = () => {
    navigator.clipboard.writeText(generatedYaml);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
    onRuleGenerated?.(generatedExpression);
  };

  return (
    <div className={`glass-panel strategy-rule-builder ${className}`} data-testid="strategy-rule-builder" style={{ padding: '20px', borderRadius: '8px' }}>
      <div style={{ marginBottom: '16px' }}>
        <h4 style={{ margin: '0 0 6px 0', fontSize: '16px', fontWeight: 600 }}>
          🛠️ Trình Soạn Quy Tắc Chiến Lược Chuẩn Hóa AST (Visual Rule Builder)
        </h4>
        <p style={{ margin: 0, fontSize: '13px', color: 'var(--text-muted)' }}>
          Tạo điều kiện Mua / Bán an toàn từ 72 tín hiệu chuẩn hóa trong hệ thống, loại bỏ lỗi cú pháp và ngăn chặn các biểu thức không hợp lệ (FR-CORE-006, F.4).
        </p>
      </div>

      {/* Rule Name Input */}
      <div style={{ marginBottom: '16px' }}>
        <label htmlFor="rule-name-input" style={{ display: 'block', fontSize: '12px', color: 'var(--text-muted)', marginBottom: '4px' }}>
          Tên quy tắc (Rule Name):
        </label>
        <input
          id="rule-name-input"
          type="text"
          value={ruleName}
          onChange={(e) => setRuleName(e.target.value)}
          style={{
            padding: '8px 12px',
            fontSize: '13px',
            borderRadius: '4px',
            background: 'rgba(255, 255, 255, 0.05)',
            border: '1px solid var(--border-color)',
            color: 'var(--text-main)',
            width: '260px',
          }}
        />
      </div>

      {/* Clauses list */}
      <div style={{ marginBottom: '16px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
          <span style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-muted)' }}>Các Mệnh Đề Điều Kiện (Clauses):</span>
          <button
            type="button"
            data-testid="add-clause-btn"
            onClick={handleAddClause}
            style={{
              padding: '4px 8px',
              fontSize: '11px',
              borderRadius: '4px',
              background: 'rgba(41, 98, 255, 0.2)',
              color: 'var(--color-primary)',
              border: '1px solid var(--color-primary)',
              cursor: 'pointer',
            }}
          >
            + Thêm Mệnh Đề
          </button>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {clauses.map((clause, idx) => {
            const sigDef = allSignals.find((s) => s.ast_alias === clause.signalAlias);
            return (
              <div
                key={idx}
                data-testid={`clause-row-${idx}`}
                style={{
                  display: 'grid',
                  gridTemplateColumns: idx === 0 ? '2.5fr 1.2fr 1fr 32px' : '70px 2.5fr 1.2fr 1fr 32px',
                  gap: '8px',
                  alignItems: 'center',
                  background: 'rgba(255, 255, 255, 0.02)',
                  padding: '8px',
                  borderRadius: '4px',
                  border: '1px solid rgba(255, 255, 255, 0.06)',
                }}
              >
                {idx > 0 && (
                  <select
                    value={clause.logicalOp}
                    onChange={(e) => handleClauseChange(idx, 'logicalOp', e.target.value)}
                    style={{
                      padding: '6px',
                      fontSize: '12px',
                      background: 'rgba(41, 98, 255, 0.2)',
                      border: '1px solid var(--color-primary)',
                      color: 'var(--color-primary)',
                      borderRadius: '4px',
                      fontWeight: 600,
                    }}
                  >
                    <option value="and" style={{ background: '#1c1f26', color: '#fff' }}>AND</option>
                    <option value="or" style={{ background: '#1c1f26', color: '#fff' }}>OR</option>
                  </select>
                )}

                <select
                  data-testid={`clause-signal-select-${idx}`}
                  value={clause.signalAlias}
                  onChange={(e) => {
                    const chosen = allSignals.find((s) => s.ast_alias === e.target.value);
                    const defaultVal = chosen?.output_type === 'bool' ? 'True' : '0';
                    const defaultOp = chosen?.output_type === 'bool' ? '==' : clause.operator;
                    handleClauseChange(idx, 'signalAlias', e.target.value);
                    handleClauseChange(idx, 'compareValue', defaultVal);
                    handleClauseChange(idx, 'operator', defaultOp);
                  }}
                  style={{
                    padding: '6px 8px',
                    fontSize: '12px',
                    background: 'rgba(255, 255, 255, 0.05)',
                    border: '1px solid var(--border-color)',
                    color: 'var(--text-main)',
                    borderRadius: '4px',
                  }}
                >
                  {Object.keys(signalsByCategory).map((cat) => (
                    <optgroup key={cat} label={`=== ${cat.toUpperCase()} ===`}>
                      {signalsByCategory[cat].map((sig) => (
                        <option key={sig.name} value={sig.ast_alias} style={{ background: '#1c1f26', color: '#fff' }}>
                          {sig.label_vi} ({sig.ast_alias})
                        </option>
                      ))}
                    </optgroup>
                  ))}
                </select>

                <select
                  value={clause.operator}
                  onChange={(e) => handleClauseChange(idx, 'operator', e.target.value)}
                  style={{
                    padding: '6px 8px',
                    fontSize: '12px',
                    background: 'rgba(255, 255, 255, 0.05)',
                    border: '1px solid var(--border-color)',
                    color: 'var(--text-main)',
                    borderRadius: '4px',
                  }}
                >
                  {COMPARISON_OPERATORS.map((op) => (
                    <option key={op.value} value={op.value} style={{ background: '#1c1f26', color: '#fff' }}>
                      {op.label}
                    </option>
                  ))}
                </select>

                {sigDef?.output_type === 'bool' ? (
                  <select
                    value={clause.compareValue}
                    onChange={(e) => handleClauseChange(idx, 'compareValue', e.target.value)}
                    style={{
                      padding: '6px 8px',
                      fontSize: '12px',
                      background: 'rgba(255, 255, 255, 0.05)',
                      border: '1px solid var(--border-color)',
                      color: 'var(--text-main)',
                      borderRadius: '4px',
                    }}
                  >
                    <option value="True" style={{ background: '#1c1f26', color: '#fff' }}>True (Đúng)</option>
                    <option value="False" style={{ background: '#1c1f26', color: '#fff' }}>False (Sai)</option>
                  </select>
                ) : (
                  <input
                    type="text"
                    value={clause.compareValue}
                    onChange={(e) => handleClauseChange(idx, 'compareValue', e.target.value)}
                    placeholder="Giá trị so sánh"
                    style={{
                      padding: '6px 8px',
                      fontSize: '12px',
                      background: 'rgba(255, 255, 255, 0.05)',
                      border: '1px solid var(--border-color)',
                      color: 'var(--text-main)',
                      borderRadius: '4px',
                    }}
                  />
                )}

                <button
                  type="button"
                  disabled={clauses.length <= 1}
                  onClick={() => handleRemoveClause(idx)}
                  style={{
                    padding: '6px',
                    background: 'transparent',
                    border: 'none',
                    color: clauses.length <= 1 ? 'gray' : 'var(--color-sell)',
                    cursor: clauses.length <= 1 ? 'not-allowed' : 'pointer',
                    fontSize: '14px',
                  }}
                  title="Xóa mệnh đề"
                >
                  ✕
                </button>
              </div>
            );
          })}
        </div>
      </div>

      {/* Generated Expression & YAML Preview */}
      <div style={{ background: 'rgba(0, 0, 0, 0.3)', padding: '14px', borderRadius: '6px', marginBottom: '16px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '6px' }}>
          <span style={{ fontSize: '12px', fontWeight: 600, color: '#58A6FF' }}>Biểu thức AST Hợp Lệ:</span>
          <button
            type="button"
            data-testid="copy-rule-yaml-btn"
            onClick={handleCopyYaml}
            style={{
              padding: '4px 10px',
              fontSize: '11px',
              borderRadius: '4px',
              background: 'rgba(255, 255, 255, 0.08)',
              border: '1px solid var(--border-color)',
              color: copied ? 'var(--color-buy)' : 'var(--text-main)',
              cursor: 'pointer',
            }}
          >
            {copied ? '✓ Đã Sao Chép YAML' : '📋 Sao Chép Cấu Hình YAML'}
          </button>
        </div>
        <div
          data-testid="generated-ast-expression"
          style={{
            fontFamily: 'monospace',
            fontSize: '13px',
            color: '#00E676',
            background: 'rgba(0, 0, 0, 0.4)',
            padding: '10px',
            borderRadius: '4px',
            wordBreak: 'break-all',
          }}
        >
          {generatedExpression || '(Chưa có điều kiện)'}
        </div>
      </div>
    </div>
  );
};
