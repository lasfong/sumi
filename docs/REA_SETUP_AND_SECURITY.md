# SUMI TRADING WORKSTATION — REA SETUP & SECURITY REVIEW

- **Session Identity:** `SESSION-01-20261110T160909+0700`
- **REA Integration State:** `SETUP_COMPLETE_RESTART_REQUIRED`

## 1. Official Package & Provenance Verification

- **Official Repository:** `https://github.com/morluto/rea`
- **Official Installation Guide:** `https://github.com/morluto/rea/blob/main/docs/getting-started/installation.md`
- **Published npm Package:** `rea-agents@6.8.0`
- **Tarball URL:** `https://registry.npmjs.org/rea-agents/-/rea-agents-6.8.0.tgz`
- **Integrity Digest:** `sha512-9QDeRnGqEDzABZyr04eLnBXBv/c6I9/TWu/6M0MD4pEfkySI/BIgwTODIbAReX3C1x6hjw9+Jlx4A2yUSPZ9ww==`
- **Shasum:** `6a2cc197e4ff7f11ba937125b3ca7e4de939906a`
- **SLSA Provenance Attestation:** `https://registry.npmjs.org/-/npm/v1/attestations/rea-agents@6.8.0` (`https://slsa.dev/provenance/v1`)
- **Supported Node.js Engines:** `^22.23.4 || ^24.22.1 || >=26.12.1`
- **Active Host Runtime:** Node.js `v26.13.0` (`npm 11.20.0`), upgraded via `brew upgrade node` from `v26.0.0` after explicit Product Owner authorization.

## 2. Pre-Setup Audit & Dry-Run Inspection

Before modifying any configuration:
1. Inspected `/Users/mizuhara/.gemini/config/mcp_config.json` top-level keys (`['doraemi-analyst', 'yumeko-operator', 'yumeko-readonly']`) without reading any credentials or secrets. Confirmed `rea` was not yet registered.
2. Confirmed `/Users/mizuhara/.gemini/config/skills` did not exist.
3. Ran `rea setup --help` and `rea setup --client antigravity --dry-run --json` from `$HOME` (outside the project repository), saving the full dry-run artifact to [`docs/evidence/rea_setup_dry_run.json`](file:///Users/mizuhara/workspace/sumi/docs/evidence/rea_setup_dry_run.json).

## 3. Authorized Setup & Doctor Verification

With explicit Product Owner authorization, executed from `$HOME`:

```bash
rea setup --client antigravity --yes --json
rea doctor --client antigravity --skill --json
```

### Verified Changes & Backups
- **MCP Registration File:** `/Users/mizuhara/.gemini/config/mcp_config.json`
  - Registered launcher: `["/opt/homebrew/bin/rea", "mcp"]`
  - Verified backup created by REA: `/Users/mizuhara/.gemini/config/mcp_config.json.rea.backup`
  - Existing MCP registrations (`doraemi-analyst`, `yumeko-operator`, `yumeko-readonly`) preserved intact.
- **Bundled Skill Installation:** `/Users/mizuhara/.gemini/config/skills/reverse-engineer-anything`
  - Skill Version: `49` (`state: "aligned"`, `installed_tool_count: 229`)
- **Scoped Doctor Result ([`docs/evidence/rea_doctor_antigravity.json`](file:///Users/mizuhara/workspace/sumi/docs/evidence/rea_doctor_antigravity.json)):**
  - `healthy: true` (for explicit scope `clients: ["antigravity"]`, `skill_clients: ["antigravity"]`)
  - `cli_package_version: "6.8.0"`
  - Catalog counts: `175` CLI commands, `229` MCP tools, `6` MCP prompts (`combined_sha256: fe938a7c6eba3763b22ae2022de76dc4a7812d35d86f1d4efd36b9962b04da0b`).

## 4. Advertised REA Tool Families (229 MCP Tools)

Per the verified REA `6.8.0` catalog (`skill` version `49`), REA provides 229 MCP tools across the following diagnostic and inspection families:
- **Session & Capability Discovery:** `binary_session`, `open_binary`, `close_binary`, `update_analysis_settings`
- **Native Binary & Mach-O / ELF / PE Inspection (when a native provider is configured):** Procedure listing, disassembly, decompilation, cross-references, strings, symbols, segments/sections, and type/comment annotations
- **Static JavaScript / Electron / Bundle Analysis:** Application bundle inspection, source-map recovery, and static JS analysis (operates out-of-the-box without external native decompilers)
- **Evidence & Ledger Management:** Structured forensic evidence export and reproducible session ledgers

## 5. Current Session Availability & Security Posture

- **Live Session Status:** `SETUP_COMPLETE_RESTART_REQUIRED`. Because Antigravity loads `mcp_config.json` at IDE startup, REA MCP tools are **not** active inside this Session 01 conversation turn and will become discoverable in a new session after restarting Antigravity.
- **Optional Native Engines:** Hopper, Ghidra, IDA, JADX, and Kuna were **not** installed or configured (per Session 01 security rules).
- **macOS TCC Permissions:** No Screen Recording, Accessibility, Automation, or Full Disk Access permissions were modified or granted automatically.
- **Transitive Dependency Note:** During `npm install --global rea-agents@6.8.0`, npm noted that transitive dependency `frida@17.23.2` had lifecycle install scripts skipped by default (`allowScripts` protection). This is safe and expected: untrusted postinstall scripts were not executed, and core REA CLI/MCP functionality operates normally.
- **Target Boundary:** REA is restricted strictly to authorized diagnostics of our own `sumi-workstation` artifacts. Reverse-engineering TradingView binaries, private APIs, credentials, or market feeds is strictly prohibited.
