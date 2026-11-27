import { getSupportedIndicators, registerIndicator } from 'klinecharts'
import { SUMI_BUILTIN_OVERRIDES, SUMI_CUSTOM_INDICATORS } from './customIndicators'

let registered = false

export function registerAllCustomIndicators(): void {
  if (registered) return
  const existing = new Set(getSupportedIndicators())
  for (const tpl of SUMI_CUSTOM_INDICATORS) {
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
