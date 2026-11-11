# REA LIVE CAPABILITY REPORT — SESSION R01

- **Session Identity:** `SESSION-R01-20261110T202837+0700`
- **REA Version:** `rea-agents@6.8.0`
- **Skill Version:** `49` (`229` MCP tools, `6` MCP prompts)
- **Report Status:** `VERIFIED` — REA was actually invoked through the live Antigravity MCP interface.

## 1. MCP Connection Verification

REA MCP server status was verified by calling `binary_session` through the live Antigravity MCP interface.

**Evidence:** `binary_session` responded with a full capability map (`open: false`, provider discovery complete) in this session, confirming that the Antigravity restart between Session 01 and Session R01 successfully activated the registered MCP server at `/opt/homebrew/bin/rea mcp`.

**Server Identity (live, from `rea doctor` stored evidence):**
- `cli_package_version: "6.8.0"`
- `sdk.server: "2.3.1"` / `sdk.client_test: "2.3.1"` / `sdk.core: "2.3.1"`
- `catalog.mcp_tools: 229` / `mcp_prompts: 6`
- `combined_sha256: fe938a7c6eba3763b22ae2022de76dc4a7812d35d86f1d4efd36b9962b04da0b`

## 2. Provider Availability Map (from live `binary_session`)

| Provider | Status | Reason | Available Operations |
|---|---|---|---|
| `native-macos` | **AVAILABLE** | Xcode CLI tools present | `inspect_macho`, `list_architectures`, `inspect_signature`, `inspect_plist`, `demangle_swift`, `capture_native_ui_scenario`, `observe_native_ui`, `observe_native_calls` |
| `rea-artifact-graph` | **AVAILABLE** | Built-in REA provider | `trace_dylib_resolution`, `inspect_artifact`, `extract_artifact`, `decode_interface_builder`, `inspect_keyed_archive`, `inspect_asset_catalog` |
| `rea-dotnet-static` | **AVAILABLE** | Built-in REA provider | `inspect_managed_artifact`, `inspect_managed_members`, `inspect_managed_native_boundaries` |
| `ghidra` | **UNAVAILABLE** | `GHIDRA_INSTALL_DIR` not set | Procedures, disassembly, decompilation (all possible once Ghidra 12.1.x installed) |
| `hopper` | **UNAVAILABLE** | App missing at `/Applications/Hopper Disassembler.app` | Procedures, disassembly, pseudocode |
| `ida` | **UNAVAILABLE** | `REA_IDA_MCP_CONFIG` not set | Procedures, disassembly, Hex-Rays pseudocode |
| `kuna` | **UNAVAILABLE** | `REA_KUNA_COMMAND` not set | Procedures, pseudocode |

## 3. Tool Availability Map (from live `binary_session`)

### Immediately Available (no target required)
| Surface | Available Tools |
|---|---|
| `session` | `binary_session`, `open_binary`, `close_binary`, `build_call_path`, `compare_artifacts`, `import_evidence_bundle`, `get_evidence_bundle`, `export_evidence_bundle`, `capture_process_scenario` |
| `application` | `analyze_javascript_application`, `analyze_web_bundle`, `update_analysis_settings`, `trace_application_feature`, `trace_javascript_semantics`, `compare_application_versions`, `project_apple_application_graph`, `inspect_analysis_view`, `build_reconstruction_obligation_ledger`, `evaluate_reconstruction_coverage` |
| `browser-provider` | `list_browser_targets`, `capture_web_screenshot`, `inspect_web_page`, `observe_web_session`, `capture_browser_scenario`, `compare_web_captures` |
| `electron-provider` | `analyze_javascript_application` (Electron mode), `capture_electron_scenario` |

### Available After `open_binary`
All 33 `official-proxy` native analysis tools become available once a binary is opened AND a native provider (Ghidra, Hopper, IDA, Kuna) is configured.

**Currently available after `open_binary` (no native provider needed):**
- `inspect_macho`, `list_architectures`, `inspect_signature`, `inspect_plist`, `demangle_swift`, `trace_dylib_resolution` — via `native-macos` provider
- `inspect_managed_artifact`, `inspect_managed_members` — via `rea-dotnet-static` provider (N/A for Rust binaries)

### Requires Additional Dependencies
| Tool Group | Required Dependency | Purpose |
|---|---|---|
| Procedure disassembly, pseudocode, cross-refs | Ghidra 12.1.x + Java, OR Hopper license, OR IDA Pro + IDA MCP config, OR Kuna | Deep native code analysis |
| `capture_native_ui_scenario`, `observe_native_calls` | macOS Screen Recording TCC permission | Dynamic/runtime analysis |
| ADB tools (`inspect_adb_client`, etc.) | `adb` binary in PATH | Android analysis (N/A for Sumi) |
| Frida instrumentation | Frida session management | Live runtime injection |

## 4. Session R01 REA Operations Executed

| # | MCP Tool | Input | Evidence ID | Result |
|---|---|---|---|---|
| 1 | `binary_session` | (none) | — | Live capability map returned; 4 providers listed; `open: false` confirmed |
| 2 | `open_binary` | `/Users/mizuhara/workspace/sumi/target/debug/sumi-workstation` | — | `format: mach-o`, `architecture: arm64`, `sha256: bf299c4d...` |
| 3 | `list_architectures` | (open binary) | `ev_93a09f1c...` | arm64 single-slice; via `lipo`; `tool: native-macos` |
| 4 | `inspect_macho` (summary) | (open binary) | `ev_a42472d0...` | 30 load commands, 14 dylib deps, 327 imports, 6208 exports, 5 segments |
| 5 | `inspect_analysis_view` (summary) | `ev_a42472d0...` | `ev_e95e76c4...` | Mach-O summary confirmed: 6587 items examined, complete |
| 6 | `trace_dylib_resolution` | (open binary) | `ev_ac47096a...` | 14 dylib edges resolved, all system frameworks as `outside-target` |
| 7 | `analyze_javascript_application` | `/Users/mizuhara/workspace/sumi/ARCHIVE/frontend` | `ev_e209114...` | ChartKernel bundle: 84,465 AST nodes visited, 16 findings, partial semantic graph |

## 5. Limitations of Current Installation

1. **No native disassembly/pseudocode** — `procedure_assembly`, `procedure_pseudo_code`, `procedure_references`, `list_procedures`, `search_strings` all require Ghidra, Hopper, IDA, or Kuna. None are installed.
2. **`binary_overview` blocked** — Requires a native analysis provider.
3. **`search_strings` blocked** — Requires a native analysis provider.
4. **TCC permissions not granted** — `capture_native_ui_scenario` and `observe_native_calls` require Screen Recording permission.
5. **JavaScript analysis hit semantic node ceiling** — `max_semantic_nodes_per_file: 20000` was reached on the ChartKernel bundle. Can be raised with `--max-semantic-nodes-per-file`.

## 6. Recommended Provider to Install Next

**Ghidra 12.1.x** (free, open-source, no license required):
- Unlocks: `list_procedures`, `procedure_assembly`, `procedure_pseudo_code`, `search_strings`, `list_strings`, `procedure_references`, `analyze_function`, `batch_decompile`
- Required: Java JDK 21+, set `GHIDRA_INSTALL_DIR` environment variable
- No new MCP configuration changes needed — REA already supports it
