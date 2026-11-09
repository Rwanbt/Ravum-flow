# Repository branch and review workflow — proposed

**Current actual state:** `main` contains the initial planning README; `dev` holds the extended proposed documentation and V6 archive. Neither is a released engine.

## Suggested branch workflow

1. Treat `main` as reviewed documentary/release baseline, not automatically as an APPROVED ADR set.
2. Treat `dev` as the integration branch for research and future separately authorized development.
3. Each agent works in `docs/reviews/<agent>` or a narrow feature branch based on `dev`. Documentation may be proposed before decisions are ratified. Do not allow multi-agent writes on the same files unsupervised.
4. Submit PR → `dev`, capture primary sources and SHA. Reviewer approves/requests changes, **not** a model vote.
5. Promote `dev` → `main` only after documentary review; this promotion does not itself mark any ADR ACCEPTED or authorize R0/R1.
6. When a future scoped GO is given, include its ID, paths, budget, test suite, rollback/stop conditions and safety proof in the PR.

## GitHub administration to configure manually

Connector permissions do not provide administration for branch rules/labels/milestones. Suggested **human-admin** settings:
- Protect `main`: require PR, at least one independent approving review, block force pushes and deletions; require CI checks **only after a real safe CI workflow exists**.
- Protect `dev`: block force pushes/deletions, prefer PR with review; no mandatory broken/unconfigured check.
- Consider labels `type:research`, `type:adr`, `type:story`, `prio:P0/P1/P2`, `gate:D0..D4`, `phase:R0..R7`, `blocked:go`, `risk:P0`; currently **NOT configured automatically**.
- Optional future milestones `D0-D4 documentary`, `R0 fixtures`, `R1-R4 core`, `R5 integrations`, `R6-R7 optional` — never assume they exist until verified.
- Do not enable an empty or dummy CI to pretend checks have passed. No workflow executing untrusted code in the current planning stage.

## Review checklist

See [PR template](../.github/PULL_REQUEST_TEMPLATE.md), [decision register](DECISION_REGISTER.md), and [GOAL](DEEPSEEK_GOAL.md).
