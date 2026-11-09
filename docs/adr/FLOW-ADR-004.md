# FLOW-ADR-004 — Sandbox et processus candidats

**Status: PROPOSED.** 2026-11-09 · Gate D2/R3 · Human choice H3

## Context

Rustc/tests/hooks historiques héritent en partie de l'environnement hôte.

## Candidate decision — not ratified

Classer TRUSTED/UNTRUSTED ; sandbox FS/network/process réelle par OS ; allowlist env, pas de secrets, refus si sécurité manquante.

## Alternatives to compare (including O0)

env_clear seule; simple redaction logs; code agent exécuté directement.

## Consequences and tradeoffs

Compatibilité hooks, coût mémoire et disparités Windows/Linux.

## Evidence required before human acceptance

Document primary source/commit SHA, competing approaches, Windows/Linux implications and 16 Go costs, reproducible negative cases from [security matrix](../SECURITY_TESTS.md), and S9 if relevant. Required E2E must remain visible. A document is not a test; NOT_TESTED/UNKNOWN must not become PASS.

## Human sign-off

Decision: **PENDING** · Actor/date/evidence: **PENDING** · Reopen: adverse data, changed rights/tooling or performance failure.
