/**
 * Cross-platform OS detection & shortcut labels (macOS vs Windows 10/11 vs Linux).
 * Ensures Sumi UI never hardcodes macOS-only glyphs (`⌘`/`⌥`) on Windows or Linux.
 */

export function isMacPlatform(): boolean {
  if (typeof navigator === 'undefined') return false
  const uaData = (navigator as Navigator & { userAgentData?: { platform?: string } }).userAgentData
  const platform = (uaData?.platform || navigator.platform || navigator.userAgent || '').toLowerCase()
  return platform.includes('mac')
}

export function isWindowsPlatform(): boolean {
  if (typeof navigator === 'undefined') return false
  const uaData = (navigator as Navigator & { userAgentData?: { platform?: string } }).userAgentData
  const platform = (uaData?.platform || navigator.platform || navigator.userAgent || '').toLowerCase()
  return platform.includes('win')
}

export function modKeyLabel(): string {
  return isMacPlatform() ? '⌘' : 'Ctrl+'
}

export function altKeyLabel(): string {
  return isMacPlatform() ? '⌥' : 'Alt+'
}

export function formatShortcut(key: string, opts?: { mod?: boolean; alt?: boolean; shift?: boolean }): string {
  const parts: string[] = []
  if (opts?.mod) parts.push(isMacPlatform() ? '⌘' : 'Ctrl')
  if (opts?.alt) parts.push(isMacPlatform() ? '⌥' : 'Alt')
  if (opts?.shift) parts.push(isMacPlatform() ? '⇧' : 'Shift')
  parts.push(key)
  return isMacPlatform() ? parts.join('') : parts.join('+')
}
