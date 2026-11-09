# DEEPSEEK-05 — ROADMAP OPTIMISÉE, 26 RF, 15 US, ARCHITECTURE CANDIDATE (D4 input)

**Statut :** `PROPOSED / NOT AUTHORIZED`. Aucun ticket n'est approuvé, aucun code n'est autorisé. Les IDs historiques sont **conservés** ; aucune formulation n'est effacée.
**Base :** [G](raw/DEEPSEEK-RAW-G-PRODUCT-UX.md) (mapping US/RF, trim, fusions), [C](raw/DEEPSEEK-RAW-C-BUILD-CI.md) (audit des tickets 012–020), [D](raw/DEEPSEEK-RAW-D-RAVUM-ADR.md) (ordre de ratification), [A](raw/DEEPSEEK-RAW-A-GIT-CONCURRENCY.md) (writer/états), [`DEEPSEEK-01`](DEEPSEEK-01-PRIOR-ART.md) (options), [`DEEPSEEK-03`](DEEPSEEK-03-THREAT-MODEL.md) (correctifs exigés).

---

## 1. Principe : ce qui est optimisé, et ce qui ne l'est pas

**Optimisé :** le chemin jusqu'à une **preuve** (S9 + test de demande), la taille du noyau, la suppression des doublons, la parallélisation des lots, les critères de sortie.
**Non optimisé :** la couverture documentaire. Un backlog plus court mais non mesuré reste un backlog non mesuré.

**Trois règles :**

1. **Aucun ID historique n'est supprimé ni renuméroté** (`RF-001..026`, `US-001..015`, `RF-0..9`, `RF-RFC-*`, `RAVUM-ADR-*`, `FLOW-ADR-*`).
2. **Une issue ouverte n'est pas une autorisation** ; tout ticket reste `BACKLOG / BLOCKED-BY-GO`.
3. **Une fusion est un renvoi**, pas une suppression : les deux IDs restent traçables vers le même contenu.

---

## 2. Défauts du backlog actuel (mesurés)

| Défaut | Mesure | Preuve |
|---|---|---|
| Traçabilité décorative | **14/26 RF sans aucune US** (54 %) ; **5 US sans RF** (005, 007, 009, 013, 015) | G §2.1-2.2 |
| Ambiguïté de phase | **9/26 RF** portent deux phases ou deux gates → aucun critère de sortie unique | G §2.1 ; `BACKLOG.md` |
| Personas hors cible | **8/15 US** sont des rôles d'organisation ou d'autres dépôts ; seules 5/15 visent le solo | G §2.2 |
| Stories prématurées | **6/15** présupposent une architecture non décidée (INV-01-A, H3, D2, H2) | G §2.2 |
| Doublons | 5 familles (identité/journal, cache/impact, orchestration, adaptateurs, distribution) | G §2.3 ; C §2.9 T-10 |
| Tickets prématurés | RF-013/014/015/018/019/020 sous-spécifiés ou prématurés ; RF-020 le plus prématuré | C §2.9 |
| Incohérence de dépendances | RF-022 dépend de R5 mais est mappé D3/R0 | C §2.9 T-09 |
| Collision d'IDs | `RF-0..9` vs `RF-001..026` coexistent | V6 §7.1 ; G §8.6 |

---

## 3. Fusions proposées (IDs conservés)

| Fusion | Contenu | Effet |
|---|---|---|
| **F1** | RF-004 + RF-005 + RF-011 → « identité, journal, reçus » | un seul contrat au lieu de trois tickets |
| **F2** | RF-012 + RF-014 → « identité de contexte + binding reçu » | le cache dépend de l'identité : un objet |
| **F3** | RF-013 + RF-015 → « exécuter / annuler / décider quoi exécuter » | une seule boucle |
| **F4** | RF-018 + RF-019 + RF-020 → **une** interface d'adaptateur, trois instances | évite trois contrats divergents |
| **F5** | RF-024 + RF-025 → famille « distribution » (hors produit) | un seul DEFER |
| **F6** | RF-006 + RF-016 + RF-017 → surface CLI/JSON unique ; UI et SDK = clients | pas de protocole séparé au MVP |
| **F7** | RF-001 + RF-002 → blocage H2 (droits/provenance) | une seule décision |
| **F8** | US-004 + US-011 + US-013 + US-015 → artefact « gouvernance et mesure » **hors stories produit** | ne pollue plus le set produit |
| **F9** | US-009 + US-010 + US-001 → « contrat de non-régression Git/outils natifs » (contraintes) | ce ne sont pas des fonctionnalités |
| **F10** | US-003 + US-008 → « ne pas toucher mon workspace » | une seule acceptation |

**Conséquences :** le backlog produit passe de **26 RF** à **≈ 17 objets de travail** (sans supprimer d'ID), et le set de stories produit de **15** à **≈ 8** (les 7 autres deviennent des contraintes ou de la gouvernance).

---

## 4. Dispositions par ticket (proposition)

| RF | Disposition | Phase | Raison |
|---|---|---|---|
| RF-001 | **garder** (bloquant) | D0/D4 | H2 : nom, licence, droits |
| RF-002 | **fusionner F7** | D0/R0 | inventaire des composants transférables |
| RF-003 | **garder** — rejouer en S9-baseline | D3/R0 | S8 non probant (§`DEEPSEEK-04`) |
| RF-004 | **fusionner F1**, scinder identité Git vs snapshot | R1 | une seule identité au MVP |
| RF-005 | **fusionner F1** — **pas de CAS maison** | R1 | objets Git + refs épinglées |
| RF-006 | **fusionner F6** — 7 verbes, `--json` | R1 | seule interface |
| RF-007 | **garder** | R2 | le cœur : admission base+préconditions |
| RF-008 | **garder**, mais renames/conflits **textuels** seulement | R2 | pas de merge sémantique |
| RF-009 | **garder** | R4 | recovery après publication |
| RF-010 | **scinder** : détection/refus de hooks (R2) vs hooks isolés (R3, conditionné H3) | R2/R3 | le refus est *must*, l'isolation est *should* |
| RF-011 | **fusionner F1** (règle de statut, pas une tâche) | R1 | étiquettes honnêtes |
| RF-012 | **garder + préciser** (workspace, features, `--locked --offline`, toolchain absente ⇒ `UNSUPPORTED`) | R3 | C §2.9 T-01 |
| RF-013 | **fusionner F3**, **sans leases/epoch** au MVP | R3/R5 | un seul writer local |
| RF-014 | **fusionner F2** | R3 | clé de contexte complète |
| RF-015 | **fusionner F3**, **sans graphe d'impact au MVP** | R3/R5 | éviter la fabrique à faux verts |
| RF-016 | **fusionner F6** | R5 | `--json` suffit |
| RF-017 | **kill MVP** | — | `git diff` + éditeur |
| RF-018 | **garder comme instance F4** | R5 | sur demande réelle |
| RF-019 | **garder comme instance F4** | R5 | full pytest d'abord |
| RF-020 | **kill MVP** (instance F4 plus tard) | — | C §2.9 T-07 |
| RF-021 | **defer** | R6 | forge optionnelle |
| RF-022 | **garder**, corriger la dépendance (D3/R0 **ou** R5, pas les deux) | D3/R0 puis R5 | C §2.9 T-09 |
| RF-023 | **kill** | — | VFS sans besoin |
| RF-024 | **defer** (F5) | R7 | hors produit |
| RF-025 | **defer** (F5) | R7 | hors produit |
| RF-026 | **defer** | R6 | appartient au backlog Ravum |

**Stories :**

| US | Disposition |
|---|---|
| US-001 | **must** (fusion F9, avec `doctor`) |
| US-002 | **must** — c'est le produit |
| US-003 | **must** (fusion F10) — scinder « ne pas toucher » (trivial) et « reprendre après crash » (coûteux) |
| US-004 | **gouvernance** (F8) — « mesurer le gain » n'est pas une capacité utilisateur |
| US-005 | **must**, mais reformulé : « un reçu qualifie un arbre et un contexte », pas « empêcher tout commit non testé » |
| US-006 | **should**, bloqué par le graphe d'impact non ratifié |
| US-007 | **must** pour le **refus**, **defer** pour la sandbox (H3) |
| US-008 | **defer** — présuppose INV-01-A et H3 ; le MVP **détecte** les hooks et délègue |
| US-009 | **contrainte** (F9), pas story |
| US-010 | **contrainte** (F9), pas story |
| US-011 | **gouvernance** (F8) |
| US-012 | **defer** (H2) |
| US-013 | **gouvernance** (F8) |
| US-014 | **must** — la page « ce que Flow ne protège pas » (meilleur rapport valeur/coût) |
| US-015 | **must comme exigence** : plan + confirmation avant mutation (pas un protocole de GO) |

---

## 5. Architecture candidate optimisée — « O2+/P2 : portail de publication vérifiée »

Nom de travail : **Flow-mini**. Ce n'est **pas** un orchestrateur de workspace, ni un VCS, ni un build system.

### 5.1 Noyau (must)

```
CLI 7 verbes (init · status · doctor · run · publish · explain · disable), --json, codes stables
   │
   ├─ ADMISSION        proposition = base commit + préconditions (contenu/mode) + auteur + clé d'idempotence
   ├─ ARBRE CANDIDAT   write-tree APRÈS hooks ; hooks exécutés via `git commit` en worktree détaché jetable
   ├─ REÇU             (tree_oid, BuildContextId, obligations, PolicyVersion, ExecutionBoundaryId, statut)
   ├─ OBLIGATIONS      graphe déclaré (build · unit · intégration · E2E · paquet) + escalade complète
   ├─ PUBLICATION      verrou writer exclusif → relecture old → CAS `update-ref` → outcome durable
   ├─ JOURNAL          append-only longueur+checksum (ou SQLite WAL FULL) ; refs de rétention `refs/flow/pending/*`
   └─ RÉCONCILIATION   non destructive ; jamais `reset --hard` ; `NEEDS_RECONCILIATION` explicite
```

**Ce qui distingue Flow de Git :** un **reçu lié à un tree exact et à un contexte**, une **politique d'obligations**, et des **refus précoces, codés, actionnables**. Pas un moteur.

### 5.2 Hors noyau (explicitement exclu du MVP)

CAS de contenu maison · VFS/overlay IDE · CRDT · merge sémantique · daemon obligatoire · multi-tenant/RBAC/remote CAS · cache distant · UI riche · `SourceSnapshotId` (sauf adaptateur Ravum) · leases/epoch (un seul writer local suffit) · ordonnanceur de builds · graphe d'impact sélectif.

### 5.3 Ce que l'architecture optimisée change par rapport à la V6

| Sujet | V6 | Proposition optimisée | Raison |
|---|---|---|---|
| Hooks | ordre Git, espace isolé, gel post-hooks | **`git commit` dans un worktree détaché** ; Flow ne réimplémente pas l'ordonnanceur de hooks | supprime 4 constats P0 d'un coup (A `R-A1`) |
| Machine d'états | 13 états + 11 écarts | **états observables seulement** ; `UNKNOWN_OUTCOME` de premier rang ; champs pour `tree_oid`, `hook_set_digest` | A §4.2-4.3 |
| Fencing | epoch + lock + CAS | **lock OS + CAS** au MVP, epoch seulement si S9 montre des collisions | A `R-A3` |
| Build | impact + cache contextuel | **graphe d'obligations** ; builds délégués aux outils natifs | C §3.3 |
| Stockage | CAS à comparer | **objets Git + journal** + refs de rétention | G, D `X-02` |
| Hooks non fiables | sandbox par OS | **refus par défaut** ; isolation seulement si H3 qualifie l'OS | B, H |
| Périmètre | orchestrateur multi-agents | **portail de publication** (P2) utile même à 1 agent ; P1 seulement après preuve | G §9.1 |

> **Point d'honnêteté :** ce candidat est plus petit que O2 complet **et** que O3. Il ne promet **aucune** isolation d'exécution non fiable. Si H1 exige O3, il faut d'abord H3 et un test AppContainer.

---

## 6. Roadmap optimisée (conditionnelle, sans autorisation implicite)

| Phase | Contenu minimal | Critère de sortie | Prérequis |
|---|---|---|---|
| **R0-a** | Fixtures jetables + harnais neutre | harnais + SHA fixtures | H5 |
| **R0-b** | Reproduction P0 (TREE-01, HOOK-02a/b/c, ENV-04a/b/c, FENCE-07a-d, FILE-09, PATH-08) | `REPRODUCED/DISPROVED/NOT_REPRODUCED` par test | H5, fixtures |
| **R0-c** | **S9-baseline** A/B/C/D/E (+F si dispo), n = 8 | `σ_d`, `T_coord/T_total`, coût worktree calibré, décision de poursuite | H4 (seuil provisoire), H5 |
| **R1** | schémas versionnés : proposition, tree OID, reçu, politique, journal, CLI 7 verbes, contrat de refus | parsing/fuzz, migrations, refus honnêtes, aucun faux vert | GO_R1 distinct |
| **R2** | `MANAGED_PATCH` : admission base+préconditions, conflits textuels, renames/deletes, réconciliation non destructive, **détection** de hooks | corpus concurrence + dirty + filetypes ; refus explicites | R1 |
| **R3** | arbre exact post-hooks, vérification depuis le tree OID, adaptateur Rust, obligations | TREE/HOOK/ENV/LEGACY/E2E satisfaits **ou** mode bloqué | R2 + H3 si hooks isolés |
| **R4** | publication CAS + intention durable + recovery + `PublishedWithResidue` | crash injection à chaque frontière ; Git utilisable sans Flow | R3 |
| **R5** | CLI/agents neutres, TS/Python **sur demande**, budgets 16 Go agrégés | scénarios 3+ agents, API cassée, E2E requis | R4 |
| **R6** | forge optionnelle, merge queue au SHA final, adaptateur Ravum opt-in | requalification au tree final ; pas de cycle | R5 + H2 |
| **R7** | VFS/CRDT/remote/multi-tenant | **séparément gated**, abandon possible | bénéfice S9 démontré |

**Changement clé :** R1 ne commence **qu'après** R0-c. C'est l'inversion la plus importante par rapport au corpus actuel : on ne spécifie plus des contrats avant de savoir si le problème dépasse le seuil.

---

## 7. Lots parallèles (futur, après GO) et périmètres d'écriture disjoints

| Lot | Contenu | Écrit dans | Dépend de | Peut démarrer |
|---|---|---|---|---|
| **L-A** | CLI, schémas, contrat de refus, codes de sortie | `crates/flow-cli`, `schema/` | — | R1 |
| **L-B** | tree OID, hooks, reçus | `crates/flow-tree`, `crates/flow-receipt` | L-A (schémas) | R2/R3 |
| **L-C** | journal, durabilité, recovery | `crates/flow-journal` | L-A | R2 |
| **L-D** | obligations, adaptateur Rust | `crates/flow-obligations`, `adapters/rust` | L-B | R3 |
| **L-E** | sandbox/hooks isolés | `crates/flow-sandbox` | L-B + H3 | R3 |
| **L-F** | publication, writer, lock | `crates/flow-writer` | L-B, L-C | R4 |

**Règles de parallélisme :** un lot = un périmètre d'écriture ; aucun lot ne touche `docs/adr/` sans décision humaine ; L-A doit geler les schémas avant que L-B/C/D/F ne codent ; L-E ne démarre jamais sans H3 ; **l'ordre de merge** est A → B → C → D → F, avec E conditionnel.

**Ressources :** 16 Go → une seule compilation lourde à la fois ; `cargo check` par lot plutôt que build complet ; aucun run concurrent avec la session Ravum.

---

## 8. Gates de merge (valables pour tout futur GO)

Un lot ne se merge que si **tous** ces points sont vrais :

1. tests du lot + tests négatifs pertinents verts, **aucun** test requis sauté ;
2. aucun statut `UNKNOWN/STALE/SKIPPED` promu en `PASS` ;
3. schéma versionné + migration descendante testée ;
4. provenance : commande, versions d'outils, hash d'environnement, hash d'artefact ;
5. `git` seul reste capable de lire, réparer, continuer ;
6. documentation mise à jour, **aucun ADR marqué ACCEPTED**, **aucune licence créée** ;
7. budget 16 Go respecté (mesure CPU/RAM agrégée) ;
8. revue humaine tracée (acteur, date, preuve) — pas un vote de modèle.

---

## 9. Ce qui reste hors roadmap (et ne doit pas y revenir sans preuve)

VFS/overlay IDE · CRDT · merge sémantique · daemon obligatoire · multi-tenant/RBAC · remote CAS · cache distant · orchestrateur de builds · graphe d'impact sélectif au MVP · UI riche · `SourceSnapshotId` hors adaptateur Ravum · leases/epoch avant collision mesurée.

**Raison :** chacun ajoute un mode de défaillance silencieux sans bénéfice mesuré (`DEEPSEEK-03` §4, `DEEPSEEK-01` §3).

---

## 10. Désaccords conservés

1. **G vs H sur l'existence d'un produit.** G veut un portail P2 utile même à 1 agent ; H juge que même P2 n'a pas de demande prouvée et qu'O0 + journal suffit. **Non tranché** — c'est H7.
2. **G vs V6 sur INV-01.** G rétrograde l'invariant « dossier canonique unique » en option, parce que S8 (en médiane) pénalise ce mode. Je ne le rétrograde pas dans ce document : je le place en **hypothèse à tester** (S9-baseline), ce qui est la position minimale commune.
3. **C vs V6 sur l'ordonnanceur.** C refuse que Flow possède l'ordonnanceur de builds ; la V6 le prévoit en R5. Retenu pour le MVP (C), réévaluable après S9.
4. **D vs G sur le nombre d'objets.** D veut ratifier 12 ADR dans un ordre strict ; G veut réduire le produit à 7 verbes. Les deux sont compatibles : l'ordre de D porte sur la **documentation**, le trim de G sur le **code**.

---

## 11. Verdict D4-input

- Le backlog est **réparable sans supprimer d'ID** : 10 fusions, 4 kills, 4 defers, 3 splits.
- L'architecture candidate optimisée est **plus petite** que O2/O3 et **ne promet rien** qu'elle ne puisse tenir : reçu exact, politique d'obligations, refus actionnables, publication CAS avec recovery.
- **Elle ne se justifie toujours pas** avant R0-c (S9-baseline) et le test de demande. C'est volontaire : la roadmap commence par une mesure, pas par du code.
- **Décision manquante :** H7 (outil interne Ravum vs produit public) conditionne le périmètre, la licence, le support et la nécessité même des 15 stories.

**Verdict : `PROPOSED` — aucune phase R1+ n'est autorisée.** La seule action suivante défendable est **R0** sous H5, avec H4 provisoire.

---

*Sources : [G](raw/DEEPSEEK-RAW-G-PRODUCT-UX.md) §2-§9, [C](raw/DEEPSEEK-RAW-C-BUILD-CI.md) §2.9-§3, [D](raw/DEEPSEEK-RAW-D-RAVUM-ADR.md) §3.4, [A](raw/DEEPSEEK-RAW-A-GIT-CONCURRENCY.md) §5, `docs/BACKLOG.md`, `docs/USER_STORIES.md`, `docs/ROADMAP.md`. Aucun code, aucune issue modifiée, aucun GO.*
