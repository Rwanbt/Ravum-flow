# FLOW-ADR-001 — Autorités Git, journal, tree et runtime

**Status: PROPOSED.** 2026-11-09 · Gate D2/R1 · Human choice H1

## Context

Git, snapshots, état dirty et journal ne sont pas une seule autorité.

## Candidate decision — not ratified

Git = état publié ; candidate Git tree = contenu à vérifier ; journal = intents/outcomes ; runtime Ravum = active version.

## Alternatives to compare (including O0)

Journal comme dépôt canonique; vérification sur working tree sale.

## Consequences and tradeoffs

Plusieurs identités à gérer; pas de confusion entre SourceSnapshotId et GitTreeOID.

## Evidence required before human acceptance

Document primary source/commit SHA, competing approaches, Windows/Linux implications and 16 Go costs, reproducible negative cases from [security matrix](../SECURITY_TESTS.md), and S9 if relevant. Required E2E must remain visible. A document is not a test; NOT_TESTED/UNKNOWN must not become PASS.

## Human sign-off

Decision: **PENDING** · Actor/date/evidence: **PENDING** · Reopen: adverse data, changed rights/tooling or performance failure.
