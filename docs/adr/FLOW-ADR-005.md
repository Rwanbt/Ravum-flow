# FLOW-ADR-005 — Publication Git, leases/fencing et reprise

**Status: PROPOSED.** 2026-11-09 · Gate D2/R4 · Human choice H1

## Context

update-ref compare une ref mais ne synchronise ni epoch de lease ni DB ni répertoire utilisateur.

## Candidate decision — not ratified

Writer local exclusif, relecture epoch+CAS ref+outcome sous verrou, journal durable idempotent, réconciliation non destructive.

## Alternatives to compare (including O0)

CAS Git seul; promesse fausse d'ACID multi-ressources.

## Consequences and tradeoffs

Git externe contourne verrou et exige détection/revalidation.

## Evidence required before human acceptance

Document primary source/commit SHA, competing approaches, Windows/Linux implications and 16 Go costs, reproducible negative cases from [security matrix](../SECURITY_TESTS.md), and S9 if relevant. Required E2E must remain visible. A document is not a test; NOT_TESTED/UNKNOWN must not become PASS.

## Human sign-off

Decision: **PENDING** · Actor/date/evidence: **PENDING** · Reopen: adverse data, changed rights/tooling or performance failure.
