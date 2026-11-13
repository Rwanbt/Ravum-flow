# FLOW-ADR-010 — Fichiers binaires, modes, liens et renommages

**Status: PROPOSED.** 2026-11-09 · Gate D2/R2/R4 · Human choice H3

## Context

Le writer historique se limite souvent à String UTF-8 et mode 100644.

## Candidate decision — not ratified

Éditions en octets + modes, delete/rename, symlink/casse, LFS/submodule ; refus explicite si non supporté.

## Alternatives to compare (including O0)

Normaliser tout en UTF-8; ignorer bit exécutable; interpréter rename comme overwrite.

## Consequences and tradeoffs

Surface complexe cross-OS, nécessaire avant promesse universelle.

## Evidence required before human acceptance

Document primary source/commit SHA, competing approaches, Windows/Linux implications and 16 Go costs, reproducible negative cases from [security matrix](../SECURITY_TESTS.md), and S9 if relevant. Required E2E must remain visible. A document is not a test; NOT_TESTED/UNKNOWN must not become PASS.

## Human sign-off

Decision: **PENDING** · Actor/date/evidence: **PENDING** · Reopen: adverse data, changed rights/tooling or performance failure.
