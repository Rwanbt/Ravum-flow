# DEEPSEEK-RAW-E — S9 : critique de S8, protocole pré-enregistré, seuils H4

> Agent E (S9 performance) · 2026-11-09 · branche `docs/deepseek-d0-d4-review` @ `1d5fddf` · statut `RAW / PROPOSED`, **aucun run exécuté**. Ce rapport est brut : le Lead consolide, je ne ratifie rien.

## 1. Mission et méthode

Concevoir un S9 pré-enregistré et falsifiable (comparateurs, workloads appariés, statistique, coûts, stop rules, seuils H4), sans l'exécuter, et dire ce que S8 prouve exactement.
Lu : `AGENTS.md` ; `docs/DEEPSEEK_GOAL.md` (Lot 3) ; V6 §6 (l. ~282–302), §9, §10 ; `docs/BENCHMARK_S9.md` ; `docs/SECURITY_TESTS.md` ; `docs/adr/FLOW-ADR-008.md` ; `docs/ROADMAP.md` ; `docs/BACKLOG.md` (RF-003, RF-022) ; `docs/USER_STORIES.md` (US-004, US-011) ; `docs/reviews/DEEPSEEK-00-BASELINE.md` ; et, en lecture seule HTTPS au SHA épinglé, l'artefact S8 `evidence/mvp0c-s8-live-summary-2026-11-09.json` et `docs/mvp/MVP-0c-s8.md`.
Interdits respectés : aucun benchmark, build, test, CI, daemon, changement de branche, `git add/commit/push`, clone ou écriture Ravum. Aucun chiffre inventé : `MEASURED_HISTORICAL` (cité), `DERIVED` (arithmétique sur valeurs citées), `ESTIMATE` (hypothèse explicite), `PROPOSED`, `NOT_REVIEWED`.
Faits baseline du Lead, cités sans re-mesure : machine cible Windows/Linux, 16 Go, i7 10C/16T, ~188 Go libres, Git 2.51.1, rustc 1.95.0, node 26.13.0, Python 3.13.7, CMake 4.1.1, Ninja 1.13.1 ; Ravum `dev=5408d9b3`, artefact S8 au SHA `6b0ac7b6`.

## 2. Critique de S8 réel

### 2.1 Faits cités (`MEASURED_HISTORICAL`)

| Bras | Échantillons (ms) | Médiane | Min–max |
|---|---|---:|---:|
| A — copies privées + intégration | 100 664 / 111 110 / 208 694 | **111 110** | 100 664–208 694 |
| B — dossier partagé + garde | 126 224 / 237 674 / 286 275 | **237 674** | 126 224–286 275 |
| C — séquentiel | 269 594 / 528 481 / 611 003 | **528 481** | 269 594–611 003 |

n = 3 campagnes par bras (3 agents par campagne, 27 sessions DSH au total). **L'unité indépendante est la campagne (n=3), pas l'agent (n=27)** : les 3 agents d'une campagne partagent la fixture, l'instant et le Lead, donc ne sont pas des réplicats indépendants. `zero_lost` et `all_tests_green` sont **auto-déclarés** par le harnais, pas observés par un tiers.

### 2.2 Ce que S8 prouve

1. Le harnais DSH peut exécuter 27 sessions, attribuer les tokens par marqueur (3/3 par campagne, 0 non attribuée), collecter des reçus, vérifier 12/12 tests et détecter une collision injectée au pilote.
2. Sur cette fixture, ce modèle et cette garde, l'hypothèse a priori « dossier partagé = plus rapide » **n'est pas observée** : B est plus lent que A dans les 3 campagnes appariées.
3. Les 9 campagnes préservent les 4 éditions (0 perte déclarée, fichiers protégés intacts, 12/12) : condition nécessaire, pas preuve de sûreté.
4. Le pilote a un vrai pré-enregistrement (SHA-256 du protocole au démarrage) : bonne pratique à conserver.

### 2.3 Ce que S8 ne prouve pas

- **Aucun vrai `git worktree`** : A = copies de répertoire + fusion 3-way en mémoire ; B = garde de version, pas le coordinateur Flow ; C = séquentiel.
- Aucune option O0 (Git natif + coordination minimale + CI existante), aucun jj/GitButler, aucun Flow.
- Aucune publication ni CI distante : seul `cargo test` local (12 tests) est dans la boucle.
- Aucune échelle : 3 modules, 4 tâches, 12 tests ; au pilote, `cargo test` pèse 84–90 % du total et `target/` fait 11,3 Mo (`MEASURED_HISTORICAL`).
- Aucun corpus adversarial : ni crash, ni hooks, ni binaire/mode/symlink, ni rename/delete, ni API cassée + consommateur inchangé, ni E2E rouge, ni dirty/untracked, ni secret canari, ni réseau.
- Pas de contraste froid/chaud (cache `target/` froid à chaque répétition), pas de contention de build (les agents n'ont pas le droit de lancer `cargo`).
- Pas de supervision humaine ventilée : `total_ms` inclut préparation, tours LLM et orchestration du Lead.
- Aucune signification statistique, aucun IC, aucun effet apparié estimé ; pas de coût de maintenance ni de distribution ; pas de stress 16 Go (3 agents, `CARGO_BUILD_JOBS=2`).

### 2.4 Usage autorisé de S8

**Ordre de grandeur uniquement.** S8 interdit de présupposer que le partage direct est plus rapide ; il n'autorise ni un classement, ni un seuil, ni la phrase « les copies privées sont supérieures ». Les plages A et B se chevauchent (A max 208 694 > B min 126 224) : **l'ordre A<B<C n'est pas démontré**.

### 2.5 Défaut d'estimateur (DERIVED)

Le rapport S8 cite « +114 % » pour B vs A : c'est le **rapport des médianes** (237 674 / 111 110 = 2,139), pas l'effet apparié. Par répétition, B/A vaut 1,254 / 2,139 / 1,372 → **ratio médian apparié 1,372 (+37 %)**, moyenne géométrique 1,544 (+54 %). Pour A→C : rapport des médianes 4,756 (+376 %) contre ratio médian apparié 2,928 (+193 %). Le test du signe 3/3 vaut p = 0,125 (unilatéral), non significatif. Conclusion : l'effet est **consistant en signe mais instable en taille et non significatif** ; « +114 % » n'est pas l'effet apparié — l'incrément médian est +37 %, environ trois fois moins. Tout S9 doit interdire le rapport des médianes et publier l'estimateur apparié.

### 2.6 La métrique n'est pas identifiée (DERIVED)

`total_ms` = préparation → collecte. En retranchant le temps du plus long agent (A/B) ou la somme des agents (C), le résidu (préparation, intégration, collecte, orchestration du Lead) va de **3 873 ms à 444 383 ms**, soit 2 % à 82,5 % du total selon la campagne. La part attribuable à la stratégie n'est donc pas identifiée : S8 mesure surtout une **orchestration d'agents LLM**, pas la mécanique VCS. Un S9 doit décomposer `T_prep`, `T_agent`, `T_integrate`, `T_verify`, `T_publish`, `T_orchestration`.

### 2.7 Pilotes antérieurs inagrégeables

| Série | Valeurs (ms) | Pourquoi non agrégeable |
|---|---|---|
| MVP-0b scripté | A 5 660,6 / B 6 677,6 / C 4 785,6 | A = vrais worktrees + 3 merges (intégration 1 243 ms) ; B = coordinateur snapshot+journal+GitWriter ; 6 tâches, 5 reps ; autre corpus |
| MVP-0c pilote scripté | 2 650,4 / 3 061,5 / 2 914,3 | agents déterministes, tokens non mesurés, `total` = agents→tests verts (hors préparation) ; A/C se chevauchent (A max 2 920 > C min 2 849) |
| MVP-0c pilote, 2ᵉ exécution | ~2 434 / 2 825 / 2 657 | même protocole, écart 5–10 % sur machine partagée : deux séries réelles, non fusionnables |
| S8 live (LLM) | 111 110 / 237 674 / 528 481 | `total` inclut préparation + supervision ; échelle 42× le pilote ; autre unité |

Raison générale : unités (ms scripté vs 100 s LLM), bras (worktree vs copie vs coordinateur), frontières de métrique (préparation incluse ou non), n, fixtures, charge machine et randomisation différents. Les agréger violerait l'échangeabilité et fabriquerait un faux « dose-réponse ».

### 2.8 « Partagé plus lent » : plausible, mécanisme non démontré

Mécanismes candidats : **M1** réessai de la garde (relecture + écriture, aller-retour d'outil, éventuellement un tour LLM) ; **M2** arbre transitoirement incohérent visible par les autres agents, build/test qui échoue puis rework ; **M3** orchestration du Lead (résolution de collision) ; **M4** verrou du `target/` partagé (Cargo sérialise les builds) ; **M5** invalidation de cache incrémental par mtimes changeants ; **M6** contention NTFS/antivirus sur un répertoire unique. S8 n'exerce ni M4 ni M5 (build unique, cache froid, agents sans `cargo`), ne rapporte pas les collisions live, et les tokens de sortie de B (42 378) ne dépassent pas ceux de A (45 240) : **M1 n'est pas confirmé par les compteurs**. B reste plausiblement plus lent, mais l'attribution causale est `UNKNOWN` ; S8 ne peut pas conclure « le partage est intrinsèquement plus lent ».

## 3. Comparateurs S9

Lettrage **figé** (le dépôt en contient trois, cf. `DOC-03`/`DOC-05`) : V6 §6.2 met D=O0/E=Flow/F=outil/G=séquentiel ; `BENCHMARK_S9.md` met D=séquentiel/E=outil/F=O0/G=Flow ; la mission met D=séquentiel/E=O0/F=outil/G=Flow. **J'adopte la mission** et j'exige que le pré-enregistrement gèle cette table avant H4 ; sinon les résultats seront mal étiquetés.

| Bras | Opérationnalisation | Ce qu'elle isole | Disponibilité | Piège à éviter |
|---|---|---|---|---|
| **A** copies privées | une copie du tree suivi par agent + intégration ; politique de cache épinglée (`A-cold` / `A-warm`) | absence d'arbre mutable partagé | `AVAILABLE` | ne copier que le suivi rend A artificiellement bon ; déclarer `target/`/`node_modules` |
| **B** vrais worktrees | `git worktree add` éphémère par agent ; `add` et `remove` chronométrés séparément ; `.git` partagé ; `core.symlinks`/`autocrlf`/`longpaths` épinglés | isolation Git réelle | `AVAILABLE` | fichiers ignorés non copiés ⇒ build froid ; `remove` peut échouer sur handles ouverts |
| **C** shared+guard | un répertoire, garde optimiste ; granularité épinglée (fichier / arbre / chemin), politique de réessai et max d'essais | disque partagé + coordination | `AVAILABLE` | une garde grossière est un homme de paille ; implémenter la meilleure garde fine |
| **D** séquentiel | un workspace, agents un par un, même travail total | coût de la sérialisation (dénominateur) | `AVAILABLE` | ce n'est pas un concurrent ; le sortir de la famille primaire |
| **E** O0 | Git natif + CI existante + coordination humaine minimale, **sans nouveau code Flow** ; worktrees/branches autorisés | « ne rien construire de nouveau » | `AVAILABLE` | O0 ne doit pas contenir de « coordinateur minimal » (ce serait déjà construire) ; O0 doit être opéré par un expert motivé |
| **F** outil existant | jj ou GitButler, si offline Windows/Linux, licence compatible, mêmes agents/prompts/CI, isolation comparable | valeur d'un outil existant | à statuer par l'agent F ; sinon `NOT_AVAILABLE(reason)` | GUI-only vs CLI, cloud-only, autre catégorie de produit : comparaison malhonnête |
| **G** Flow | `MANAGED_PATCH` (O2) puis O3 hybride, prototype versionné, SHA épinglé, même API agents | valeur incrémentale de Flow | **`NOT_AVAILABLE` aujourd'hui** (Flow sans code ; 47 issues, PR #33) | un mock/simulateur de G pour un résultat confirmatoire est interdit |

`NOT_AVAILABLE` est un statut, pas un échec : l'arme est exclue de la famille primaire, listée à part, et n'est jamais comptée comme une perte. Si G est `NOT_AVAILABLE`, **l'étage 2 ne peut pas tourner** ; l'étage 1 (falsifier la prémisse) reste possible. Statuts : `AVAILABLE | NOT_AVAILABLE(reason) | INVALID(reason) | FAILED(reason)`.

## 4. Workloads appariés et critère primaire

**Unité d'appariement** : l'instance = (cellule de workload, fixture, commit de base, jeu de tâches, modèle, prompt, régime de cache). Une instance donne **un run par bras**, dans un ordre randomisé ; la campagne est l'unité d'analyse. Les agents sont nichés : jamais n = nombre d'agents.

| ID | Workload | Mécanisme testé |
|---|---|---|
| W0 | 1 agent, petit dépôt | plancher de surcoût de coordination (Flow ne doit pas perdre ici) |
| W1 | 3 agents, 3 fichiers disjoints | contrôle sans conflit |
| W2 | 2 agents, même fichier, hunks/fonctions disjoints | cas S8 (t1/t3) |
| W3 | même hunk, intentions compatibles | fusion textuelle + vérification sémantique |
| W4 | même symbole, intentions incompatibles | conflit sémantique réel |
| W5 | rename (agent 1) + usage/import (agent 2) | conflit de chemin |
| W6 | suppression (agent 1) + modification (agent 2) | delete/modify |
| W7 | binaire + mode 100755 + symlink 120000 + casse Unicode (Windows) | fidélité des types |
| W8 | API cassée + consommateur inchangé | build doit échouer ; tests ciblés ne masquent pas |
| W9 | E2E rouge préexistant | rouge visible, pas de faux vert |
| W10 | cache froid vs chaud (2 régimes × tous bras) | coût de setup/cache |
| W11 | dirty/untracked au t0 | préservation sans reset |
| W12 | crash injecté (agent/coordinateur) + reprise | recovery idempotente |
| W13 | hooks : pre-commit modifie l'index ; hook refuse ; hook écrit un fichier dirty | ordre et isolation |
| W14 | offline ; CI distante indisponible/queued | publication bloquée/queued, pas de faux vert |
| W15 | lockfile/toolchain/flags changés | invalidation de cache |
| W16 | échelle S/M/L (Rust multi-crate ; Rust+TS+Python) sous 16 Go | effets d'échelle |

**Critère primaire** : `T_verified` = `t_qualified − t0`, où `t0` = remise du prompt au premier agent (à figer), et `t_qualified` = horodatage où un **Git tree OID gelé** a passé toutes les vérifications requises dans le `BuildContextId` et la `PolicyVersion` déclarés, et — si la politique de la tâche l'exige — où la **CI distante au tree de merge-résultat** est verte. « Qualifié » exige : tree OID enregistré, checks exécutés depuis ce tree, aucun `UNKNOWN/STALE/SKIPPED` promu, E2E requis exécuté, zéro perte. Si aucun tree qualifié avant `T_cap`, le run est un **échec** : pas d'imputation de temps ; rapporter le taux d'échec et, si analyse de survie, censurer à `T_cap`. Un test local vert ne vaut pas qualification si la politique exige la CI distante.

Secondaires : `T_published`, `T_first_green`, temps de rework, conflits (nombre/classe/résolution), tentatives, pertes (0 exigé), E2E exécutés/omis avec raisons, CPU, **RSS agrégée pic**, swap, disque pic/IO, tokens (input/output/cache), supervision humaine (durée, interventions), ergonomie, coût de maintenance.

## 5. Design statistique

- **Différences appariées** : pour chaque instance i, `d_i = ln(T_G,i / T_E,i)`. Estimer le ratio médian apparié, la moyenne géométrique et un **IC bootstrap BCa** (10 000 rééchantillonnages d'instances), + la différence absolue en secondes.
- **Interdit** : exiger que deux IC **marginaux** ne se chevauchent pas (l'erreur déjà corrigée par V6) ; comparer des médianes sans structure appariée ; rapporter un rapport de médianes comme effet apparié (l'erreur S8) ; p-value sur n=3.
- **Exploratoire** : n ≥ 8 instances (V6), pour estimer `σ_d` et la part `T_coord/T_total` ; aucun test d'hypothèse.
- **Confirmatoire** : `n = ceil((z_{1−α/2}+z_{1−β})² · σ_d² / (ln(1−δ))²)`, `σ_d` gonflé d'un facteur de prudence pré-enregistré ; si `n` dépasse le budget, **ne pas lancer** et rapporter « non démontrable dans le budget » (règle honnête).
- **Ordre/blocage** : chaque instance exécute tous les bras d'affilée, ordre randomisé (carré latin pour k bras), rotation entre jours ; sonde de charge avant/après ; refroidissement entre blocs ; dérive thermique en covariable si mesurable ; aucune exclusion post-hoc d'un run lent.
- **Apprentissage** : session d'agent neuve à chaque run, même prompt ; si la même classe est répétée, contrebalancer ; un warm-up par bras exclu a priori.
- **Supervision humaine** : chronomètre, `t0`/`t_end` figés, journal des interventions (durée, raison) ; superviseur aveugle au seuil (et au bras si possible) ; double mesure sur ≥20 % des runs, désaccord rapporté.
- **Tokens/coût** : tokens déclarés par le fournisseur ; grille de prix datée et épinglée ; `C_tokens = in·p_in + out·p_out + cache·p_cache` ; jamais `processedTokens` seul (92–96 % de cache dans S8, `DERIVED`). `C_total = C_tokens + C_human + C_infra + C_maintenance`.
- **Multiplicité** : un contraste primaire (G vs E) ; secondaires descriptifs ou Holm ; aucune pêche aux sous-groupes ; classes de déploiement pré-enregistrées.

## 6. Coût des vrais worktrees

**Faits structurels** (`SOURCE_READ`, non mesurés ici) : `git worktree add` écrit le tree suivi (checkout) mais **ne duplique pas les objets** `.git` ; il **ne copie pas** les fichiers ignorés (`target/`, `node_modules`, `.venv`) ; `remove` supprime le tree et peut échouer sur handles ouverts, d'où `prune`. Windows : antivirus (Defender) scanne créations et suppressions, NTFS coûteux en petits fichiers, `MAX_PATH`, symlinks soumis à privilège (`core.symlinks=false` par défaut ⇒ mode 120000 non fidèle), CRLF. Linux : pas d'AV par défaut, reflink (btrfs/XFS) peut rendre la copie quasi gratuite, overlayfs possible, limites inotify. Cargo sérialise les builds partageant un `target/` (verrou) : cache partagé = attente, caches séparés = disque.

**Modèle paramétré** (aucune valeur supposée) :

```text
n        agents par campagne ; q ∈ {cold, warm_copy, sccache, shared}
S_tr/F_tr  octets/nb de fichiers suivis ; S_dep/S_bld  dépendances/build
v_copy   débit copie (o/s) ; v_fs  créations/s ; t_av  scan AV/fichier
t_wt     surcoût fixe worktree add ; t_rm  suppression/fichier
T_A_setup = n·(S_tr/v_copy + F_tr/v_fs + [q=warm_copy]·(S_dep+S_bld)/v_copy + [q=cold]·(t_dep+t_bld_cold))
T_B_setup = n·(t_wt + S_tr/v_copy + F_tr·(1/v_fs + t_av) + [q=cold]·(t_dep+t_bld_cold)
              + [q=warm_copy]·(S_dep+S_bld)/v_copy + [q=sccache]·(t_bld_cold·miss + t_bld_warm·hit))
T_cleanup_B = n·(t_rm + F_tr·(1/v_fs + t_av))
Disk_A = n·(S_tr + [q=warm_copy]·(S_dep+S_bld))
Disk_B = n·(S_tr + S_dep + S_bld) + S_git_admin
T_total = T_setup + T_agent + T_integrate + T_verify + T_publish + T_orchestration + T_cleanup (+T_lock si q=shared)
```

**Paramètres à calibrer** (P0, après `GO_R0_FIXTURES`, 5 répétitions, médiane [min–max]) : `v_copy, v_fs, t_av, t_wt, t_rm, t_dep, t_bld_cold, t_bld_warm, t_lock, S_*, F_*`. Le fixture S8 (`target/` 11,3 Mo) ne représente pas un dépôt réel : `S_bld` doit être **mesuré**, pas supposé. **Scénarios à rapporter** : `q=cold` (pire cas disque/temps), `q=warm_copy` (copie du cache), `q=sccache` (cache partagé content-addressed), `q=shared` (verrou Cargo). Aucun chiffre de coût n'est avancé ici : le modèle prédit, la calibration décide.

## 7. Stop rules et conditions éliminatoires

**Par run** : violation de protocole (mauvais SHA, prompt, reçu manquant, session non attribuée), `T_cap`, OOM/swap, charge externe au-delà du seuil pré-enregistré ⇒ `INVALID`, re-run unique dans un bloc ultérieur (max pré-enregistré).
**Par bras** (quelle que soit la vitesse) : perte silencieuse d'une modification validée ou dirty ; corruption/récupération non maîtrisée ; publication d'un arbre autre que le qualifié ; fuite de secret fictif en mode annoncé protégé ; E2E requis masqué ; `UNKNOWN` promu `PASSED` ⇒ bras disqualifié, rapporté.
**Par étude** : futilité (effet observé < δ/2 avec IC excluant δ), budget épuisé, `n` requis inatteignable, `σ_d` trop grand ⇒ arrêt et « non démontrable ».
**Règle O0** : si E (O0) — ou F (outil existant) — est **non inférieur** à G dans une marge d'équivalence pré-enregistrée sur `T_verified`, avec zéro condition éliminatoire et un `C_total` plus faible (maintenance/distribution incluse), alors **ne pas construire** les couches transactionnelles de Flow. La formulation V6 « même sûreté et productivité à coût moindre » est infalsifiable sans marge : le pré-enregistrement doit fixer `δ_eq` et une grille de coût de maintenance (LOC, modes de défaillance, charge d'exploitation, licence) **avant** les résultats.

## 8. Seuils H4 candidats et template de pré-enregistrement

Aucun seuil n'est ratifié ici. Les trois candidats portent sur la **réduction du ratio médian apparié de `T_verified` vs le meilleur baseline (E, sinon A/B)**, avec IC 95 % inférieur > 0, `C_total` non inférieur (≤ +10 %, à figer) et zéro condition éliminatoire.

| Candidat | Règle | Justification | Coût/risque |
|---|---|---|---|
| **H4-15** | ≥ 15 % de réduction, IC 95 % bas > 0, `T_coord/T_total` ≥ 20 % sur ≥ 1 classe | plus petit effet qui change une décision ; seuil déjà proposé par la revue antérieure | proche du bruit (5–10 % entre runs S8) ⇒ n élevé, risque de faux négatif |
| **H4-20** | ≥ 20 %, `T_coord` ≥ 25 % | marge au-dessus du bruit ; valeur médiane déjà citée (20 %) | peut être inatteignable si build/CI domine ; faux négatif |
| **H4-25** | ≥ 25 % + plancher absolu `{{ABS_FLOOR}}` s/campagne + déploiement limité aux classes gagnantes | conservateur ; protège contre la complexité pour un gain marginal | peut rejeter un outil utile ; 25 % d'une petite tâche peut être sous le bruit |

**Implications en heures/an** (`ESTIMATE`, hypothèses explicites) : `heures/an = N_livraisons/an × T_tâche(h) × δ`. Exemple illustratif : 900 livraisons/an × 0,33 h × 15 % = **~45 h/an** ; à 20 % = ~60 h/an ; à 25 % = ~75 h/an. Ces valeurs dépendent de `N` et `T` : l'humain doit les remplir ; elles ne sont pas des mesures.

**Coût de campagne** (`ESTIMATE` symbolique) : `R = n_pairs·k + n_warmup·k` runs ; `H_machine = Σ T_run` ; `H_human = Σ T_run·s + analyse` ; `C_tokens = Σ(in·p_in + out·p_out + cache·p_cache)` ; `jours = H_machine / capacité_journalière` (une campagne à la fois sur 16 Go). Le corpus adversarial (crash/hooks/secrets) est budgété séparément.

**Template de pré-enregistrement à trous** — à remplir, hacher et committer **avant** le premier run ; toute modification = nouvelle version + nouveau hash + nouveau run :

```yaml
protocol_version: {{VERSION}}
frozen_at_utc: {{DATE_ISO}}
signatory: {{HUMAN_OWNER}}
doc_sha256: {{HASH_BEFORE_RUN}}
hypothesis: "G réduit T_verified de ≥ {{DELTA}} vs E sur {{TASK_CLASS}}"
primary_contrast: G_vs_E
primary_estimand: paired_median_log_ratio
effect_threshold: {{DELTA}}
equivalence_margin_o0: {{DELTA_EQ}}
absolute_floor_seconds: {{ABS_FLOOR}}
cost_non_inferiority_margin: {{EPS_COST}}
arms:
  A: {cache_policy: {{A_CACHE}}, merge: {{A_MERGE}}}
  B: {worktree_config: {core.symlinks: {{SYMLINKS}}, autocrlf: {{CRLF}}, longpaths: {{LONGPATHS}}},
      cache_policy: {{B_CACHE}}, measure_add_remove: true}
  C: {guard_granularity: {{GUARD}}, max_attempts: {{MAX_ATTEMPTS}}}
  D: {role: denominator}
  E: {o0_definition: "git+ci+human, no new flow code", operator: {{O0_OPERATOR}}}
  F: {tool: {{TOOL}}, version: {{TOOL_VERSION}}, availability: {{AVAIL}}}
  G: {version_sha: {{G_SHA}}, status: {{G_STATUS}}}
fixtures: {sha: {{FIXTURE_SHA}}, cells: {{W_CELLS}}, red_before_proof: {{RED_PROOF}}}
machine: {os: {{OS}}, cpu: {{CPU}}, ram_gb: 16, disk_free_gb: {{DISK}}, av: {{AV}}}
toolchain: {git: 2.51.1, rustc: 1.95.0, node: 26.13.0, python: 3.13.7, cmake: 4.1.1, ninja: 1.13.1}
model: {provider: {{PROVIDER}}, id: {{MODEL}}, temperature: {{TEMP}}, prompt_sha256: {{PROMPT_SHA}}}
design: {n_exploratory: ">=8", n_confirmatory: {{N_CONF}}, alpha: {{ALPHA}}, power: {{POWER}},
         bootstrap_resamples: 10000, order: randomized_block_latin_square, warmup_per_arm: 1}
human: {supervisor: {{SUPERVISOR}}, blinding: {{BLINDING}}, intervention_log: true, hourly_rate: {{RATE}}}
t_cap_seconds: {{T_CAP}}
stop_rules: [protocol_violation, t_cap, oom_swap, external_load, futility, budget]
elimination: [silent_loss, unqualified_publish, secret_leak, masked_e2e, unknown_promoted, unrecoverable_crash]
exclusions: none_post_hoc
rerun_policy: {max_reruns: {{MAX_RERUNS}}, reason_required: true}
analysis_script_sha256: {{ANALYSIS_SHA}}
reporting: {all_runs: true, no_ratio_of_medians: true, no_marginal_ci_test: true, p95_only_if_n_ge: 20}
decision_rule: "superiority AND cost_non_inferiority AND zero_elimination AND o0_not_equivalent"
```

## 9. Réponse contradictoire : quel protocole peut falsifier la valeur de Flow ?

Proposition à falsifier : « G (MANAGED_PATCH/O3) réduit `T_verified` de ≥ δ vs le meilleur baseline, sur au moins une classe, sans régression de sûreté ni de coût total. » Six falsificateurs :

1. **F1 — prémisse** : mesurer `T_coord/T_total` (part du temps attribuable à la coordination VCS, par décomposition). Si elle est < δ sur toutes les classes, aucun coordinateur ne peut produire δ : l'hypothèse est **infalsifiable en pratique** ; déclarer la futilité avant de dépenser en confirmatoire.
2. **F2 — non-infériorité O0** : si E est dans `±δ_eq` et moins coûteux (maintenance incluse), Flow n'a pas de valeur démontrée. C'est une **falsification positive**, pas un échec de détection.
3. **F3 — outil existant** : si F (jj/GitButler) est dans `±δ_eq` et moins coûteux, Flow doit battre F, pas seulement O0 ; sinon P est fausse.
4. **F4 — mécanisme** : si la décomposition montre que les gains de G (setup/intégration) sont toujours inférieurs à ses surcoûts (reçus, sandbox, replay, matérialisation), P est fausse mécaniquement, même si un agrégat semble favorable.
5. **F5 — classe adverse** : sur W0 (1 agent, petit dépôt), G doit être dans le bruit ; s'il est matériellement plus lent, G ne peut pas être le défaut et P (« universel ») est fausse.
6. **F6 — sûreté** : toute condition éliminatoire rend P fausse quelle que soit la vitesse.

Le protocole qui **peut** falsifier Flow est donc : O0 opéré par un expert motivé, mêmes agents/prompts/CI, métrique décomposée, différences appariées, n calibré, seuil et marge figés avant le run, et publication/CI incluse quand la tâche l'exige. Le protocole qui **ne peut pas** le falsifier : O0 homme de paille, n=3, rapport de médianes, IC marginaux, G simulé, ou absence de `T_coord`.

## 10. Désaccords réels, limites, références

**Désaccords** : (1) le « +114 % » de S8 est un rapport de médianes, pas un effet apparié (+37 % médian) ; (2) S8 live affirme un classement que le pilote, au même n=3, refusait explicitement ; (3) V6 §6.2 définit O0 avec un « coordinateur minimal », ce qui n'est plus O0 ; (4) exiger des IC marginaux non chevauchants (recommandation Claude) est le mauvais test ; (5) « 0 perte 9/9 » n'est pas une preuve de sûreté (auto-déclaré, sans corpus adversarial) ; (6) inclure D (séquentiel) dans la famille primaire gonfle l'effet et la multiplicité : c'est un dénominateur ; (7) utiliser `processedTokens` comme coût ignore que 92–96 % sont des lectures de cache.

**Limites / `NOT_REVIEWED`** : aucun run, aucune mesure nouvelle ; je n'ai pas lu les 9 preuves JSON par campagne (seulement l'agrégat et le rapport) ; le hash de protocole du run live n'est pas vérifié ; le code du harnais (`run.mjs`) n'est pas lu, donc la frontière exacte de `total_ms` est `SOURCE_READ` ; jj/GitButler non évalués (agent F) ; les paramètres Windows/Linux du §6 ne sont pas mesurés (`PLAUSIBLE_NOT_REPRODUCED`) ; G est `NOT_AVAILABLE` ; les formules statistiques ne sont pas simulées ; la contention 16 Go avec l'autre session Rust n'est pas testée.

**Références datées (2026-11-09)** : `docs/DEEPSEEK_GOAL.md` ; V6 `docs/reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md` §6 l. 282–302, §9 l. 401–429, §10 l. 433–457 ; `docs/BENCHMARK_S9.md` ; `docs/SECURITY_TESTS.md` ; `docs/adr/FLOW-ADR-008.md` ; `docs/ROADMAP.md` ; `docs/BACKLOG.md` (RF-003, RF-022) ; `docs/USER_STORIES.md` (US-004, US-011) ; `docs/reviews/DEEPSEEK-00-BASELINE.md` (DOC-03/DOC-05) ; Ravum `evidence/mvp0c-s8-live-summary-2026-11-09.json` et `docs/mvp/MVP-0c-s8.md` au SHA `6b0ac7b6` (lecture HTTPS seule, aucun clone/git).
