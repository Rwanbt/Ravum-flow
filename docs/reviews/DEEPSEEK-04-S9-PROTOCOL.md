# DEEPSEEK-04 — PROTOCOLE S9 PRÉ-ENREGISTRÉ (D3, NON EXÉCUTÉ)

**Statut :** `DRAFT / NOT RUN / NOT AUTHORIZED`. Aucun benchmark, aucune mesure, aucun build, aucune fixture exécutée pendant cette revue. Toute exécution exige `GO_R0_FIXTURES` (H5) et un seuil figé (H4).
**Base :** [E](raw/DEEPSEEK-RAW-E-S9-PERFORMANCE.md) (design complet, 227 lignes), V6 §6, [`BENCHMARK_S9.md`](../BENCHMARK_S9.md), [A](raw/DEEPSEEK-RAW-A-GIT-CONCURRENCY.md) §3.4-A-26, [C](raw/DEEPSEEK-RAW-C-BUILD-CI.md) §2.8, [G](raw/DEEPSEEK-RAW-G-PRODUCT-UX.md) §9.4-9.5, [H](raw/DEEPSEEK-RAW-H-ADVERSARIAL.md) §E1.
**Vérification Lead :** l'artefact S8 a été lu directement (voir §2). Les chiffres ne sont pas repris de mémoire.

---

## 1. Pourquoi S9 est le seul falsificateur

Tout le dossier repose sur une affirmation non testée : « un coordinateur de Flow réduit le temps jusqu'au livrable vérifié ». Aucune autre pièce ne peut la réfuter :

- les documents (V4→V6) ne mesurent rien ;
- les revues d'IA ne sont pas des mesures ;
- les 42 ADR ne tranchent pas une question empirique ;
- la seule mesure existante (S8) ne compare **ni** vrais worktrees **ni** Flow.

Sans S9, H1 ne peut être qu'une préférence. **C'est pour cela que H4 et H5 sont les deux décisions qui débloquent tout le reste.**

---

## 2. Audit de S8 — ce que l'artefact dit vraiment

Source : `Rwanbt/Ravum@6b0ac7b6` → `evidence/mvp0c-s8-live-summary-2026-11-09.json` (lu le 2026-11-09). `n = 3` par bras.

| Bras | échantillons (ms) | médiane | plage | chevauchement |
|---|---|---|---|---|
| A — copies privées | 100 664 / 111 110 / 208 694 | 111 110 | 100,7–208,7 s | **avec B** |
| B — dossier partagé + garde | 126 224 / 237 674 / 286 275 | 237 674 | 126,2–286,3 s | **avec A** |
| C — séquentiel | 269 594 / 528 481 / 611 003 | 528 481 | 269,6–611,0 s | avec B |

**Ce que S8 prouve :** des campagnes réelles ont eu lieu (27 sessions), avec des durées mesurées, sur une fixture Rust réduite, avec `zero_lost` et `all_tests_green` **déclarés** à 9/9.

**Ce qu'il ne prouve pas :**

1. **L'ordre A < B < C.** Les plages se chevauchent, n = 3, aucun test de différence appariée. Un rapport de médianes n'est pas un effet apparié (E §5).
2. **L'absence de perte** au sens général : un booléen auto-déclaré sur 9 campagnes n'est pas un test adversarial.
3. **Le coût des worktrees** : aucun `git worktree` n'a été mesuré. **La prémisse du produit n'a jamais été mesurée.**
4. **La comparabilité** : l'agrégat ne porte ni seed, ni SHA de fixture, ni appariement des tâches, ni charge système. Les micro-tâches (7–17 messages/agent) ne représentent pas une tâche de développement.
5. **La généralisation** : fixture unique, machine partagée, un seul opérateur.

### 2.1 Dérivés statistiques (agent E) — deux corrections supplémentaires

L'agent E a dérivé des données par campagne, au-delà de l'agrégat :

- **Un rapport de médianes n'est pas un effet apparié.** `C/A` en médianes = ×4,76 (+376 %) ; l'**effet apparié médian** `A→C` est ×3,93 (**+193 %**). L'écart n'est pas cosmétique : c'est la différence entre « la variante multiplie par 4,8 » et « la variante ajoute un facteur 3,9 sur des paires comparables ». Test du signe 3/3 : **p = 0,125** — aucun résultat significatif à n = 3.
- **La métrique n'est pas identifiée.** `total_ms` inclut la préparation et l'orchestration du Lead, avec un résidu de **3,9 à 444 s (2 % à 82,5 % du total)** selon la campagne. Autrement dit, S8 mesure surtout **l'orchestration LLM**, pas la mécanique VCS. Une part importante du temps total n'est pas attribuée à la stratégie comparée.

**Conséquences obligatoires :**

- S8 ne doit plus être cité comme classement (« copies plus rapide que partagé »). Au mieux : ordre de grandeur et calibration.
- La V6 et `BENCHMARK_S9.md` doivent cesser de présenter 111,1 s / 237,7 s / 528,5 s comme un résultat établissant une hiérarchie.
- Les pilotes scriptés antérieurs (2,65 s / 3,06 s / 2,91 s) ne doivent **jamais** être agrégés : autre workload, autre échelle.

---

## 3. Comparateurs canoniques (résout `X-06`)

`BENCHMARK_S9.md` et la V6 §6.2 utilisent les mêmes lettres pour des bras différents. **Décision de forme proposée (non ratifiée) : la table ci-dessous est la seule nomenclature S9.** L'ancienne table de `BENCHMARK_S9.md:8` est déclarée **superseded** et conservée pour l'historique.

| Lettre | Bras | Définition opérationnelle | Disponibilité à enregistrer |
|---|---|---|---|
| **A** | copies privées | Copie complète du dépôt (suivi + dépendances) par agent, intégration sérialisée. | toujours |
| **B** | **vrais worktrees** | `git worktree add` éphémère par agent ; `add` et `remove` **chronométrés séparément** ; politique `core.symlinks`, `autocrlf`, `longpaths` figée. | toujours |
| **C** | dossier partagé + garde | Un seul dossier, garde optimiste, retry/merge. | toujours |
| **D** | séquentiel témoin | Un agent à la fois, aucun parallélisme. | toujours |
| **E** | **O0** | Git natif + hooks natifs + CI existante + coordination humaine/script, **aucun code Flow**. | toujours |
| **F** | outil existant | `jj`, GitButler, etc. **seulement** si comparable honnêtement (hooks, offline, licence, OS). Sinon `NOT_AVAILABLE`. | conditionnelle |
| **G** | Flow candidat | `MANAGED_PATCH` seul (O2), puis hybride (O3) **quand il existera**. | **pas disponible aujourd'hui** |

> **Règle :** `NOT_AVAILABLE` n'est **jamais** un échec expérimental, et ne doit jamais être remplacé par une variante « approximative » non étiquetée.

**Sans G, S9 ne peut pas décider H1.** Il peut seulement établir la baseline O0/O1-A/B et le coût réel des worktrees — ce qui est déjà **suffisant pour tuer ou réduire le projet** (voir §10). C'est pourquoi la première campagne utile est **S9-baseline (A/B/C/D/E/F)**, indépendante de Flow.

---

## 4. Workloads appariés et critère primaire

**Unité d'appariement :** l'instance = (cellule de workload, fixture, commit de base, jeu de tâches, modèle, prompt, régime de cache). Une instance donne **un run par bras**, ordre randomisé. **L'unité d'analyse est la campagne**, jamais le nombre d'agents.

| ID | Workload | Ce qu'il teste |
|---|---|---|
| W0 | 1 agent, petit dépôt | **plancher** : Flow ne doit pas perdre ici |
| W1 | 3 agents, fichiers disjoints | contrôle sans conflit |
| W2 | 2 agents, même fichier, hunks disjoints | cas S8 |
| W3 | même hunk, intentions compatibles | merge + vérification |
| W4 | même symbole, intentions incompatibles | conflit sémantique |
| W5 | rename + usage/import | conflit de chemin |
| W6 | suppression + modification | delete/modify |
| W7 | binaire + `100755` + `120000` + casse Unicode | fidélité des types |
| W8 | API cassée + consommateur inchangé | pas de « impacted green » |
| W9 | E2E rouge préexistant | rouge visible |
| W10 | cache froid vs chaud (2 régimes) | coût de setup |
| W11 | dirty/untracked au t0 | préservation |
| W12 | crash injecté + reprise | recovery |
| W13 | hooks (modifie l'index / refuse / écrit un dirty) | ordre et isolation |
| W14 | offline / CI indisponible | pas de faux vert |
| W15 | lockfile/toolchain/flags changés | invalidation |
| W16 | échelle S/M/L (Rust multi-crate ; Rust+TS+Python), 16 Go | effets d'échelle |

**Critère primaire — `T_verified` :** `t_qualified − t0`, où `t0` = remise du prompt au premier agent, et `t_qualified` = horodatage où un **Git tree OID gelé** a passé **toutes** les obligations requises dans le `BuildContextId` et la `PolicyVersion` déclarés, et — si la politique l'exige — la **CI distante au tree de merge-résultat**.

« Qualifié » exige : tree OID enregistré ; checks exécutés **depuis ce tree** ; aucun `UNKNOWN/STALE/SKIPPED` promu ; E2E requis exécuté ; zéro perte. Si aucun tree qualifié avant `T_cap`, le run est un **échec** — pas un temps imputé ; rapporter le taux d'échec.

**Secondaires :** `T_published`, temps de rework, conflits (nombre/classe/résolution), tentatives, pertes (0 exigé), E2E exécutés/omis **avec raisons**, CPU, RSS agrégée pic, swap, disque pic/IO, tokens (in/out/cache), supervision humaine (durée, interventions), coût total.

> Un test local vert **ne vaut pas** qualification si la politique exige la CI distante : c'est exactement le faux vert que le corpus combat.

---

## 5. Design statistique (le point où les revues précédentes se trompaient)

- **Différences appariées :** pour chaque instance `i`, `d_i = ln(T_G,i / T_E,i)`. Estimer le **ratio médian apparié** + **IC bootstrap BCa** (10 000 rééchantillonnages), et la différence absolue en secondes.
- **Interdit :** exiger que deux IC **marginaux** ne se chevauchent pas (erreur corrigée par la V6) ; comparer des médianes sans structure appariée ; rapporter un **rapport de médianes** comme effet apparié (erreur de S8) ; p-value sur n = 3.
- **Exploratoire :** n ≥ 8 instances, pour estimer `σ_d` et la part `T_coord/T_total`. **Aucun test d'hypothèse.**
- **Confirmatoire :** `n = ceil((z_{1−α/2} + z_{1−β})² · σ_d² / (ln(1−δ))²)`, `σ_d` gonflé d'un facteur de prudence **pré-enregistré**. Si `n` dépasse le budget : **ne pas lancer** et rapporter « non démontrable dans le budget » — c'est un résultat honnête.
- **Ordre :** tous les bras dans la même instance, ordre randomisé (carré latin pour k bras), rotation entre jours, sonde de charge, refroidissement, dérive thermique en covariable.
- **Apprentissage :** session d'agent neuve à chaque run, même prompt, warm-up par bras exclu **a priori**.
- **Supervision :** chronomètre, journal d'interventions, superviseur **aveugle au seuil** (et au bras si possible), double mesure sur ≥ 20 % des runs.
- **Coût :** `C_total = C_tokens + C_human + C_infra + C_maintenance`. Ne jamais utiliser `processedTokens` seul (92–96 % de cache dans S8).
- **Multiplicité :** un contraste primaire (G vs E) ; secondaires descriptifs ou Holm ; aucune pêche aux sous-groupes.

---

## 6. Coût réel des vrais worktrees — modèle, pas chiffres

**Faits structurels (`SOURCE_READ`) :** `git worktree add` écrit le tree suivi mais ne duplique pas les objets `.git` ; il **ne copie pas** les fichiers ignorés (`target/`, `node_modules/`, `.venv/`). `remove` échoue parfois sur handles ouverts → `prune`. Windows : Defender scanne créations/suppressions, NTFS coûteux en petits fichiers, symlinks privilégiés, `core.symlinks=false` par défaut ⇒ mode `120000` non fidèle. Linux : reflink possible, overlayfs possible. Cargo sérialise les builds qui partagent un `target/` : cache partagé = attente ; caches séparés = disque.

**Modèle paramétré** (aucune valeur supposée ; à calibrer en P0) :

```text
T_A_setup = n·(S_tr/v_copy + F_tr/v_fs + [q=warm_copy]·(S_dep+S_bld)/v_copy + [q=cold]·(t_dep+t_bld_cold))
T_B_setup = n·(t_wt + S_tr/v_copy + F_tr·(1/v_fs + t_av) + [q=cold]·(t_dep+t_bld_cold)
              + [q=warm_copy]·(S_dep+S_bld)/v_copy + [q=sccache]·(t_bld_cold·miss + t_bld_warm·hit))
T_cleanup_B = n·(t_rm + F_tr·(1/v_fs + t_av))
Disk_A = n·(S_tr + [q=warm_copy]·(S_dep+S_bld))
Disk_B = n·(S_tr + S_dep + S_bld) + S_git_admin
T_total = T_setup + T_agent + T_integrate + T_verify + T_publish + T_orchestration + T_cleanup (+T_lock)
```

**Paramètres à mesurer après GO (5 répétitions, médiane [min–max]) :** `v_copy, v_fs, t_av, t_wt, t_rm, t_dep, t_bld_cold, t_bld_warm, t_lock, S_*, F_*`.
**Scénarios à rapporter :** `cold`, `warm_copy`, `sccache`, `shared`. **Aucun chiffre de coût n'est avancé ici.** Le fixture S8 (`target/` 11,3 Mo) ne représente pas un dépôt réel : `S_bld` doit être **mesuré**, pas supposé.

---

## 7. Règles d'arrêt et conditions éliminatoires

**Par run :** violation de protocole (mauvais SHA, prompt, reçu manquant), `T_cap`, OOM/swap, charge externe au-delà du seuil pré-enregistré ⇒ `INVALID`, re-run unique dans un bloc ultérieur (max pré-enregistré).

**Par bras — élimination quelle que soit la vitesse :**

1. perte silencieuse d'une modification validée ou dirty ;
2. corruption / récupération non maîtrisée ;
3. publication d'un arbre **autre** que celui qualifié ;
4. fuite d'un **secret fictif** en mode annoncé protégé ;
5. E2E requis masqué ;
6. `UNKNOWN` promu `PASSED`.

**Par étude :** futilité (effet observé < δ/2 avec IC excluant δ), budget épuisé, `n` inatteignable, `σ_d` trop grand ⇒ arrêt et « non démontrable ».

**Règle O0 (la plus importante) :** si **E** (ou **F**) est **non inférieur** à G dans une **marge d'équivalence pré-enregistrée** `δ_eq` sur `T_verified`, avec zéro condition éliminatoire et un `C_total` plus faible (maintenance incluse), alors **ne pas construire** les couches transactionnelles de Flow. La formulation « même sûreté et productivité à coût moindre » est **infalsifiable sans marge** : il faut fixer `δ_eq` et une grille de coût de maintenance **avant** les résultats.

---

## 8. Seuils H4 candidats (à trancher par l'humain)

Aucun seuil n'est ratifié. Les trois candidats portent sur la **réduction du ratio médian apparié de `T_verified` vs le meilleur baseline**, avec IC 95 % inférieur > 0, `C_total` non inférieur (≤ +10 %, à figer) et zéro condition éliminatoire.

| Candidat | Règle | Justification | Risque |
|---|---|---|---|
| **H4-15** | ≥ 15 % de réduction ; IC bas > 0 ; `T_coord/T_total` ≥ 20 % sur ≥ 1 classe | plus petit effet qui change une décision ; seuil déjà proposé (V6 `:714`) | proche du bruit (5–10 % entre runs S8) → n élevé, faux négatif |
| **H4-20** | ≥ 20 % ; `T_coord` ≥ 25 % | marge au-dessus du bruit ; valeur médiane citée ailleurs | peut être inatteignable si build/CI domine |
| **H4-25** | ≥ 25 % + **plancher absolu** `{{ABS_FLOOR}}` s/campagne + déploiement limité aux classes gagnantes | conservateur ; protège contre la complexité marginale | peut rejeter un outil utile |

**Traduction en heures/an** (`ESTIMATE`, hypothèses à remplir par l'humain) : `heures/an = N_livraisons/an × T_tâche(h) × δ`.
Exemple illustratif **non mesuré** : 900 livraisons/an × 0,33 h × 15 % ≈ **45 h/an** ; 20 % ≈ 60 h/an ; 25 % ≈ 75 h/an. **Ce n'est pas une mesure** ; c'est la forme du calcul que H4 doit trancher.

**Coût de la campagne** (`ESTIMATE`) : `R = (n_pairs + n_warmup)·k` runs ; `H_machine = Σ T_run` ; `H_human = Σ T_run·s + analyse` ; une seule campagne à la fois sur 16 Go.
Ordre de grandeur pour **S9-baseline** (A/B/C/D/E, sans G) : `n = 8` instances × 5 bras ≈ **40 runs** ; à 10–30 min/run ≈ **7–20 h machine**, ≈ **15–30 h humaines** de supervision, plus **3–5 jours** d'outillage/fixtures. *À confirmer par un pilote de 5 runs ; ne pas engager le confirmatoire sur cette base.*

---

## 9. Template de pré-enregistrement (à remplir, hacher, committer avant le premier run)

Toute modification après le premier run = **nouvelle version + nouveau hash + nouveau run**.

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

---

## 10. Réponse à la question : quel protocole peut falsifier la valeur de Flow ?

Hypothèse à falsifier : « G réduit `T_verified` de ≥ δ vs le meilleur baseline, sur au moins une classe, sans régression de sûreté ni de coût total. » Six falsificateurs :

1. **Prémisse nulle** — `T_coord/T_total < δ` sur toutes les classes ⇒ aucun coordinateur ne peut produire δ : **futilité démontrée avant dépense**.
2. **Non-infériorité O0** — E dans `±δ_eq` et moins coûteux ⇒ Flow n'a pas de valeur démontrée. C'est une **falsification positive**.
3. **Outil existant** — F dans `±δ_eq` et moins coûteux ⇒ Flow doit battre F, pas seulement O0.
4. **Plancher mono-agent** — G plus lent que E à W0 ⇒ Flow doit être retiré du chemin humain, réservé aux agents.
5. **Élimination** — une seule condition éliminatoire ⇒ bras disqualifié, quelle que soit la vitesse.
6. **Budget** — `n` confirmatoire inatteignable ⇒ « non démontrable » est le résultat ; ne pas construire sur une intuition.

**Ce que S9 ne peut PAS décider :** H2 (droits), H3 (politique sandbox), H6 (handoff Ravum), H7 (interne vs produit). S9 ne prouve **aucune** garantie de sécurité et ne remplace pas un test adversarial (matrice `DEEPSEEK-03` §8).

---

## 11. Recommandation bornée

**Prochaine étape proposée (une seule autorisation) :** `GO_R0_FIXTURES` limité à **S9-baseline + reproduction P0**, réparti en trois lots :

| Lot | Contenu | Sortie |
|---|---|---|
| R0-a | Fixtures synthétiques jetables + harnais de mesure neutre (aucun code Flow) | harnais + SHA fixtures |
| R0-b | Reproduction des P0 sur fixtures (TREE-01, HOOK-02a/b/c, ENV-04a/b/c, FENCE-07a-d, FILE-09, PATH-08) | `REPRODUCED / DISPROVED / NOT_REPRODUCED` par test |
| R0-c | **S9-baseline** A/B/C/D/E (+F si disponible), `n = 8` exploratoire | `σ_d`, `T_coord/T_total`, coût worktree calibré, décision de poursuite |

**Ce que ce GO n'autorise pas :** écrire du code Flow (R1+), modifier Ravum, activer une CI Flow, publier une licence, lancer le confirmatoire.

**Budget indicatif** (`ESTIMATE`, à figer par H5) : 3–5 jours d'outillage, ≈ 15–30 h humaines de supervision, ≈ 7–20 h machine, 1 campagne à la fois sur 16 Go. **Aucun run concurrent avec la session Ravum.**

**Verdict D3 : `DRAFT` — non franchi.** Le protocole est pré-enregistrable ; il manque **deux chiffres humains** (seuil δ et budget) et **une autorisation** (H5).

---

*Sources : [E](raw/DEEPSEEK-RAW-E-S9-PERFORMANCE.md), artefact S8 cité (§2), V6 §6, `docs/BENCHMARK_S9.md`, `docs/SECURITY_TESTS.md`, rapports A/C/G/H. Aucun benchmark exécuté, aucun chiffre inventé : les valeurs absentes sont marquées `{{…}}` ou `ESTIMATE`.*
