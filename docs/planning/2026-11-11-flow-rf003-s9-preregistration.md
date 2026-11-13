# Flow — RF-003 : complément de pré-enregistrement S9 (documentaire, **aucun run**)

Statut : **complément de protocole.** `S9` reste `DRAFT` et `NOT_RUN` ; `H4` (seuil, plancher, budget,
`n`, alpha, puissance) reste **HUMAN-DECISION** ; aucun nouveau `GO` n'existe. Ce fichier ne remplace
pas [BENCHMARK_S9](../BENCHMARK_S9.md) (autorité de nommage des bras et historique S8) : il comble les
écarts opératoires que la fiche #9 demande (coût humain/CPU/RAM, appariement, veto perte, matrice
d'outillage).

Base : `feat/flow-production-candidate` @ `2ad5bf5`, 2026-11-11. Ni fixture, ni script, ni benchmark
lancé.

## 1. Ce qui est déjà pré-enregistré (ne pas réécrire)

| Élément | Où | État |
|---|---|---|
| Bras canoniques **A** copies privées, **B** vrais `git worktree`, **C** partagé + garde, **D** séquentiel, **E** O0 instrumenté, **F** outil existant, **G** Flow | [BENCHMARK_S9](../BENCHMARK_S9.md) §Canonical arms | `DRAFT` |
| Table d'alias (S8-A→A, S8-B→**C**, S8-C→D, V6 D→E, V6 E→G…) | idem §Alias | `DRAFT` |
| Historique S8 : 3 runs/bras, médianes 100,7–208,7 s (A), 126,2–286,3 s (C), 269,6–611,0 s (D) ; **pas une hiérarchie** (intervalles qui se chevauchent), sign test 3/3 → p = 0,125 unilatéral | idem §S8 | `MEASURED_HISTORICAL` |
| Hypothèse, critère primaire `T_verified`, workloads, contrôle, métriques, règles d'élimination | idem §Hypothesis…§Elimination | `DRAFT` |
| Template de pré-enregistrement (YAML, hash avant premier run) | [DEEPSEEK-04](../reviews/DEEPSEEK-04-S9-PROTOCOL.md) §9 | `DRAFT` |

**Règle conservée** : un bras simulé ne compte jamais (`F` = `NOT_AVAILABLE` s'il n'est pas installé ;
`G` non mesurable tant qu'un prototype Flow n'a pas passé ses gates) ; un changement après le premier
run = nouvelle version + nouveau hash + nouveau run.

## 2. Écart 1 — matrice de disponibilité des bras (par OS, 16 Go)

À remplir par l'opérateur avant tout run ; chaque cellule est `AVAILABLE` / `NOT_AVAILABLE` / `UNKNOWN`.
Aucune n'est affirmée ici.

| Bras | Windows | Linux | Dépendance de disponibilité | Coût disque |
|---|---|---|---|---|
| A copies privées | `UNKNOWN` | `UNKNOWN` | espace disque = N copies du dépôt | N × taille dépôt |
| B vrais worktrees | `UNKNOWN` | `UNKNOWN` | `git worktree` + config (`core.symlinks`, `core.autocrlf`, `core.longpaths`) | arbre partagé, objets partagés |
| C partagé + garde | `UNKNOWN` | `UNKNOWN` | garde advisory ; un outil non géré peut écrire sans elle | 1 × dépôt |
| D séquentiel | `UNKNOWN` | `UNKNOWN` | aucun outil supplémentaire | 1 × dépôt |
| E O0 instrumenté | `UNKNOWN` | `UNKNOWN` | Git natif + CI + hooks natifs + coordination légère | 1 × dépôt |
| F outil existant | `UNKNOWN` | `UNKNOWN` | licence + offline + version de l'outil ; **jamais simulé** | dépend de l'outil |
| G Flow | `UNKNOWN` (PR #54) | `UNKNOWN` | prototype ayant passé ses gates ; sinon `NOT_RUN` | scratch temporaire |

Contre-exemple à consigner : une modalité impossible sous une licence ou une plateforme (ex. outil
`FSL` non redistribuable) est `NOT_AVAILABLE`, **pas** un échec expérimental.

## 3. Écart 2 — appariement et randomisation (opérationnel)

| Règle | Spécification |
|---|---|
| Unité de paire | `(tâche, graine, état d'hôte)` ; les bras d'une même paire partagent la tâche et la graine |
| Ordre | blocs aléatoires (Latin square) par paire ; ordre consigné avant le run |
| Blocage | même machine, même régime de cache (froid/chaud déclaré), même charge concurrente (aucune campagne parallèle) |
| Échauffement | 1 run d'échauffement par bras, **exclu** du calcul |
| Données manquantes | un bras qui échoue pour une raison d'**environnement** est rejoué une fois ; un échec **produit** reste un échec (jamais imputé à l'environnement) |
| Analyse | différence/ratio **apparié** par paire ; bootstrap sur les paires ; ne pas exiger des IC marginaux disjoints |
| `n` | `n >= 8` = reconnaissance exploratoire (pilote `n = 5` S8 conservé comme historique) ; `n` confirmatoire = `HUMAN-DECISION` |

## 4. Écart 3 — coût humain, CPU, RAM, disque

| Mesure | Méthode (à exécuter plus tard, jamais ici) | Unité |
|---|---|---|
| Supervision humaine | chronomètre démarré à la remise de la tâche, arrêté à `T_verified` ; l'opérateur note chaque intervention (lecture, arbitrage, correction) | secondes, nombre d'interventions |
| Tokens | journal du fournisseur, par paire et par bras | tokens entrée/sortie |
| CPU | échantillonnage `Get-Counter` (Windows) / `pidstat -u 1` (Linux) sur l'arbre de processus du bras | % moyen + intégrale CPU·s |
| RAM | échantillonnage `Get-Counter '\Memory\Committed Bytes'` (Windows) / `pidstat -r 1` + `memory.peak` cgroup v2 (Linux) **agrégé** sur le groupe de processus | pic Mio |
| Disque | taille de l'espace de travail avant/après + `du`/`Get-ChildItem` ; pic pendant le run si outillage le permet | Mio |
| Budget | RAM agrégée plafonnée à 16 Go ; au-delà, le run est invalide (`RESOURCE_EXHAUSTED`), pas « lent » | — |

Ces méthodes sont **spécifiées**, pas exécutées : aucun échantillon n'existe.

## 5. Écart 4 — veto perte (conditions éliminatoires)

| Cas | Déclencheur | Effet |
|---|---|---|
| Perte silencieuse | manifeste de hash avant/après : un fichier modifié/validé, `dirty` ou `untracked` disparaît sans trace | **NO-GO immédiat** du bras et du run |
| E2E requis masqué | un E2E exigé par la politique n'apparaît pas dans la preuve | NO-GO |
| `UNKNOWN` promu `PASSED` | statut non exécuté présenté comme succès | NO-GO |
| Fuite de secret canari | canari fictif retrouvé hors du périmètre annoncé | NO-GO |
| Ref/état non récupérable | recovery impossible après interruption | NO-GO |

Test **positif** (contrôle) : un bras volontairement propre conserve le manifeste → doit passer.
Test **négatif** : un bras avec suppression scriptée d'un fichier `untracked` → doit être détecté et
arrêter le run. Les deux sont `NOT_RUN` (aucune exécution dans cette séance).

## 6. Écart 5 — ce qui reste strictement humain (`H4`)

| Paramètre | Valeur | Statut |
|---|---|---|
| effet seuil (`DELTA`) | — | **HUMAN-DECISION** |
| marge d'équivalence O0 | — | **HUMAN-DECISION** |
| plancher absolu (secondes) | — | **HUMAN-DECISION** |
| budget CPU/RAM/temps et nombre de paires | — | **HUMAN-DECISION** |
| `n` confirmatoire, alpha, puissance | — | **HUMAN-DECISION** |
| autorisation de campagne (`GO` S9) | — | **non accordée** (`H5` : nouveau `GO` distinct requis) |

Aucun de ces champs n'est rempli ; un remplissage par un agent serait une décision déguisée.

## 7. Écart 6 — métriques brutes attendues (gabarit vide)

Table à remplir par run, sans réinterprétation (`NOT_RUN` aujourd'hui) :

| run | bras | tâche | graine | ordre | T_verified (s) | CPU·s | pic RAM (Mio) | disque (Mio) | tokens | interventions | E2E | statut |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| — | A | — | — | — | — | — | — | — | — | — | — | `NOT_RUN` |

## 8. Faits, recommandation, objections

**Faits** : S8 conservé (médianes, chevauchement, p = 0,125) ; les bras A/B/C/D/E/F/G sont déjà
canoniques ; `H4` non tranché ; aucun run S9 n'existe ; `F`/`G` ne peuvent pas être simulés.

**Recommandation non contraignante** (`PROPOSED`) : remplir la matrice §2 et le YAML de
DEEPSEEK-04 §9, hacher, puis demander `H4` + `GO` ; ne pas démarrer avant. Objections : (a) le coût de
la supervision humaine chronométrée est sous-estimé dans le budget ; (b) `B` (vrais worktrees) n'a
**jamais** été mesuré, même en S8, donc le bras clé du produit est le moins outillé ; (c) `F` peut être
`NOT_AVAILABLE` sous licence, et un dossier sans `F` ne compare pas à l'état de l'art.

## 9. Limites

- Aucune exécution, aucune fixture, aucun script : tout ce qui est méthode est `PROPOSED`.
- Les versions d'outils citées par le template (`git 2.51.1`, `rustc 1.95.0`, `node 26.13.0`,
  `python 3.13.7`) sont celles du gabarit historique, **non re-mesurées ici**.
- Document produit par un agent ; il ne clôt ni `D3` ni `H4`.

## Annexe — liens

Issue #9 · #44 (US-011) · [BENCHMARK_S9](../BENCHMARK_S9.md) ·
[DEEPSEEK-04](../reviews/DEEPSEEK-04-S9-PROTOCOL.md) · [V6 §6](../reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md) ·
[registre](../DECISION_REGISTER.md) · [H5](2026-11-11-flow-h5-consumed-go.md) ·
[H1](2026-11-11-flow-h1-options.md)
