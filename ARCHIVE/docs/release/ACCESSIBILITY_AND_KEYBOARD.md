# Sumi Accessibility & Keyboard Navigation Contract

## Overview

Sumi is designed as a high-efficiency desktop trading-practice workstation. It provides clear visual hierarchy, accessible color contrast ratios, focus-trap isolation during modal interactions, and dedicated keyboard navigation shortcuts across Replay, Chart, and Trade controls.

## Standard Keyboard Shortcuts

| Shortcut | Context | Action |
| --- | --- | --- |
| `Space` | Replay Workspace | Play / Pause continuous replay playback |
| `RightArrow` or `N` | Replay Workspace | Step forward 1 candle (`advance`) |
| `LeftArrow` or `P` | Replay Workspace | Step backward 1 candle (`rewind`) |
| `B` | Replay Workspace | Open / trigger Buy order modal/panel |
| `S` | Replay Workspace | Open / trigger Sell order modal/panel |
| `X` | Replay Workspace | Flatten current active position |
| `Alt+1` | Global Navigation | Jump to Dashboard (`/`) |
| `Alt+2` | Global Navigation | Jump to Replay (`/replay`) |
| `Alt+3` | Global Navigation | Jump to Trade Journal (`/journal`) |
| `Alt+4` | Global Navigation | Jump to Analytics (`/analytics`) |
| `Alt+5` | Global Navigation | Jump to Strategy Lab (`/strategy-lab`) |
| `Alt+6` | Global Navigation | Jump to Data Catalog & Sync (`/import`) |
| `Esc` | Dialogs / Modals | Cancel active drawing creation, close modal, or dismiss drawer |

## Focus & Input Conflict Prevention

- **Input Scope Isolation**: Keyboard navigation shortcuts are automatically suspended whenever user focus is within a text input, dropdown, date picker, or numeric form field.
- **Focus Trapping**: Modal dialogs (such as strategy configuration, provider selection, or order entry confirmation) trap keyboard focus within the modal container to prevent accidental background chart navigation.
- **ARIA & Contrast**: All interactive UI controls maintain WCAG AA visual contrast, explicit `aria-label` descriptors, and visual focus rings.
