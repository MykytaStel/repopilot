def parse_payload(raw):
    try:
        return raw["payload"]
    except KeyError:
        return None


def require_ready(value):
    if value is None:
        raise ValueError("value is required")
    return value
