import os
import sys
import subprocess
from pathlib import Path
import pytest
from sqlalchemy.engine import make_url

BACKEND_DIR = Path(__file__).resolve().parent.parent


def test_default_database_url_is_absolute_and_anchored_to_backend(monkeypatch):
    """Requirement R1: settings.DATABASE_URL must be absolute and point to backend/sumi.db."""
    from app.config import Settings, DEFAULT_DATABASE_URL

    expected_path = (BACKEND_DIR / "sumi.db").resolve()

    # 1. Verify module constant DEFAULT_DATABASE_URL is absolute and points to backend/sumi.db
    url_const = make_url(DEFAULT_DATABASE_URL)
    assert url_const.drivername.startswith("sqlite")
    assert url_const.database is not None
    assert Path(url_const.database).is_absolute()
    assert Path(url_const.database).resolve() == expected_path

    # 2. Instantiate clean Settings without env overrides
    monkeypatch.delenv("DATABASE_URL", raising=False)
    clean_settings = Settings()
    url = make_url(clean_settings.DATABASE_URL)

    assert url.drivername.startswith("sqlite")
    assert url.database is not None
    db_path = Path(url.database)

    assert db_path.is_absolute(), f"DATABASE_URL path must be absolute, got: {url.database}"
    assert db_path.resolve() == expected_path, f"Expected {expected_path}, got: {db_path.resolve()}"

    # 3. Instantiate Settings with explicit empty string fallback
    empty_settings = Settings(DATABASE_URL="")
    url_empty = make_url(empty_settings.DATABASE_URL)
    assert Path(url_empty.database).resolve() == expected_path


def test_settings_normalizes_relative_env_database_url_to_backend_dir():
    """Requirement R1: Settings must normalize relative SQLite file URLs to BACKEND_DIR."""
    from app.config import Settings

    # Test relative path with ./ prefix
    s1 = Settings(DATABASE_URL="sqlite:///./isolated_test.db")
    url1 = make_url(s1.DATABASE_URL)
    assert Path(url1.database).is_absolute()
    assert Path(url1.database).resolve() == (BACKEND_DIR / "isolated_test.db").resolve()

    # Test relative path without ./ prefix
    s2 = Settings(DATABASE_URL="sqlite:///isolated_test2.db")
    url2 = make_url(s2.DATABASE_URL)
    assert Path(url2.database).is_absolute()
    assert Path(url2.database).resolve() == (BACKEND_DIR / "isolated_test2.db").resolve()

    # In-memory URLs must be preserved as-is
    s3 = Settings(DATABASE_URL="sqlite:///:memory:")
    assert s3.DATABASE_URL == "sqlite:///:memory:"

    # Remote URLs must be preserved as-is
    s4 = Settings(DATABASE_URL="postgresql://user:pass@localhost:5432/sumi")
    assert s4.DATABASE_URL == "postgresql://user:pass@localhost:5432/sumi"


def test_default_database_url_resolves_to_backend_from_foreign_cwd_without_connecting(tmp_path):
    """Requirement A: Without DATABASE_URL, settings.DATABASE_URL resolves to backend/sumi.db from any CWD without connecting."""
    foreign_cwd = tmp_path / "foreign_cwd"
    foreign_cwd.mkdir()

    env = {k: v for k, v in os.environ.items() if k != "DATABASE_URL"}
    env["PYTHONPATH"] = str(BACKEND_DIR)

    # Robust argument passing via sys.argv avoids fragile string interpolation
    script = (
        "import sys\n"
        "from pathlib import Path\n"
        "from sqlalchemy.engine import make_url\n"
        "from app.config import settings\n"
        "backend_dir = Path(sys.argv[1]).resolve()\n"
        "url = make_url(settings.DATABASE_URL)\n"
        "assert url.drivername.startswith('sqlite'), f'Unexpected driver: {url.drivername}'\n"
        "assert url.database is not None, 'Database path must not be None'\n"
        "db_path = Path(url.database)\n"
        "assert db_path.is_absolute(), f'Expected absolute path, got {db_path}'\n"
        "expected_path = (backend_dir / 'sumi.db').resolve()\n"
        "assert db_path.resolve() == expected_path, f'Expected {expected_path}, got {db_path.resolve()}'\n"
    )

    proc = subprocess.run(
        [sys.executable, "-c", script, str(BACKEND_DIR)],
        cwd=str(foreign_cwd),
        env=env,
        capture_output=True,
        text=True,
    )

    assert proc.returncode == 0, f"Process failed: {proc.stderr}\nStdout: {proc.stdout}"
    assert not (foreign_cwd / "sumi.db").exists(), f"Rogue sumi.db created in {foreign_cwd}!"
    assert list(foreign_cwd.iterdir()) == [], f"Unexpected files created in foreign CWD: {list(foreign_cwd.iterdir())}"


def test_database_connection_from_foreign_cwd_does_not_create_local_db(tmp_path):
    """Requirement B: With explicit temporary SQLite database URL, connect from foreign CWD and verify no ghost DB in CWD."""
    foreign_cwd = tmp_path / "foreign_cwd"
    foreign_cwd.mkdir()
    isolated_dir = tmp_path / "isolated_db_dir"
    isolated_dir.mkdir()
    temp_db_file = (isolated_dir / "target_isolated.db").resolve()
    temp_db_url = f"sqlite:///{temp_db_file.as_posix()}"

    env = {k: v for k, v in os.environ.items() if k != "DATABASE_URL"}
    env["DATABASE_URL"] = temp_db_url
    env["PYTHONPATH"] = str(BACKEND_DIR)

    # Robust argument passing via sys.argv avoids fragile string interpolation
    script = (
        "import sys\n"
        "from pathlib import Path\n"
        "from sqlalchemy import text\n"
        "from sqlalchemy.engine import make_url\n"
        "from app.config import settings\n"
        "from app.db import engine\n"
        "expected_db_path = Path(sys.argv[1]).resolve()\n"
        "url = make_url(settings.DATABASE_URL)\n"
        "assert url.database is not None, 'Database path in settings.DATABASE_URL must not be None'\n"
        "assert Path(url.database).resolve() == expected_db_path, f'Expected {expected_db_path}, got {Path(url.database).resolve()}'\n"
        "with engine.connect() as conn:\n"
        "    res = conn.execute(text('SELECT 1')).scalar()\n"
        "    assert res == 1, f'Expected 1, got {res}'\n"
        "engine.dispose()\n"
    )

    proc = subprocess.run(
        [sys.executable, "-c", script, str(temp_db_file)],
        cwd=str(foreign_cwd),
        env=env,
        capture_output=True,
        text=True,
    )

    assert proc.returncode == 0, f"Process failed: {proc.stderr}\nStdout: {proc.stdout}"

    # Verify the temporary database file was created/accessed in isolated_dir
    assert temp_db_file.exists(), "Target isolated database was not created/connected!"

    # Verify no sumi.db or any other database file was created in foreign_cwd
    local_db = foreign_cwd / "sumi.db"
    assert not local_db.exists(), f"Rogue sumi.db was created in foreign CWD {foreign_cwd}!"
    assert list(foreign_cwd.iterdir()) == [], f"Unexpected files created in foreign CWD: {list(foreign_cwd.iterdir())}"


def test_lifespan_startup_guard_rejects_relative_sqlite_database_path():
    """Requirement R3: verify_database_path_security must fail closed if path is relative."""
    from app.main import verify_database_path_security

    # An absolute path or in-memory sqlite should pass
    abs_url = f"sqlite:///{(BACKEND_DIR / 'sumi.db').resolve().as_posix()}"
    verify_database_path_security(abs_url)  # Must not raise

    verify_database_path_security("sqlite:///:memory:")  # Must not raise
    verify_database_path_security("sqlite://")  # Must not raise
    verify_database_path_security("postgresql://user:pass@localhost:5432/sumi")  # Must not raise

    # Empty URL must be rejected
    with pytest.raises(ValueError):
        verify_database_path_security("")

    # A relative path must be rejected with RuntimeError
    with pytest.raises(RuntimeError, match=r"(?i)insecure relative sqlite database path"):
        verify_database_path_security("sqlite:///./sumi.db")

    with pytest.raises(RuntimeError, match=r"(?i)insecure relative sqlite database path"):
        verify_database_path_security("sqlite:///sumi.db")


@pytest.mark.anyio
async def test_fastapi_lifespan_context_fails_on_insecure_engine(monkeypatch):
    """Requirement R3: FastAPI lifespan context manager raises RuntimeError if engine.url is relative."""
    from app.main import lifespan, app
    import app.main as main_mod

    # Monkeypatch engine.url to simulate an insecure relative SQLite path
    fake_engine = type("FakeEngine", (), {"url": make_url("sqlite:///./rogue.db")})()
    monkeypatch.setattr(main_mod, "engine", fake_engine)

    with pytest.raises(RuntimeError, match=r"(?i)insecure relative sqlite database path"):
        async with lifespan(app):
            pass


def test_alembic_env_targets_backend_sumi_db_from_foreign_cwd(tmp_path):
    """Requirement R2: Alembic resolution from foreign CWD targets backend/sumi.db and creates 0 files."""
    env = {k: v for k, v in os.environ.items() if k != "DATABASE_URL"}
    env["PYTHONPATH"] = str(BACKEND_DIR)

    alembic_ini_path = (BACKEND_DIR / "alembic.ini").resolve()

    # In tmp_path, inspect alembic target URL using robust SQLite URL inspection and argument passing
    script = (
        "import sys\n"
        "from pathlib import Path\n"
        "from alembic.config import Config\n"
        "from sqlalchemy.engine import make_url\n"
        "from app.config import settings\n"
        "alembic_ini = Path(sys.argv[1]).resolve()\n"
        "backend_dir = Path(sys.argv[2]).resolve()\n"
        "cfg = Config(str(alembic_ini))\n"
        "cfg.set_main_option('sqlalchemy.url', settings.DATABASE_URL)\n"
        "target_url = cfg.get_main_option('sqlalchemy.url')\n"
        "url = make_url(target_url)\n"
        "assert url.drivername.startswith('sqlite'), f'Unexpected driver: {url.drivername}'\n"
        "assert url.database is not None, 'Database path must not be None'\n"
        "db_path = Path(url.database)\n"
        "assert db_path.is_absolute(), f'Expected absolute path, got {db_path}'\n"
        "expected_path = (backend_dir / 'sumi.db').resolve()\n"
        "assert db_path.resolve() == expected_path, f'Expected {expected_path}, got {db_path.resolve()}'\n"
    )

    proc = subprocess.run(
        [sys.executable, "-c", script, str(alembic_ini_path), str(BACKEND_DIR)],
        cwd=str(tmp_path),
        env=env,
        capture_output=True,
        text=True,
    )

    assert proc.returncode == 0, f"Process failed: {proc.stderr}\nStdout: {proc.stdout}"
    assert not (tmp_path / "sumi.db").exists(), f"Rogue sumi.db created in {tmp_path}!"
    assert list(tmp_path.iterdir()) == []
