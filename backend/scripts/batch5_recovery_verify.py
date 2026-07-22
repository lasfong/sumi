"""Batch 5 temporary-SQLite backup and semantic verification utility."""

from __future__ import annotations

import argparse
import hashlib
import json
import sqlite3
from pathlib import Path


def semantic_snapshot(path: Path) -> dict:
    connection = sqlite3.connect(path)
    connection.row_factory = sqlite3.Row
    try:
        tables = [row[0] for row in connection.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
        )]
        content: dict[str, list[dict]] = {}
        schemas: dict[str, str] = {}
        for table in tables:
            schemas[table] = connection.execute(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name=?", (table,)
            ).fetchone()[0]
            columns = [row[1] for row in connection.execute(f'PRAGMA table_info("{table}")')]
            order = ", ".join(f'"{column}"' for column in columns)
            rows = connection.execute(f'SELECT * FROM "{table}" ORDER BY {order}').fetchall()
            content[table] = [{key: normalize(row[key]) for key in row.keys()} for row in rows]
        canonical = json.dumps({"schemas": schemas, "content": content}, sort_keys=True, separators=(",", ":"))
        return {
            "path": str(path),
            "tables": tables,
            "counts": {table: len(content[table]) for table in tables},
            "semanticSha256": hashlib.sha256(canonical.encode()).hexdigest(),
            "schemaVersion": connection.execute("PRAGMA user_version").fetchone()[0],
        }
    finally:
        connection.close()


def normalize(value):
    if isinstance(value, bytes):
        return {"bytesSha256": hashlib.sha256(value).hexdigest(), "length": len(value)}
    return value


def backup(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    source_connection = sqlite3.connect(source)
    destination_connection = sqlite3.connect(destination)
    try:
        source_connection.backup(destination_connection)
    finally:
        destination_connection.close()
        source_connection.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--backup", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--require-application-schema", action="store_true")
    args = parser.parse_args()
    result = {"source": semantic_snapshot(args.source)}
    if args.require_application_schema:
        required = {"candles", "decisions", "drawing_states", "executions", "journal_entries", "orders", "positions", "replay_sessions", "symbols", "trades"}
        missing = sorted(required - set(result["source"]["tables"]))
        result["applicationSchema"] = {"required": sorted(required), "missing": missing, "pass": not missing}
        if missing:
            raise SystemExit(f"Production-format copy is missing required tables: {missing}")
    if args.backup:
        backup(args.source, args.backup)
        result["backup"] = semantic_snapshot(args.backup)
        result["semanticEqual"] = result["source"]["semanticSha256"] == result["backup"]["semanticSha256"]
        if not result["semanticEqual"]:
            raise SystemExit("SQLite online backup semantic checksum mismatch")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
