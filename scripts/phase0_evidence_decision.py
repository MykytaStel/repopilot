"""Decision and coverage projection for the Phase 0 evidence audit."""

from __future__ import annotations

from typing import Any


def _coverage(tracks: list[dict[str, Any]]) -> dict[str, int]:
    return {
        "tracks_total": len(tracks),
        "protocols_valid": sum(track["protocol_status"] == "valid" for track in tracks),
        "protocols_invalid": sum(track["protocol_status"] != "valid" for track in tracks),
        "evidence_valid": sum(track["evidence_status"] == "valid" for track in tracks),
        "evidence_pending": sum(
            track["evidence_status"] in {"artifact-missing", "observed-unlabeled", "pending-labels"}
            for track in tracks
        ),
        "evidence_invalid": sum(track["evidence_status"] in {"invalid", "protocol-invalid"} for track in tracks),
        "evidence_limited": sum(
            track["evidence_status"] == "valid" and track.get("scope") != "independent-adjudication"
            for track in tracks
        ),
    }


def _is_verified(track: dict[str, Any]) -> bool:
    return (
        track["protocol_status"] == "valid"
        and track["evidence_status"] == "valid"
        and track.get("scope") == "independent-adjudication"
    )


def _blocking_reason(track: dict[str, Any]) -> str:
    if track.get("reason"):
        return str(track["reason"])
    status = track["evidence_status"]
    if status == "artifact-missing":
        return "validated evidence packet is missing"
    if status == "observed-unlabeled":
        return "observations exist but are not independently labeled"
    if status == "pending-labels":
        return "required labels or metrics are pending"
    if status == "protocol-invalid":
        return "protocol inputs are invalid"
    if status == "invalid":
        return "evidence artifact is invalid"
    if status == "valid" and track.get("scope") != "independent-adjudication":
        return f"scope is {track.get('scope', 'unknown')}; independent adjudication is required"
    return "evidence does not meet the independent verification contract"


def decision_summary(tracks: list[dict[str, Any]]) -> dict[str, Any]:
    """Return the stable human and machine decision projection for tracks."""
    coverage = _coverage(tracks)
    if coverage["protocols_invalid"] or coverage["evidence_invalid"]:
        decision = "invalid"
    elif tracks and all(_is_verified(track) for track in tracks):
        decision = "verified"
    else:
        decision = "blocked"
    blocking_reasons = [
        {
            "track": track["id"],
            "status": track["evidence_status"],
            "reason": _blocking_reason(track),
        }
        for track in tracks
        if not _is_verified(track)
    ]
    next_actions = [
        {
            "track": track["id"],
            "action": track["next_action"],
            "blocking": not _is_verified(track),
        }
        for track in tracks
    ]
    return {
        "decision": decision,
        "coverage": coverage,
        "blocking_reasons": blocking_reasons,
        "next_actions": next_actions,
    }


def coverage_line(coverage: dict[str, int]) -> str:
    return (
        "Coverage: "
        f"{coverage['tracks_total']} tracks; "
        f"protocols {coverage['protocols_valid']}/{coverage['tracks_total']} valid; "
        f"evidence valid={coverage['evidence_valid']}, "
        f"pending={coverage['evidence_pending']}, "
        f"invalid={coverage['evidence_invalid']}, "
        f"limited={coverage['evidence_limited']}"
    )


def action_lines(actions: list[dict[str, Any]]) -> list[str]:
    return [
        f"- {item['track']}: {item['action']}" + (" [blocking]" if item["blocking"] else "")
        for item in actions
    ]
