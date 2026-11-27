import { getSupportedIndicators, registerIndicator } from 'klinecharts'
import { SUMI_BUILTIN_OVERRIDES, SUMI_CUSTOM_INDICATORS } from './customIndicators'
import { WAVE4_FUNDAMENTAL_INDICATORS } from './fundamentalIndicators'
import { YUMEKO_REAL_INDICATORS } from './qmvRealIndicators'
import { WAVE3_INDICATORS } from './wave3Indicators'

let registered = false

export function registerAllCustomIndicators(): void {
  if (registered) return
  const existing = new Set(getSupportedIndicators())
  for (const tpl of [
    ...SUMI_CUSTOM_INDICATORS,
    ...YUMEKO_REAL_INDICATORS,
    ...WAVE3_INDICATORS,
    ...WAVE4_FUNDAMENTAL_INDICATORS,
  ]) {
    if (!existing.has(tpl.name) || tpl.name === 'ICHIMOKU') {
      registerIndicator(tpl as Parameters<typeof registerIndicator>[0])
    }
  }
  // Override built-in MACD, RSI, and CCI so Chart === QMV Screener === Strategy Tester 1:1.
  for (const overrideTpl of SUMI_BUILTIN_OVERRIDES) {
    registerIndicator(overrideTpl as Parameters<typeof registerIndicator>[0])
  }
  registered = true
}
