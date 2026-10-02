"""Service settings read from a JSON file."""

import json
from typing import Any


def load(path: str) -> dict[str, Any]:
    with open(path) as handle:
        data = json.load(handle)
    if not isinstance(data, dict):
        raise ValueError("settings must be a JSON object")
    return data


def port(settings: dict[str, Any]) -> int:
    """The service port; 8080 when the file does not set one."""
    value = settings.get("port", 8080)
    if not isinstance(value, int):
        raise ValueError("port must be an integer")
    return value


def hosts(settings: dict[str, Any]) -> list[str]:
    """Allowed hosts; none when the file does not list any."""
    value = settings.get("hosts", [])
    if not isinstance(value, list) or not all(isinstance(host, str) for host in value):
        raise ValueError("hosts must be a list of strings")
    return value
