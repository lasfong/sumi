import { getSupportedOverlays, registerOverlay } from 'klinecharts'
import { PRO_UPGRADED_OVERLAYS } from './proOverlays'
import { TRADINGVIEW_CUSTOM_OVERLAYS } from './tradingViewOverlays'
import { VOLUME_STRUCTURE_OVERLAYS } from './volumeStructureOverlays'

let registered = false

/**
 * Registers all 17 upgraded klinecharts-pro overlays + 4 TradingView signature overlays
 * + 2 Wave 3 Volume Structure overlays (`anchoredVwap`, `fixedRangeVolumeProfile`)
 * strictly BEFORE any `klinecharts.init()` call.
 */
export function registerAllCustomOverlays(): void {
  if (registered) return
  const existing = new Set(getSupportedOverlays())
  for (const tpl of [...PRO_UPGRADED_OVERLAYS, ...TRADINGVIEW_CUSTOM_OVERLAYS, ...VOLUME_STRUCTURE_OVERLAYS]) {
    if (!existing.has(tpl.name)) {
      registerOverlay(tpl)
    }
  }
  registered = true
}
