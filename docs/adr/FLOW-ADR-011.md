# FLOW-ADR-011 — Offline, ressources et entreprise

**Status: PROPOSED.** 2026-11-09 · Gate D3/R7 · Human choice H1/H4

## Context

Un daemon/VFS/CRDT/remote obligatoire compromettrait la sobriété du solo.

## Candidate decision — not ratified

CLI local-first sans LLM/serveur, workers bornés 16 Go; remote/VFS/enterprise uniquement R7 après S9 favorable.

## Alternatives to compare (including O0)

Cloud obligatoire; daemon monolithique; Kubernetes au MVP.

## Consequences and tradeoffs

Fonctions entreprise différées, contrôle des coûts.

## Evidence required before human acceptance

Document primary source/commit SHA, competing approaches, Windows/Linux implications and 16 Go costs, reproducible negative cases from [security matrix](../SECURITY_TESTS.md), and S9 if relevant. Required E2E must remain visible. A document is not a test; NOT_TESTED/UNKNOWN must not become PASS.

## Human sign-off

Decision: **PENDING** · Actor/date/evidence: **PENDING** · Reopen: adverse data, changed rights/tooling or performance failure.
