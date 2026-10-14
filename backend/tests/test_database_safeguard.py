"""Tests for Sumi V3 Fail-Closed Database Test Safeguard.

Verifies that any attempt by any test to connect to, open, modify, or delete
the production database (backend/sumi.db) is intercepted and fails closed immediately,
while safe temporary and in-memory databases and read-only checksum operations succeed.
"""

from __future__ import annotations

import hashlib
import io
import os
import sqlite3
from pathlib import Path
import pytest
from unittest.mock import patch
from sqlalchemy import create_engine

from tests_safety import (
    PROD_DB_BASELINE_SHA256,
    PROD_DB_BASELINE_SIZE,
    PROD_DB_PATH,
    capture_isolated_database_baseline,
    capture_production_database_baseline,
    compute_database_hash,
    get_session_database_baseline,
    is_session_baseline_locked,
    verify_production_database_integrity,
)


def test_sqlite3_connect_absolute_path_aborts():
    """Direct sqlite3.connect targeting production DB absolute path must abort immediately."""
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*strictly prohibited"):
        sqlite3.connect(str(PROD_DB_PATH))


def test_sqlite3_connect_relative_path_aborts():
    """Direct sqlite3.connect targeting production DB relative path must abort immediately."""
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*strictly prohibited"):
        sqlite3.connect("backend/sumi.db")

    rel_path = os.path.relpath(PROD_DB_PATH, Path.cwd())
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*strictly prohibited"):
        sqlite3.connect(rel_path)


def test_sqlite3_connect_path_object_aborts():
    """Direct sqlite3.connect targeting Path object must abort immediately."""
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*strictly prohibited"):
        sqlite3.connect(PROD_DB_PATH)


def test_sqlite3_connect_uri_aborts():
    """Direct sqlite3.connect with SQLite URI targeting production DB must abort immediately."""
    uri = f"file:{PROD_DB_PATH.as_posix()}?mode=ro"
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*strictly prohibited"):
        sqlite3.connect(uri, uri=True)


def test_sqlite3_connect_auxiliary_files_abort():
    """Direct sqlite3.connect targeting auxiliary files (e.g. sumi.db-wal) must abort immediately."""
    wal_path = str(PROD_DB_PATH) + "-wal"
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*strictly prohibited"):
        sqlite3.connect(wal_path)


def test_sqlalchemy_engine_connect_to_production_db_aborts():
    """SQLAlchemy create_engine targeting production DB must abort on connect."""
    engine = create_engine(f"sqlite:///{PROD_DB_PATH.as_posix()}")
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*strictly prohibited|SQLAlchemy Engine"):
        with engine.connect():
            pass


def test_mutating_open_modes_abort():
    """Mutating open modes ('w', 'wb', 'r+', 'a') targeting production DB must abort immediately."""
    for mode in ["w", "wb", "r+", "a", "a+"]:
        with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*Mutating open"):
            open(PROD_DB_PATH, mode)


def test_os_filesystem_mutation_aborts():
    """Filesystem deletion, rename, replace, and chmod calls must abort immediately."""
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*Deletion attempted"):
        os.remove(PROD_DB_PATH)

    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*Deletion attempted"):
        os.unlink(PROD_DB_PATH)

    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*Renaming attempted"):
        os.rename(PROD_DB_PATH, "/tmp/sumi-renamed.db")

    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*Replacement attempted"):
        os.replace(PROD_DB_PATH, "/tmp/sumi-replaced.db")

    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*Chmod attempted"):
        os.chmod(PROD_DB_PATH, 0o644)


def test_os_open_mutating_flags_abort():
    """Low-level os.open with write/mutation flags targeting production DB must abort immediately."""
    mutating_flags = [
        os.O_WRONLY,
        os.O_RDWR,
        os.O_CREAT,
        os.O_APPEND,
        os.O_TRUNC,
        os.O_WRONLY | os.O_CREAT,
        os.O_RDWR | os.O_APPEND,
    ]
    for flag in mutating_flags:
        with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*Mutating os\.open"):
            os.open(str(PROD_DB_PATH), flag)

        with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*Mutating os\.open"):
            os.open(PROD_DB_PATH, flag)


def test_os_open_read_only_is_permitted():
    """Low-level os.open with os.O_RDONLY targeting production DB is permitted for checksum inspection."""
    fd = os.open(str(PROD_DB_PATH), os.O_RDONLY)
    assert fd >= 0
    try:
        chunk = os.read(fd, 64)
        assert len(chunk) == 64
        assert len(hashlib.sha256(chunk).hexdigest()) == 64
    finally:
        os.close(fd)


def test_symlink_evasion_aborts(tmp_path: Path):
    """Symlink pointing to production DB must be resolved and all connection/write attempts abort."""
    symlink_path = tmp_path / "symlink_sumi.db"
    try:
        symlink_path.symlink_to(PROD_DB_PATH)
    except OSError:
        pytest.skip("Symlink creation not permitted in this environment")

    # sqlite3 connect aborts
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*strictly prohibited"):
        sqlite3.connect(str(symlink_path))

    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*strictly prohibited"):
        sqlite3.connect(symlink_path)

    # builtins.open aborts
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*Mutating open"):
        open(symlink_path, "w")

    # os.open aborts
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*Mutating os\.open"):
        os.open(str(symlink_path), os.O_WRONLY)


def test_relative_and_variant_sqlite_uris_abort():
    """Relative and complex SQLite URI variants targeting production DB must abort immediately."""
    uris = [
        "file:backend/sumi.db",
        "file:./backend/sumi.db",
        "file:backend/sumi.db?mode=ro",
        "file:./backend/sumi.db?mode=ro&cache=private",
        f"file:{PROD_DB_PATH.as_posix()}?mode=ro",
        f"file:{PROD_DB_PATH.as_posix()}?mode=rw",
    ]
    for uri in uris:
        with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*strictly prohibited"):
            sqlite3.connect(uri, uri=True)


def test_auxiliary_files_connect_and_mutations_abort():
    """Auxiliary files (.db-wal, .db-shm, .db-journal) must be blocked across sqlite3, open, and os.open."""
    for suffix in ["-wal", "-shm", "-journal"]:
        aux_path = str(PROD_DB_PATH) + suffix

        # sqlite3 connect aborts
        with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*strictly prohibited"):
            sqlite3.connect(aux_path)

        # builtins.open aborts
        with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*Mutating open"):
            open(aux_path, "w")

        # os.open aborts
        with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*Mutating os\.open"):
            os.open(aux_path, os.O_WRONLY | os.O_CREAT)


def test_c_extension_sqlite3_connect_aborts():
    """Direct invocation of _sqlite3.connect must be intercepted and abort."""
    import _sqlite3

    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*strictly prohibited"):
        _sqlite3.connect(str(PROD_DB_PATH))


def test_path_traversal_and_whitespace_evasion_aborts():
    """Traversals and trailing whitespace targeting production DB must abort."""
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*strictly prohibited"):
        sqlite3.connect("backend/../backend/sumi.db")

    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD.*strictly prohibited"):
        sqlite3.connect(f"{PROD_DB_PATH} ")


def test_read_only_sha256_inspection_is_permitted():
    """Read-only inspection via open(..., 'rb') for SHA-256 verification must succeed."""
    with open(PROD_DB_PATH, "rb") as f:
        data = f.read()
    digest = hashlib.sha256(data).hexdigest()
    is_valid, baseline_sha256, baseline_size = verify_production_database_integrity()
    assert is_valid is True
    assert digest == baseline_sha256
    assert len(data) == baseline_size


def test_verify_production_database_integrity_helper():
    """Integrity verification helper returns True and valid sha256/size."""
    is_valid, sha256, size = verify_production_database_integrity()
    assert is_valid is True
    assert isinstance(sha256, str) and len(sha256) == 64
    assert isinstance(size, int) and size > 0


def test_dynamic_baseline_capture_and_zero_delta_verification():
    """Dynamic baseline captures current state and verify_production_database_integrity asserts zero delta."""
    exists, sha256, size = capture_production_database_baseline()
    assert exists is True
    assert isinstance(sha256, str) and len(sha256) == 64
    assert isinstance(size, int) and size > 0

    # Matching baseline returns True
    is_valid, curr_sha, curr_size = verify_production_database_integrity(baseline=(exists, sha256, size))
    assert is_valid is True
    assert curr_sha == sha256
    assert curr_size == size

    # Tampered sha256 returns False
    is_valid, _, _ = verify_production_database_integrity(baseline=(True, "0" * 64, size))
    assert is_valid is False

    # Tampered size returns False
    is_valid, _, _ = verify_production_database_integrity(baseline=(True, sha256, size + 1024))
    assert is_valid is False

    # Unexpected existence returns False
    is_valid, _, _ = verify_production_database_integrity(baseline=(False, None, 0))
    assert is_valid is False


def test_session_baseline_is_immutable_against_repeated_capture():
    """Repeated calls to capture_production_database_baseline cannot overwrite initial session baseline."""
    assert is_session_baseline_locked() is True
    b1 = capture_production_database_baseline()
    assert b1[0] is True
    assert isinstance(b1[1], str) and len(b1[1]) == 64

    # Second call returns identical baseline
    b2 = capture_production_database_baseline()
    assert b1 == b2

    # Under mock simulating altered disk hash, capture_production_database_baseline still returns locked baseline
    with patch("tests_safety.compute_production_database_hash", return_value=("deadbeef" * 8, 999999)):
        b3 = capture_production_database_baseline()
        assert b3 == b1
        assert b3 != ("deadbeef" * 8, 999999)

    # Global session state was not overwritten
    assert get_session_database_baseline() == b1


def test_repeated_capture_cannot_hide_simulated_integrity_mismatch():
    """Repeated calls to capture_production_database_baseline cannot hide a simulated earlier integrity mismatch."""
    b_orig = capture_production_database_baseline()
    tampered_hash = "0123456789abcdef" * 4
    tampered_size = b_orig[2] + 4096

    # Simulate database modification on disk during test execution
    with patch("tests_safety.compute_production_database_hash", return_value=(tampered_hash, tampered_size)):
        # Attempt to overwrite baseline by re-calling capture_production_database_baseline
        b_after = capture_production_database_baseline()
        # Immutable baseline refused overwrite and returned initial baseline
        assert b_after == b_orig
        assert b_after[1] != tampered_hash

        # verify_production_database_integrity against session baseline detects mismatch
        is_valid, curr_sha, curr_size = verify_production_database_integrity()
        assert is_valid is False
        assert curr_sha == tampered_hash
        assert curr_size == tampered_size


def test_baseline_capture_on_isolated_state_does_not_mutate_session_baseline(tmp_path: Path):
    """Helper tests testing baseline capture operate on isolated state without mutating active session baseline."""
    b_session_before = get_session_database_baseline()

    # Create isolated database file
    isolated_db = tmp_path / "isolated_baseline_test.db"
    conn = sqlite3.connect(str(isolated_db))
    conn.execute("CREATE TABLE t (id INT)")
    conn.commit()
    conn.close()

    # Capture isolated baseline
    iso_exists, iso_sha, iso_size = capture_isolated_database_baseline(isolated_db)
    assert iso_exists is True
    assert isinstance(iso_sha, str) and len(iso_sha) == 64
    assert iso_size > 0

    # Session baseline was completely untouched
    b_session_after = get_session_database_baseline()
    assert b_session_before == b_session_after

    # Compute database hash works on isolated paths
    hash_calc, size_calc = compute_database_hash(isolated_db)
    assert hash_calc == iso_sha
    assert size_calc == iso_size


def test_deprecated_baseline_constants_available():
    """Deprecated PROD_DB_BASELINE_SHA256 and PROD_DB_BASELINE_SIZE constants remain for backward compatibility."""
    assert isinstance(PROD_DB_BASELINE_SHA256, str)
    assert len(PROD_DB_BASELINE_SHA256) == 64
    assert isinstance(PROD_DB_BASELINE_SIZE, int)
    assert PROD_DB_BASELINE_SIZE > 0


def test_safe_in_memory_sqlite_is_permitted():
    """In-memory SQLite connections (:memory:) must succeed without interference."""
    conn = sqlite3.connect(":memory:")
    cursor = conn.cursor()
    cursor.execute("SELECT 42")
    res = cursor.fetchone()[0]
    conn.close()
    assert res == 42

    engine = create_engine("sqlite:///:memory:")
    with engine.connect() as c:
        pass


def test_safe_isolated_temp_file_db_is_permitted(tmp_path: Path):
    """Temporary file SQLite connections in isolated directories must succeed without interference."""
    isolated_db_path = tmp_path / "isolated.db"
    conn = sqlite3.connect(str(isolated_db_path))
    cursor = conn.cursor()
    cursor.execute("CREATE TABLE test_table (id INTEGER PRIMARY KEY, val TEXT)")
    cursor.execute("INSERT INTO test_table (val) VALUES ('safe')")
    conn.commit()
    conn.close()

    engine = create_engine(f"sqlite:///{isolated_db_path.as_posix()}")
    with engine.connect() as c:
        pass
    assert isolated_db_path.exists()


def test_settings_database_url_string_inspection_is_permitted():
    """Inspecting settings.DATABASE_URL string without connecting must not trigger safeguard."""
    from app.config import settings

    url_str = settings.DATABASE_URL
    assert isinstance(url_str, str)
    assert "sqlite" in url_str


def test_pathlib_mutating_open_aborts():
    """Mutating Path.open calls ('w', 'a', 'r+', etc.) targeting production DB must abort immediately."""
    for mode in ["w", "wb", "r+", "a", "a+"]:
        with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
            Path(PROD_DB_PATH).open(mode)

        with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
            Path(PROD_DB_PATH).open(mode=mode)


def test_pathlib_write_bytes_and_write_text_abort():
    """Path.write_bytes and Path.write_text targeting production DB must abort immediately."""
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
        Path(PROD_DB_PATH).write_bytes(b"corrupted")

    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
        Path(PROD_DB_PATH).write_text("corrupted")


def test_pathlib_unlink_rename_replace_chmod_abort():
    """Path.unlink, rename, replace, chmod targeting production DB must abort immediately."""
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
        Path(PROD_DB_PATH).unlink()

    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
        Path(PROD_DB_PATH).rename("/tmp/sumi-path-renamed.db")

    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
        Path(PROD_DB_PATH).replace("/tmp/sumi-path-replaced.db")

    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
        Path(PROD_DB_PATH).chmod(0o644)


def test_io_open_mutating_modes_abort():
    """Direct io.open with mutating modes targeting production DB must abort immediately."""
    for mode in ["w", "wb", "r+", "a", "a+"]:
        with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
            io.open(PROD_DB_PATH, mode)

        with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
            io.open(str(PROD_DB_PATH), mode=mode)


def test_pathlib_and_io_read_only_is_permitted():
    """Read-only access via Path.open('rb'), Path.read_bytes(), and io.open('rb') is permitted for SHA-256 verification."""
    _, expected_sha, expected_size = get_session_database_baseline()
    with Path(PROD_DB_PATH).open("rb") as f:
        digest1 = hashlib.sha256(f.read()).hexdigest()
    assert digest1 == expected_sha

    raw_bytes = Path(PROD_DB_PATH).read_bytes()
    assert len(raw_bytes) == expected_size
    assert hashlib.sha256(raw_bytes).hexdigest() == expected_sha

    with io.open(PROD_DB_PATH, "rb") as f:
        digest2 = hashlib.sha256(f.read()).hexdigest()
    assert digest2 == expected_sha

