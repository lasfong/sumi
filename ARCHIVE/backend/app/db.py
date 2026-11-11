import functools
import sqlite3
from contextlib import contextmanager
from sqlalchemy import create_engine, event, Engine
from sqlalchemy.orm import sessionmaker, declarative_base, Session, with_loader_criteria
from app.config import settings


@event.listens_for(Engine, "connect")
def _set_sqlite_pragma(dbapi_connection, connection_record):
    """Ensure SQLite foreign key constraints are enforced for every DBAPI connection."""
    if isinstance(dbapi_connection, sqlite3.Connection):
        cursor = dbapi_connection.cursor()
        cursor.execute("PRAGMA foreign_keys=ON")
        cursor.close()


_candle_cls = None


def _get_candle_cls():
    global _candle_cls
    if _candle_cls is None:
        from app.models.candle import Candle
        _candle_cls = Candle
    return _candle_cls


@event.listens_for(Session, "do_orm_execute")
def _filter_quarantine_candles(execute_state):
    """Quarantine isolation boundary (ADR-PROV-001).

    By default, any SELECT query reading Candle entities excludes quarantine rows:
        source LIKE 'quarantine:%'
    unless explicitly opted-in via execution_options(include_quarantine=True).
    """
    if (
        execute_state.is_select
        and not execute_state.execution_options.get("include_quarantine", False)
    ):
        Candle = _get_candle_cls()
        execute_state.statement = execute_state.statement.options(
            with_loader_criteria(
                Candle,
                lambda cls: (cls.source.is_(None)) | (~cls.source.like("quarantine:%")),
                include_aliases=True
            )
        )


@contextmanager
def _atomic_context(session):
    try:
        yield session
        session.commit()
    except Exception:
        session.rollback()
        raise


def atomic(target):
    """Transactional boundary helper.
    
    Can be used as a context manager:
        with atomic(db):
            ...
            
    Or as a function decorator:
        @atomic
        def my_write_operation(db: Session, ...):
            ...
    """
    if hasattr(target, "commit") and hasattr(target, "rollback"):
        return _atomic_context(target)

    func = target

    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        session = kwargs.get("db") or kwargs.get("session")
        if session is None:
            for arg in args:
                if hasattr(arg, "commit") and hasattr(arg, "rollback"):
                    session = arg
                    break
        if session is not None:
            with _atomic_context(session):
                return func(*args, **kwargs)
        return func(*args, **kwargs)

    return wrapper


engine = create_engine(
    settings.DATABASE_URL
)
SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)

Base = declarative_base()

