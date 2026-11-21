//! Pure `wasm32-unknown-unknown`-compatible Keyboard Shortcut & Hotkey Profile Engine
//! (`Milestone R14 / Wave 13`).
//!
//! Replaces static shortcut lists (`KeyboardShortcutsModal.tsx` & `globalShortcutPolicy.ts`)
//! with conflict-free selectable profiles (`StandardCharting`, `InstitutionalDomFast`,
//! `ReplayPractice`, `Custom`), custom per-action overrides persisted inside
//! [`crate::viewport::WorkspaceTemplate`], reserved navigation/palette key protection, and
//! deterministic modal/input focus eligibility gating (`is_global_hotkey_eligible`).

use std::fmt;

use serde::{Deserialize, Serialize};

/// Executable workstation shortcut actions governed by [`HotkeyProfile`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HotkeyAction {
    ReplayPlayPause,
    ReplayStepForward,
    ReplayStepBack,
    ReplayCutTool,
    QuickBuyMarket,
    QuickSellMarket,
    FlattenPosition,
    CancelAllPending,
    ToggleTradingDock,
    ToggleDomLadderView,
    CenterDomOnMark,
    ToggleScannerDock,
    ToggleJournalDock,
    ToggleAlertsDock,
    ToggleStrategyDock,
    CycleGridLayout,
    OpenShortcutsModal,
}

impl HotkeyAction {
    pub const ALL: [Self; 17] = [
        Self::ReplayPlayPause,
        Self::ReplayStepForward,
        Self::ReplayStepBack,
        Self::ReplayCutTool,
        Self::QuickBuyMarket,
        Self::QuickSellMarket,
        Self::FlattenPosition,
        Self::CancelAllPending,
        Self::ToggleTradingDock,
        Self::ToggleDomLadderView,
        Self::CenterDomOnMark,
        Self::ToggleScannerDock,
        Self::ToggleJournalDock,
        Self::ToggleAlertsDock,
        Self::ToggleStrategyDock,
        Self::CycleGridLayout,
        Self::OpenShortcutsModal,
    ];

    #[inline]
    #[must_use]
    pub const fn as_id(self) -> &'static str {
        match self {
            Self::ReplayPlayPause => "ReplayPlayPause",
            Self::ReplayStepForward => "ReplayStepForward",
            Self::ReplayStepBack => "ReplayStepBack",
            Self::ReplayCutTool => "ReplayCutTool",
            Self::QuickBuyMarket => "QuickBuyMarket",
            Self::QuickSellMarket => "QuickSellMarket",
            Self::FlattenPosition => "FlattenPosition",
            Self::CancelAllPending => "CancelAllPending",
            Self::ToggleTradingDock => "ToggleTradingDock",
            Self::ToggleDomLadderView => "ToggleDomLadderView",
            Self::CenterDomOnMark => "CenterDomOnMark",
            Self::ToggleScannerDock => "ToggleScannerDock",
            Self::ToggleJournalDock => "ToggleJournalDock",
            Self::ToggleAlertsDock => "ToggleAlertsDock",
            Self::ToggleStrategyDock => "ToggleStrategyDock",
            Self::CycleGridLayout => "CycleGridLayout",
            Self::OpenShortcutsModal => "OpenShortcutsModal",
        }
    }

    #[must_use]
    pub fn from_id(raw: &str) -> Option<Self> {
        let norm = raw.trim();
        Self::ALL
            .into_iter()
            .find(|a| a.as_id().eq_ignore_ascii_case(norm))
    }

    #[inline]
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReplayPlayPause => "Replay Play / Pause",
            Self::ReplayStepForward => "Replay Step Forward (+1 Bar)",
            Self::ReplayStepBack => "Replay Step Back (-1 Bar)",
            Self::ReplayCutTool => "Replay Cut-Bar Tool",
            Self::QuickBuyMarket => "Quick Market Buy (Order/DOM Lot)",
            Self::QuickSellMarket => "Quick Market Sell (Order/DOM Lot)",
            Self::FlattenPosition => "Flatten Open Position",
            Self::CancelAllPending => "Cancel All Working Orders",
            Self::ToggleTradingDock => "Toggle Paper Trading / DOM Dock",
            Self::ToggleDomLadderView => "Toggle Order Ticket vs DOM Ladder",
            Self::CenterDomOnMark => "Center DOM Ladder on Mark Price",
            Self::ToggleScannerDock => "Toggle Market Scanner & Breadth Dock",
            Self::ToggleJournalDock => "Toggle Risk Planner & Journal Dock",
            Self::ToggleAlertsDock => "Toggle Alerts Dock",
            Self::ToggleStrategyDock => "Toggle Strategy Lab Dock",
            Self::CycleGridLayout => "Cycle Multi-Chart Grid (1x1..2x2)",
            Self::OpenShortcutsModal => "Open Keyboard Shortcuts & Hotkeys",
        }
    }

    #[inline]
    #[must_use]
    pub const fn category(self) -> &'static str {
        match self {
            Self::ReplayPlayPause
            | Self::ReplayStepForward
            | Self::ReplayStepBack
            | Self::ReplayCutTool => "Replay",
            Self::QuickBuyMarket
            | Self::QuickSellMarket
            | Self::FlattenPosition
            | Self::CancelAllPending
            | Self::CenterDomOnMark => "Execution & DOM",
            Self::ToggleTradingDock
            | Self::ToggleDomLadderView
            | Self::ToggleScannerDock
            | Self::ToggleJournalDock
            | Self::ToggleAlertsDock
            | Self::ToggleStrategyDock => "Panels & Docks",
            Self::CycleGridLayout | Self::OpenShortcutsModal => "Workspace",
        }
    }
}

/// Bindable non-reserved keyboard keys supported in [`HotkeyChord`].
///
/// Reserved system/navigation keys (`Escape`, `Tab`, `Enter`, `Backspace`, `Delete`,
/// and `Cmd/Ctrl+K`) are intentionally excluded by type construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HotkeyKey {
    Space,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Comma,
    Period,
    Slash,
    Question,
    BracketLeft,
    BracketRight,
}

impl HotkeyKey {
    pub const ALL: [Self; 58] = [
        Self::Space,
        Self::ArrowLeft,
        Self::ArrowRight,
        Self::ArrowUp,
        Self::ArrowDown,
        Self::A,
        Self::B,
        Self::C,
        Self::D,
        Self::E,
        Self::F,
        Self::G,
        Self::H,
        Self::I,
        Self::J,
        Self::K,
        Self::L,
        Self::M,
        Self::N,
        Self::O,
        Self::P,
        Self::Q,
        Self::R,
        Self::S,
        Self::T,
        Self::U,
        Self::V,
        Self::W,
        Self::X,
        Self::Y,
        Self::Z,
        Self::Num1,
        Self::Num2,
        Self::Num3,
        Self::Num4,
        Self::Num5,
        Self::Num6,
        Self::Num7,
        Self::Num8,
        Self::Num9,
        Self::F1,
        Self::F2,
        Self::F3,
        Self::F4,
        Self::F5,
        Self::F6,
        Self::F7,
        Self::F8,
        Self::F9,
        Self::F10,
        Self::F11,
        Self::F12,
        Self::Comma,
        Self::Period,
        Self::Slash,
        Self::Question,
        Self::BracketLeft,
        Self::BracketRight,
    ];

    #[inline]
    #[must_use]
    pub const fn as_id(self) -> &'static str {
        match self {
            Self::Space => "Space",
            Self::ArrowLeft => "ArrowLeft",
            Self::ArrowRight => "ArrowRight",
            Self::ArrowUp => "ArrowUp",
            Self::ArrowDown => "ArrowDown",
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::F => "F",
            Self::G => "G",
            Self::H => "H",
            Self::I => "I",
            Self::J => "J",
            Self::K => "K",
            Self::L => "L",
            Self::M => "M",
            Self::N => "N",
            Self::O => "O",
            Self::P => "P",
            Self::Q => "Q",
            Self::R => "R",
            Self::S => "S",
            Self::T => "T",
            Self::U => "U",
            Self::V => "V",
            Self::W => "W",
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
            Self::Num1 => "Num1",
            Self::Num2 => "Num2",
            Self::Num3 => "Num3",
            Self::Num4 => "Num4",
            Self::Num5 => "Num5",
            Self::Num6 => "Num6",
            Self::Num7 => "Num7",
            Self::Num8 => "Num8",
            Self::Num9 => "Num9",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::F12 => "F12",
            Self::Comma => "Comma",
            Self::Period => "Period",
            Self::Slash => "Slash",
            Self::Question => "Question",
            Self::BracketLeft => "BracketLeft",
            Self::BracketRight => "BracketRight",
        }
    }

    #[must_use]
    pub fn from_id(raw: &str) -> Option<Self> {
        let norm = raw.trim();
        Self::ALL
            .into_iter()
            .find(|k| k.as_id().eq_ignore_ascii_case(norm))
    }

    #[inline]
    #[must_use]
    pub const fn display_label(self) -> &'static str {
        match self {
            Self::Space => "Space",
            Self::ArrowLeft => "Left",
            Self::ArrowRight => "Right",
            Self::ArrowUp => "Up",
            Self::ArrowDown => "Down",
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::F => "F",
            Self::G => "G",
            Self::H => "H",
            Self::I => "I",
            Self::J => "J",
            Self::K => "K",
            Self::L => "L",
            Self::M => "M",
            Self::N => "N",
            Self::O => "O",
            Self::P => "P",
            Self::Q => "Q",
            Self::R => "R",
            Self::S => "S",
            Self::T => "T",
            Self::U => "U",
            Self::V => "V",
            Self::W => "W",
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
            Self::Num1 => "1",
            Self::Num2 => "2",
            Self::Num3 => "3",
            Self::Num4 => "4",
            Self::Num5 => "5",
            Self::Num6 => "6",
            Self::Num7 => "7",
            Self::Num8 => "8",
            Self::Num9 => "9",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::F12 => "F12",
            Self::Comma => ",",
            Self::Period => ".",
            Self::Slash => "/",
            Self::Question => "?",
            Self::BracketLeft => "[",
            Self::BracketRight => "]",
        }
    }
}

/// Canonical modifier state for global workstation hotkeys (`Cmd`/`Ctrl` reserved for palette & OS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyModifiers {
    pub shift: bool,
    pub alt: bool,
}

/// Canonical key + modifiers binding chord.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HotkeyChord {
    pub key: HotkeyKey,
    #[serde(default)]
    pub modifiers: HotkeyModifiers,
}

impl HotkeyChord {
    #[inline]
    #[must_use]
    pub const fn new(key: HotkeyKey, modifiers: HotkeyModifiers) -> Self {
        match key {
            HotkeyKey::Question => Self {
                key: HotkeyKey::Question,
                modifiers: HotkeyModifiers {
                    shift: false,
                    alt: modifiers.alt,
                },
            },
            HotkeyKey::Slash if modifiers.shift => Self {
                key: HotkeyKey::Question,
                modifiers: HotkeyModifiers {
                    shift: false,
                    alt: modifiers.alt,
                },
            },
            _ => Self { key, modifiers },
        }
    }

    #[inline]
    #[must_use]
    pub const fn plain(key: HotkeyKey) -> Self {
        Self::new(
            key,
            HotkeyModifiers {
                shift: false,
                alt: false,
            },
        )
    }

    #[inline]
    #[must_use]
    pub const fn shifted(key: HotkeyKey) -> Self {
        Self::new(
            key,
            HotkeyModifiers {
                shift: true,
                alt: false,
            },
        )
    }

    /// Formats this chord into a concise keycap badge (e.g., `"Shift+B"`, `"Space"`, `"?"`).
    #[must_use]
    pub fn format_badge(self) -> String {
        let mut parts = String::new();
        if self.modifiers.alt {
            parts.push_str("Alt+");
        }
        if self.modifiers.shift {
            parts.push_str("Shift+");
        }
        parts.push_str(self.key.display_label());
        parts
    }
}

impl fmt::Display for HotkeyChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format_badge())
    }
}

/// Built-in & user-customized hotkey presets persisted across sessions and templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum HotkeyProfileId {
    #[default]
    StandardCharting,
    InstitutionalDomFast,
    ReplayPractice,
    Custom,
}

impl HotkeyProfileId {
    pub const ALL: [Self; 4] = [
        Self::StandardCharting,
        Self::InstitutionalDomFast,
        Self::ReplayPractice,
        Self::Custom,
    ];

    #[inline]
    #[must_use]
    pub const fn as_id(self) -> &'static str {
        match self {
            Self::StandardCharting => "StandardCharting",
            Self::InstitutionalDomFast => "InstitutionalDomFast",
            Self::ReplayPractice => "ReplayPractice",
            Self::Custom => "Custom",
        }
    }

    #[must_use]
    pub fn from_id(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "standardcharting" | "standard" => Some(Self::StandardCharting),
            "institutionaldomfast" | "domfast" | "dom_fast" => Some(Self::InstitutionalDomFast),
            "replaypractice" | "replay" => Some(Self::ReplayPractice),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    #[inline]
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::StandardCharting => "Standard",
            Self::InstitutionalDomFast => "DOM Fast",
            Self::ReplayPractice => "Replay",
            Self::Custom => "Custom",
        }
    }

    #[inline]
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::StandardCharting => {
                "Balanced charting workflow: Shift+Order execution, Space replay, T/D/M/J/A/L panels"
            }
            Self::InstitutionalDomFast => {
                "Low-latency DOM scalping: unshifted B/S Market, F Flatten, X Cancel All, D/C ladder"
            }
            Self::ReplayPractice => {
                "Bar Replay drill layout: Space play/pause, N/P step, R cut bar, B/S/F execution"
            }
            Self::Custom => "User-rebound conflict-free keyboard shortcut profile",
        }
    }
}

/// Serializable override entry mapping an action to either a custom chord or `None` (unbound).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HotkeyOverride {
    pub action: HotkeyAction,
    pub chord: Option<HotkeyChord>,
}

fn default_base_preset() -> HotkeyProfileId {
    HotkeyProfileId::StandardCharting
}

/// Complete conflict-free binding table backing global hotkey dispatch and UI badges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyProfile {
    pub id: HotkeyProfileId,
    #[serde(default = "default_base_preset")]
    base_preset: HotkeyProfileId,
    bindings: Vec<(HotkeyAction, Option<HotkeyChord>)>,
}

impl Default for HotkeyProfile {
    fn default() -> Self {
        Self::from_preset(HotkeyProfileId::StandardCharting)
    }
}

fn preset_default_chord(profile: HotkeyProfileId, action: HotkeyAction) -> Option<HotkeyChord> {
    match profile {
        HotkeyProfileId::StandardCharting | HotkeyProfileId::Custom => match action {
            HotkeyAction::ReplayPlayPause => Some(HotkeyChord::plain(HotkeyKey::Space)),
            HotkeyAction::ReplayStepForward => Some(HotkeyChord::plain(HotkeyKey::ArrowRight)),
            HotkeyAction::ReplayStepBack => Some(HotkeyChord::plain(HotkeyKey::ArrowLeft)),
            HotkeyAction::ReplayCutTool => Some(HotkeyChord::shifted(HotkeyKey::R)),
            HotkeyAction::QuickBuyMarket => Some(HotkeyChord::shifted(HotkeyKey::B)),
            HotkeyAction::QuickSellMarket => Some(HotkeyChord::shifted(HotkeyKey::S)),
            HotkeyAction::FlattenPosition => Some(HotkeyChord::shifted(HotkeyKey::F)),
            HotkeyAction::CancelAllPending => Some(HotkeyChord::shifted(HotkeyKey::C)),
            HotkeyAction::ToggleTradingDock => Some(HotkeyChord::plain(HotkeyKey::T)),
            HotkeyAction::ToggleDomLadderView => Some(HotkeyChord::plain(HotkeyKey::D)),
            HotkeyAction::CenterDomOnMark => Some(HotkeyChord::plain(HotkeyKey::C)),
            HotkeyAction::ToggleScannerDock => Some(HotkeyChord::plain(HotkeyKey::M)),
            HotkeyAction::ToggleJournalDock => Some(HotkeyChord::plain(HotkeyKey::J)),
            HotkeyAction::ToggleAlertsDock => Some(HotkeyChord::plain(HotkeyKey::A)),
            HotkeyAction::ToggleStrategyDock => Some(HotkeyChord::plain(HotkeyKey::L)),
            HotkeyAction::CycleGridLayout => Some(HotkeyChord::plain(HotkeyKey::G)),
            HotkeyAction::OpenShortcutsModal => Some(HotkeyChord::plain(HotkeyKey::Question)),
        },
        HotkeyProfileId::InstitutionalDomFast => match action {
            HotkeyAction::ReplayPlayPause => Some(HotkeyChord::plain(HotkeyKey::Space)),
            HotkeyAction::ReplayStepForward => Some(HotkeyChord::plain(HotkeyKey::ArrowRight)),
            HotkeyAction::ReplayStepBack => Some(HotkeyChord::plain(HotkeyKey::ArrowLeft)),
            HotkeyAction::ReplayCutTool => Some(HotkeyChord::shifted(HotkeyKey::R)),
            HotkeyAction::QuickBuyMarket => Some(HotkeyChord::plain(HotkeyKey::B)),
            HotkeyAction::QuickSellMarket => Some(HotkeyChord::plain(HotkeyKey::S)),
            HotkeyAction::FlattenPosition => Some(HotkeyChord::plain(HotkeyKey::F)),
            HotkeyAction::CancelAllPending => Some(HotkeyChord::plain(HotkeyKey::X)),
            HotkeyAction::ToggleTradingDock => Some(HotkeyChord::plain(HotkeyKey::T)),
            HotkeyAction::ToggleDomLadderView => Some(HotkeyChord::plain(HotkeyKey::D)),
            HotkeyAction::CenterDomOnMark => Some(HotkeyChord::plain(HotkeyKey::C)),
            HotkeyAction::ToggleScannerDock => Some(HotkeyChord::shifted(HotkeyKey::M)),
            HotkeyAction::ToggleJournalDock => Some(HotkeyChord::shifted(HotkeyKey::J)),
            HotkeyAction::ToggleAlertsDock => Some(HotkeyChord::shifted(HotkeyKey::A)),
            HotkeyAction::ToggleStrategyDock => Some(HotkeyChord::shifted(HotkeyKey::L)),
            HotkeyAction::CycleGridLayout => Some(HotkeyChord::shifted(HotkeyKey::G)),
            HotkeyAction::OpenShortcutsModal => Some(HotkeyChord::plain(HotkeyKey::Question)),
        },
        HotkeyProfileId::ReplayPractice => match action {
            HotkeyAction::ReplayPlayPause => Some(HotkeyChord::plain(HotkeyKey::Space)),
            HotkeyAction::ReplayStepForward => Some(HotkeyChord::plain(HotkeyKey::N)),
            HotkeyAction::ReplayStepBack => Some(HotkeyChord::plain(HotkeyKey::P)),
            HotkeyAction::ReplayCutTool => Some(HotkeyChord::plain(HotkeyKey::R)),
            HotkeyAction::QuickBuyMarket => Some(HotkeyChord::plain(HotkeyKey::B)),
            HotkeyAction::QuickSellMarket => Some(HotkeyChord::plain(HotkeyKey::S)),
            HotkeyAction::FlattenPosition => Some(HotkeyChord::plain(HotkeyKey::F)),
            HotkeyAction::CancelAllPending => Some(HotkeyChord::plain(HotkeyKey::X)),
            HotkeyAction::ToggleTradingDock => Some(HotkeyChord::plain(HotkeyKey::T)),
            HotkeyAction::ToggleDomLadderView => Some(HotkeyChord::plain(HotkeyKey::D)),
            HotkeyAction::CenterDomOnMark => Some(HotkeyChord::plain(HotkeyKey::C)),
            HotkeyAction::ToggleScannerDock => Some(HotkeyChord::shifted(HotkeyKey::M)),
            HotkeyAction::ToggleJournalDock => Some(HotkeyChord::shifted(HotkeyKey::J)),
            HotkeyAction::ToggleAlertsDock => Some(HotkeyChord::shifted(HotkeyKey::A)),
            HotkeyAction::ToggleStrategyDock => Some(HotkeyChord::shifted(HotkeyKey::L)),
            HotkeyAction::CycleGridLayout => Some(HotkeyChord::plain(HotkeyKey::G)),
            HotkeyAction::OpenShortcutsModal => Some(HotkeyChord::plain(HotkeyKey::Question)),
        },
    }
}

impl HotkeyProfile {
    /// Materializes one of the built-in conflict-free hotkey presets.
    #[must_use]
    pub fn from_preset(id: HotkeyProfileId) -> Self {
        let base_preset = if id == HotkeyProfileId::Custom {
            HotkeyProfileId::StandardCharting
        } else {
            id
        };
        let bindings = HotkeyAction::ALL
            .into_iter()
            .map(|action| (action, preset_default_chord(id, action)))
            .collect();
        Self {
            id,
            base_preset,
            bindings,
        }
    }

    /// Constructs a preset, optionally restoring a custom profile's underlying `base_preset`, and
    /// deterministically applies [`HotkeyOverride`] edits while keeping the resulting binding table
    /// 100% conflict-free.
    #[must_use]
    pub fn from_preset_with_base_and_overrides(
        id: HotkeyProfileId,
        base_preset: Option<HotkeyProfileId>,
        overrides: &[HotkeyOverride],
    ) -> Self {
        let mut profile = match (id, base_preset) {
            (HotkeyProfileId::Custom, Some(bp)) if bp != HotkeyProfileId::Custom => {
                let mut seeded = Self::from_preset(bp);
                seeded.id = HotkeyProfileId::Custom;
                seeded
            }
            _ => Self::from_preset(id),
        };
        for ov in overrides {
            profile.set_binding_preserving_id(ov.action, ov.chord);
        }
        profile
    }

    /// Constructs a preset and deterministically applies [`HotkeyOverride`] edits while keeping
    /// the resulting binding table 100% conflict-free.
    #[must_use]
    pub fn from_preset_and_overrides(id: HotkeyProfileId, overrides: &[HotkeyOverride]) -> Self {
        Self::from_preset_with_base_and_overrides(id, None, overrides)
    }

    /// Returns the underlying preset identifier remembered across `Custom` rebinds so
    /// resetting a customized preset restores its true baseline (e.g. `InstitutionalDomFast`).
    #[inline]
    #[must_use]
    pub const fn base_preset(&self) -> HotkeyProfileId {
        if matches!(self.base_preset, HotkeyProfileId::Custom) {
            HotkeyProfileId::StandardCharting
        } else {
            self.base_preset
        }
    }

    /// Restores this profile to its recorded `base_preset` (`StandardCharting`, `InstitutionalDomFast`, or `ReplayPractice`).
    pub fn reset_to_base_preset(&mut self) {
        *self = Self::from_preset(self.base_preset());
    }

    /// Normalizes a deserialized [`HotkeyProfile`] so every [`HotkeyAction`] appears once and
    /// duplicate chords are deduplicated deterministically.
    pub fn sanitize(&mut self) {
        if self.base_preset == HotkeyProfileId::Custom {
            self.base_preset = HotkeyProfileId::StandardCharting;
        }
        if self.id != HotkeyProfileId::Custom {
            self.base_preset = self.id;
        }
        let seed_id = if self.id == HotkeyProfileId::Custom {
            self.base_preset
        } else {
            self.id
        };
        let mut canonical = Self::from_preset(seed_id);
        let raw_entries = std::mem::take(&mut self.bindings);
        for (action, chord) in raw_entries {
            canonical.set_binding_preserving_id(
                action,
                chord.map(|c| HotkeyChord::new(c.key, c.modifiers)),
            );
        }
        self.bindings = canonical.bindings;
    }

    #[must_use]
    pub fn bindings(&self) -> &[(HotkeyAction, Option<HotkeyChord>)] {
        &self.bindings
    }

    #[must_use]
    pub fn chord_for(&self, action: HotkeyAction) -> Option<HotkeyChord> {
        self.bindings
            .iter()
            .find(|(a, _)| *a == action)
            .and_then(|(_, c)| *c)
    }

    /// Resolves a canonical keyboard chord into its bound [`HotkeyAction`], if any.
    #[must_use]
    pub fn resolve(&self, chord: HotkeyChord) -> Option<HotkeyAction> {
        let norm = HotkeyChord::new(chord.key, chord.modifiers);
        self.bindings
            .iter()
            .find(|(_, c)| *c == Some(norm))
            .map(|(a, _)| *a)
    }

    fn set_binding_preserving_id(&mut self, action: HotkeyAction, chord: Option<HotkeyChord>) {
        let norm = chord.map(|c| HotkeyChord::new(c.key, c.modifiers));
        if let Some(target_chord) = norm {
            for (existing_action, existing_chord) in &mut self.bindings {
                if *existing_action != action && *existing_chord == Some(target_chord) {
                    *existing_chord = None;
                }
            }
        }
        if let Some((_, slot)) = self.bindings.iter_mut().find(|(a, _)| *a == action) {
            *slot = norm;
        } else {
            self.bindings.push((action, norm));
        }
    }

    /// Rebinds `action` to `chord` (`None` unbinds), clearing any conflicting prior binding and
    /// switching `self.id` to [`HotkeyProfileId::Custom`].
    pub fn rebind(&mut self, action: HotkeyAction, chord: Option<HotkeyChord>) {
        self.set_binding_preserving_id(action, chord);
        self.id = HotkeyProfileId::Custom;
    }

    /// Extracts only those actions whose active chord differs from its underlying preset baseline.
    #[must_use]
    pub fn overrides_from_preset(&self) -> Vec<HotkeyOverride> {
        let ref_id = if self.id == HotkeyProfileId::Custom {
            self.base_preset()
        } else {
            self.id
        };
        let mut out = Vec::new();
        for &(action, chord) in &self.bindings {
            let baseline = preset_default_chord(ref_id, action);
            if chord != baseline {
                out.push(HotkeyOverride { action, chord });
            }
        }
        out
    }

    /// Returns any duplicate chord conflicts (`(chord, first_action, second_action)`).
    /// All constructors and `rebind` guarantee an empty vector.
    #[must_use]
    pub fn conflicts(&self) -> Vec<(HotkeyChord, HotkeyAction, HotkeyAction)> {
        let mut out = Vec::new();
        for i in 0..self.bindings.len() {
            let Some(ci) = self.bindings[i].1 else {
                continue;
            };
            for j in (i + 1)..self.bindings.len() {
                if self.bindings[j].1 == Some(ci) {
                    out.push((ci, self.bindings[i].0, self.bindings[j].0));
                }
            }
        }
        out
    }
}

/// Global keyboard eligibility policy: suppresses global
/// transport/order hotkeys whenever any modal is open, a text/numeric widget holds keyboard focus,
/// or a system command modifier (`Cmd`/`Ctrl`) is held.
#[inline]
#[must_use]
pub const fn is_global_hotkey_eligible(
    any_modal_open: bool,
    wants_keyboard_input: bool,
    command_modifier_down: bool,
) -> bool {
    !any_modal_open && !wants_keyboard_input && !command_modifier_down
}
