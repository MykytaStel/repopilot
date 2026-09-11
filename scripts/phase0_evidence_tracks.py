"""Evidence-track adapters for the Phase 0 report."""

from __future__ import annotations

from typing import Any

from differential_artifact import validate_artifact as validate_differential_artifact
from differential_contract import DifferentialManifestError
from differential_pilot_metrics import validate_pilot_metrics
from real_history_adjudication import validate_adjudication
from real_history_artifact import validate_collection_artifact
from real_history_contract import HoldoutManifestError
from real_history_metrics import validate_metrics

from phase0_evidence_model import (
    Phase0Paths,
    input_hashes,
    path_label,
    short_error,
    validate_differential_protocol,
    validate_real_history_protocol,
)


def real_history_track(paths: Phase0Paths) -> dict[str, Any]:
    track, protocol = validate_real_history_protocol(paths)
    if protocol is None:
        return track
    corpus, protocol_name, cases = protocol
    track.update(
        protocol={"corpus": corpus, "name": protocol_name, "cases": len(cases)},
        limitations=[
            "Six pinned cases are a corpus-only holdout, not a product-wide estimate.",
            "No precision, recall, or utility claim is allowed from unlabeled observations.",
        ],
    )
    if paths.real_history_artifact is None:
        return track
    return _real_history_observations(paths, track, paths.real_history_artifact)


def _real_history_observations(
    paths: Phase0Paths, track: dict[str, Any], artifact: Any
) -> dict[str, Any]:
    track["artifact"] = path_label(artifact, paths.root)
    track["hashes"] = input_hashes((("collection", artifact),))
    try:
        result = validate_collection_artifact(
            artifact, paths.manifest, paths.rules_reference, paths.zoo_manifest
        )
    except (HoldoutManifestError, OSError) as error:
        track.update(
            evidence_status="invalid",
            scope="none",
            reason=short_error("real-history collection artifact", error),
            next_action="replace the collection packet with one produced by the pinned collector",
        )
        return track
    track["observation"] = result
    track.update(
        evidence_status="observed-unlabeled",
        label_state="pending",
        scope="execution-observation-only",
        next_action="complete two independent blinded worksheets and one adjudication",
    )
    if _has_no_real_history_labels(paths):
        return track
    return _real_history_labels(paths, track, artifact)


def _has_no_real_history_labels(paths: Phase0Paths) -> bool:
    return not any((paths.annotation_a, paths.annotation_b, paths.adjudication, paths.real_history_metrics))


def _real_history_labels(
    paths: Phase0Paths, track: dict[str, Any], artifact: Any
) -> dict[str, Any]:
    labels = (paths.annotation_a, paths.annotation_b, paths.adjudication)
    if not all(labels):
        track.update(
            evidence_status="pending-labels",
            reason="annotation_a, annotation_b, and adjudication are all required",
            next_action="supply both independent worksheets and the adjudication packet",
        )
        return track
    track["hashes"] = input_hashes(
        (
            ("collection", artifact),
            ("annotation_a", paths.annotation_a),
            ("annotation_b", paths.annotation_b),
            ("adjudication", paths.adjudication),
            ("metrics", paths.real_history_metrics),
        )
    )
    try:
        validate_adjudication(
            paths.adjudication,
            paths.annotation_a,
            paths.annotation_b,
            artifact,
            paths.manifest,
            paths.rules_reference,
            paths.zoo_manifest,
        )
    except (HoldoutManifestError, OSError) as error:
        track.update(
            evidence_status="pending-labels",
            reason=short_error("real-history label packet", error),
            next_action="finish or repair the independent labels and adjudication",
        )
        return track
    if paths.real_history_metrics is None:
        track.update(
            evidence_status="pending-labels",
            reason="adjudication is present but the recomputed metrics artifact is missing",
            next_action="generate and validate the real-history metrics artifact",
        )
        return track
    return _real_history_metrics(paths, track, artifact)


def _real_history_metrics(
    paths: Phase0Paths, track: dict[str, Any], artifact: Any
) -> dict[str, Any]:
    try:
        metrics = validate_metrics(
            paths.real_history_metrics,
            paths.adjudication,
            paths.annotation_a,
            paths.annotation_b,
            artifact,
            paths.manifest,
            paths.rules_reference,
            paths.zoo_manifest,
        )
    except (HoldoutManifestError, OSError) as error:
        track.update(
            evidence_status="invalid",
            reason=short_error("real-history metrics", error),
            next_action="regenerate the metrics artifact from the validated packet",
        )
        return track
    track.update(
        evidence_status="valid",
        label_state="adjudicated",
        scope="independent-adjudication",
        metrics=metrics,
        next_action="review the bounded corpus-only result and retain its Wilson intervals",
    )
    return track


def differential_track(paths: Phase0Paths) -> dict[str, Any]:
    track, protocol = validate_differential_protocol(paths)
    if protocol is None:
        return track
    track.update(
        protocol={
            "corpus": protocol["corpus"],
            "name": protocol["protocol"],
            "repetitions": protocol["repetitions"],
        },
        limitations=[
            "Measurements are bound to the preregistered corpus, workload, and resource policy.",
            "Unavailable or mismatched evidence remains unavailable; it is not scored as a pass.",
        ],
    )
    if paths.differential_artifact is None:
        return track
    return _differential_observations(paths, track, paths.differential_artifact)


def _differential_observations(
    paths: Phase0Paths, track: dict[str, Any], artifact: Any
) -> dict[str, Any]:
    track["artifact"] = path_label(artifact, paths.root)
    track["hashes"] = input_hashes((("differential", artifact),))
    try:
        result = validate_differential_artifact(
            artifact,
            paths.manifest,
            paths.differential_manifest,
            paths.rules_reference,
            paths.zoo_manifest,
        )
    except (HoldoutManifestError, DifferentialManifestError, OSError) as error:
        track.update(
            evidence_status="invalid",
            scope="none",
            reason=short_error("differential artifact", error),
            next_action="replace the differential packet with one produced by the pinned runner",
        )
        return track
    track["observation"] = result
    track.update(
        evidence_status="observed-unlabeled",
        label_state="pending",
        scope="execution-observation-only",
        next_action="complete the blinded pilot worksheet and recomputed pilot metrics",
    )
    if paths.differential_pilot is None and paths.differential_metrics is None:
        return track
    if paths.differential_pilot is None or paths.differential_metrics is None:
        track.update(
            evidence_status="pending-labels",
            reason="the differential pilot worksheet and metrics artifact must be supplied together",
            next_action="supply both differential pilot inputs",
        )
        return track
    return _differential_metrics(paths, track, artifact)


def _differential_metrics(
    paths: Phase0Paths, track: dict[str, Any], artifact: Any
) -> dict[str, Any]:
    track["hashes"] = input_hashes(
        (("differential", artifact), ("pilot", paths.differential_pilot), ("metrics", paths.differential_metrics))
    )
    try:
        metrics = validate_pilot_metrics(
            paths.differential_metrics,
            artifact,
            paths.differential_pilot,
            paths.manifest,
            paths.differential_manifest,
            paths.rules_reference,
            paths.zoo_manifest,
        )
    except (HoldoutManifestError, DifferentialManifestError, OSError) as error:
        track.update(
            evidence_status="invalid",
            reason=short_error("differential pilot metrics", error),
            next_action="regenerate the metrics artifact from the validated pilot packet",
        )
        return track
    track.update(
        evidence_status="valid",
        label_state="single-expert",
        scope="single-expert-exploratory",
        metrics=metrics,
        limitations=track["limitations"]
        + ["A single-expert pilot is exploratory and does not establish independent utility."],
        next_action="treat the pilot as exploratory and complete the preregistered independent utility evidence",
    )
    return track
