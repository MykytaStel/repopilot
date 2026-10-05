//! Whole test files whose tests all moved to other changed files.
//!
//! Deleting or emptying a test file is reported by the behavioral
//! `test-deleted-or-emptied` signal, which sees one file at a time. When every
//! test of that file appears, by qualified name, in another file of the same
//! change, the change moved the tests and removed none.

use super::FileEvidence;
use super::accounting::difference;
use std::collections::{BTreeMap, BTreeSet};

/// Paths of deleted or emptied test files whose every test arrived elsewhere.
/// A file with even one test that went nowhere stays a deletion.
pub fn moved_test_files(files: &[FileEvidence]) -> BTreeSet<String> {
    let mut arrived: BTreeMap<&str, usize> = BTreeMap::new();
    for file in files {
        for (name, count) in difference(&file.post.tests, &file.pre.tests) {
            *arrived.entry(name).or_default() += count;
        }
    }
    let mut moved = BTreeSet::new();
    for file in files {
        if !file.existed_before || !file.removal_reported_elsewhere || file.pre.tests.is_empty() {
            continue;
        }
        let left = difference(&file.pre.tests, &file.post.tests);
        let all_arrived = left
            .iter()
            .all(|(name, count)| arrived.get(name).is_some_and(|pool| pool >= count));
        if all_arrived {
            for (name, count) in left {
                if let Some(pool) = arrived.get_mut(name) {
                    *pool -= count;
                }
            }
            moved.insert(file.path.clone());
        }
    }
    moved
}
