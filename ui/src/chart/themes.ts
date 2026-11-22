import type { CandleType, DeepPartial, Styles, TooltipFeatureStyle } from 'klinecharts'
import type { ChartStyleKind, ThemeMode } from '../types/market'

const FONT_FAMILY = '-apple-system, BlinkMacSystemFont, "Trebuchet MS", Roboto, Ubuntu, sans-serif'

/**
 * Crisp 0..13px SVG path feature buttons rendered directly inside KLineChart v10's
 * on-chart indicator legend (Eye Hide/Show, Gear Settings, X Delete) — identical to TradingView!
 */
export function buildIndicatorTooltipFeatures(
  theme: ThemeMode,
  visible = true,
): TooltipFeatureStyle[] {
  const isDark = theme === 'dark'
  const color = isDark ? '#8A8E9B' : '#5D606B'
  const activeColor = isDark ? '#FFFFFF' : '#131722'
  const activeBg = isDark ? 'rgba(255, 255, 255, 0.14)' : 'rgba(19, 23, 34, 0.09)'
  const eyePath = visible
    ? 'M1 6.5 Q6.5 1.8 12 6.5 Q6.5 11.2 1 6.5 Z M6.5 4.8 A1.7 1.7 0 1 0 6.5 8.2 A1.7 1.7 0 1 0 6.5 4.8 Z'
    : 'M1 6.5 Q6.5 1.8 12 6.5 Q6.5 11.2 1 6.5 Z M2 11 L11 2'

  return [
    {
      id: 'toggle_visible',
      position: 'middle',
      marginLeft: 6,
      marginTop: 1,
      marginRight: 1,
      marginBottom: 1,
      paddingLeft: 3,
      paddingTop: 2,
      paddingRight: 3,
      paddingBottom: 2,
      size: 13,
      color: visible ? color : '#2962FF',
      activeColor,
      backgroundColor: 'transparent',
      activeBackgroundColor: activeBg,
      borderRadius: 3,
      type: 'path',
      content: {
        style: 'stroke',
        lineWidth: 1.25,
        path: eyePath,
      },
    },
    {
      id: 'settings',
      position: 'middle',
      marginLeft: 2,
      marginTop: 1,
      marginRight: 1,
      marginBottom: 1,
      paddingLeft: 3,
      paddingTop: 2,
      paddingRight: 3,
      paddingBottom: 2,
      size: 13,
      color,
      activeColor,
      backgroundColor: 'transparent',
      activeBackgroundColor: activeBg,
      borderRadius: 3,
      type: 'path',
      content: {
        style: 'stroke',
        lineWidth: 1.25,
        path: 'M6.5 4.2 A2.3 2.3 0 1 0 6.5 8.8 A2.3 2.3 0 1 0 6.5 4.2 Z M6.5 1 L6.5 2.6 M6.5 10.4 L6.5 12 M1 6.5 L2.6 6.5 M10.4 6.5 L12 6.5 M2.6 2.6 L3.8 3.8 M9.2 9.2 L10.4 10.4 M10.4 2.6 L9.2 3.8 M3.8 9.2 L2.6 10.4',
      },
    },
    {
      id: 'remove',
      position: 'middle',
      marginLeft: 2,
      marginTop: 1,
      marginRight: 4,
      marginBottom: 1,
      paddingLeft: 3,
      paddingTop: 2,
      paddingRight: 3,
      paddingBottom: 2,
      size: 13,
      color,
      activeColor: '#F23645',
      backgroundColor: 'transparent',
      activeBackgroundColor: 'rgba(242, 54, 69, 0.16)',
      borderRadius: 3,
      type: 'path',
      content: {
        style: 'stroke',
        lineWidth: 1.35,
        path: 'M3 3 L10 10 M10 3 L3 10',
      },
    },
  ]
}

export function buildSuperchartsStyles(
  theme: ThemeMode,
  chartStyle: ChartStyleKind,
  showCountdown: boolean,
): DeepPartial<Styles> {
  const isDark = theme === 'dark'

  const gridColor = isDark ? 'rgba(42, 46, 57, 0.55)' : 'rgba(209, 212, 220, 0.72)'
  const borderColor = isDark ? '#2A2E39' : '#D6DAE3'
  const textPrimary = isDark ? '#D1D4DC' : '#131722'
  const textSecondary = isDark ? '#787B86' : '#5D606B'
  const crosshairBg = isDark ? '#363A45' : '#434651'
  const crosshairLine = isDark ? '#787B86' : '#8A8E9B'

  const upColor = '#089981'
  const downColor = '#F23645'
  const accent = '#2962FF'

  let candleType: CandleType = 'candle_solid'
  let areaBg: string | Array<{ offset: number; color: string }> = [
    { offset: 0, color: 'rgba(41, 98, 255, 0.02)' },
    { offset: 1, color: 'rgba(41, 98, 255, 0.28)' },
  ]
  if (chartStyle === 'candle_stroke') {
    candleType = 'candle_up_stroke'
  } else if (chartStyle === 'ohlc') {
    candleType = 'ohlc'
  } else if (chartStyle === 'line') {
    candleType = 'area'
    areaBg = 'transparent'
  } else if (chartStyle === 'area') {
    candleType = 'area'
  } else {
    candleType = 'candle_solid'
  }

  return {
    grid: {
      show: true,
      horizontal: {
        show: true,
        size: 1,
        color: gridColor,
        style: 'solid',
      },
      vertical: {
        show: true,
        size: 1,
        color: gridColor,
        style: 'solid',
      },
    },
    candle: {
      type: candleType,
      bar: {
        compareRule: 'current_open',
        upColor,
        downColor,
        noChangeColor: textSecondary,
        upBorderColor: upColor,
        downBorderColor: downColor,
        noChangeBorderColor: textSecondary,
        upWickColor: upColor,
        downWickColor: downColor,
        noChangeWickColor: textSecondary,
      },
      area: {
        lineSize: 2,
        lineColor: accent,
        value: 'close',
        smooth: false,
        backgroundColor: areaBg,
        point: {
          show: true,
          color: accent,
          radius: 3,
          rippleColor: 'rgba(41, 98, 255, 0.28)',
          rippleRadius: 7,
          animation: true,
          animationDuration: 1200,
        },
      },
      priceMark: {
        show: true,
        high: {
          show: true,
          color: textSecondary,
          textOffset: 5,
          textSize: 10,
          textFamily: FONT_FAMILY,
          textWeight: 'normal',
        },
        low: {
          show: true,
          color: textSecondary,
          textOffset: 5,
          textSize: 10,
          textFamily: FONT_FAMILY,
          textWeight: 'normal',
        },
        last: {
          show: true,
          compareRule: 'current_open',
          upColor,
          downColor,
          noChangeColor: textSecondary,
          line: {
            show: true,
            style: 'dashed',
            dashedValue: [3, 3],
            size: 1,
          },
          text: {
            show: true,
            style: 'fill',
            size: 11,
            paddingLeft: 5,
            paddingTop: 3,
            paddingRight: 5,
            paddingBottom: 3,
            color: '#FFFFFF',
            family: FONT_FAMILY,
            weight: '600',
            borderRadius: 2,
          },
          extendTexts: showCountdown
            ? [
                {
                  show: true,
                  position: 'below_price',
                  updateInterval: 1000,
                  style: 'fill',
                  size: 10,
                  paddingLeft: 5,
                  paddingTop: 2,
                  paddingRight: 5,
                  paddingBottom: 2,
                  borderColor: 'transparent',
                  borderStyle: 'solid',
                  borderSize: 0,
                  borderDashedValue: [2, 2],
                  color: '#FFFFFF',
                  family: FONT_FAMILY,
                  weight: '500',
                  borderRadius: 2,
                  backgroundColor: null,
                },
              ]
            : [],
        },
      },
      tooltip: {
        showRule: 'always',
        showType: 'standard',
        offsetLeft: 8,
        offsetTop: 6,
        title: {
          show: true,
          size: 13,
          family: FONT_FAMILY,
          weight: '600',
          color: textPrimary,
          marginLeft: 6,
          marginTop: 2,
          marginRight: 8,
          marginBottom: 2,
          template: '{ticker} · {period}',
        },
        legend: {
          size: 11,
          family: FONT_FAMILY,
          weight: '500',
          color: textSecondary,
          marginLeft: 6,
          marginTop: 2,
          marginRight: 6,
          marginBottom: 2,
          defaultValue: '—',
          template: ({ current, prev }) => {
            if (!current) return []
            const base = prev?.close ?? current.open
            const chg = current.close - base
            const pct = base > 0 ? (chg / base) * 100 : 0
            const valColor = current.close >= current.open ? upColor : downColor
            const sign = chg >= 0 ? '+' : ''
            return [
              { title: { text: 'O ', color: textSecondary }, value: { text: '{open}', color: valColor } },
              { title: { text: 'H ', color: textSecondary }, value: { text: '{high}', color: valColor } },
              { title: { text: 'L ', color: textSecondary }, value: { text: '{low}', color: valColor } },
              { title: { text: 'C ', color: textSecondary }, value: { text: '{close}', color: valColor } },
              {
                title: { text: '', color: textSecondary },
                value: { text: `${sign}${pct.toFixed(2)}%`, color: chg >= 0 ? upColor : downColor },
              },
              { title: { text: 'Vol ', color: textSecondary }, value: { text: '{volume}', color: textSecondary } },
            ]
          },
        },
      },
    },
    indicator: {
      ohlc: {
        compareRule: 'current_open',
        upColor: 'rgba(8, 153, 129, 0.55)',
        downColor: 'rgba(242, 54, 69, 0.55)',
        noChangeColor: textSecondary,
      },
      bars: [
        {
          style: 'fill',
          borderStyle: 'solid',
          borderSize: 1,
          borderDashedValue: [2, 2],
          upColor: 'rgba(8, 153, 129, 0.45)',
          downColor: 'rgba(242, 54, 69, 0.45)',
          noChangeColor: 'rgba(120, 123, 134, 0.45)',
        },
      ],
      lines: isDark
        ? [
            { style: 'solid', smooth: false, size: 1.5, dashedValue: [2, 2], color: '#2962FF' },
            { style: 'solid', smooth: false, size: 1.5, dashedValue: [2, 2], color: '#FF9800' },
            { style: 'solid', smooth: false, size: 1.5, dashedValue: [2, 2], color: '#E040FB' },
            { style: 'solid', smooth: false, size: 1.5, dashedValue: [2, 2], color: '#00BCD4' },
            { style: 'solid', smooth: false, size: 1.5, dashedValue: [2, 2], color: '#FBC02D' },
          ]
        : [
            { style: 'solid', smooth: false, size: 1.6, dashedValue: [2, 2], color: '#2962FF' },
            { style: 'solid', smooth: false, size: 1.6, dashedValue: [2, 2], color: '#D97706' },
            { style: 'solid', smooth: false, size: 1.6, dashedValue: [2, 2], color: '#9333EA' },
            { style: 'solid', smooth: false, size: 1.6, dashedValue: [2, 2], color: '#0E7490' },
            { style: 'solid', smooth: false, size: 1.6, dashedValue: [2, 2], color: '#B45309' },
          ],
      lastValueMark: {
        show: true,
        text: {
          show: true,
          style: 'fill',
          color: '#FFFFFF',
          size: 11,
          family: FONT_FAMILY,
          weight: '500',
          paddingLeft: 4,
          paddingTop: 2,
          paddingRight: 4,
          paddingBottom: 2,
          borderRadius: 2,
        },
      },
      tooltip: {
        showRule: 'always',
        showType: 'standard',
        title: {
          show: true,
          showName: true,
          showParams: true,
          size: 11,
          family: FONT_FAMILY,
          weight: '500',
          color: textSecondary,
        },
        legend: {
          size: 11,
          family: FONT_FAMILY,
          weight: '500',
          color: textSecondary,
          defaultValue: '—',
        },
        features: buildIndicatorTooltipFeatures(theme, true),
      },
    },
    xAxis: {
      show: true,
      axisLine: { show: true, color: borderColor, size: 1 },
      tickLine: { show: true, size: 1, length: 3, color: borderColor },
      tickText: {
        show: true,
        color: textSecondary,
        size: 11,
        family: FONT_FAMILY,
        weight: 'normal',
      },
    },
    yAxis: {
      show: true,
      axisLine: { show: true, color: borderColor, size: 1 },
      tickLine: { show: true, size: 1, length: 3, color: borderColor },
      tickText: {
        show: true,
        color: textSecondary,
        size: 11,
        family: FONT_FAMILY,
        weight: 'normal',
        marginStart: 5,
        marginEnd: 8,
      },
    },
    separator: {
      size: 1,
      color: borderColor,
      fill: true,
      activeBackgroundColor: 'rgba(41, 98, 255, 0.2)',
    },
    crosshair: {
      show: true,
      horizontal: {
        show: true,
        line: { show: true, style: 'dashed', dashedValue: [4, 4], size: 1, color: crosshairLine },
        text: {
          show: true,
          style: 'fill',
          color: '#FFFFFF',
          size: 11,
          family: FONT_FAMILY,
          weight: '500',
          borderStyle: 'solid',
          borderSize: 0,
          borderColor: crosshairBg,
          borderRadius: 2,
          paddingLeft: 5,
          paddingRight: 5,
          paddingTop: 3,
          paddingBottom: 3,
          backgroundColor: crosshairBg,
        },
      },
      vertical: {
        show: true,
        line: { show: true, style: 'dashed', dashedValue: [4, 4], size: 1, color: crosshairLine },
        text: {
          show: true,
          style: 'fill',
          color: '#FFFFFF',
          size: 11,
          family: FONT_FAMILY,
          weight: '500',
          borderStyle: 'solid',
          borderSize: 0,
          borderColor: crosshairBg,
          borderRadius: 2,
          paddingLeft: 6,
          paddingRight: 6,
          paddingTop: 3,
          paddingBottom: 3,
          backgroundColor: crosshairBg,
        },
      },
    },
    overlay: {
      point: {
        color: '#2962FF',
        borderColor: 'rgba(41, 98, 255, 0.35)',
        borderSize: 2,
        radius: 4,
        activeColor: '#2962FF',
        activeBorderColor: '#FFFFFF',
        activeBorderSize: 2,
        activeRadius: 5,
      },
      line: {
        style: 'solid',
        smooth: false,
        color: '#2962FF',
        size: 1.5,
        dashedValue: [4, 4],
      },
      rect: {
        style: 'stroke_fill',
        color: 'rgba(41, 98, 255, 0.16)',
        borderColor: '#2962FF',
        borderSize: 1.5,
        borderRadius: 0,
        borderStyle: 'solid',
        borderDashedValue: [4, 4],
      },
      polygon: {
        style: 'stroke_fill',
        color: 'rgba(41, 98, 255, 0.16)',
        borderColor: '#2962FF',
        borderSize: 1.5,
        borderStyle: 'solid',
        borderDashedValue: [4, 4],
      },
      circle: {
        style: 'stroke_fill',
        color: 'rgba(41, 98, 255, 0.16)',
        borderColor: '#2962FF',
        borderSize: 1.5,
        borderStyle: 'solid',
        borderDashedValue: [4, 4],
      },
      arc: {
        style: 'solid',
        color: '#2962FF',
        size: 1.5,
        dashedValue: [4, 4],
      },
      text: {
        style: 'fill',
        color: '#FFFFFF',
        size: 11,
        family: FONT_FAMILY,
        weight: '500',
        borderStyle: 'solid',
        borderSize: 0,
        borderColor: '#2962FF',
        borderRadius: 3,
        paddingLeft: 5,
        paddingRight: 5,
        paddingTop: 2,
        paddingBottom: 2,
        backgroundColor: '#2962FF',
      },
    },
  }
}
