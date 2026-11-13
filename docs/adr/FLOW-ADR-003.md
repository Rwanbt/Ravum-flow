# FLOW-ADR-003 — Tree exact, hooks et recettes

**Status: PROPOSED.** 2026-11-09 · Gate D2/R3 · Human choice H3

## Context

GitWriter Ravum historique écrit le tree avant hooks et ne garantit pas la même entrée de build que l'arbre publié.

## Candidate decision — not ratified

Hooks autorisés dans l'ordre Git en espace isolé ; gel post-hooks du tree ; tests sur ce tree et BuildContextId ; reçu versionné avant CAS ref.

## Alternatives to compare (including O0)

Tester le workspace réel; comparer SHA propriétaires et OID Git; réutiliser anciens verts.

## Consequences and tradeoffs

Hooks non idempotents et fichiers non stagés demandent une politique de refus/repréparation.

## Evidence required before human acceptance

Document primary source/commit SHA, competing approaches, Windows/Linux implications and 16 Go costs, reproducible negative cases from [security matrix](../SECURITY_TESTS.md), and S9 if relevant. Required E2E must remain visible. A document is not a test; NOT_TESTED/UNKNOWN must not become PASS.

## Human sign-off

Decision: **PENDING** · Actor/date/evidence: **PENDING** · Reopen: adverse data, changed rights/tooling or performance failure.
