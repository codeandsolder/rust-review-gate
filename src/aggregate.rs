use std::collections::BTreeSet;

use serde::Serialize;

use crate::{
    model::{Finding, ModelRun},
    rules::HELPER_COMPLETE_MIGRATION_RULE,
};

#[derive(Debug, Clone, Serialize)]
pub struct ConsensusFinding {
    pub file: String,
    pub line: u32,
    pub rule: String,
    pub subject: Option<String>,
    pub votes: usize,
    pub sample_votes: usize,
    pub voters: Vec<String>,
    pub severity: String,
    pub confidence: f32,
    pub title: String,
    pub explanation: String,
    pub evidence: String,
    pub suggested_test: Option<String>,
}

struct Cluster {
    file: String,
    line: u32,
    min_line: u32,
    max_line: u32,
    rule: String,
    subject: Option<String>,
    voters: BTreeSet<String>,
    sample_voters: BTreeSet<(String, usize)>,
    representative: Finding,
}

#[must_use]
pub fn consensus(
    runs: &[ModelRun],
    min_votes: usize,
    line_tolerance: u32,
) -> Vec<ConsensusFinding> {
    let mut clusters: Vec<Cluster> = Vec::new();

    for run in runs {
        for finding in &run.review.findings {
            let existing = clusters
                .iter_mut()
                .find(|cluster| same_cluster(cluster, finding, line_tolerance));

            if let Some(cluster) = existing {
                cluster.voters.insert(run.model.clone());
                cluster
                    .sample_voters
                    .insert((run.model.clone(), run.sample));
                cluster.min_line = cluster.min_line.min(finding.line);
                cluster.max_line = cluster.max_line.max(finding.line);
                if stronger(finding, &cluster.representative) {
                    cluster.line = finding.line;
                    cluster.representative = finding.clone();
                }
            } else {
                clusters.push(Cluster {
                    file: finding.file.clone(),
                    line: finding.line,
                    min_line: finding.line,
                    max_line: finding.line,
                    rule: finding.rule.clone(),
                    subject: finding.subject.clone(),
                    voters: BTreeSet::from([run.model.clone()]),
                    sample_voters: BTreeSet::from([(run.model.clone(), run.sample)]),
                    representative: finding.clone(),
                });
            }
        }
    }

    let mut accepted: Vec<_> = clusters
        .into_iter()
        .filter(|cluster| cluster.voters.len() >= min_votes)
        .map(|cluster| ConsensusFinding {
            file: cluster.file,
            line: cluster.line,
            rule: cluster.rule,
            subject: cluster.subject,
            votes: cluster.voters.len(),
            sample_votes: cluster.sample_voters.len(),
            voters: cluster.voters.into_iter().collect(),
            severity: cluster.representative.severity,
            confidence: cluster.representative.confidence,
            title: cluster.representative.title,
            explanation: cluster.representative.explanation,
            evidence: cluster.representative.evidence,
            suggested_test: cluster.representative.suggested_test,
        })
        .collect();

    accepted.sort_by(|a, b| (&a.file, a.line, &a.rule).cmp(&(&b.file, b.line, &b.rule)));
    accepted
}

fn same_cluster(cluster: &Cluster, finding: &Finding, line_tolerance: u32) -> bool {
    if cluster.file != finding.file || cluster.rule != finding.rule {
        return false;
    }

    // Relational helper-migration defects can be anchored to either the new
    // helper or a sibling path left behind. When both reviewers identify the
    // same exact abstraction, that semantic key is stronger than line distance.
    // Without such a key, keep the ordinary strict line-neighborhood rule.
    if finding.rule == HELPER_COMPLETE_MIGRATION_RULE {
        match (cluster.subject.as_deref(), finding.subject.as_deref()) {
            (Some(cluster_subject), Some(finding_subject)) => {
                return cluster_subject == finding_subject;
            }
            (Some(_), None) | (None, Some(_)) => return false,
            (None, None) => {}
        }
    }
    let min_line = cluster.min_line.min(finding.line);
    let max_line = cluster.max_line.max(finding.line);
    max_line.abs_diff(min_line) <= line_tolerance
}

fn stronger(candidate: &Finding, current: &Finding) -> bool {
    severity_rank(&candidate.severity)
        .cmp(&severity_rank(&current.severity))
        .then_with(|| candidate.confidence.total_cmp(&current.confidence))
        .is_gt()
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "block" => 3,
        "major" => 2,
        "minor" => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::consensus;
    use crate::model::{Finding, ModelRun, Review};

    fn run(model: &str, line: u32, rule: &str) -> ModelRun {
        ModelRun {
            model: model.to_owned(),
            sample: 0,
            review: Review {
                verdict: "comment".to_owned(),
                findings: vec![Finding {
                    severity: "major".to_owned(),
                    confidence: 0.9,
                    file: "src/lib.rs".to_owned(),
                    line,
                    title: "problem".to_owned(),
                    explanation: "breaks under a concrete condition".to_owned(),
                    evidence: "changed branch".to_owned(),
                    rule: rule.to_owned(),
                    subject: None,
                    supporting_rules: Vec::new(),
                    suggested_test: None,
                }],
            },
            prompt_tokens: None,
            cached_prompt_tokens: None,
            cache_write_tokens: None,
            completion_tokens: None,
            cost: None,
            cache_discount: None,
            elapsed_ms: 1,
        }
    }

    #[test]
    fn requires_distinct_model_agreement() {
        let runs = vec![
            run("a", 10, "displayed-error"),
            run("b", 12, "displayed-error"),
            run("c", 40, "api-contract"),
        ];
        let accepted = consensus(&runs, 2, 3);
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].rule, "displayed-error");
        assert_eq!(accepted[0].votes, 2);
    }

    #[test]
    fn supporting_rules_do_not_create_consensus() {
        let mut first = run("a", 10, "displayed-error");
        first.review.findings[0]
            .supporting_rules
            .push("anti-over-abstraction".to_owned());
        let mut second = run("b", 11, "api-contract");
        second.review.findings[0]
            .supporting_rules
            .push("anti-over-abstraction".to_owned());

        assert!(consensus(&[first, second], 2, 3).is_empty());
    }

    #[test]
    fn repeated_samples_do_not_multiply_model_votes() {
        let first = run("a", 10, "displayed-error");
        let mut second = run("a", 11, "displayed-error");
        second.sample = 1;
        assert!(consensus(&[first, second], 2, 3).is_empty());
    }

    #[test]
    fn helper_migration_consensus_can_span_sibling_hunks() {
        let mut first = run("a", 8, "helper-complete-migration");
        let mut second = run("b", 200, "helper-complete-migration");
        first.review.findings[0].subject = Some("record_write_result".to_owned());
        second.review.findings[0].subject = Some("record_write_result".to_owned());
        let accepted = consensus(&[first, second], 2, 4);
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].votes, 2);
        assert_eq!(accepted[0].subject.as_deref(), Some("record_write_result"));
    }

    #[test]
    fn unrelated_helper_migrations_do_not_merge_by_rule_alone() {
        let mut first = run("a", 8, "helper-complete-migration");
        let mut second = run("b", 9, "helper-complete-migration");
        first.review.findings[0].subject = Some("record_write_result".to_owned());
        second.review.findings[0].subject = Some("record_read_result".to_owned());
        assert!(consensus(&[first, second], 2, 4).is_empty());
    }

    #[test]
    fn helper_migration_without_subject_remains_line_local() {
        let first = run("a", 8, "helper-complete-migration");
        let second = run("b", 35, "helper-complete-migration");
        assert!(consensus(&[first, second], 2, 4).is_empty());
    }

    #[test]
    fn helper_migration_does_not_mix_subjectful_and_subjectless_clusters() {
        let first = run("a", 8, "helper-complete-migration");
        let mut second = run("b", 9, "helper-complete-migration");
        second.review.findings[0].subject = Some("record_write_result".to_owned());
        assert!(consensus(&[first, second], 2, 4).is_empty());
    }

    #[test]
    fn line_local_rules_still_require_nearby_anchors() {
        let first = run("a", 8, "displayed-error");
        let second = run("b", 35, "displayed-error");
        assert!(consensus(&[first, second], 2, 4).is_empty());
    }

    #[test]
    fn line_cluster_boundary_does_not_follow_representative_strength() {
        let mut first = run("a", 10, "displayed-error");
        first.review.findings[0].confidence = 0.99;
        let second = run("b", 13, "displayed-error");
        let third = run("c", 16, "displayed-error");

        let accepted = consensus(&[first, second, third], 2, 4);
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].votes, 2);
    }
}
