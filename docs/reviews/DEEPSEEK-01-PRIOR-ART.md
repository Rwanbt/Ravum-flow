# DEEPSEEK-01 — ART ANTÉRIEUR ET COMPARAISON O0–O4 (D1)

**Statut :** `PROPOSED`. Aucune option n'est désignée gagnante. Ce document compare, il ne ratifie pas.
**Base :** [F](raw/DEEPSEEK-RAW-F-PRIOR-ART.md) (fiches outils, 38 sources datées), [A](raw/DEEPSEEK-RAW-A-GIT-CONCURRENCY.md) (§3 O0), [C](raw/DEEPSEEK-RAW-C-BUILD-CI.md) (§3 ordonnanceur), [G](raw/DEEPSEEK-RAW-G-PRODUCT-UX.md) (§9 produit), [H](raw/DEEPSEEK-RAW-H-ADVERSARIAL.md) (§S1-S8), [`DEEPSEEK-00`](DEEPSEEK-00-BASELINE.md).
**Règle du mandat :** O0 doit rester une option de sortie viable, et l'abandon de Flow doit être une recommandation acceptable.

---

## 1. Ce que chaque option est réellement

| Option | Définition opérationnelle | Ce qu'elle n'est pas |
|---|---|---|
| **O0** | Git natif + copies privées ou `git worktree` éphémères + intégration sérialisée (script) + CI existante. | Pas un « dossier magique » ; pas de reçu unifié, pas d'attribution. |
| **O1-A** | O0 outillé : un wrapper (~300–500 lignes) au-dessus de `git worktree` + `git commit` + un reçu JSON + un `pre-publish` hook. | Pas un produit ; pas de moteur. |
| **O1-B** | Jujutsu (`jj`) comme client VCS, Git en backend. | Pas un gate de hooks : `Hooks: No`. |
| **O1-C** | GitButler comme UI/CLI de branches parallèles. | Pas une isolation : même répertoire ; isolation seulement via `but worktree` (0.23.0). |
| **O1-D** | Scion (`GoogleCloudPlatform/scion`) comme orchestrateur. | Pas local-first : Hub + conteneurs ; **Windows = client seulement**. |
| **O2** | Flow mince : `MANAGED_PATCH`, arbre candidat Git exact, reçu, writer Git contrôlé, journal, CLI. | Pas de sandbox d'exécution ; hooks refusés ou délégués. |
| **O3** | O2 + **vue disque privée jetable** pour hooks/builds/outils non intégrés. | Pas de `git worktree` obligatoire, mais **pas** « zéro matérialisation ». |
| **O4** | VFS / CRDT / cache distant / multi-tenant. | Différé : ne se justifie par aucune mesure. |

**Définition crédible de O1 (F) : O1-A**, c'est-à-dire l'assemblage `Git + worktree éphémère par agent + hooks natifs + CI impactée + merge queue optionnelle + script d'intégration `update-ref` CAS`. C'est le seul assemblage qui coche « hooks dans l'ordre Git », « offline », « Windows/Linux », « licence libre » et « coût quasi nul ».

---

## 2. Faits d'outils (sourcés, datés 2026-11-09)

| Outil | Version observée | Licence | Windows | Offline | Points durs vérifiés |
|---|---|---|---|---|---|
| **Git `worktree`** | Git 2.51.1 (machine) | GPL-2.0 (outil) | oui | oui | doc Git : « multiple checkout … still experimental » ; partage le `.git` réel |
| **jj** | 0.47.1 | Apache-2.0 | MSVC | oui | **hooks : non** ; `.gitattributes` non ; submodules/LFS non ; UTF-8 only ; corruption possible sur FS partagé (#2193) |
| **GitButler** | 0.23.2 | **FSL-1.1-MIT** (non-OSI, conversion MIT à 2 ans, « Competing Use » interdit) | MSI | oui | branches parallèles = **même dossier** ; isolation via `but worktree` seulement |
| **Scion (Google)** | pré-1.0 | Apache-2.0 | **client Hub seulement** ; serveur « not supported on Windows » | **non** (Hub) | conteneurs obligatoires ; non supporté officiellement par Google |
| **Delta / DeltaDB (Zed)** | bêta | DeltaDB non publiée (`NOT_REVIEWED`) ; plugin AGPL-3.0 | ? | **non** (remote Git requis) | sync entre clients, pas un gate |
| **Cargo / tsc / pytest / CMake-Ninja** | versions machine | — | oui | oui | ce sont déjà des build systems incrémentaux |

**Point de désaccord non résolu :** le corpus V6 affirme l'existence de **deux** « Scion » (Google Apache-2.0 et un Scion interne d'Anthropic décrit le 13/10/2026). L'agent F n'a trouvé **aucune** source primaire du second et conclut à une erreur de source. Je ne tranche pas : `NOT_REVIEWED` sur le Scion Anthropic, et **recommandation de ne pas le citer** tant qu'une source primaire n'est pas fournie. Le Scion Google, lui, est vérifié et **n'est pas** une base local-first.

---

## 3. Table de comparaison (critères objectifs)

Notation : ✅ satisfait · ◐ partiel/conditionnel · ❌ non · ? `NOT_REVIEWED`.

| Critère | O0 | O1-A wrapper | O1-B jj | O1-C GitButler | O2 mince | O3 hybride | O4 |
|---|---|---|---|---|---|---|---|
| Hooks Git dans l'ordre, tree post-hooks | ✅ (via `git commit`) | ✅ | ❌ (pas de hooks) | ◐ | ◐ (à construire) | ✅ (si sandbox) | ? |
| Isolation FS réelle par agent | ◐ (worktree/copie) | ◐ | ✅ (workspaces) | ❌ (même dossier) | ❌ | ✅ **si** sandbox OS | ✅ (VFS) |
| Arbre exact + reçu vérifiable | ❌ | ◐ (JSON maison) | ❌ | ❌ | ✅ | ✅ | ✅ |
| Publication contrôlée / CAS | ◐ (script) | ✅ | ✅ (natif) | ✅ (natif) | ✅ | ✅ | ✅ |
| Non-écrasement des dirty humains | ◐ (discipline) | ◐ | ✅ | ◐ | ◐ | ✅ (vue privée) | ✅ |
| Multi-langage sans réimplémenter les builds | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Offline complet | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ (cache distant) |
| Windows/Linux | ✅ | ✅ | ✅ | ✅ | ✅ | ◐ (**sandbox Windows non résolue**) | ? |
| Licence compatible distribution | ✅ | ✅ | ✅ | ❌ FSL (non-OSI) | ✅ | ✅ | ? |
| Coût de développement | ~0 | **3–5 j** (G) | ~0 | ~0 | **300–500 h/an** (G) | **1 100–1 900 h/an** (G) | inconnu |
| Coût de maintenance | ~0 | 80–200 h/an | faible (amont) | faible (amont) | 300–500 h/an | 1 100–1 900 h/an | élevé |
| Faux vert E2E évité | ◐ (CI humaine) | ◐ | ◐ | ◐ | ✅ | ✅ | ✅ |
| Verdict de l'agent | **baseline à battre** | **le vrai concurrent** | disqualifié comme gate | UX seulement | défendable | **bloqué par H3** | différé |

---

## 4. Coût total (estimations explicites, à réviser)

| Option | Coût développeur (`ESTIMATE`) | Coût utilisateur | Valeur attendue |
|---|---|---|---|
| O0 | 0 | 0 | 0 par définition ; le problème reste |
| O1-A wrapper | 3–5 j (24–40 h) | ~30 min | 20–50 h/an **si** ≥ 3 agents |
| O2 mince | 300–500 h | 1–2 h | 100–200 h/an (hypothèse) |
| O3 complet | 1 100–1 900 h | jours | non démontrée ; risque négatif |
| O4 | > 2 000 h | — | nulle sans besoin prouvé |

**Lecture brutale (G) :** avec **un seul utilisateur**, la maintenance de O2 (300–500 h/an) dépasse la valeur plausible (100–200 h/an). Flow n'est rentable que si (a) il sert d'abord l'auteur sur Ravum et que les heures sont gagnées sur le langage, ou (b) il acquiert des utilisateurs. D'où la décision manquante **H7 : outil interne Ravum ou produit public ?**

---

## 5. Le fait qui change la comparaison : Ravum implémente déjà le noyau

L'agent H a établi, et je l'ai re-vérifié par API :

- PR **#85** fusionnée le 2026-11-09T11:26:56Z : « publication Git transactionnelle (15 tests de crash) », « protocole sémantique v2 », « CI par impact conservateur » — **81 744 lignes ajoutées, 329 fichiers**.
- PR **#87** fusionnée au SHA épinglé : « overlays transactionnels, incrémentalité avancée » — 24 fichiers.
- `dev` contient `mvp.yml` (13 528 o) + `mvp-impact.yml` (18 172 o) + `docs-check.yml` ; **`dev` est protégée** avec le check requis `mvp`.
- Une PR brouillon **#101** (`feat/lang-core-1.0-rc`) est ouverte : la session Ravum continue.

**Conséquence pour O2/O3 :** le writer transactionnel, les reçus, la CI d'impact et la coordination multi-agents **existent déjà dans Ravum**, non licenciés, en évolution. Flow fait face à un trilemme (H §S1) : dupliquer (double orchestration, interdite par `COR-13`), dépendre (licence absente + API instable), ou réécrire from scratch (coût complet pour un composant qui existe à côté).

**Ce que cela ne prouve pas :** que le coordinateur Ravum soit **utilisable hors Ravum** (couplage au langage, à ses crates, à ses conventions). C'est précisément la question à trancher avant H1 — et elle n'est pas tranchée. Elle est testable : extraire une CLI minimale du coordinateur Ravum et l'utiliser sur un dépôt Rust non-Ravum, sous licence accordée.

---

## 6. Le seul chiffre disponible, et ce qu'il ne dit pas

L'artefact S8 (`evidence/mvp0c-s8-live-summary-2026-11-09.json`, SHA `6b0ac7b6`, lu le 2026-11-09) donne, pour **n = 3** par bras :

| Bras | échantillons (ms) | médiane | plage |
|---|---|---|---|
| A copies privées | 100 664 / 111 110 / 208 694 | 111 110 | **100,7 – 208,7 s** |
| B dossier partagé + garde | 126 224 / 237 674 / 286 275 | 237 674 | **126,2 – 286,3 s** |
| C séquentiel | 269 594 / 528 481 / 611 003 | 528 481 | **269,6 – 611,0 s** |

**Les plages A et B se chevauchent.** Avec n = 3, aucun intervalle, aucun test de différence appariée, aucun seed ni SHA de fixture documenté dans l'agrégat, **l'ordre A < B < C n'est pas établi**. Les booléens `zero_lost` et `all_tests_green` sont **auto-déclarés**.

De plus, S8 ne mesure **aucun** `git worktree` ni Flow : il compare copies privées, dossier partagé, séquentiel. **Le coût qui motive le produit (worktrees/copies) n'a donc jamais été mesuré**, et la seule mesure existante pénalise, en médiane, la prémisse d'`INV-01` (dossier canonique unique).

**Usage autorisé de S8 :** ordre de grandeur et calibration de protocole. **Usage interdit :** classement d'options, argument de vente, justification de H1. *(Position de H, retenue.)*

---

## 7. Raisons d'abandon, option par option

| Option | Raison d'abandon si… | Preuve manquante |
|---|---|---|
| O0 | …un besoin récurrent ≥ 3 agents est mesuré **et** le coût d'intégration manuelle dépasse le seuil H4 | journal de 10 jours (H K2) |
| O1-A | …le wrapper ne fait pas gagner ≥ 15 % sur le temps total vérifié à ≥ 3 agents | S9 apparié |
| O1-B (jj) | …des hooks sont requis (projets avec lint/format/signature) : jj **perd la gate** | liste des projets cibles |
| O1-C (GitButler) | …la licence FSL bloque la distribution ou l'intégration | revue licence |
| O1-D (Scion) | …Windows est la cible principale (serveur non supporté) | — (fait établi) |
| O2 | …S9 et le test de demande ne montrent pas un gain net ; ou si Ravum expose déjà l'équivalent | S9 + H2/H7 |
| O3 | …aucune sandbox Windows crédible (AppContainer non qualifié) : O3 devient indisponible sur l'OS de référence | test AppContainer (5 j) |
| O4 | …aucun besoin mesuré (état actuel) | — |

---

## 8. Désaccords réels entre agents

1. **F vs corpus :** « Scion Anthropic » n'a aucune source primaire retrouvée → F conclut à une erreur ; G/H ne se prononcent pas ; je maintiens `NOT_REVIEWED` et interdis la citation. **Divergence non résolue.**
2. **A vs F sur jj :** F le juge « excellent VCS, mauvais gate » ; A le **disqualifie comme O1** pour tout projet à hooks. Compatible si l'on distingue « outil utilisateur optionnel » (F) et « option de remplacement » (A) : retenu.
3. **G vs H sur la cible :** G recommande un **portail de publication vérifiée** (P2) utile même à 1 agent ; H juge que même P2 n'a pas de demande prouvée et que O0 + journal suffit. Les deux positions sont conservées ; elles convergent sur « mesurer avant de construire » et divergent sur « construire un portail ».
4. **A vs V6 sur O0 :** A affirme qu'O0 **suffit** pour la tranche Git/concurrence (via `git commit` en worktree + CAS), et qu'il ne manque que politique + reçu. C'est plus favorable à O0 que la V6, qui suppose un writer spécifique. Retenu comme la position la mieux argumentée du dossier.

---

## 9. Verdict D1

- **O0 est une vraie option de sortie**, pas une figure rhétorique : un script de 300–500 lignes couvre la tranche Git/hooks/CAS, et S8 ne démontre pas le contraire.
- **O1-A est le concurrent crédible** : hooks natifs, offline, licence libre, coût quasi nul. Il bat Flow sur l'orchestration.
- **O2 mince garde un delta défendable** : reçu lié `tree + contexte`, politique d'obligations, refus explicites. C'est de la **politique et de la provenance**, pas un moteur VCS.
- **O3 est bloqué par H3** : sans sandbox Windows crédible, le mode phare est indisponible sur l'OS de référence.
- **O4 reste différé**, sans justification mesurée.
- **Le noyau revendiqué existe déjà dans Ravum** : la question H1 n'est plus « O0 vs Flow » mais « finir/durcir le coordinateur Ravum et l'exposer » vs « construire un second produit ».

**Recommandation provisoire (à valider par H1, pas ratifiée) :** ne pas choisir d'architecture avant (a) le test de demande de G (une semaine, coût quasi nul) et (b) S9 sur **vrais worktrees** contre O0/O1-A. Si ces deux preuves manquent, la réponse par défaut est **O0 instrumenté**, pas O2.

---

*Sources : fiches F (jj 0.47.1, GitButler 0.23.2, Scion, Delta, git-worktree, Cargo/Nx/Bazel/Buck2/Pants), A §3, C §3, G §9, H §S1-S8, artefact S8 cité, API GitHub du 2026-11-09. Aucune installation, aucun test, aucun benchmark.*
