use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ReviewRule {
    pub id: &'static str,
    pub summary: &'static str,
}

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
        id: "helper-complete-migration",
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

#[must_use]
pub fn prompt_text() -> String {
    REVIEW_RULES
        .iter()
        .map(|rule| format!("- {}: {}", rule.id, rule.summary))
        .collect::<Vec<_>>()
        .join("\n")
}
