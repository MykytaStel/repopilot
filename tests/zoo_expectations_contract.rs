//! Contract for the zoo reviewed-precision expectation layer.
//!
//! Drives the pure `scripts/zoo_expectations.py` parse + match + render helpers
//! with synthetic findings and in-memory expectation text. It is hermetic: no
//! network, no `.zoo/` clones, no scanner build — just `python3` importing the
//! module via `PYTHONPATH`. This keeps `cargo test --all` deterministic while
//! still exercising the matcher that the opt-in `zoo_regression` gate relies on.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn python_expectation_matcher_contract() {
    let root = repo_root();
    let scripts = root.join("scripts");
    let temp = tempfile::tempdir().expect("temp dir");
    let driver = temp.path().join("driver.py");
    std::fs::write(&driver, DRIVER).expect("write driver");

    let output = Command::new("python3")
        .arg(&driver)
        .env("PYTHONPATH", &scripts)
        .output()
        .expect("run python3 driver (is python3 on PATH?)");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "expectation driver failed.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("ALL OK"),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

const DRIVER: &str = r##"
import pathlib
import sys
import tempfile

import zoo_expectations as ze
import zoo_triage as zt

HEAD = 'schema_version=1\nrepo="repo"\ndefault_coverage="exhaustive"\nstrict_coverage="selective"\n'


def live(fid, rule, path, line=None, line_end=None, profile="default",
         sev="HIGH", conf="HIGH", prio="P0", title="t", snippet="s"):
    return ze.LiveFinding(profile, fid, rule, path, line, line_end, sev, conf, prio, title, snippet)


def parse(text, repo="repo"):
    return ze.parse_expectation_text(text, repo, "repo.toml")


def block(profile, fid, rule, path, disp, reason, line=None):
    out = (f'[[finding]]\nprofile="{profile}"\nfinding_id="{fid}"\nrule_id="{rule}"\n'
           f'path="{path}"\ndisposition="{disp}"\nreason="{reason}"\n')
    if line is not None:
        out += f"line={line}\n"
    return out


def kinds(diags):
    return sorted({d.kind for d in diags})


def check(cond, msg):
    if not cond:
        print("FAIL:", msg)
        sys.exit(1)


# 1. every default finding labeled exactly once passes
exp, d = parse(HEAD + block("default", "id1:p:h", "rule.a", "p/a.ts", "actionable", "r"))
check(d == [], "1 parse clean")
md, rev = ze.match_repo("repo", exp, [live("id1:p:h", "rule.a", "p/a.ts")], [])
check(md == [], "1 no diags: " + str(kinds(md)))
check(rev.actionable == 1 and rev.labeled == 1, "1 counts")

# 2. unlabeled default finding fails
exp, _ = parse(HEAD)
md, _ = ze.match_repo("repo", exp, [live("idX", "rule.a", "p/a.ts")], [])
check(ze.UNLABELED_DEFAULT_FINDING in kinds(md), "2 unlabeled")

# 3. stale default expectation fails
exp, _ = parse(HEAD + block("default", "idStale", "rule.a", "p/a.ts", "actionable", "r"))
md, _ = ze.match_repo("repo", exp, [], [])
check(ze.STALE_EXPECTATION in kinds(md), "3 stale")

# 4. duplicate expectation fails
body = block("default", "id1", "rule.a", "p/a.ts", "actionable", "r")
_, d = parse(HEAD + body + body)
check(ze.DUPLICATE_EXPECTATION in kinds(d), "4 duplicate")

# 5. invalid disposition fails
_, d = parse(HEAD + block("default", "id1", "rule.a", "p/a.ts", "REVIEW_REQUIRED", "r"))
check(ze.INVALID_EXPECTATION in kinds(d), "5 invalid disposition")

# 6. empty reason fails
_, d = parse(HEAD + block("default", "id1", "rule.a", "p/a.ts", "actionable", ""))
check(ze.INVALID_EXPECTATION in kinds(d), "6 empty reason")

# 7. rule/path mismatch fails even when the finding id matches
lives = [live("id1", "rule.a", "p/a.ts")]
exp, _ = parse(HEAD + block("default", "id1", "rule.WRONG", "p/a.ts", "actionable", "r"))
md, _ = ze.match_repo("repo", exp, lives, [])
ks = kinds(md)
check(ze.STALE_EXPECTATION in ks and ze.UNLABELED_DEFAULT_FINDING in ks, "7 rule mismatch " + str(ks))
exp, _ = parse(HEAD + block("default", "id1", "rule.a", "p/WRONG.ts", "actionable", "r"))
md, _ = ze.match_repo("repo", exp, lives, [])
ks = kinds(md)
check(ze.STALE_EXPECTATION in ks and ze.UNLABELED_DEFAULT_FINDING in ks, "7 path mismatch " + str(ks))

# 8. stable-id collision handled without merging distinct live findings
coll = [live("idC", "rule.a", "p/a.ts", line=10), live("idC", "rule.a", "p/a.ts", line=20)]
exp, d = parse(HEAD
               + block("default", "idC", "rule.a", "p/a.ts", "actionable", "first", line=10)
               + block("default", "idC", "rule.a", "p/a.ts", "valid-but-accepted", "second", line=20))
check(d == [], "8a parse")
md, rev = ze.match_repo("repo", exp, coll, [])
check(md == [], "8a clean: " + str(kinds(md)))
check(rev.labeled == 2, "8a both matched individually")
# without line, the two identical entries collapse (duplicate) and the survivor is ambiguous
exp, d = parse(HEAD
               + block("default", "idC", "rule.a", "p/a.ts", "actionable", "first")
               + block("default", "idC", "rule.a", "p/a.ts", "valid-but-accepted", "second"))
check(ze.DUPLICATE_EXPECTATION in kinds(d), "8b duplicate")
md, _ = ze.match_repo("repo", exp, coll, [])
check(ze.AMBIGUOUS_EXPECTATION in kinds(md), "8b ambiguous " + str(kinds(md)))

# 9. selective strict anchor passes while unrelated strict findings stay unlabeled
exp, _ = parse(HEAD + block("strict", "idS", "rule.s", "p/s.ts", "valid-but-accepted", "anchor"))
strict_lives = [live("idS", "rule.s", "p/s.ts", profile="strict"),
                live("idOther", "rule.s", "p/o.ts", profile="strict")]
md, rev = ze.match_repo("repo", exp, [], strict_lives)
check(md == [], "9 clean: " + str(kinds(md)))
check(rev.strict_anchors == 1, "9 anchors")

# 10. missing strict anchor fails
exp, _ = parse(HEAD + block("strict", "idGone", "rule.s", "p/s.ts", "actionable", "anchor"))
md, _ = ze.match_repo("repo", exp, [], [])
check(ze.MISSING_STRICT_RECALL_ANCHOR in kinds(md), "10 missing anchor")

# 11. false-positive is surfaced but never suppresses the finding
exp, _ = parse(HEAD + block("default", "idFP", "rule.a", "p/a.ts", "false-positive", "calibration debt"))
md, rev = ze.match_repo("repo", exp, [live("idFP", "rule.a", "p/a.ts")], [])
ks = kinds(md)
check(ze.KNOWN_FALSE_POSITIVE in ks, "11 surfaced")
check(not ze.is_failure(ze.KNOWN_FALSE_POSITIVE), "11 fp is not a failure")
check(all(not ze.is_failure(k) for k in ks), "11 no failing kinds " + str(ks))
check(rev.false_positive == 1 and rev.labeled == 1, "11 still labeled, not suppressed")

# 12. triage output carries evidence + a non-valid placeholder disposition
lf = live("idT", "rule.a", "p/a.ts", line=42, snippet="boom()", title="Title")
out = zt.render_triage_entry("repo", lf)
for needle in ["idT", "rule.a", "p/a.ts:42", "boom()", "Title", "REVIEW_REQUIRED", "P0", "HIGH"]:
    check(needle in out, "12 missing " + needle)
_, d = parse(HEAD + block("default", "idT", "rule.a", "p/a.ts", "REVIEW_REQUIRED", "x"))
check(ze.INVALID_EXPECTATION in kinds(d), "12 review_required rejected")
exp, _ = parse(HEAD)
check(len(ze.unlabeled_default(exp, [lf])) == 1, "12 unlabeled helper")

# 13. expectation diagnostics are a separate channel from snapshot drift
exp, _ = parse(HEAD + block("default", "id1", "rule.a", "p/a.ts", "actionable", "r"))
ok, _ = ze.match_repo("repo", exp, [live("id1", "rule.a", "p/a.ts")], [])
bad, _ = ze.match_repo("repo", exp, [live("idX", "rule.a", "p/a.ts")], [])
check(ze.SNAPSHOT_DRIFT not in {x.kind for x in ok + bad}, "13 matcher never emits drift")

# 14. empty-default repo still requires a valid expectation file
exp, d = parse(HEAD)
check(exp is not None and d == [] and exp.findings == (), "14 empty valid")
miss = pathlib.Path(tempfile.gettempdir()) / "zoo-expectations-missing-xyz.toml"
if miss.exists():
    miss.unlink()
_, d = ze.parse_expectation_file(miss, "repo")
check(ze.MISSING_EXPECTATION_FILE in kinds(d), "14 missing file")

# 15. manifest repo without an expectation file fails
d = ze.coverage_diagnostics(["a", "b"], ["a", "b"], ["a"])
check(ze.MISSING_EXPECTATION_FILE in kinds(d) and any(x.repo == "b" for x in d), "15 missing manifest file")

# 16. extra expectation file for an unknown repo fails
d = ze.coverage_diagnostics(["a"], ["a"], ["a", "ghost"])
check(ze.UNKNOWN_EXPECTATION_FILE in kinds(d) and any(x.repo == "ghost" for x in d), "16 unknown file")

# header validation: schema, repo mismatch, unknown profile, unknown field
_, d = parse('schema_version=2\nrepo="repo"\n')
check(ze.INVALID_EXPECTATION in kinds(d), "schema version")
_, d = parse('schema_version=1\nrepo="other"\n', repo="repo")
check(ze.INVALID_EXPECTATION in kinds(d), "repo mismatch")
_, d = parse(HEAD + '[[finding]]\nprofile="weird"\nfinding_id="i"\nrule_id="r"\npath="p"\ndisposition="actionable"\nreason="x"\n')
check(ze.INVALID_EXPECTATION in kinds(d), "unknown profile")
_, d = parse(HEAD + '[[finding]]\nprofile="default"\nfinding_id="i"\nrule_id="r"\npath="p"\ndisposition="actionable"\nreason="x"\nbogus="y"\n')
check(ze.INVALID_EXPECTATION in kinds(d), "unknown field")

print("ALL OK")
"##;
