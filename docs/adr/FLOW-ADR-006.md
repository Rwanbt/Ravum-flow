# FLOW-ADR-006 — Stockage journal et rétention

**Status: PROPOSED.** 2026-11-09 · Gate D2/R1 · Human choice H4

## Context

V0.1 proposait un CAS générique sans preuve de nécessité.

## Candidate decision — not ratified

Comparer objets Git pour les blobs + journal SQLite WAL FULL ou append-only synchronisé; GC/pinning/recovery prouvés.

## Alternatives to compare (including O0)

Reprendre ravum-store Todo; CAS maison dès MVP; SQLite supposé transactionner Git.

## Consequences and tradeoffs

Coût fsync, pannes et garbage collection à mesurer.

## Evidence required before human acceptance

Document primary source/commit SHA, competing approaches, Windows/Linux implications and 16 Go costs, reproducible negative cases from [security matrix](../SECURITY_TESTS.md), and S9 if relevant. Required E2E must remain visible. A document is not a test; NOT_TESTED/UNKNOWN must not become PASS.

## Human sign-off

Decision: **PENDING** · Actor/date/evidence: **PENDING** · Reopen: adverse data, changed rights/tooling or performance failure.
