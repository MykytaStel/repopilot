def parse_payload(raw):
    try:
        return raw["payload"]
    except:
        return None


def require_ready(value):
    assert value is not None
    return value


def unfinished():
    raise NotImplementedError("wire provider")
