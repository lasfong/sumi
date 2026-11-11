"""Sumi V3 Test Safety and Database Isolation Safeguard.

Enforces fail-closed protection against any test attempting to connect to,
open, modify, or delete the production SQLite database (`backend/sumi.db`).

Non-Negotiable Invariants:
1. Production database `backend/sumi.db` is strictly NEVER connected to or opened in tests.
2. Read-only SHA-256 integrity checks (`open(..., 'rb')`) are explicitly permitted.
3. String path inspection (e.g. `settings.DATABASE_URL` CWD tests) without connecting is permitted.
4. All test databases must be isolated temporary instances (in-memory or tmp_path).
"""

from __future__ import annotations

import _io
import builtins
import hashlib
import io
import os
import sys
from pathlib import Path
from typing import Any
from urllib.parse import unquote, urlparse

# Canonical paths and baseline hashes
BACKEND_DIR = Path(__file__).resolve().parent
PROD_DB_PATH = (BACKEND_DIR / "sumi.db").resolve()
PROD_DIR = BACKEND_DIR.resolve()

# Backward compatibility legacy constants (marked deprecated)
PROD_DB_BASELINE_SHA256 = "ef5c035c27d49de1de51a59e28e1cac5109d92240bcb600239f2180d49128f3e"
PROD_DB_BASELINE_SIZE = 1499136

_SAFEGUARD_INSTALLED = False
_AUDIT_HOOK_INSTALLED = False

# Dynamic session-level baseline state (strictly immutable once captured)
_RUN_BASELINE_CAPTURED = False
_RUN_BASELINE_EXISTS = False
_RUN_BASELINE_SHA256: str | None = None
_RUN_BASELINE_SIZE: int = 0
_RUN_BASELINE_AUX_FILES: set[str] = set()
_SESSION_BASELINE_LOCKED = False

# Saved original functions
_orig_builtin_open = builtins.open
_orig_io_open = io.open
_orig_c_io_open = _io.open
_orig_os_open = os.open
_orig_sqlite3_connect: Any = None
_orig_dbapi2_connect: Any = None
_orig_c_connect: Any = None
_orig_os_remove = os.remove
_orig_os_unlink = os.unlink
_orig_os_rename = os.rename
_orig_os_replace = os.replace
_orig_os_chmod = os.chmod
_orig_path_open = Path.open
_orig_path_unlink = Path.unlink
_orig_path_rename = Path.rename
_orig_path_replace = Path.replace
_orig_path_chmod = Path.chmod
_orig_path_write_bytes = Path.write_bytes
_orig_path_write_text = Path.write_text


def _get_auxiliary_files() -> set[str]:
    """List any existing auxiliary files for sumi.db (wal, shm, journal) in the production directory."""
    if not PROD_DIR.exists():
        return set()
    return {
        f.name
        for f in PROD_DIR.iterdir()
        if f.name.startswith("sumi.db-") or f.name.startswith("sumi.db.")
    }


def compute_database_hash(target_path: Path | str) -> tuple[str | None, int]:
    """Compute current SHA-256 and size of a database file in chunks without opening in write mode."""
    path = Path(target_path).resolve()
    if not path.exists():
        return None, 0
    size = path.stat().st_size
    h = hashlib.sha256()
    with _orig_builtin_open(path, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest(), size


def compute_production_database_hash() -> tuple[str | None, int]:
    """Compute current SHA-256 and size of backend/sumi.db in chunks without opening in write mode."""
    return compute_database_hash(PROD_DB_PATH)


def capture_production_database_baseline() -> tuple[bool, str | None, int]:
    """Capture a dynamic session-level baseline of backend/sumi.db prior to test execution.

    Once captured during a test session, the baseline is strictly immutable to prevent
    subsequent calls from overwriting the session baseline or masking an earlier
    integrity mismatch.

    Returns:
        (exists, sha256, size)
    """
    global _RUN_BASELINE_CAPTURED, _RUN_BASELINE_EXISTS, _RUN_BASELINE_SHA256, _RUN_BASELINE_SIZE, _RUN_BASELINE_AUX_FILES, _SESSION_BASELINE_LOCKED

    if _SESSION_BASELINE_LOCKED or _RUN_BASELINE_CAPTURED:
        return _RUN_BASELINE_EXISTS, _RUN_BASELINE_SHA256, _RUN_BASELINE_SIZE

    if not PROD_DB_PATH.exists():
        _RUN_BASELINE_EXISTS = False
        _RUN_BASELINE_SHA256 = None
        _RUN_BASELINE_SIZE = 0
    else:
        _RUN_BASELINE_EXISTS = True
        _RUN_BASELINE_SHA256, _RUN_BASELINE_SIZE = compute_production_database_hash()

    _RUN_BASELINE_AUX_FILES = _get_auxiliary_files()
    _RUN_BASELINE_CAPTURED = True
    _SESSION_BASELINE_LOCKED = True
    return _RUN_BASELINE_EXISTS, _RUN_BASELINE_SHA256, _RUN_BASELINE_SIZE


def is_session_baseline_locked() -> bool:
    """Check if the session database baseline has already been captured and locked."""
    return _SESSION_BASELINE_LOCKED or _RUN_BASELINE_CAPTURED


def get_session_database_baseline() -> tuple[bool, str | None, int]:
    """Get the active session-level database baseline."""
    return _RUN_BASELINE_EXISTS, _RUN_BASELINE_SHA256, _RUN_BASELINE_SIZE


def capture_isolated_database_baseline(db_path: Path | str) -> tuple[bool, str | None, int]:
    """Capture an isolated, local baseline for a specific database path without mutating or affecting the session baseline.

    Useful for helper and unit tests operating on isolated temporary databases.
    """
    path = Path(db_path).resolve()
    if not path.exists():
        return False, None, 0
    sha256, size = compute_database_hash(path)
    return True, sha256, size


def is_production_database(target: Any) -> bool:
    """Check if a path or connection target resolves to the production database or its auxiliary files."""
    if target is None:
        return False

    if isinstance(target, bytes):
        try:
            target = target.decode("utf-8")
        except Exception:
            return False

    if isinstance(target, os.PathLike):
        target = os.fspath(target)

    if not isinstance(target, str):
        return False

    target_clean = target.strip()
    if target_clean in (":memory:", ""):
        return False

    # Extract path from SQLite URI if formatted as URI
    if target_clean.startswith("file:"):
        parsed = urlparse(target_clean)
        target_path_str = unquote(parsed.path)
        if not target_path_str:
            return False
    else:
        target_path_str = target_clean

    try:
        candidate = Path(target_path_str).resolve()
        if candidate == PROD_DB_PATH:
            return True
        # Intercept auxiliary SQLite files in production directory (e.g. sumi.db-wal, sumi.db-shm, sumi.db-journal)
        if candidate.parent == PROD_DIR and (
            candidate.name == "sumi.db"
            or candidate.name.startswith("sumi.db-")
            or candidate.name.startswith("sumi.db.")
        ):
            return True

        # Also resolve relative to repository root if target_path_str was relative to root
        repo_root = PROD_DIR.parent
        candidate_from_root = (repo_root / target_path_str).resolve()
        if candidate_from_root == PROD_DB_PATH:
            return True
        if candidate_from_root.parent == PROD_DIR and (
            candidate_from_root.name == "sumi.db"
            or candidate_from_root.name.startswith("sumi.db-")
            or candidate_from_root.name.startswith("sumi.db.")
        ):
            return True
    except Exception:
        pass

    return False


def verify_production_database_integrity(
    baseline: tuple[bool, str | None, int] | None = None,
) -> tuple[bool, str | None, int]:
    """Verify that backend/sumi.db has not been modified since baseline capture (zero-delta assertion).

    If baseline is passed as (exists, sha256, size), verifies against that baseline.
    If baseline is None, verifies against the run-local captured baseline (capturing one if not yet captured).

    Returns:
        (is_valid, current_sha256, current_size)
    """
    global _RUN_BASELINE_CAPTURED, _RUN_BASELINE_EXISTS, _RUN_BASELINE_SHA256, _RUN_BASELINE_SIZE, _RUN_BASELINE_AUX_FILES

    if baseline is not None:
        expected_exists, expected_sha256, expected_size = baseline
        check_aux = False
    else:
        if not _RUN_BASELINE_CAPTURED:
            capture_production_database_baseline()
            return True, _RUN_BASELINE_SHA256, _RUN_BASELINE_SIZE
        expected_exists = _RUN_BASELINE_EXISTS
        expected_sha256 = _RUN_BASELINE_SHA256
        expected_size = _RUN_BASELINE_SIZE
        check_aux = True

    current_exists = PROD_DB_PATH.exists()

    if not expected_exists:
        if current_exists:
            curr_sha, curr_size = compute_production_database_hash()
            return False, curr_sha, curr_size
        return True, None, 0

    if not current_exists:
        return False, None, 0

    current_sha256, current_size = compute_production_database_hash()

    if check_aux:
        current_aux = _get_auxiliary_files()
        new_aux = current_aux - _RUN_BASELINE_AUX_FILES
        if new_aux:
            return False, current_sha256, current_size

    is_valid = (current_sha256 == expected_sha256 and current_size == expected_size)
    return is_valid, current_sha256, current_size


def _guarded_sqlite3_connect(database: Any, *args: Any, **kwargs: Any) -> Any:
    """Intercept direct sqlite3.connect calls targeting production database."""
    if is_production_database(database):
        raise RuntimeError(
            f"FAIL-CLOSED SAFEGUARD: Connection to production database '{database}' "
            f"(canonical path: '{PROD_DB_PATH}') is strictly prohibited during test execution! "
            f"All test executions must use isolated temporary databases (e.g. sqlite:///:memory: or tmp_path)."
        )
    return _orig_sqlite3_connect(database, *args, **kwargs)


def _guarded_builtin_open(file: Any, mode: str = "r", *args: Any, **kwargs: Any) -> Any:
    """Intercept mutating open() calls targeting production database, while allowing read-only SHA-256 inspection."""
    if is_production_database(file):
        mode_str = str(kwargs.get("mode", mode))
        if any(c in mode_str for c in ("w", "a", "+", "x")):
            raise RuntimeError(
                f"FAIL-CLOSED SAFEGUARD: Mutating open(mode='{mode_str}') attempted on production database: '{file}'! "
                f"Production database is strictly read-only for SHA-256 checksum verification during testing."
            )
    return _orig_builtin_open(file, mode, *args, **kwargs)


def _guarded_io_open(file: Any, mode: str = "r", *args: Any, **kwargs: Any) -> Any:
    """Intercept mutating io.open() calls targeting production database."""
    if is_production_database(file):
        mode_str = str(kwargs.get("mode", mode))
        if any(c in mode_str for c in ("w", "a", "+", "x")):
            raise RuntimeError(
                f"FAIL-CLOSED SAFEGUARD: Mutating io.open(mode='{mode_str}') attempted on production database: '{file}'! "
                f"Production database is strictly read-only for SHA-256 checksum verification during testing."
            )
    return _orig_io_open(file, mode, *args, **kwargs)


def _guarded_path_open(self: Path, mode: str = "r", *args: Any, **kwargs: Any) -> Any:
    """Intercept mutating Path.open() calls targeting production database."""
    if is_production_database(self):
        mode_str = str(kwargs.get("mode", mode))
        if any(c in mode_str for c in ("w", "a", "+", "x")):
            raise RuntimeError(
                f"FAIL-CLOSED SAFEGUARD: Mutating Path.open(mode='{mode_str}') attempted on production database: '{self}'! "
                f"Production database is strictly protected from modification during test execution."
            )
    return _orig_path_open(self, mode, *args, **kwargs)


def _guarded_path_unlink(self: Path, *args: Any, **kwargs: Any) -> Any:
    if is_production_database(self):
        raise RuntimeError(f"FAIL-CLOSED SAFEGUARD: Deletion attempted on production database via Path.unlink: '{self}'!")
    return _orig_path_unlink(self, *args, **kwargs)


def _guarded_path_rename(self: Path, target: Any, *args: Any, **kwargs: Any) -> Any:
    if is_production_database(self) or is_production_database(target):
        raise RuntimeError(f"FAIL-CLOSED SAFEGUARD: Renaming attempted on production database via Path.rename: '{self}' -> '{target}'!")
    return _orig_path_rename(self, target, *args, **kwargs)


def _guarded_path_replace(self: Path, target: Any, *args: Any, **kwargs: Any) -> Any:
    if is_production_database(self) or is_production_database(target):
        raise RuntimeError(f"FAIL-CLOSED SAFEGUARD: Replacement attempted on production database via Path.replace: '{self}' -> '{target}'!")
    return _orig_path_replace(self, target, *args, **kwargs)


def _guarded_path_chmod(self: Path, mode: int, *args: Any, **kwargs: Any) -> Any:
    if is_production_database(self):
        raise RuntimeError(f"FAIL-CLOSED SAFEGUARD: Chmod attempted on production database via Path.chmod: '{self}'!")
    return _orig_path_chmod(self, mode, *args, **kwargs)


def _guarded_path_write_bytes(self: Path, data: bytes) -> int:
    if is_production_database(self):
        raise RuntimeError(f"FAIL-CLOSED SAFEGUARD: Mutating Path.write_bytes attempted on production database: '{self}'!")
    return _orig_path_write_bytes(self, data)


def _guarded_path_write_text(self: Path, data: str, *args: Any, **kwargs: Any) -> int:
    if is_production_database(self):
        raise RuntimeError(f"FAIL-CLOSED SAFEGUARD: Mutating Path.write_text attempted on production database: '{self}'!")
    return _orig_path_write_text(self, data, *args, **kwargs)


def _guarded_os_remove(path: Any, *args: Any, **kwargs: Any) -> Any:
    if is_production_database(path):
        raise RuntimeError(f"FAIL-CLOSED SAFEGUARD: Deletion attempted on production database: '{path}'!")
    return _orig_os_remove(path, *args, **kwargs)


def _guarded_os_unlink(path: Any, *args: Any, **kwargs: Any) -> Any:
    if is_production_database(path):
        raise RuntimeError(f"FAIL-CLOSED SAFEGUARD: Deletion attempted on production database: '{path}'!")
    return _orig_os_unlink(path, *args, **kwargs)


def _guarded_os_rename(src: Any, dst: Any, *args: Any, **kwargs: Any) -> Any:
    if is_production_database(src) or is_production_database(dst):
        raise RuntimeError(f"FAIL-CLOSED SAFEGUARD: Renaming attempted on production database: '{src}' -> '{dst}'!")
    return _orig_os_rename(src, dst, *args, **kwargs)


def _guarded_os_replace(src: Any, dst: Any, *args: Any, **kwargs: Any) -> Any:
    if is_production_database(src) or is_production_database(dst):
        raise RuntimeError(f"FAIL-CLOSED SAFEGUARD: Replacement attempted on production database: '{src}' -> '{dst}'!")
    return _orig_os_replace(src, dst, *args, **kwargs)


def _guarded_os_chmod(path: Any, mode: Any, *args: Any, **kwargs: Any) -> Any:
    if is_production_database(path):
        raise RuntimeError(f"FAIL-CLOSED SAFEGUARD: Chmod attempted on production database: '{path}'!")
    return _orig_os_chmod(path, mode, *args, **kwargs)


def _guarded_os_open(path: Any, flags: int, *args: Any, **kwargs: Any) -> int:
    """Intercept mutating os.open() calls targeting production database or auxiliary files."""
    if is_production_database(path):
        write_flags = (
            os.O_WRONLY
            | os.O_RDWR
            | os.O_CREAT
            | os.O_APPEND
            | os.O_TRUNC
            | getattr(os, "O_EXCL", 0)
        )
        if flags & write_flags:
            raise RuntimeError(
                f"FAIL-CLOSED SAFEGUARD: Mutating os.open(flags={flags}) attempted on production database: '{path}'! "
                f"Production database is strictly protected from modification during test execution."
            )
    return _orig_os_open(path, flags, *args, **kwargs)


def _safeguard_audit_hook(event: str, args: tuple[Any, ...]) -> None:
    """CPython native runtime audit hook enforcing deepest fail-closed boundary."""
    if event == "open":
        file = args[0] if len(args) > 0 else None
        mode = args[1] if len(args) > 1 else None
        flags = args[2] if len(args) > 2 else 0
        if is_production_database(file):
            mode_str = str(mode) if mode is not None else ""
            if any(c in mode_str for c in ("w", "a", "+", "x")):
                raise RuntimeError(
                    f"FAIL-CLOSED SAFEGUARD: Mutating open(mode='{mode_str}') on production database '{file}' caught by runtime audit hook!"
                )
            write_flags = (
                os.O_WRONLY
                | os.O_RDWR
                | os.O_CREAT
                | os.O_APPEND
                | os.O_TRUNC
                | getattr(os, "O_EXCL", 0)
            )
            if isinstance(flags, int) and (flags & write_flags):
                raise RuntimeError(
                    f"FAIL-CLOSED SAFEGUARD: Mutating open(flags={flags}) on production database '{file}' caught by runtime audit hook!"
                )
    elif event == "sqlite3.connect":
        database = args[0] if len(args) > 0 else None
        if is_production_database(database):
            raise RuntimeError(
                f"FAIL-CLOSED SAFEGUARD: Connection to production database '{database}' caught by runtime audit hook!"
            )


def install_database_safeguard() -> None:
    """Install the fail-closed database safeguard across sqlite3, SQLAlchemy, builtins, io, pathlib, and os modules.

    Idempotent: safe to invoke repeatedly.
    """
    global _SAFEGUARD_INSTALLED, _AUDIT_HOOK_INSTALLED, _orig_sqlite3_connect, _orig_dbapi2_connect, _orig_c_connect

    if _SAFEGUARD_INSTALLED:
        return

    import sqlite3
    import sqlite3.dbapi2
    import _sqlite3

    _orig_sqlite3_connect = sqlite3.connect
    _orig_dbapi2_connect = sqlite3.dbapi2.connect
    _orig_c_connect = _sqlite3.connect

    # Patch sqlite3 modules at all entry points
    sqlite3.connect = _guarded_sqlite3_connect
    sqlite3.dbapi2.connect = _guarded_sqlite3_connect
    _sqlite3.connect = _guarded_sqlite3_connect

    # Patch filesystem mutation hooks across builtins, io, and os
    builtins.open = _guarded_builtin_open
    io.open = _guarded_io_open
    _io.open = _guarded_io_open
    os.open = _guarded_os_open
    os.remove = _guarded_os_remove
    os.unlink = _guarded_os_unlink
    os.rename = _guarded_os_rename
    os.replace = _guarded_os_replace
    os.chmod = _guarded_os_chmod

    # Patch pathlib.Path mutation hooks
    Path.open = _guarded_path_open
    Path.unlink = _guarded_path_unlink
    Path.rename = _guarded_path_rename
    Path.replace = _guarded_path_replace
    Path.chmod = _guarded_path_chmod
    Path.write_bytes = _guarded_path_write_bytes
    Path.write_text = _guarded_path_write_text

    # Install native runtime audit hook as deepest fail-closed layer
    if not _AUDIT_HOOK_INSTALLED:
        try:
            sys.addaudithook(_safeguard_audit_hook)
            _AUDIT_HOOK_INSTALLED = True
        except Exception:
            pass

    # Patch SQLAlchemy Engine do_connect event if SQLAlchemy is available
    try:
        from sqlalchemy import Engine, event

        @event.listens_for(Engine, "do_connect")
        def _guard_engine_do_connect(dialect: Any, conn_rec: Any, cargs: Any, cparams: Any) -> None:
            if dialect.name == "sqlite" and cargs:
                db_target = cargs[0]
                if is_production_database(db_target):
                    raise RuntimeError(
                        f"FAIL-CLOSED SAFEGUARD: SQLAlchemy Engine attempted connection to production database: '{db_target}' "
                        f"(canonical path: '{PROD_DB_PATH}')! "
                        f"All tests must use isolated temporary databases (e.g. sqlite:///:memory: or tmp_path)."
                    )
    except ImportError:
        pass

    _SAFEGUARD_INSTALLED = True
