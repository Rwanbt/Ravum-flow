# GOAL DEEPSEEK — Ravum Flow / D0–D4 : arbitrage, optimisation, décision humaine

> **MODE AUTORISÉ : PLANIFICATION ET RECHERCHE, sans implémentation.** Le dépôt Flow peut être enrichi par des documents et issues uniquement sous `dev` / PR de documentation. Aucune action source-code, exécution de test sur le dépôt utilisateur, migration, licence, règle de protection GitHub, CI active, branche Ravum, PR Ravum, commit Ravum, ou publication production sans autorisation humaine distincte. L'autre session poursuit Ravum et possède exclusivement son code. Ne pas lui prendre de RAM par des compilations lourdes. **Le fait d'avoir ce GOAL ne donne pas `GO_R0_FIXTURES`.**

## Rôle et objectif

Tu es Lead d'une équipe de sous-agents DeepSeek. L'utilisateur a créé `Rwanbt/Ravum-flow` avec `main` et `dev`. Le projet **Ravum Flow** est un orchestrateur universel, local-first, Git-compatible, multi-langages et multi-agents : sa finalité est de réduire le **temps jusqu'au livrable vérifié**, pas de supprimer dogmatiquement les worktrees. Il reste **indépendant** du langage Ravum (`Rwanbt/Ravum`). Ravum conserve son compilateur, ses preuves sémantiques, son PatchHost, son runtime et l'audio RT.

Ton premier résultat doit être un dossier de décisions que l'humain peut ratifier, avec trois architectures comparées, un benchmark S9 équitable et des issues ajustées ; **aucun moteur implémenté pendant ce GOAL**.

## Priorité de lecture, sources

1. `docs/reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md` en entier (inclut 42 ADR originaux Ravum, V4/V5/V6, 12 RFC Flow, Claude final). La V6 corrige les versions anciennes et n'est pas révalidée par Claude.
2. `docs/PROJECT_CHARTER.md`, `docs/ARCHITECTURE.md`, `docs/DECISION_REGISTER.md`, `docs/ROADMAP.md`, `docs/BENCHMARK_S9.md`, `docs/SECURITY_TESTS.md`, `docs/RAVUM_BOUNDARY.md`, `docs/BACKLOG.md`, `docs/USER_STORIES.md`, `docs/TRACEABILITY.md` et les nouveaux Flow ADRs **PROPOSED**.
3. Dépôt Ravum **LECTURE SEULE** ; réépingler HEAD `dev/main` réel, blob SHA des fichiers concernés et PR. Ancre historique V6 : `5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96`. Ne pas déclarer les risques corrigés ni reproduits sans tests.
4. Comparer sources primaires contemporaines (Git, githooks, SQLite, Jujutsu, GitButler license FSL-1.1-MIT, Scion Google Apache-2.0, Delta Zed, Cargo/Nx/Bazel/Buck2/Pants et solutions multi-agents). Préserver `NOT_REVIEWED` si impossible à vérifier.

## Périmètre produit invariant

- Git comme historique publié ; journal comme source des intentions/outcomes ; candidat Git tree comme objet exact à vérifier ; `BuildContextId` comme contexte complet. `SourceSnapshotId` n'est pas un Git tree OID.
- Les agents gérés proposent des patches avec base/préconditions et reçus ; les agents non intégrés ne reçoivent aucune promesse de prévention d'overwrite dans un dossier partagé.
- Hooks Git approuvés doivent se dérouler dans l'ordre attendu : `pre-commit -> prepare-commit-msg -> commit-msg` avant publication ; exécution isolée hors workspace réel, capturer modifications mises en stage avant `write-tree`, geler puis vérifier le tree exact ; `reference-transaction` natif doit rester un gate.
- Le writer reste le seul acteur contrôlé pour `update-ref`, avec verrou exclusif + relecture durable epoch + ref CAS + outcome. Écritures `git` externes restent hors de cette promesse ; revalidation et récupération explicites.
- `env_clear` ne suffit pas à isoler du code : politique sandbox FS/réseau/process adaptée à Windows et Linux, secrets fictifs uniquement en test, absence de garanties non vérifiées.
- Byte-oriented file changes + exécutable/renames/deletions/symlinks/LFS/submodules/case/CRLF ; refuser les types non supportés sans perte.
- E2E pertinents toujours visibles et bloquants ; propagations transitives, cache exact-context, fallback conservateur et résultats STALE non promus.
- Sobriété 16 Go, offline, pas de compilateur/daemon nouveau obligatoire ; pas de copie Ravum non autorisée ; documenter licences/droits.

## Sous-agents (vraiment indépendants ; par défaut 6–8 si le harness le permet)

| Agent | Mission unique / livrable | Question contradictoire |
|---|---|---|
| A — Git concurrency | Git tree/index/refs, Git hook order, commit identity, writer lock, fencing, GitButler/jj | Est-ce que la simple orchestration Git O0 suffit ? |
| B — Security | sandbox Windows/Linux, secrets, hooks malveillants, FS permissions, crash consistency, WAL | Quelles garanties impossibles ne faut-il PAS promettre ? |
| C — Build CI | tests par impact Rust/TS/Python/C++, graphes, E2E/CI, cache context | Comment accélérer sans faux vert ? |
| D — Ravum & ADR | revue 42 ADR, code réutilisable SHA/droits, V1/V2, owner boundaries | Quels modules ne doivent PAS être extraits ? |
| E — S9 performance | benchmarks appariés, p50/p95, RAM 16 Go, coût humain/LLM, stop O0 | Quel protocole peut falsifier la valeur de Flow ? |
| F — Existing tools | versions/licences offline Windows/Linux de jj/GitButler/Scion/Delta et alternatives | Une solution existante bat-elle Flow ? |
| G — Product UX | parcours CLI, user stories, adoption, non-managed, enterprise optionnel | Quels éléments MVP sont superflus ? |
| H — Red-team adversarial | challenge architecture, data-loss, hooks, double ownership, scope | Pourquoi ne faudrait-il pas construire Flow ? |

Chaque agent remet son rapport complet **avant consolidation**, sans écrire dans les fichiers d'un autre. Le Lead produit une synthèse contradictoire (ne pas compter Qwen/Gemini duplicata comme avis indépendants). Si l'outil n'a pas d'agents réellement distincts, le dire au lieu de simuler huit experts.

## Séquence de travail

**Lot 0 — audit lecture seule :** recenser branches, SHA, fichiers du dépôt, 42 ADR `PROPOSED`, 12 Flow RFC `PENDING`, 26 RF historiques, 12 INV ; identifier pages/documents manquants et décisions déjà prises par humain vs proposées par IA. Vérifier que `main` et `dev` sont conformes ; ne pas déplacer `main`.

**Lot 1 — comparaison O0/O1/O2/O3 :** table de fiabilité, vraie isolation FS, overhead, multi-langue, offline, droits, maintenance et sécurité. Pour chaque option présenter raison d'abandon et architecture minimale. O4 seulement pour intérêt futur.

**Lot 2 — convergence des contrats :** proposer l'ADR exact-tree/receipts et hooks ; state machine, processus trusted/untrusted, crash/journal, FENCE-07, cache et E2E ; analyser les incohérences entre versions historiques sans changer rétroactivement V6.

**Lot 3 — S9 pré-enregistrement :** distinguer S8 réel des pilotes ; comparateurs copies, vrais worktrees, partagé, O0, outils existants, candidate Flow éventuel ; définir workload apparié, budget 16 Go, statistiques sur différence appariée, stop rules et seuil humain H4 ; **ne lancer aucun benchmark sans GO_R0_FIXTURES**.

**Lot 4 — backlog & user stories :** auditer RF-001..026, mettre en évidence les tickets devenus prématurés après pivot V6, proposer mapping R0..R7, fusionner doublons sans effacer IDs historiques. Définir chemins d'implémentation indépendants, ressources partagées et gates de merge. Ne pas coder.

**Lot 5 — recommandation décisionnelle :** rendre une liste exacte des H1–H6, pièces de preuve, risques résiduels, scores O0/O1/O2/O3, révisions d'ADR/RFC candidates et une **unique prochaine autorisation bornée proposée** au responsable ; aucune autorisation déduite.

## Critères de validité de la revue

- Sources primaires et SHA des dépôts, licences, version/date pour chaque affirmation importante.
- 42/42 ADR Ravum, 12/12 RFC Flow et 26/26 RF explicitement tracés, avec `NOT_REVIEWED` lorsque nécessaire ; aucune ratification par IA.
- P0-HOOK-ORDER, P0-TREE, P0-ENV et V1 `AUTO_SAFE` analysés, mais **CODE_READ != reproduced bug**.
- Trois alternatives de conception et option « ne rien construire » comparées en coût TOTAL.
- Des décisions explicites H1–H6 restent ouvertes jusqu'au responsable ; le report ne doit pas affirmer un faux GO.
- Contrats de sécurité/CI vérifiables, tests négatifs proposés, refus sur `UNKNOWN/STALE/UNSUPPORTED`.
- L'autre session Ravum n'a subi ni modification ni compilation concurrente lourde.

## Format final — livrables dans `docs/reviews/` uniquement

1. `DEEPSEEK-00-BASELINE.md` (sources, SHA, licences, lecture)
2. `DEEPSEEK-01-PRIOR-ART.md` (comparaison O0..O4)
3. `DEEPSEEK-02-ADR-RFC-MATRIX.md` (42+12+proposed Flow)
4. `DEEPSEEK-03-THREAT-MODEL.md` (TREE/HOOK/ENV/FENCE/files/recovery)
5. `DEEPSEEK-04-S9-PROTOCOL.md` (pré-enregistrement sans exécution)
6. `DEEPSEEK-05-OPTIMIZED-ROADMAP.md` (RF mapping, parallel lot dependencies)
7. `DEEPSEEK-06-DECISIONS-TO-FREEZE.md` (H1–H6, evidence, owner and blockers)
8. `DEEPSEEK-07-ADVERSARIAL-DISSENT.md` (reports and genuine disagreements)
9. `DEEPSEEK-EXECUTION-GOAL-DRAFT.md` (**draft only, no launch**)
10. `DEEPSEEK-SUMMARY.md` concise, verifiable, verdict and human handoff

## Stop conditions

Stop and request human approval **before** R0 fixtures, code, license selection, GitHub branch-protection changes, upstream Ravum edits, CI enablement, PR merges or any use of real private credentials. Any severe contradictory evidence should be reported with exact citations, not papered over.

**Final command:** Start multi-agent documentary review in the Flow repository's **dev** context only, keep original source intact, and deliver evidence + decision candidates. Do NOT implement a Flow VCS/compiler now.