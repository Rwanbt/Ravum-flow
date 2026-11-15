//! Command implementation; shared helpers stay in the parent module.

use super::*;

pub fn cmd_publish(
    root: &Path,
    reference: Option<String>,
    expected_old: Option<String>,
    message: Option<String>,
    candidate: Option<String>,
    dry_run: bool,
) -> Result<Json, String> {
    let config = state::load_config(root)?;
    if config.disabled {
        return Ok(refused_code(
            "DISABLED",
            "Flow is disabled in this repository",
        ));
    }
    if let Err(reason) = validate_obligations(&config.obligations) {
        return Ok(refused_code("INVALID_POLICY", reason));
    }
    if let Some(refusal) = adoption_gate(root, &config)? {
        return Ok(refusal);
    }
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

    // Publish to a branch only. `HEAD`, a tag or a remote ref is either
    // ambiguous or the wrong object to move for "this tree is verified".
    let reference = match reference {
        Some(r) => {
            if !r.starts_with("refs/heads/") {
                return Ok(refused_code(
                    "REF_NOT_ALLOWED",
                    format!("--ref {r:?} must be a full branch ref under refs/heads/"),
                ));
            }
            r
        }
        None => current_ref(&git)?,
    };
    if reference.len() > 1024 || reference.contains('\0') || reference.contains(' ') {
        return Ok(refused_code(
            "REF_NOT_ALLOWED",
            format!("--ref {reference:?} is not a usable ref name"),
        ));
    }
    if git.run(&["check-ref-format", &reference])?.status != 0 {
        return Ok(refused_code(
            "REF_NOT_ALLOWED",
            format!("--ref {reference:?} is not a valid ref name (git check-ref-format)"),
        ));
    }
    // A symbolic ref would move its target while this command reports the name
    // the operator passed: they would believe the commit landed on `alias` when
    // it landed on something else. Refuse rather than report a misleading name.
    let symref = git.run(&["symbolic-ref", "-q", &reference])?;
    if symref.status == 0 {
        return Ok(refused_code(
            "REF_IS_SYMREF",
            format!(
                "--ref {reference:?} is a symbolic ref to {:?}: publish to the real branch ref",
                symref.stdout.trim()
            ),
        ));
    }

    let _lock = match crate::lock::acquire_lock(root, "publish") {
        Ok(guard) => guard,
        Err(e) => return Ok(lock_refusal(root, e)),
    };

    // Publish exactly the tree that `run` verified: the candidate's tree when
    // `--candidate` is given, the index tree otherwise.
    let tree = match &candidate_record {
        Some(record) => record.tree_oid.clone(),
        None => index_tree_without_locking(&git)?,
    };
    if candidate_record.is_some() {
        let present =
            git.run_bounded(&["cat-file", "-e", &format!("{tree}^{{tree}}")], None, None)?;
        if present.status != 0 {
            return Ok(refused_code(
                "CANDIDATE_TREE_MISSING",
                format!("candidate tree {tree} is no longer in the object store: nothing coherent can be published"),
            ));
        }
    }
    let receipt = match Receipt::load(root, &tree)? {
        Some(r) => r,
        None => {
            return Ok(refused_code(
                "NO_RECEIPT",
                format!("no complete receipt for tree {tree}: run `flow run` on this exact index tree first (a selective run never produces one)"),
            ))
        }
    };

    // 1. the receipt must be written by this version, unaltered.
    if receipt.schema != state::SCHEMA_RECEIPT {
        return Ok(refused_code(
            "RECEIPT_SCHEMA_STALE",
            format!(
                "receipt schema {:?} is not {}: re-run `flow run`",
                receipt.schema,
                state::SCHEMA_RECEIPT
            ),
        ));
    }
    if let Err(e) = state::verify_receipt_integrity(&receipt.json) {
        return Ok(refused_code("RECEIPT_INTEGRITY", e));
    }
    // 2. it must name this tree, and cover the whole declared policy.
    if receipt.tree_oid != tree {
        return Ok(refused_code(
            "TREE_MISMATCH",
            format!(
                "receipt integrity error: file for {tree} names tree {:?}",
                receipt.tree_oid
            ),
        ));
    }
    if receipt.scope_mode != "complete" {
        return Ok(refused_code(
            "SCOPE_INCOMPLETE",
            format!(
                "receipt scope is {:?}: a selective run is diagnostic and never publishable",
                receipt.scope_mode
            ),
        ));
    }
    if !receipt.missing_required.is_empty() {
        return Ok(refused_code(
            "MISSING_REQUIRED",
            format!(
                "receipt does not cover required obligations {:?}",
                receipt.missing_required
            ),
        ));
    }
    // 3. the policy must still be the policy the receipt proves. This is
    //    recomputed from the current configuration, never read from the receipt.
    let policy = effective_policy(&config, &config.obligations);
    let expected_digest = policy_digest(&policy);
    if receipt.policy_digest != expected_digest {
        return Ok(refused_code(
            "STALE_POLICY",
            format!(
                "the receipt proves policy {} but the current policy is {}: any edit to .flow/config.json, and any `--timeout` override at run time, changes the proven policy - re-run `flow run` without overrides",
                short(&receipt.policy_digest),
                short(&expected_digest)
            ),
        ));
    }
    if receipt.policy_version != config.policy_version {
        return Ok(refused_code(
            "POLICY_VERSION_MISMATCH",
            format!(
                "receipt policy_version {:?} != current {:?}",
                receipt.policy_version, config.policy_version
            ),
        ));
    }
    if receipt.execution_boundary != config.execution_boundary {
        return Ok(refused_code(
            "EXECUTION_BOUNDARY_MISMATCH",
            format!(
                "receipt execution_boundary {:?} != current {:?}",
                receipt.execution_boundary, config.execution_boundary
            ),
        ));
    }
    // The context covers the toolchain and platform the run actually used; a Git
    // upgrade between run and publish changes it, and the receipt is then no
    // longer a statement about the tool that would publish.
    let expected_context = build_context_id(&git, &tree, &policy, &config, root);
    if receipt.context_id != expected_context {
        return Ok(refused_code(
            "CONTEXT_MISMATCH",
            format!(
                "the receipt context is {} but this environment now computes {}: an allowlisted environment value, an obligation program's bytes, a declared probe's answer, or the platform changed since the run",
                short(&receipt.context_id),
                short(&expected_context)
            ),
        ));
    }
    // 4. every currently required obligation must be present *and* passed. The
    //    global status alone is not enough: a missing entry is not a pass.
    for ob in config.obligations.iter().filter(|o| o.required) {
        match receipt
            .obligations
            .iter()
            .find(|x| x.get("id").and_then(Json::as_str) == Some(ob.id.as_str()))
        {
            None => {
                return Ok(refused_code(
                    "MISSING_REQUIRED",
                    format!("required obligation {:?} is absent from the receipt", ob.id),
                ))
            }
            Some(entry) if entry.get("status").and_then(Json::as_str) != Some("PASSED") => {
                return Ok(refused_code(
                    "REQUIRED_NOT_PASSED",
                    format!(
                        "required obligation {:?} has status {:?}",
                        ob.id,
                        entry
                            .get("status")
                            .and_then(Json::as_str)
                            .unwrap_or("UNKNOWN")
                    ),
                ))
            }
            Some(_) => {}
        }
    }
    if receipt.status != "PASSED" {
        return Ok(refused_code(
            "STATUS_NOT_PASSED",
            format!(
                "receipt status is {}: a non-PASSED receipt is never publishable",
                receipt.status
            ),
        ));
    }
    // 5. attribution: the receipt proves an exact tree, not necessarily the
    //    candidate the operator names. When `--candidate` is given, the record
    //    must name that candidate; otherwise the publication would report C2
    //    while its evidence belongs to C1. Without `--candidate`, the receipt's
    //    candidate is exposed in the output and journal instead of guessed.
    let receipt_candidate_id = receipt
        .json
        .get("candidate_id")
        .and_then(Json::as_str)
        .unwrap_or("")
        .to_string();
    if let Some(record) = &candidate_record {
        if receipt_candidate_id != record.candidate_id {
            return Ok(refused_code(
                "RECEIPT_CANDIDATE_MISMATCH",
                format!(
                    "the receipt for tree {tree} was written by candidate {:?}, but this publication names candidate {}: run `flow run --candidate {}` first, or publish without --candidate and read receipt_candidate_id",
                    if receipt_candidate_id.is_empty() {
                        "<index tree, no candidate>"
                    } else {
                        &receipt_candidate_id
                    },
                    record.candidate_id,
                    record.candidate_id
                ),
            ));
        }
    }
    // 6. the verification commit must still describe the same tree, when the
    //    object still exists (it may have been pruned; absence is not a failure).
    if !receipt.commit_oid.is_empty() {
        if let Ok(t) = git.capture(&["rev-parse", &format!("{}^{{tree}}", receipt.commit_oid)]) {
            if t != tree {
                return Ok(refused_code(
                    "RECEIPT_COMMIT_MISMATCH",
                    format!(
                        "receipt commit {} carries tree {t}, not the verified {tree}",
                        receipt.commit_oid
                    ),
                ));
            }
        }
    }

    // The evidence the receipt names must still be there and unchanged. The seal
    // covers the names and hashes, not the bytes on disk: a deleted or swapped log
    // is not evidence, and publication must be at least as strict as `explain`.
    let (missing_logs, tampered_logs, logs_ok) = verify_receipt_logs(root, &receipt);
    if !missing_logs.is_empty() {
        return Ok(refused_code(
            "MISSING_LOGS",
            format!(
                "the receipt for tree {tree} names {} log(s) that no longer exist (for example {:?}): the evidence is incomplete, re-run `flow run`",
                missing_logs.len(),
                &missing_logs[..missing_logs.len().min(3)]
            ),
        ));
    }
    if !logs_ok {
        return Ok(refused_code(
            "LOG_INTEGRITY_INVALID",
            format!(
                "the receipt for tree {tree} names log(s) whose bytes no longer hash to the recorded digest (for example {:?}): refusing to publish swapped evidence",
                &tampered_logs[..tampered_logs.len().min(3)]
            ),
        ));
    }

    let old = match expected_old {
        Some(o) => o,
        None => match git.show_ref(&reference)? {
            Some(o) => o,
            None => {
                return Ok(refused_code(
                    "UNKNOWN_REF",
                    format!("cannot resolve {reference}"),
                ))
            }
        },
    };
    let current = git.show_ref(&reference)?;
    if current.as_deref() != Some(old.as_str()) {
        return Ok(refused_code(
            "STALE_REF",
            format!("stale expected old OID: ref is {current:?}, expected {old}"),
        ));
    }
    // Publishing a tree the ref already carries would only add an empty commit.
    if let Ok(current_tree) = git.capture(&["rev-parse", &format!("{old}^{{tree}}")]) {
        if current_tree == tree {
            return Ok(refused_code(
                "ALREADY_PUBLISHED",
                format!("{reference} already points at a commit whose tree is {tree}: nothing to publish"),
            ));
        }
    }

    // A candidate is a statement about "base + these proposals". Publishing it on
    // top of a different commit would silently claim a merge Flow never made.
    if let Some(record) = &candidate_record {
        if record.base_commit != old {
            return Ok(refused_code(
                "BASE_MISMATCH",
                format!(
                    "candidate {} was assembled on base {} but this publication would land on {}: re-assemble on the new base and re-run, or pass the matching --expected-old",
                    record.candidate_id, record.base_commit, old
                ),
            ));
        }
    }
    if dry_run {
        // Everything above is the same preflight a real publication performs.
        // Nothing below this point has happened yet: no commit object, no ref
        // movement, no journal line.
        return Ok(Json::obj(vec![
            ("status", Json::str("PLAN")),
            ("schema", Json::str(state::SCHEMA_PLAN)),
            ("dry_run", Json::Bool(true)),
            ("ref", Json::str(reference)),
            ("old", Json::str(old)),
            ("tree", Json::str(tree.clone())),
            (
                "candidate_id",
                match &candidate_record {
                    Some(record) => Json::str(record.candidate_id.clone()),
                    None => Json::Null,
                },
            ),
            ("receipt_candidate_id", Json::str(receipt_candidate_id.clone())),
            ("context_id", Json::str(receipt.context_id.clone())),
            ("policy_digest", Json::str(receipt.policy_digest.clone())),
            ("policy_version", Json::str(receipt.policy_version.clone())),
            (
                "execution_boundary",
                Json::str(receipt.execution_boundary.clone()),
            ),
            (
                "obligations",
                Json::Arr(
                    receipt
                        .obligations
                        .iter()
                        .filter_map(|o| o.get("id").and_then(Json::as_str).map(Json::str))
                        .collect(),
                ),
            ),
            (
                "changes",
                match &candidate_record {
                    Some(record) => {
                        Json::Arr(record.changes.iter().map(|c| c.to_json()).collect())
                    }
                    None => Json::Arr(Vec::new()),
                },
            ),
            (
                "journal",
                Json::str(".flow/journal.jsonl".to_string()),
            ),
            (
                "note",
                Json::str("plan only: no commit was created, no ref was moved, no journal record was appended"),
            ),
        ]));
    }
    let message = message.unwrap_or_else(|| format!("flow: publish verified tree {tree}"));
    let commit = git.commit_tree(&tree, &old, &message)?;
    // The commit we are about to publish must carry exactly the qualified tree.
    let prepared_tree = git.capture(&["rev-parse", &format!("{commit}^{{tree}}")])?;
    if prepared_tree != tree {
        return Ok(refused_code(
            "COMMIT_TREE_MISMATCH",
            format!(
                "prepared commit {commit} carries tree {prepared_tree}, not the verified {tree}"
            ),
        ));
    }
    let id = format!(
        "{}-{}-{}",
        state::iso8601_now(),
        &commit[..12.min(commit.len())],
        state::unique_suffix()
    );
    if crash_point("before-intent") {
        std::process::exit(89);
    }
    state::append_journal(
        root,
        &Json::obj(vec![
            ("event", Json::str("intent")),
            ("id", Json::str(id.clone())),
            ("ref", Json::str(reference.clone())),
            ("old", Json::str(old.clone())),
            ("new", Json::str(commit.clone())),
            ("tree", Json::str(tree.clone())),
            (
                "candidate_id",
                if receipt_candidate_id.is_empty() {
                    Json::Null
                } else {
                    Json::str(receipt_candidate_id.clone())
                },
            ),
            ("at", Json::str(state::iso8601_now())),
        ]),
    )?;
    if crash_point("after-intent") {
        std::process::exit(89);
    }
    let recheck = git.show_ref(&reference)?;
    if recheck.as_deref() != Some(old.as_str()) {
        state::append_journal(
            root,
            &Json::obj(vec![
                ("event", Json::str("outcome")),
                ("id", Json::str(id)),
                ("status", Json::str("REFUSED_STALE")),
                ("observed", Json::str(recheck.unwrap_or_default())),
                ("at", Json::str(state::iso8601_now())),
            ]),
        )?;
        return Ok(refused_code(
            "REF_MOVED",
            "ref moved between intent and CAS: nothing was published",
        ));
    }
    let res = git.update_ref(&reference, &commit, &old)?;
    if res.status != 0 {
        state::append_journal(
            root,
            &Json::obj(vec![
                ("event", Json::str("outcome")),
                ("id", Json::str(id)),
                ("status", Json::str("REFUSED")),
                ("stderr", Json::str(gitcli::redact(res.stderr.trim()))),
                ("at", Json::str(state::iso8601_now())),
            ]),
        )?;
        return Ok(refused_code(
            "UPDATE_REF_REFUSED",
            format!("git update-ref refused: {}", res.stderr.trim()),
        ));
    }
    if crash_point("after-ref") {
        std::process::exit(89);
    }
    // Post-CAS observation: the ref must now carry exactly the prepared commit.
    // Anything else is an outcome Flow did not plan, so it is reported as needing
    // reconciliation rather than as a success.
    let observed = git.show_ref(&reference)?;
    if observed.as_deref() != Some(commit.as_str()) {
        state::append_journal(
            root,
            &Json::obj(vec![
                ("event", Json::str("outcome")),
                ("id", Json::str(id)),
                ("status", Json::str(state::STATE_NEEDS_RECONCILIATION)),
                ("observed", Json::str(observed.unwrap_or_default())),
                ("at", Json::str(state::iso8601_now())),
            ]),
        )?;
        return Ok(refused_code(
            "NEEDS_RECONCILIATION",
            format!("update-ref reported success but {reference} does not carry {commit}: inspect with `flow reconcile`"),
        ));
    }
    state::append_journal(
        root,
        &Json::obj(vec![
            ("event", Json::str("outcome")),
            ("id", Json::str(id)),
            ("status", Json::str("PUBLISHED")),
            ("commit", Json::str(commit.clone())),
            ("at", Json::str(state::iso8601_now())),
        ]),
    )?;
    Ok(Json::obj(vec![
        ("status", Json::str("PUBLISHED")),
        (
            "candidate_id",
            match &candidate_record {
                Some(record) => Json::str(record.candidate_id.clone()),
                None => Json::Null,
            },
        ),
        ("ref", Json::str(reference)),
        ("old", Json::str(old)),
        ("commit", Json::str(commit)),
        ("tree", Json::str(tree)),
        ("context_id", Json::str(receipt.context_id.clone())),
        ("policy_digest", Json::str(receipt.policy_digest.clone())),
        ("policy_version", Json::str(receipt.policy_version.clone())),
        (
            "execution_boundary",
            Json::str(receipt.execution_boundary.clone()),
        ),
        ("receipt_logs", Json::Num(receipt.logs.len() as f64)),
        ("receipt_run_id", Json::str(receipt.run_id.clone())),
        (
            "receipt_candidate_id",
            Json::str(receipt_candidate_id.clone()),
        ),
        ("receipt_started_at", Json::str(receipt.started_at.clone())),
        (
            "receipt_finished_at",
            Json::str(receipt.finished_at.clone()),
        ),
        ("receipt_commit", Json::str(receipt.commit_oid.clone())),
        (
            "obligations",
            Json::Arr(
                receipt
                    .obligations
                    .iter()
                    .filter_map(|o| {
                        o.get("id")
                            .and_then(Json::as_str)
                            .map(|id| Json::str(id.to_string()))
                    })
                    .collect(),
            ),
        ),
        (
            "worktree_diff",
            Json::Arr(
                receipt
                    .worktree_diff
                    .iter()
                    .map(|p| Json::str(p.clone()))
                    .collect(),
            ),
        ),
        (
            "note",
            Json::str("only refs were moved; the working tree was not modified and may now differ"),
        ),
    ]))
}
