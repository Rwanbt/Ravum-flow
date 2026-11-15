//! Command implementation; shared helpers stay in the parent module.

use super::*;

pub fn cmd_explain(
    root: &Path,
    tree: Option<String>,
    candidate: Option<String>,
) -> Result<Json, String> {
    let git = Git::new(root);
    let candidate_record = match candidate.as_deref() {
        None => None,
        Some(id) => match crate::proposal::Candidate::load(root, id)? {
            Some(record) => Some(record),
            None => {
                return Ok(refused_code(
                    "UNKNOWN_CANDIDATE",
                    format!("no candidate {id:?} in .flow/candidates/"),
                ))
            }
        },
    };
    let tree = match (&tree, &candidate_record) {
        (Some(t), _) => t.clone(),
        (None, Some(record)) => record.tree_oid.clone(),
        (None, None) => git.head_tree()?,
    };
    let receipt = Receipt::load(root, &tree)?;
    let records = state::read_journal(root)?;
    let related: Vec<Json> = records
        .iter()
        .filter(|r| r.get("tree").and_then(Json::as_str) == Some(tree.as_str()))
        .cloned()
        .collect();
    // Proposals are attributed to a candidate, so explain can name who proposed
    // what without the caller having to correlate ids by hand.
    let proposals = match &candidate_record {
        None => Json::Arr(Vec::new()),
        Some(record) => Json::Arr(
            record
                .proposals
                .iter()
                .filter_map(|id| crate::proposal::Proposal::load(root, id).ok().flatten())
                .map(|proposal| {
                    Json::obj(vec![
                        ("proposal_id", Json::str(proposal.proposal_id)),
                        ("agent_id", Json::str(proposal.agent_id)),
                        ("task_id", Json::str(proposal.task_id)),
                        ("base_commit", Json::str(proposal.base_commit)),
                        ("tree_oid", Json::str(proposal.tree_oid)),
                    ])
                })
                .collect(),
        ),
    };
    // A receipt is evidence only if its content still matches its digest and the
    // logs it names are present. Inspection must say so explicitly: displaying a
    // tampered receipt as `PASSED` is the false green Flow refuses at publication.
    let (
        receipt_json,
        receipt_trust,
        receipt_display,
        evidence_complete,
        missing_logs,
        log_integrity,
        tampered_logs,
    ) = match &receipt {
        None => (
            Json::Null,
            "NO_RECEIPT",
            Json::Null,
            false,
            Vec::<String>::new(),
            "NO_RECEIPT",
            Vec::<String>::new(),
        ),
        Some(r) => {
            let integrity_ok = state::verify_receipt_integrity(&r.json).is_ok();
            // A receipt that names log digests is only trustworthy if the files
            // still hash to them: swapped evidence is not evidence.
            let (missing, tampered, log_ok) = verify_receipt_logs(root, r);
            let log_integrity = if r.log_digests.is_empty() {
                "NO_LOG_DIGESTS"
            } else if log_ok {
                "OK"
            } else {
                "LOG_INTEGRITY_INVALID"
            };
            let display = if integrity_ok {
                Json::str(r.status.clone())
            } else {
                Json::str("RECEIPT_INTEGRITY_INVALID")
            };
            (
                r.json.clone(),
                if integrity_ok {
                    "INTEGRITY_OK"
                } else {
                    "RECEIPT_INTEGRITY_INVALID"
                },
                display,
                integrity_ok && missing.is_empty() && log_ok,
                missing,
                log_integrity,
                tampered,
            )
        }
    };
    Ok(Json::obj(vec![
        ("status", Json::str("EXPLAIN")),
        ("tree", Json::str(tree)),
        (
            "candidate_id",
            match &candidate_record {
                Some(record) => Json::str(record.candidate_id.clone()),
                None => Json::Null,
            },
        ),
        (
            "candidate",
            match &candidate_record {
                Some(record) => record.json.clone(),
                None => Json::Null,
            },
        ),
        ("proposals", proposals),
        ("receipt", receipt_json),
        ("receipt_trust", Json::str(receipt_trust)),
        (
            "run_id",
            match &receipt {
                Some(r) => Json::str(r.run_id.clone()),
                None => Json::Null,
            },
        ),
        ("receipt_status_display", receipt_display),
        ("evidence_complete", Json::Bool(evidence_complete)),
        ("log_integrity", Json::str(log_integrity)),
        (
            "tampered_logs",
            Json::Arr(tampered_logs.into_iter().map(Json::str).collect()),
        ),
        (
            "missing_logs",
            Json::Arr(missing_logs.into_iter().map(Json::str).collect()),
        ),
        ("journal", Json::Arr(related)),
    ]))
}
