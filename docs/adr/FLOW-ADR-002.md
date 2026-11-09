# FLOW-ADR-002 — Modes d'agents et isolation

**Status: PROPOSED.** 2026-11-09 · Gate D2/R2 · Human choice H1/H3

## Context

Les outils non intégrés écrivent directement sur disque et contournent le coordinateur.

## Candidate decision — not ratified

MANAGED_PATCH avec CAS préconditions ; ISOLATED_EXEC jetable pour outils ordinaires ; OBSERVED_UNMANAGED seulement best effort ; VIRTUALIZED différé.

## Alternatives to compare (including O0)

Dossier partagé brut; worktree permanent obligatoire; VFS immédiat.

## Consequences and tradeoffs

Mesurer frais d'exécution temporaire et expliquer limites des garanties.

## Evidence required before human acceptance

Document primary source/commit SHA, competing approaches, Windows/Linux implications and 16 Go costs, reproducible negative cases from [security matrix](../SECURITY_TESTS.md), and S9 if relevant. Required E2E must remain visible. A document is not a test; NOT_TESTED/UNKNOWN must not become PASS.

## Human sign-off

Decision: **PENDING** · Actor/date/evidence: **PENDING** · Reopen: adverse data, changed rights/tooling or performance failure.
