import type { PriceAlertRecord } from '../types/market'

let sharedAudioCtx: AudioContext | null = null

/**
 * Plays a pleasant, non-startling two-tone synthesized bell chime via Web Audio API.
 * Requires zero external audio assets and degrades silently in headless/restricted environments.
 */
export function playAlertChime(): void {
  if (typeof window === 'undefined') return
  const AudioCtx =
    window.AudioContext ??
    (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext
  if (!AudioCtx) return

  try {
    if (!sharedAudioCtx || sharedAudioCtx.state === 'closed') {
      sharedAudioCtx = new AudioCtx()
    }
    const ctx = sharedAudioCtx
    if (ctx.state === 'suspended') {
      void ctx.resume().catch(() => {})
    }
    const now = ctx.currentTime
    const notes = [
      { freq: 1046.5, start: 0, dur: 0.14 }, // C6
      { freq: 1318.5, start: 0.11, dur: 0.22 }, // E6
    ]
    for (const n of notes) {
      const osc = ctx.createOscillator()
      const gain = ctx.createGain()
      osc.type = 'sine'
      osc.frequency.setValueAtTime(n.freq, now + n.start)
      gain.gain.setValueAtTime(0.0001, now + n.start)
      gain.gain.exponentialRampToValueAtTime(0.12, now + n.start + 0.018)
      gain.gain.exponentialRampToValueAtTime(0.0001, now + n.start + n.dur)
      osc.connect(gain)
      gain.connect(ctx.destination)
      osc.start(now + n.start)
      osc.stop(now + n.start + n.dur + 0.02)
    }
  } catch {
    // Audio playback blocked or unsupported; ignore safely.
  }
}

/**
 * Triggers audio chime and optional OS/Web Notification when an alert fires.
 */
export function notifyAlertTriggered(alert: PriceAlertRecord): void {
  if (alert.soundEnabled !== false) {
    playAlertChime()
  }
  if (typeof window === 'undefined' || !('Notification' in window)) return
  try {
    const title = `Sumi Alert · ${alert.ticker}`
    const body = alert.message || `${alert.condition} @ ${alert.targetPrice.toLocaleString('en-US')}`
    if (Notification.permission === 'granted') {
      new Notification(title, { body })
    }
  } catch {
    // Notification API unavailable in embedded webview; toast already surfaced.
  }
}
