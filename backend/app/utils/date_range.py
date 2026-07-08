from datetime import date, datetime, time, timedelta


def start_at(value) -> datetime:
    if isinstance(value, datetime):
        return value
    if isinstance(value, date):
        return datetime.combine(value, time.min)
    if isinstance(value, str):
        parsed = datetime.fromisoformat(value)
        return parsed if "T" in value or " " in value else datetime.combine(parsed.date(), time.min)
    return value


def end_before(value) -> datetime:
    if isinstance(value, datetime):
        return value
    if isinstance(value, date):
        return datetime.combine(value + timedelta(days=1), time.min)
    if isinstance(value, str):
        parsed = datetime.fromisoformat(value)
        return parsed if "T" in value or " " in value else datetime.combine(parsed.date() + timedelta(days=1), time.min)
    return value
