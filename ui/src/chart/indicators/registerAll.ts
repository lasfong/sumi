import { getSupportedIndicators, registerIndicator } from 'klinecharts'
import { SUMI_CUSTOM_INDICATORS } from './customIndicators'

let registered = false

export function registerAllCustomIndicators(): void {
  if (registered) return
  const existing = new Set(getSupportedIndicators())
  for (const tpl of SUMI_CUSTOM_INDICATORS) {
    if (!existing.has(tpl.name)) {
      registerIndicator(tpl as Parameters<typeof registerIndicator>[0])
    }
  }
  registered = true
}
