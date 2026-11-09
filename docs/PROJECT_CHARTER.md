# Project charter — PROPOSED
Updated 2026-11-09. Source: [V6 archive](reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md). No technical acceptance implied.

## Problem
Multiple coding agents generate overlapping edits, costly worktrees/copies and repetitive compilation/CI; pull requests and E2E queues accumulate. The goal is shorter **time to a demonstrably verified deliverable**, including coordination and human supervision, without increasing data loss or false-green CI.

## Audience and scope
Solo local developer first (Windows/Linux, 16 GB RAM); teams and large monorepos later. Rust first through native tools, then TS/JS, Python, C/C++, Go; Ravum optional. Everything must work without a cloud account, online LLM or Flow server for local use.

## Product invariants (historical INV-01..12)
1. One canonical user directory; immutable proposals/overlays. **INV-01-A is only a proposed amendment** to explicitly permit disposable materialized sandboxes or temporary worktrees for hooks/unmanaged agents.
2. No silently lost write; version/precondition checks.
3. Transactional logical promotion; idempotent recovery from interrupted publication.
4. Validation receipt bound to exact candidate tree **and** build context.
5. Honest statuses; bounded proof claims only.
6. Language-neutral core and truthful adapter capabilities.
7. Native Git interoperability, no lock-in.
8. Offline-local mode.
9. Bounded CPU/RAM/disk and cancellation.
10. Mandatory failures and E2E remain visible.
11. Reversible disable/uninstall; Git remains readable.
12. Measure end-to-end verified delivery rather than no-worktree ideology.

These are historical proposals, not ratified replacements. The complete literal language is in V6.

## Explicit non-goals for MVP
New source-control format, new compiler, full CRDT/VFS, proprietary remote CAS, distributed service, cloud lock-in, universal semantic merging, proof of bug-free code, mandatory daemon, and hot swap within Flow.

## Success/failure definition
- Correctness eliminates variants with lost writes, publication of unverified trees, secret exposure, false-green E2E or destructive reconciliation.
- Benefit measured against O0 and O1 with paired experiments and pre-registered human threshold and budget.
- When existing tools deliver the same validated outcome at equal/lower total cost, recommend **not building** a new engine.

## Historical Ravum principles
P0-A..P0-I (continuous feedback, native performance, memory safety, bounded proofs, diagnostics, RT/DSP, universal devices, 16 GB resource budget, time to verified deliverable). Flow must not reimplement Ravum language semantics.