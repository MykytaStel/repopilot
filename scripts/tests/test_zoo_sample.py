from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

import zoo_sample as zsam  # noqa: E402  (after the sys.path fix above)
import zoo_triage as zt  # noqa: E402
from zoo_expectations import LiveFinding  # noqa: E402


def live(rule_id: str, path: str, line: int, finding_id: str | None = None) -> LiveFinding:
    return LiveFinding(
        profile="strict",
        id=finding_id or f"{rule_id}:{path}:{line:04d}",
        rule_id=rule_id,
        path=path,
        line=line,
        line_end=line,
        severity="LOW",
        confidence="HIGH",
        priority="P3",
        title="title",
        snippet="snippet",
    )


class EvenStrideTest(unittest.TestCase):
    def test_spans_the_population_instead_of_taking_a_prefix(self):
        # Catches a sampler that returns the alphabetically first N findings,
        # which would let one crowded directory stand in for the whole zoo.
        self.assertEqual(zsam.even_stride(100, 5), [0, 25, 50, 74, 99])
        self.assertEqual(zsam.even_stride(10, 3), [0, 4, 9])

    def test_returns_every_index_when_the_population_fits(self):
        self.assertEqual(zsam.even_stride(3, 20), [0, 1, 2])
        self.assertEqual(zsam.even_stride(4, 4), [0, 1, 2, 3])

    def test_degenerate_inputs_are_empty_or_single(self):
        self.assertEqual(zsam.even_stride(0, 5), [])
        self.assertEqual(zsam.even_stride(5, 0), [])
        self.assertEqual(zsam.even_stride(-1, 5), [])
        self.assertEqual(zsam.even_stride(9, 1), [0])

    def test_never_returns_a_duplicate_index(self):
        # Rounding two strides onto the same index must not offer the same
        # finding twice for labeling.
        for population in range(1, 60):
            for limit in range(1, 60):
                indices = zsam.even_stride(population, limit)
                self.assertEqual(len(indices), len(set(indices)), (population, limit))
                self.assertTrue(all(0 <= i < population for i in indices), (population, limit))


class BuildSampleTest(unittest.TestCase):
    def setUp(self):
        self.live_by_repo = {
            "wagtail": [live("architecture.dead-module", f"w/{i}.py", i) for i in range(4)],
            "excalidraw": [
                live("architecture.dead-module", "e/a.ts", 1),
                live("code-quality.long-function", "e/b.ts", 2),
            ],
        }

    def test_only_the_requested_rule_is_sampled(self):
        sample = zsam.build_sample(
            "architecture.dead-module", "strict", self.live_by_repo, {}, limit=20
        )
        self.assertEqual(sample.population, 5)
        self.assertTrue(all(c.finding.rule_id == "architecture.dead-module" for c in sample.selected))
        self.assertEqual(sample.repos_with_findings, ("excalidraw", "wagtail"))

    def test_ordering_is_stable_across_dict_iteration_order(self):
        # Catches a sample that depends on repo insertion order, which would
        # make the labeled evidence unreproducible on another machine.
        reversed_input = dict(reversed(list(self.live_by_repo.items())))
        first = zsam.build_sample("architecture.dead-module", "strict", self.live_by_repo, {}, limit=3)
        second = zsam.build_sample("architecture.dead-module", "strict", reversed_input, {}, limit=3)
        self.assertEqual(
            [c.finding.id for c in first.selected],
            [c.finding.id for c in second.selected],
        )

    def test_already_labeled_findings_are_excluded_and_counted(self):
        # Re-running after a labeling pass must deepen coverage, not re-offer
        # the same entries.
        labeled = {"wagtail": {"architecture.dead-module:w/0.py:0000"}}
        sample = zsam.build_sample(
            "architecture.dead-module", "strict", self.live_by_repo, labeled, limit=20
        )
        self.assertEqual(sample.population, 4)
        self.assertEqual(sample.already_labeled, 1)
        self.assertNotIn(
            "architecture.dead-module:w/0.py:0000", [c.finding.id for c in sample.selected]
        )

    def test_exhaustive_only_when_every_candidate_is_selected(self):
        whole = zsam.build_sample("architecture.dead-module", "strict", self.live_by_repo, {}, limit=20)
        self.assertTrue(whole.is_exhaustive)
        subset = zsam.build_sample("architecture.dead-module", "strict", self.live_by_repo, {}, limit=2)
        self.assertFalse(subset.is_exhaustive)

    def test_unknown_rule_yields_an_empty_sample(self):
        sample = zsam.build_sample("architecture.nope", "strict", self.live_by_repo, {}, limit=20)
        self.assertEqual(sample.population, 0)
        self.assertEqual(sample.selected, ())
        self.assertIn("no strict-profile findings", zsam.render_frame(sample))


class RenderFrameTest(unittest.TestCase):
    def test_a_subset_is_labeled_as_a_sample(self):
        # Catches a frame that lets a 20-of-1762 subset read as measured
        # coverage of the rule.
        live_by_repo = {"wagtail": [live("architecture.dead-module", f"w/{i}.py", i) for i in range(50)]}
        frame = zsam.render_frame(
            zsam.build_sample("architecture.dead-module", "strict", live_by_repo, {}, limit=5)
        )
        self.assertIn("SAMPLE, not exhaustive", frame)
        self.assertIn("population:  50 unlabeled", frame)

    def test_full_coverage_is_not_called_a_sample(self):
        live_by_repo = {"wagtail": [live("architecture.dead-module", "w/0.py", 0)]}
        frame = zsam.render_frame(
            zsam.build_sample("architecture.dead-module", "strict", live_by_repo, {}, limit=5)
        )
        self.assertIn("exhaustive for the unlabeled population", frame)


class SkeletonEvidenceTest(unittest.TestCase):
    def test_sampler_skeletons_declare_the_sample_evidence_kind(self):
        # Catches emitting a skeleton a maintainer would paste as an anchor,
        # which would silently keep the rule out of the sampled scorecard.
        skeleton = zt.triage_skeleton(live("architecture.dead-module", "a.py", 3), evidence="sample")
        self.assertIn('evidence = "sample"', skeleton)

    def test_triage_skeletons_stay_unmarked(self):
        skeleton = zt.triage_skeleton(live("architecture.dead-module", "a.py", 3))
        self.assertNotIn("evidence", skeleton)


if __name__ == "__main__":
    unittest.main()
