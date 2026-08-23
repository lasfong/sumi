# Sumi Supported Platforms & Local-First Privacy Policy

## 1. Supported Platform Matrix

Sumi is built to run reliably as a desktop application on modern standard environments.

| Environment | Requirement / Spec | Verification Status |
| --- | --- | --- |
| **Operating System** | Windows 10/11 (64-bit), macOS 12+, Linux (Ubuntu 22.04+) | Verified on Windows 11 & Linux |
| **Python** | Python 3.12 or 3.13 (`pandas-ta>=0.4.71b0`, `fastapi`, `sqlalchemy`) | Verified Python 3.13 |
| **Node.js** | Node.js v20.x or v24.x (`npm 10+`) | Verified Node v24.14.0 |
| **Display Resolution** | Recommended: 1440×1000 or 1920×1080; Minimum: 1280×800 | Verified 1440×1000 & 1280×800 |

## 2. 100% Local-First Privacy Guarantee

Sumi is strictly a local-first technical-analysis replay workstation.

- **Zero Network Telemetry**: Sumi collects no telemetry, analytics, tracking pings, or usage statistics.
- **Private Workstation Data**: User replay sessions, market decisions, trade plans, position sizes, journal entries, emotion logs, and strategy definitions remain 100% inside your local SQLite database (`backend/sumi.db`) and local filesystem.
- **Provider Boundary Isolation**: Optional online data synchronization (PRO-11 / SSI Open API / `vnstock`) operates on-demand only upon explicit user trigger. No private user trading or replay activity is sent to external data providers.
- **Zero Cloud Leakage**: No data is uploaded to remote servers or third-party cloud analytics services.
