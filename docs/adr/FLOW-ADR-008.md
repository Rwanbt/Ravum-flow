# FLOW-ADR-008 — Benchmark S9 et option zéro

**Status: PROPOSED.** 2026-11-09 · Gate D3/R0 · Human choice H4/H5

## Context

S8 compare copies privées, partage direct et séquentiel, pas vrais worktrees ni Flow.

## Candidate decision — not ratified

S9 apparié avec copies, vrais worktrees, O0/O1/O2/O3 si disponibles; seuil/échantillon/budget humain avant essai; veto perte/secret/CI.

## Alternatives to compare (including O0)

S8 déclaré victoire; seuil choisi après résultat; benchmark vendeur.

## Consequences and tradeoffs

Variabilité LLM, supervision et ressources 16 Go à capturer.

## Evidence required before human acceptance

Document primary source/commit SHA, competing approaches, Windows/Linux implications and 16 Go costs, reproducible negative cases from [security matrix](../SECURITY_TESTS.md), and S9 if relevant. Required E2E must remain visible. A document is not a test; NOT_TESTED/UNKNOWN must not become PASS.

## Human sign-off

Decision: **PENDING** · Actor/date/evidence: **PENDING** · Reopen: adverse data, changed rights/tooling or performance failure.
