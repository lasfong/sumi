import React, { useEffect, useMemo, useRef, useState } from 'react';
import { calculateReplaySignals } from '../../api/signalsApi';
import type { SignalOutputPoint } from '../../types/signals';

const STRICT_DAILY_TIMESTAMP_REGEX =
  /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.(\d+))?)?(?:Z|([+-])(\d{2})(?::?(\d{2}))?)?)?$/;

/**
 * Parses and strictly validates a daily market timestamp.
 * Returns the canonical YYYY-MM-DD string representing the Vietnam market date,
 * or null if invalid or impossible calendar date.
 */
const parseStrictDailyDate = (value?: string | null): string | null => {
  if (!value || typeof value !== 'string') return null;
  const match = value.match(STRICT_DAILY_TIMESTAMP_REGEX);
  if (!match) return null;

  const year = parseInt(match[1], 10);
  const month = parseInt(match[2], 10);
  const day = parseInt(match[3], 10);

  // Validate calendar date via UTC to prevent local timezone shift
  const utcDate = new Date(Date.UTC(year, month - 1, day));
  if (
    utcDate.getUTCFullYear() !== year ||
    utcDate.getUTCMonth() + 1 !== month ||
    utcDate.getUTCDate() !== day
  ) {
    return null;
  }

  // Validate time components if present
  if (match[4] !== undefined && match[5] !== undefined) {
    const hours = parseInt(match[4], 10);
    const minutes = parseInt(match[5], 10);
    if (hours < 0 || hours > 23 || minutes < 0 || minutes > 59) {
      return null;
    }
    if (match[6] !== undefined) {
      const seconds = parseInt(match[6], 10);
      if (seconds < 0 || seconds > 59) {
        return null;
      }
    }
    if (match[9] !== undefined) {
      const tzHours = parseInt(match[9], 10);
      const tzMinutes = match[10] !== undefined ? parseInt(match[10], 10) : 0;
      if (tzHours < 0 || tzHours > 14 || tzMinutes < 0 || tzMinutes > 59) {
        return null;
      }
    }
  }

  return `${match[1]}-${match[2]}-${match[3]}`;
};

export interface SignalInspectorProps {
  sessionId?: number | null;
  currentIndex?: number;
  timeframe?: string;
  currentTimestamp?: string;
}

interface AcceptedSignalData {
  occurrenceToken: symbol;
  generation: number;
  sessionId: number;
  index: number;
  timeframe: string;
  timestamp?: string;
  period: number;
  multiplier: number;
  resolvedPeriod: number;
  threshold: number | null;
  paramsHash: string;
  point: SignalOutputPoint;
}

interface ApiErrorData {
  occurrenceToken: symbol;
  message: string;
}

export const SignalInspector: React.FC<SignalInspectorProps> = ({
  sessionId,
  currentIndex = 0,
  timeframe = '1D',
  currentTimestamp,
}) => {
  const [periodInput, setPeriodInput] = useState<string>('20');
  const [multiplierInput, setMultiplierInput] = useState<string>('2.0');

  const [acceptedData, setAcceptedData] = useState<AcceptedSignalData | null>(null);
  const [apiError, setApiError] = useState<ApiErrorData | null>(null);
  const [isOpen, setIsOpen] = useState<boolean>(true);

  // Monotonically increasing request generation token
  const requestIdRef = useRef<number>(0);

  const isUnsupportedTimeframe = !!timeframe && timeframe !== '1D';

  const parsedPeriod = (() => {
    const trimmed = periodInput.trim();
    if (!/^-?\d+$/.test(trimmed)) return null;
    const n = Number(trimmed);
    if (!Number.isInteger(n)) return null;
    return n;
  })();

  const isPeriodValid = parsedPeriod !== null && parsedPeriod >= 1 && parsedPeriod <= 252;

  const parsedMultiplier = (() => {
    const trimmed = multiplierInput.trim();
    if (trimmed === '') return null;
    const n = Number(trimmed);
    if (!Number.isFinite(n)) return null;
    return n;
  })();

  const isMultiplierValid =
    parsedMultiplier !== null && parsedMultiplier > 0 && parsedMultiplier <= 100;

  const parsedCurrentTimestamp =
    currentTimestamp !== undefined ? parseStrictDailyDate(currentTimestamp) : null;

  const isTimestampValid =
    currentTimestamp === undefined || parsedCurrentTimestamp !== null;

  const validationError = !isPeriodValid
    ? 'Tham số chu kỳ (period) phải là số nguyên từ 1 đến 252.'
    : !isMultiplierValid
    ? 'Hệ số nhân (multiplier) phải lớn hơn 0 và nhỏ hơn hoặc bằng 100.'
    : !isTimestampValid
    ? 'Dấu thời gian nến yêu cầu không hợp lệ hoặc không đúng định dạng.'
    : null;

  const currentKey =
    !validationError && !!sessionId && !isUnsupportedTimeframe
      ? `${sessionId}_${currentIndex}_${parsedPeriod}_${parsedMultiplier}_${timeframe}_${parsedCurrentTimestamp ?? ''}`
      : null;

  // Render identity changes whenever currentKey changes, including A -> B -> A
  const occurrenceToken = useMemo(() => Symbol(currentKey ?? 'invalid'), [currentKey]);

  // Verify whether the currently accepted data strictly matches the visible context
  const isCurrentData =
    acceptedData !== null &&
    acceptedData.occurrenceToken === occurrenceToken &&
    acceptedData.sessionId === sessionId &&
    acceptedData.index === currentIndex &&
    acceptedData.timeframe === timeframe &&
    acceptedData.period === parsedPeriod &&
    acceptedData.multiplier === parsedMultiplier &&
    (currentTimestamp === undefined ||
      (parsedCurrentTimestamp !== null &&
        parseStrictDailyDate(acceptedData.point.timestamp) === parsedCurrentTimestamp &&
        parseStrictDailyDate(acceptedData.point.available_at_timestamp) === parsedCurrentTimestamp));

  const isCurrentError = apiError !== null && apiError.occurrenceToken === occurrenceToken;
  const errorMessage = isCurrentError ? apiError.message : null;

  const activePoint = isCurrentData ? acceptedData.point : null;
  const activeParamsHash = isCurrentData ? acceptedData.paramsHash : null;
  const activeResolvedPeriod = isCurrentData ? acceptedData.resolvedPeriod : null;

  const activeLoading =
    !validationError &&
    !!sessionId &&
    !isUnsupportedTimeframe &&
    !isCurrentData &&
    !errorMessage;

  useEffect(() => {
    if (
      !sessionId ||
      isUnsupportedTimeframe ||
      !isPeriodValid ||
      !isMultiplierValid ||
      !isTimestampValid ||
      parsedPeriod === null ||
      parsedMultiplier === null ||
      !currentKey
    ) {
      requestIdRef.current += 1;
      return;
    }

    // Monotonically increment generation
    const generation = ++requestIdRef.current;

    const capturedOccurrenceToken = occurrenceToken;
    const capturedSessionId = sessionId;
    const capturedIndex = currentIndex;
    const capturedTimeframe = timeframe;
    const capturedTimestamp = currentTimestamp;
    const capturedPeriod = parsedPeriod;
    const capturedMultiplier = parsedMultiplier;

    calculateReplaySignals(capturedSessionId, {
      signals: [
        {
          name: 'volume.spike',
          version: '1.0.0',
          params: { period: capturedPeriod, multiplier: capturedMultiplier },
        },
      ],
    })
      .then((response) => {
        // Discard late response if generation has changed
        if (generation !== requestIdRef.current) {
          return;
        }

        const failValidation = (message: string) => {
          setAcceptedData(null);
          setApiError({
            occurrenceToken: capturedOccurrenceToken,
            message,
          });
        };

        // Validate response envelope
        if (!response) {
          failValidation('Không nhận được dữ liệu phản hồi từ máy chủ');
          return;
        }
        if (response.session_id !== capturedSessionId) {
          failValidation(
            `Mã phiên không khớp (yêu cầu ${capturedSessionId}, nhận ${response.session_id})`
          );
          return;
        }
        if (response.observed_current_index !== capturedIndex) {
          failValidation(
            `Chỉ số nến quan sát không khớp (yêu cầu ${capturedIndex}, nhận ${response.observed_current_index})`
          );
          return;
        }
        if (response.timeframe !== capturedTimeframe) {
          failValidation(
            `Khung thời gian không khớp (yêu cầu ${capturedTimeframe}, nhận ${response.timeframe})`
          );
          return;
        }

        // Validate volume.spike series
        const spikeResult = response.results?.find((r) => r.signal_name === 'volume.spike');
        if (!spikeResult) {
          failValidation('Thiếu kết quả tín hiệu volume.spike trong phản hồi');
          return;
        }
        if (spikeResult.signal_version !== '1.0.0') {
          failValidation(
            `Phiên bản tín hiệu không khớp (yêu cầu 1.0.0, nhận ${spikeResult.signal_version})`
          );
          return;
        }
        if (
          typeof spikeResult.params_hash !== 'string' ||
          spikeResult.params_hash.trim().length === 0
        ) {
          failValidation('Mã băm tham số params_hash không hợp lệ hoặc rỗng');
          return;
        }
        if (!spikeResult.resolved_params) {
          failValidation('Thiếu thông tin tham số đã giải quyết (resolved_params)');
          return;
        }
        if (spikeResult.resolved_params.period !== capturedPeriod) {
          failValidation(
            `Tham số chu kỳ không khớp (yêu cầu ${capturedPeriod}, nhận ${spikeResult.resolved_params.period})`
          );
          return;
        }
        if (
          typeof spikeResult.resolved_params.multiplier !== 'number' ||
          spikeResult.resolved_params.multiplier !== capturedMultiplier
        ) {
          failValidation(
            `Hệ số nhân không khớp (yêu cầu ${capturedMultiplier}, nhận ${spikeResult.resolved_params.multiplier})`
          );
          return;
        }

        // Validate points: reject future points
        if (!Array.isArray(spikeResult.points)) {
          failValidation('Danh sách điểm dữ liệu points không hợp lệ');
          return;
        }
        const hasFuturePoints = spikeResult.points.some((p) => p.bar_index > capturedIndex);
        if (hasFuturePoints) {
          failValidation('Phản hồi chứa điểm dữ liệu tương lai');
          return;
        }

        // Exactly one point matching requested index (no fallback to last point)
        const matchingPoints = spikeResult.points.filter((p) => p.bar_index === capturedIndex);
        if (matchingPoints.length !== 1) {
          failValidation(
            `Cần đúng 1 điểm dữ liệu cho nến ${capturedIndex}, nhưng nhận được ${matchingPoints.length}`
          );
          return;
        }

        const matchedPoint = matchingPoints[0];

        // Validate availability event and index
        if (matchedPoint.availability_event !== 'BAR_CLOSE') {
          failValidation(`Sự kiện khả dụng không hợp lệ (${matchedPoint.availability_event})`);
          return;
        }
        if (matchedPoint.available_at_index !== capturedIndex) {
          failValidation(
            `Chỉ số nến khả dụng không khớp (yêu cầu ${capturedIndex}, nhận ${matchedPoint.available_at_index})`
          );
          return;
        }

        // Validate timestamp when currentTimestamp is supplied
        if (capturedTimestamp !== undefined) {
          const expectedKey = parseStrictDailyDate(capturedTimestamp);
          if (!expectedKey) {
            failValidation('Dấu thời gian nến yêu cầu không hợp lệ');
            return;
          }
          const pointDateKey = parseStrictDailyDate(matchedPoint.timestamp);
          if (!pointDateKey || pointDateKey !== expectedKey) {
            failValidation('Dấu thời gian điểm tín hiệu không hợp lệ hoặc không khớp nến hiện tại');
            return;
          }
          const availDateKey = parseStrictDailyDate(matchedPoint.available_at_timestamp);
          if (!availDateKey || availDateKey !== expectedKey) {
            failValidation('Dấu thời gian khả dụng không hợp lệ hoặc không khớp nến hiện tại');
            return;
          }
        }

        // All identity checks passed: accept response
        setAcceptedData({
          occurrenceToken: capturedOccurrenceToken,
          generation,
          sessionId: capturedSessionId,
          index: capturedIndex,
          timeframe: capturedTimeframe,
          timestamp: capturedTimestamp,
          period: capturedPeriod,
          multiplier: capturedMultiplier,
          resolvedPeriod: spikeResult.resolved_params.period,
          threshold: matchedPoint.threshold,
          paramsHash: spikeResult.params_hash,
          point: matchedPoint,
        });
        setApiError(null);
      })
      .catch((err) => {
        // A late rejected promise cannot replace a newer success or show an error
        if (generation !== requestIdRef.current) {
          return;
        }
        const detail =
          err.response?.data?.detail || err.message || 'Lỗi khi tính toán tín hiệu';
        setAcceptedData(null);
        setApiError({
          occurrenceToken: capturedOccurrenceToken,
          message: detail,
        });
      });

    return () => {
      requestIdRef.current += 1;
    };
  }, [
    occurrenceToken,
    sessionId,
    currentIndex,
    timeframe,
    currentTimestamp,
    parsedPeriod,
    parsedMultiplier,
    isPeriodValid,
    isMultiplierValid,
    isTimestampValid,
    isUnsupportedTimeframe,
    currentKey,
  ]);

  return (
    <div
      className="panel signal-inspector"
      data-testid="signal-inspector"
      style={{
        padding: '10px 12px',
        display: 'grid',
        gap: '8px',
        background: 'rgba(19, 23, 34, 0.85)',
        border: '1px solid rgba(88, 166, 255, 0.25)',
        borderRadius: '8px',
      }}
    >
      {/* Header */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          cursor: 'pointer',
        }}
        onClick={() => setIsOpen(!isOpen)}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span style={{ fontSize: '13px' }}>⚡</span>
          <strong style={{ fontSize: '12px', color: 'var(--text-main, #e6edf3)' }}>
            Tín Hiệu: Volume Spike
          </strong>
          <span
            style={{
              fontSize: '10px',
              padding: '1px 5px',
              borderRadius: '4px',
              background: 'rgba(56, 139, 253, 0.15)',
              color: '#58a6ff',
            }}
          >
            v1.0.0
          </span>
        </div>
        <span style={{ fontSize: '11px', color: 'var(--text-muted, #8b949e)' }}>
          {isOpen ? '▲' : '▼'}
        </span>
      </div>

      {isOpen && (
        <div style={{ display: 'grid', gap: '8px' }}>
          {/* Parameter Inputs */}
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px' }}>
            <div>
              <label
                style={{
                  fontSize: '10px',
                  color: 'var(--text-muted, #8b949e)',
                  display: 'block',
                  marginBottom: '2px',
                }}
              >
                Chu kỳ (Period)
              </label>
              <input
                type="number"
                data-testid="signal-period-input"
                aria-label="Signal Period"
                step="1"
                min={1}
                max={252}
                value={periodInput}
                onChange={(e) => setPeriodInput(e.target.value)}
                style={{
                  width: '100%',
                  padding: '4px 6px',
                  fontSize: '11px',
                  borderRadius: '4px',
                  background: 'rgba(13, 17, 23, 0.9)',
                  border: '1px solid rgba(88, 166, 255, 0.3)',
                  color: '#fff',
                  boxSizing: 'border-box',
                }}
              />
            </div>
            <div>
              <label
                style={{
                  fontSize: '10px',
                  color: 'var(--text-muted, #8b949e)',
                  display: 'block',
                  marginBottom: '2px',
                }}
              >
                Hệ số (Multiplier)
              </label>
              <input
                type="number"
                data-testid="signal-multiplier-input"
                aria-label="Signal Multiplier"
                step="any"
                value={multiplierInput}
                onChange={(e) => setMultiplierInput(e.target.value)}
                style={{
                  width: '100%',
                  padding: '4px 6px',
                  fontSize: '11px',
                  borderRadius: '4px',
                  background: 'rgba(13, 17, 23, 0.9)',
                  border: '1px solid rgba(88, 166, 255, 0.3)',
                  color: '#fff',
                  boxSizing: 'border-box',
                }}
              />
            </div>
          </div>

          {/* Unsupported Timeframe Alert */}
          {isUnsupportedTimeframe && (
            <div
              data-testid="signal-unsupported-timeframe"
              style={{
                padding: '6px 8px',
                borderRadius: '4px',
                background: 'rgba(210, 153, 34, 0.15)',
                border: '1px solid rgba(210, 153, 34, 0.4)',
                color: '#d29922',
                fontSize: '11px',
              }}
            >
              ⚠️ Khung thời gian {timeframe} chưa được hỗ trợ. Vui lòng chọn 1D.
            </div>
          )}

          {/* Validation Error Alert */}
          {validationError && (
            <div
              data-testid="signal-validation-error"
              style={{
                padding: '6px 8px',
                borderRadius: '4px',
                background: 'rgba(248, 81, 73, 0.15)',
                border: '1px solid rgba(248, 81, 73, 0.4)',
                color: '#f85149',
                fontSize: '11px',
              }}
            >
              {validationError}
            </div>
          )}

          {/* API Error Alert */}
          {errorMessage && (
            <div
              data-testid="signal-error"
              style={{
                padding: '6px 8px',
                borderRadius: '4px',
                background: 'rgba(248, 81, 73, 0.15)',
                border: '1px solid rgba(248, 81, 73, 0.4)',
                color: '#f85149',
                fontSize: '11px',
              }}
            >
              Lỗi: {errorMessage}
            </div>
          )}

          {/* Loading Indicator */}
          {activeLoading && (
            <div
              data-testid="signal-loading"
              style={{
                fontSize: '11px',
                color: 'var(--text-muted, #8b949e)',
                textAlign: 'center',
                padding: '8px 0',
              }}
            >
              ⏳ Đang tính toán tín hiệu...
            </div>
          )}

          {/* Result Display */}
          {!activeLoading && !validationError && !errorMessage && activePoint && (
            <div
              data-testid="signal-results"
              style={{
                display: 'grid',
                gap: '6px',
                background: 'rgba(13, 17, 23, 0.6)',
                padding: '8px',
                borderRadius: '6px',
                border: '1px solid rgba(255, 255, 255, 0.08)',
              }}
            >
              {/* Row 1: Spike Status and Quality */}
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ fontSize: '11px', color: 'var(--text-muted, #8b949e)' }}>
                  Trạng thái:
                </span>
                <div style={{ display: 'flex', gap: '6px', alignItems: 'center' }}>
                  <span
                    data-testid="signal-quality-badge"
                    style={{
                      fontSize: '10px',
                      padding: '1px 6px',
                      borderRadius: '4px',
                      fontWeight: 600,
                      background:
                        activePoint.quality === 'VALID'
                          ? 'rgba(46, 160, 67, 0.2)'
                          : activePoint.quality === 'INSUFFICIENT_HISTORY'
                          ? 'rgba(210, 153, 34, 0.2)'
                          : 'rgba(248, 81, 73, 0.2)',
                      color:
                        activePoint.quality === 'VALID'
                          ? '#3fb950'
                          : activePoint.quality === 'INSUFFICIENT_HISTORY'
                          ? '#d29922'
                          : '#f85149',
                    }}
                  >
                    {activePoint.quality}
                  </span>
                  <strong
                    data-testid="signal-spike-status"
                    style={{
                      fontSize: '11px',
                      color: activePoint.value === true ? '#3fb950' : '#8b949e',
                    }}
                  >
                    {activePoint.value === true
                      ? 'ĐỘT BIẾN'
                      : activePoint.value === false
                      ? 'BÌNH THƯỜNG'
                      : 'CHƯA ĐỦ DỮ LIỆU'}
                  </strong>
                </div>
              </div>

              {/* Row 2: RVOL & Threshold */}
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ fontSize: '11px', color: 'var(--text-muted, #8b949e)' }}>
                  RVOL / Ngưỡng:
                </span>
                <span style={{ fontSize: '11px', color: '#fff', fontWeight: 500 }}>
                  <span data-testid="signal-rvol-value">
                    {activePoint.relative_volume !== null
                      ? activePoint.relative_volume.toFixed(2)
                      : 'N/A'}
                  </span>
                  {' / '}
                  <span data-testid="signal-threshold-value">
                    {activePoint.threshold !== null ? `${activePoint.threshold}x` : 'N/A'}
                  </span>
                </span>
              </div>

              {/* Row 3: Current Volume / Baseline Volume */}
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ fontSize: '11px', color: 'var(--text-muted, #8b949e)' }}>
                  KL / TB ({activeResolvedPeriod !== null ? `${activeResolvedPeriod} phiên` : 'N/A'}):
                </span>
                <span style={{ fontSize: '11px', color: 'var(--text-muted, #8b949e)' }}>
                  <span data-testid="signal-current-volume" style={{ color: '#e6edf3' }}>
                    {activePoint.current_volume !== null
                      ? Math.round(activePoint.current_volume).toLocaleString('vi-VN')
                      : 'N/A'}
                  </span>
                  {' / '}
                  <span data-testid="signal-baseline">
                    {activePoint.baseline !== null
                      ? Math.round(activePoint.baseline).toLocaleString('vi-VN')
                      : 'N/A'}
                  </span>
                </span>
              </div>

              {/* Row 4: Reason Code */}
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ fontSize: '11px', color: 'var(--text-muted, #8b949e)' }}>Lý do:</span>
                <span
                  data-testid="signal-reason"
                  style={{ fontSize: '11px', color: '#58a6ff', fontFamily: 'monospace' }}
                >
                  {activePoint.reasons.join(', ') || 'N/A'}
                </span>
              </div>

              {/* Row 5: Availability & Bar info */}
              <div
                data-testid="signal-availability"
                style={{
                  fontSize: '10px',
                  color: 'var(--text-muted, #8b949e)',
                  borderTop: '1px solid rgba(255, 255, 256, 0.06)',
                  paddingTop: '4px',
                  display: 'flex',
                  justifyContent: 'space-between',
                }}
              >
                <span>Khớp: {activePoint.availability_event}</span>
                <span>Bar #{activePoint.bar_index}</span>
              </div>

              {/* Metadata hash */}
              {activeParamsHash && (
                <div
                  data-testid="signal-params-hash"
                  style={{
                    fontSize: '9px',
                    color: 'rgba(139, 148, 158, 0.6)',
                    fontFamily: 'monospace',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                  title={activeParamsHash}
                >
                  Hash: {activeParamsHash}
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
