import { getSupportedOverlays, registerOverlay } from 'klinecharts'
import { PRO_UPGRADED_OVERLAYS } from './proOverlays'
import { TRADINGVIEW_CUSTOM_OVERLAYS } from './tradingViewOverlays'

let registered = false

/**
 * Registers all 17 upgraded klinecharts-pro overlays + 4 TradingView signature overlays (+ VN band line)
 * strictly BEFORE any `klinecharts.init()` call.
 * Combined with KLineChart v10's 16 built-in overlays, exposes 37+ drawing tools.
 */
export function registerAllCustomOverlays(): void {
  if (registered) return
  const existing = new Set(getSupportedOverlays())
  for (const tpl of [...PRO_UPGRADED_OVERLAYS, ...TRADINGVIEW_CUSTOM_OVERLAYS]) {
    if (!existing.has(tpl.name)) {
      registerOverlay(tpl)
    }
  }
  registered = true
}
