# FLOW-ADR-007 — Build natif, cache et E2E

**Status: PROPOSED.** 2026-11-09 · Gate D2/R3/R5 · Human choice H4

## Context

Analyse impact mal bornée peut laisser un consommateur inchangé cassé et un faux cache hit.

## Candidate decision — not ratified

Cargo/tsc/pytest/CMake natifs via adaptateurs ; graphe conservateur ; cache contextuel ; E2E requis sans masquage.

## Alternatives to compare (including O0)

Salsa comme build universel; supprimer tests rouges; compiler sans dépendants.

## Consequences and tradeoffs

Temps supplémentaire quand graphe partiel; bénéfice mesuré au S9.

## Evidence required before human acceptance

Document primary source/commit SHA, competing approaches, Windows/Linux implications and 16 Go costs, reproducible negative cases from [security matrix](../SECURITY_TESTS.md), and S9 if relevant. Required E2E must remain visible. A document is not a test; NOT_TESTED/UNKNOWN must not become PASS.

## Human sign-off

Decision: **PENDING** · Actor/date/evidence: **PENDING** · Reopen: adverse data, changed rights/tooling or performance failure.
