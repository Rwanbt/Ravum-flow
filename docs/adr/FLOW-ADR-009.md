# FLOW-ADR-009 — Droits et interface Ravum

**Status: PROPOSED.** 2026-11-09 · Gate D0/D4 · Human choice H2/H6

## Context

Code Ravum et licence non ratifiés, risques de double ownership.

## Candidate decision — not ratified

Inventaire auteurs/SHAs/tests/dépendances ; adaptateur opt-in, bibliothèque licite ou clean-room ; pas de cycle ni code audio RT Flow.

## Alternatives to compare (including O0)

Copier crates; choisir licence par IA; reprendre ravum-store comme CAS.

## Consequences and tradeoffs

Maintenance des API doubles et coûts de migration.

## Evidence required before human acceptance

Document primary source/commit SHA, competing approaches, Windows/Linux implications and 16 Go costs, reproducible negative cases from [security matrix](../SECURITY_TESTS.md), and S9 if relevant. Required E2E must remain visible. A document is not a test; NOT_TESTED/UNKNOWN must not become PASS.

## Human sign-off

Decision: **PENDING** · Actor/date/evidence: **PENDING** · Reopen: adverse data, changed rights/tooling or performance failure.
