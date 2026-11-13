# Backlog historique — 26/26 tâches, aucun développement autorisé

**Source :** V6 archive, annexe BACKLOG historique. Les intitulés ci-dessous sont repris pour conserver les identifiants. Les phases sont de **nouvelles propositions de mapping**, et les prérequis historiques ne valent pas autorisation de commencer. Le benchmark S8 est passé ; RF-003/022 doivent être réexaminés sous S9, avec de vrais worktrees et une option zéro.

| ID | Priorité | Tâche historique | Dépendances originales | Gate historique | Mapping proposé |
|---|---|---|---|---|---|
| RF-001 | P0 | Décision de nom, visibilité et licence ; vérification droits auteur | — | Décisions humaines consignées | D0/D4 |
| RF-002 | P0 | Inventaire API/dépendances/tests des composants Ravum transférables | RF-001 | Matrice + SHA + provenance | D0/R0 |
| RF-003 | P0 | Benchmark S8 pré-enregistré : A/B/C, fixtures et charge | — | Baselines reproductibles, métriques brutes | D3/R0 |
| RF-004 | P0 | Protocole SourceSnapshot + ChangeProposal versionné | RF-001 | Compat tests/cas négatifs | R1 |
| RF-005 | P0 | CAS local, journal durable et GC borné | RF-004 | Corruption/troncature détectées | R1 (CAS conditionnel) |
| RF-006 | P0 | CLI snapshot/status/doctor, hors ligne | RF-004/005 | Pas de modification des sources utilisateur | R1 |
| RF-007 | P0 | Proposition patch sur base immuable + CAS | RF-005 | 0 écrasement silencieux | R2 |
| RF-008 | P0 | Conflits rename/hunk/same-symbol, rebase et undo | RF-007 | Corpus adversarial complet | R2 |
| RF-009 | P0 | Recovery crash journal/MVCC + chaos tests | RF-007 | Reprise idempotente | R4 |
| RF-010 | P0 | Writer Git transactionnel et hooks | RF-007, RF-002 | CAS ref + crash tests multi-OS | R3/R4 |
| RF-011 | P0 | Protocole d'évaluation v2 (texte ≠ sémantique) | RF-004/007 | Pas de AUTO_SAFE textuel | R1/R3 |
| RF-012 | P0 | Adapter Rust : cargo metadata, tests et diagnostics | RF-011, RF-002 | Propagation inter-crates réelle | R3 |
| RF-013 | P0 | Scheduler, annulation, budgets, coalescence OFF | RF-005/011 | Résultats stale jamais acceptés | R3/R5 |
| RF-014 | P0 | Cache et identity context complète | RF-012/013 | Tests cache poison et changements externes | R3/R5 |
| RF-015 | P0 | CI conservatrice impact + E2E et merge suite | RF-012/014 | Tous les échecs pertinents visibles | R3/R5 |
| RF-016 | P1 | Protocol JSONL stable / SDK agents | RF-006/011 | 3 agents indépendants | R5 |
| RF-017 | P1 | UI revue des patches et politique de publication | RF-016 | Décision humaine traçable | R5 |
| RF-018 | P1 | Adapter TypeScript/JavaScript | RF-013/014 | TS rouge→vert + lockfile invalidation | R5 |
| RF-019 | P1 | Adapter Python | RF-013/014 | Imports dynamiques fallback conservateur | R5 |
| RF-020 | P1 | Adapter C/C++ | RF-013/014 | Headers/options/ABI invalidations | R5 |
| RF-021 | P1 | CI et publication GitHub/GitLab | RF-010/015 | PR et MR avec reçu exact | R6 |
| RF-022 | P1 | Benchmark S8 D (Flow) vs alternatives avec vrais agents | RF-016/015 | Mesures contradictoires de coût, sûreté, variance | D3/R0 puis R5 |
| RF-023 | P2 | Overlay VFS sécurisé / intégrations IDE | RF-007/016 | Tests OS et confinement | R7 DEFER |
| RF-024 | P2 | Sync multi-machines/auth/RBAC/leases | RF-010/016 | Aucune divergence sous partitions | R7 DEFER |
| RF-025 | P2 | CAS distant, remote executors et mono-repo géant | RF-024 | Isolation et performances mesurées | R7 DEFER |
| RF-026 | P2 | Adapter natif Ravum et PatchHost interface | RF-013/014 | Réversible ; fail-closed et RT | R6 opt-in |

## Gestion des tickets

Chaque issue GitHub correspond à un identifiant RF stable. Les tickets R1–R7 sont **BACKLOG / BLOCKED-BY-GO** jusqu'à une autorisation explicite. Aucun agent ne peut considérer une issue ouverte comme permission de coder.

Une modification de mapping doit mettre à jour cette table et la traçabilité ; ne jamais effacer la formulation historique. Les 26 IDs ont été créés avant toute priorisation technique. Pour les détails des critères de non-régression : [security tests](SECURITY_TESTS.md), [roadmap](ROADMAP.md), [S9](BENCHMARK_S9.md), [V6 complete](reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md).
