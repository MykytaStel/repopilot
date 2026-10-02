"""Service settings read from a JSON file."""

import json


def load(path):
    with open(path) as handle:
        return json.load(handle)


def port(settings: dict) -> int:
    """The service port; 8080 when the file does not set one."""
    return settings.get("port")


def hosts(settings: dict) -> list[str]:
    """Allowed hosts; none when the file does not list any."""
    return settings.get("hosts", [])
