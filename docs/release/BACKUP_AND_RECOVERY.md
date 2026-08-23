# Sumi Backup & Data Recovery Guide

## 1. Database File Backup & Restore

Sumi stores all system catalog data, daily/weekly candles, replay practice sessions, trades, decisions, checklists, journal entries, and sync audit manifests in `backend/sumi.db`.

### Creating a Manual Backup
To create an offline backup of your Sumi database:
1. Close any active Sumi processes.
2. Copy `backend/sumi.db` to your backup location:
   ```bash
   cp backend/sumi.db ~/Backups/sumi_backup_$(date +%Y%m%d).db
   ```
3. Record or verify the SHA-256 checksum:
   ```powershell
   Get-FileHash -Algorithm SHA256 backend\sumi.db
   ```

### Restoring from Backup
1. Stop Sumi backend/frontend instances.
2. Replace `backend/sumi.db` with your backup copy.
3. Start Sumi and navigate to `/import` or `/replay` to resume practice.

## 2. Local CSV & JSON Journal Export Recovery

In addition to full database backups, Sumi provides one-click structured export from the Journal page:
- **JSON Export**: Downloads complete session journal payloads including trade entry/exit, checklist snapshots, risk metrics, and emotion logs.
- **CSV Export**: Exports trade history compatible with standard spreadsheet analysis.

Exports can be accessed via `GET /api/replay/sessions/{session_id}/journal/export`.

## 3. Database Invariant & Recovery Isolation

- Automated unit tests and product UAT never alter production `backend/sumi.db`. They operate on temporary isolated database instances.
- If a market data sync run is interrupted by power loss or system failure, Sumi's transaction boundaries ensure SQLite database integrity. Use the One-Click Sync Rollback feature (`/import`) to restore catalog state if a partial sync run occurs.
