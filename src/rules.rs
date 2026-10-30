use std::collections::BTreeSet;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::diff::ParsedDiff;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ReviewRule {
    pub id: &'static str,
    pub summary: &'static str,
}

#[derive(Debug, Deserialize)]
struct RulePack {
    source_commit: String,
    rule_count: usize,
    rules: Vec<RustSkillRule>,
}

#[derive(Debug, Deserialize)]
struct RustSkillRule {
    id: String,
    category: String,
    priority: String,
    summary: String,
}

pub const HELPER_COMPLETE_MIGRATION_RULE: &str = "helper-complete-migration";

pub const REVIEW_RULES: &[ReviewRule] = &[
    ReviewRule {
        id: "invariant-owner",
        summary: "Fix and review the transition that owns an invariant, not a downstream symptom.",
    },
    ReviewRule {
        id: "failure-path-semantics",
        summary: "Compare retries, fallback, writes after failed probes, shutdown, cached errors, and best-effort behavior before/after.",
    },
    ReviewRule {
        id: "displayed-error",
        summary: "Audit the error representation actually displayed or logged; preserving only the source chain is insufficient when callers render Display.",
    },
    ReviewRule {
        id: HELPER_COMPLETE_MIGRATION_RULE,
        summary: "A new helper should own all equivalent sibling paths in its abstraction or remain local; partial migrations leave divergent behavior.",
    },
    ReviewRule {
        id: "internal-compatibility",
        summary: "Do not add optional state, duplicate constructors, or wrapper APIs just to preserve crate-internal or test-only interfaces.",
    },
    ReviewRule {
        id: "scope-discipline",
        summary: "Keep unrelated refactors, test cleanup, and style churn out of behavior changes.",
    },
    ReviewRule {
        id: "state-regression",
        summary: "Stateful fixes need tests for both the returned result and the resulting state/no stale state.",
    },
    ReviewRule {
        id: "boundary-tests",
        summary: "Exercise empty/missing/malformed/non-UTF8/wraparound/cfg/platform/failure branches implied by the change.",
    },
    ReviewRule {
        id: "analogous-semantics",
        summary: "Match neighboring option/env/config semantics unless there is an explicit reason to differ.",
    },
    ReviewRule {
        id: "shared-coupled-literal",
        summary: "Coupled protocol strings or literals that must change together should have one definition.",
    },
    ReviewRule {
        id: "doc-attachment",
        summary: "Structural edits can silently move doc comments onto the wrong item even when compilation succeeds.",
    },
    ReviewRule {
        id: "no-panic-external-input",
        summary: "Externally triggered or recoverable failures should propagate as errors instead of unwrap/expect/panic.",
    },
    ReviewRule {
        id: "unsafe-invariant",
        summary: "Every unsafe operation needs a local invariant proof and minimal scope; unsafe APIs need documented caller obligations.",
    },
    ReviewRule {
        id: "no-lock-across-await",
        summary: "Do not hold a synchronous mutex/RwLock guard across await unless the lock is explicitly designed for async suspension and the semantics demand it.",
    },
    ReviewRule {
        id: "cancellation-shutdown",
        summary: "Async changes must preserve cancellation, shutdown, task ownership, boundedness, and resource cleanup semantics.",
    },
    ReviewRule {
        id: "api-contract",
        summary: "Do not accidentally change public signatures, serialized keys, error text/reasons, config precedence, MSRV, or feature/target behavior.",
    },
    ReviewRule {
        id: "performance-evidence",
        summary: "Performance claims need representative measurement; avoid allocation/locking/generalization changes justified only by intuition.",
    },
];

fn load_rulepack() -> Result<RulePack> {
    let pack: RulePack = serde_json::from_str(include_str!("../rulepack/rust-skills2.json"))
        .context("decode embedded rust-skills2 rulepack")?;
    if pack.rule_count != pack.rules.len() {
        anyhow::bail!(
            "rust-skills2 rulepack count mismatch: declared {}, decoded {}",
            pack.rule_count,
            pack.rules.len()
        );
    }
    Ok(pack)
}

/// Return every rule identifier accepted in model findings.
///
/// # Errors
/// Returns an error when the embedded rust-skills2 rulepack is invalid or internally inconsistent.
pub fn known_rule_ids() -> Result<BTreeSet<String>> {
    let pack = load_rulepack()?;
    let mut ids = REVIEW_RULES
        .iter()
        .map(|rule| rule.id.to_owned())
        .collect::<BTreeSet<_>>();
    ids.extend(pack.rules.into_iter().map(|rule| rule.id));
    Ok(ids)
}

/// Render the deterministic rule subset applicable to a parsed diff.
///
/// # Errors
/// Returns an error when the embedded rust-skills2 rulepack is invalid or internally inconsistent.
pub fn prompt_text(diff: &ParsedDiff) -> Result<String> {
    let pack = load_rulepack()?;

    let prefixes = applicable_prefixes(diff);
    let selected: Vec<_> = pack
        .rules
        .iter()
        .filter(|rule| prefixes.iter().any(|prefix| rule.id.starts_with(prefix)))
        .collect();

    let mut sections = Vec::with_capacity(2);
    sections.push(format!(
        "sccache-derived project gates (cite these whenever they match):\n{}",
        REVIEW_RULES
            .iter()
            .map(|rule| format!("- {}: {}", rule.id, rule.summary))
            .collect::<Vec<_>>()
            .join("\n")
    ));
    sections.push(format!(
        "rust-skills2 snapshot {} — {} of {} rules selected deterministically for this diff:\n{}",
        pack.source_commit,
        selected.len(),
        pack.rule_count,
        selected
            .iter()
            .map(|rule| format!(
                "- {} [{} / {}]: {}",
                rule.id, rule.category, rule.priority, rule.summary
            ))
            .collect::<Vec<_>>()
            .join("\n")
    ));
    Ok(sections.join("\n\n"))
}

const ADDED_RULE_TRIGGERS: &[(&[&str], &[&str])] = &[
    (
        &[
            "result<",
            "err(",
            "error",
            "anyhow",
            "thiserror",
            ".context(",
            "map_err",
            "?;",
            ")?",
            ".unwrap(",
            ".expect(",
            "panic!(",
        ],
        &["err-"],
    ),
    (
        &[
            "#[test]",
            "assert!(",
            "assert_eq!(",
            "assert_ne!(",
            "proptest",
            "quickcheck",
        ],
        &["test-"],
    ),
    (
        &[
            "#[allow(",
            "#[warn(",
            "#[deny(",
            "#[expect(",
            "clippy::",
            "rustflags",
        ],
        &["lint-"],
    ),
    (
        &["unsafe", "*const", "*mut", "extern \"c\"", "no_mangle"],
        &["unsafe-"],
    ),
    (
        &[
            "async ", ".await", "tokio", "spawn(", "mutex", "rwlock", "atomic", "channel", "mpsc",
            "oneshot", "joinset",
        ],
        &["async-", "conc-"],
    ),
    (
        &[
            "pub ", "pub(", "trait ", "impl ", "struct ", "enum ", "where ",
        ],
        &["api-", "type-", "trait-", "conv-"],
    ),
    (
        &[
            "vec<",
            "string",
            "box<",
            "arc<",
            ".clone(",
            ".collect(",
            "capacity",
            "alloc",
            "cache",
            "hot path",
            "benchmark",
        ],
        &["mem-", "perf-", "opt-"],
    ),
    (
        &[
            "checked_",
            "saturating_",
            "wrapping_",
            "overflow",
            "usize",
            "u64",
            "i64",
            " as u",
            " as i",
        ],
        &["num-"],
    ),
    (&["serde", "serialize", "deserialize"], &["serde-"]),
    (&["match ", "if let ", "matches!("], &["pat-"]),
    (
        &["macro_rules!", "proc_macro", "quote!", "syn::"],
        &["macro-"],
    ),
    (&["fn ", "struct ", "enum ", "trait ", "const "], &["name-"]),
    (
        &[
            "hashmap",
            "hashset",
            "btreemap",
            "btreeset",
            "vecdeque",
            "binaryheap",
        ],
        &["coll-"],
    ),
    (&["const ", "static ", "const fn"], &["const-"]),
    (
        &[
            "tracing", "log::", "debug!(", "info!(", "warn!(", "error!(", "metrics",
        ],
        &["obs-"],
    ),
    (&["///", "//! "], &["doc-"]),
];

const FILE_RULE_TRIGGERS: &[(&[&str], &[&str])] = &[
    (&["readme", "docs/"], &["doc-"]),
    (&["tests/", "_test.rs", "tests.rs"], &["test-"]),
    (
        &["clippy.toml", ".cargo/config", "rust-toolchain"],
        &["lint-"],
    ),
    (
        &[
            "cargo.toml",
            "cargo.lock",
            "build.rs",
            "rust-toolchain",
            ".github/",
        ],
        &["proj-"],
    ),
];

fn applicable_prefixes(diff: &ParsedDiff) -> BTreeSet<&'static str> {
    let mut prefixes = BTreeSet::from(["own-", "anti-"]);
    let added = diff
        .added_lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();
    let files = diff
        .files
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();

    apply_triggers(&mut prefixes, &added, ADDED_RULE_TRIGGERS);
    apply_triggers(&mut prefixes, &files, FILE_RULE_TRIGGERS);
    prefixes
}

fn apply_triggers(
    prefixes: &mut BTreeSet<&'static str>,
    haystack: &str,
    triggers: &'static [(&'static [&'static str], &'static [&'static str])],
) {
    for (needles, additions) in triggers {
        if contains_any(haystack, needles) {
            prefixes.extend(additions.iter().copied());
        }
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::{applicable_prefixes, known_rule_ids, prompt_text};
    use crate::diff::ParsedDiff;

    #[test]
    fn selects_core_rules_without_unrelated_error_or_lint_families() {
        let diff = ParsedDiff::parse(
            "diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1 +1 @@\n-old\n+let value = left + right;\n",
        );
        let prefixes = applicable_prefixes(&diff);
        assert!(prefixes.contains("own-"));
        assert!(prefixes.contains("anti-"));
        assert!(!prefixes.contains("err-"));
        assert!(!prefixes.contains("lint-"));
    }

    #[test]
    fn known_rules_include_project_and_vendored_ids() {
        let ids = known_rule_ids();
        assert!(ids.is_ok());
        if let Ok(ids) = ids {
            assert!(ids.contains("failure-path-semantics"));
            assert!(ids.contains("unsafe-safety-comment"));
        }
    }

    #[test]
    fn selects_unsafe_and_async_rule_families() {
        let diff = ParsedDiff::parse(
            "diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1 +1,3 @@\n-old\n+pub async unsafe fn changed() {\n+    work().await;\n+}\n",
        );
        let prefixes = applicable_prefixes(&diff);
        assert!(prefixes.contains("unsafe-"));
        assert!(prefixes.contains("async-"));
        assert!(prefixes.contains("api-"));
        let prompt = prompt_text(&diff);
        assert!(prompt.is_ok());
        if let Ok(prompt) = prompt {
            assert!(prompt.contains("rust-skills2 snapshot"));
            assert!(prompt.contains("unsafe-safety-comment"));
        }
    }
}
