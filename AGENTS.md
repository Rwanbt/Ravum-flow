# AGENTS.md — Ravum Flow
**Source of truth:** [V6](docs/reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md) + this repo's current planning files. This file is operational guidance, not a decision ratification.

## Current access policy
- **GO_PLANNING:** allowed: read/research, issues, drafts, documentation and independently reasoned recommendations.
- **NO-GO IMPLEMENTATION:** do not write source code, run live-repository benchmarks, migrate Ravum code, generate real builds, modify Ravum, start daemons, add license or enable CI without a separate scoped human GO.
- The other Ravum development session owns `Rwanbt/Ravum`. It must not be interrupted or modified by Flow agents.
- For any future authorization: record `GO(scope, repository, branch, allowed_paths, budgets, evidence, stop_conditions)`. Silence does not grant GO.

## Correctness
- A patch's textual applicability ≠ semantic validation. Legacy Ravum V1 `AUTO_SAFE` maps to `TEXTUALLY_APPLICABLE` only.
- Prepare candidate Git tree, run **reviewed** hooks in the normal Git order inside isolated disposable execution, freeze tree, verify from exactly that tree + build context + policy, then publish with ref CAS and durable recovery.
- `SourceSnapshotId` and Git tree OID have different namespaces. Never compare them directly.
- `FAILED`, `STALE`, `TIMEOUT`, `UNKNOWN`, `UNSUPPORTED` and omitted required E2E are not `PASSED`.
- Git ref CAS by itself does not fence a stale worker; use writer-local serialization + durable epoch where leases are used, and document unmanaged Git bypass.
- No destructive reset or silent overwrite of dirty/untracked user files; unsupported file type/path must fail closed.

## Research & multi-agent protocol
1. Each agent has a disjoint subject and produces a raw signed-off report **before** the Lead collates findings.
2. Pin exact branch SHA, URLs, licenses, release versions and source time; do not count repeated model output as independent evidence.
3. Classify every claim: `CODE_READ / SOURCE_READ / MEASURED / PROPOSED / NOT_REPRODUCED / UNKNOWN / NOT_REVIEWED`.
4. Keep alternative O0 ('build nothing') viable and benchmark fairly.
5. Documents must be concise, numbered, cross-linked and maintain historical IDs unchanged.
6. Risk P0 findings about Ravum go to a separate **handoff draft**; no direct changes.

## Resource & privacy constraints
Reference machine Windows/Linux, 16 GB RAM. Avoid concurrent heavy Cargo jobs; only disposable fixtures upon future explicit GO. Do not leak credentials, source code or personal paths into external model logs. Model output is not a security proof.

## Expected first handoff
Matrix D0–D4, H1–H6, architecture alternatives, refreshed S9 protocol, rights/source audit, ADR proposals with alternatives, dependency map for RF-001..026, minimal next-phase recommendation. **Stop before code.**