from datetime import date, datetime, time, timedelta, timezone


def normalize_datetime(dt: datetime | date | str | None) -> datetime | None:
    if dt is None:
        return None
    if isinstance(dt, str):
        dt = datetime.fromisoformat(dt)
    if isinstance(dt, date) and not isinstance(dt, datetime):
        dt = datetime.combine(dt, time.min)
    if isinstance(dt, datetime) and dt.tzinfo is not None:
        return dt.astimezone(timezone.utc).replace(tzinfo=None)
    return dt


def start_at(value) -> datetime:
    if isinstance(value, str):
        parsed = datetime.fromisoformat(value)
        dt = parsed if ("T" in value or " " in value) else datetime.combine(parsed.date(), time.min)
    elif isinstance(value, datetime):
        dt = value
    elif isinstance(value, date):
        dt = datetime.combine(value, time.min)
    else:
        dt = value
    if isinstance(dt, datetime) and dt.tzinfo is not None:
        dt = dt.astimezone(timezone.utc).replace(tzinfo=None)
    return dt


def end_before(value) -> datetime:
    if isinstance(value, str):
        parsed = datetime.fromisoformat(value)
        dt = parsed if ("T" in value or " " in value) else datetime.combine(parsed.date() + timedelta(days=1), time.min)
    elif isinstance(value, datetime):
        dt = value
    elif isinstance(value, date):
        dt = datetime.combine(value + timedelta(days=1), time.min)
    else:
        dt = value
    if isinstance(dt, datetime) and dt.tzinfo is not None:
        dt = dt.astimezone(timezone.utc).replace(tzinfo=None)
    return dt
