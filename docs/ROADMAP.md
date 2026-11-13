# Gates and roadmap (candidate; not an implementation order)
**Historical mapping is preserved, not renumbered silently.**

## Documentary gates (do not imply implementation permission)
- **D0**: provenance, P0-A..I, 42 original Ravum ADRs, Flow INV-01..12, RF-RFC-001..012, rights and SHA inventory.
- **D1**: independent adversarial review; duplicate reports not counted twice; alternatives O0/O1 seriously assessed.
- **D2**: exact-tree/receipt/hook transaction model, subprocess trust policy, fences, recovery, filetypes, E2E.
- **D3**: pre-registered S9 comparison, numerical human threshold and budget, negative tests and stop rules.
- **D4**: human approval of provisional architecture, rights and **scoped** next action.

## Conditional future execution phases
| Phase | Content | Main exit gate | Allowed without new GO? |
|---|---|---|---|
| R0 | disposable reference fixtures, P0 negative-test reproductions, O0/O1 baseline | reproducible evidence incl. limits | **NO** |
| R1 | minimal versioned proposal/tree/context/receipt/journal/CLI contracts | deterministic parsers + fail closed | NO |
| R2 | managed patches, logical MVCC, isolated-exec fallback; no silent loss | concurrency and filetype cases | NO |
| R3 | correct hook order, freeze post-hooks tree, hermetic-enough verification, Rust adapter | TREE/HOOK/ENV/LEGACY/E2E gates | NO |
| R4 | Git publish CAS, durable intent/recovery, non-destructive reconciliation | crash injection under Windows/Linux | NO |
| R5 | provider-neutral CLI/agents; TS/Python/C++/Go adapters by demand | changed API/unchanged consumer and E2E | NO |
| R6 | optional forge and Ravum adapter without cyclic ownership | exact final-merge qualification | NO |
| R7 | enterprise VFS/CRDT/distribution/cache only after measured need | new threat model and benchmarks | NO |

## Historic mapping
RF-0→D0/D3/R0; RF-1→R1; RF-2→R2; RF-3→R4 **after R3**; RF-4→R3/R5; RF-5→R5; RF-6→R5; RF-7→R6; RF-8→R7; RF-9→R6.

## Principles
- A proposal can be admitted without being publishable; promotion requires a verification policy.
- S9 must beat O0/O1 on correctness + total cost before new transactional complexity is justified.
- `UNKNOWN` is not passed. Full required CI and E2E remain; targeted tests do not hide failures.
- R0 is not authorization to change Ravum's working tree or fix its writer.
- R1–R7 must be explicitly approved one phase at a time.