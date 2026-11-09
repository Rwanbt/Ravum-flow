# DEEPSEEK — SYNTHÈSE, VERDICT ET HANDOFF HUMAIN

**Date :** 2026-11-09 (Europe/Paris).
**Branche de livraison :** `docs/deepseek-d0-d4-review`, issue de `dev@1d5fddf` de `Rwanbt/Ravum-flow`.
**Périmètre respecté :** aucun code, aucun build, aucun test, aucun benchmark, aucune CI, aucune migration, **aucune modification de `Rwanbt/Ravum`** (lecture seule, SHA réépinglé), aucune licence, aucun ADR/RFC ratifié, aucune PR fusionnée.
**Statut global :** `DOCUMENT-CORRECTED / HUMAN-DECISIONS-PENDING / NO-GO-IMPLEMENTATION`. Aucun `GO`, même implicite.

---

## 1. Ce qui a été livré

| # | Livrable | Contenu |
|---|---|---|
| 00 | [Baseline](DEEPSEEK-00-BASELINE.md) | SHA des deux dépôts, 42+12+12 objets, blobs, droits, 8 écarts documentaires |
| 01 | [Art antérieur O0–O4](DEEPSEEK-01-PRIOR-ART.md) | 9 options comparées, O1-A crédible, trilemme d'ownership, coût total |
| 02 | [Matrice ADR/RFC](DEEPSEEK-02-ADR-RFC-MATRIX.md) | 42 RAVUM-ADR (13 D/11 I/18 N), 12 RFC, 12 FLOW-ADR, 12 contradictions nommées, ordre de ratification |
| 03 | [Threat model](DEEPSEEK-03-THREAT-MODEL.md) | P0 consolidés, 11 trouvailles hors V6, matrice sandbox, 15 promesses interdites, 23 tests canoniques |
| 04 | [Protocole S9](DEEPSEEK-04-S9-PROTOCOL.md) | Pré-enregistrement complet, 7 comparateurs, 17 workloads, statistique appariée, 3 seuils, template à trous |
| 05 | [Roadmap optimisée](DEEPSEEK-05-OPTIMIZED-ROADMAP.md) | 10 fusions, dispositions des 26 RF et 15 US, architecture candidate « Flow-mini », 6 lots parallèles, gates |
| 06 | [Décisions H1–H6 (+H7–H8)](DEEPSEEK-06-DECISIONS-TO-FREEZE.md) | Options, preuves, recommandations non contraignantes, propriétaire, bloqueur |
| 07 | [Dissensus](DEEPSEEK-07-ADVERSARIAL-DISSENT.md) | 13 désaccords, 4 corrections au Lead, 7 points laissés ouverts |
| — | [GOAL d'exécution](DEEPSEEK-EXECUTION-GOAL-DRAFT.md) | **Brouillon, non lancé**, préconditions et périmètre R0 |
| — | `raw/` | 8 rapports bruts signés (A, B, C, D, E, F, G, H) |

**Méthode :** 8 sous-agents à contextes disjoints, périmètres d'écriture disjoints, **aucune lecture croisée**. Sept rapports produits ; un agent (E) a échoué puis a été remplacé par un contexte neuf. Les quatre lectures du code Ravum (A, B, D, Lead) sont concordantes.
**Limite d'indépendance :** tous les agents utilisent la **même famille de modèle**. L'indépendance est procédurale, **pas épistémique** ; ce dossier ne prétend donc pas à un panel d'experts. Il vaut par la traçabilité (SHA, fichiers:lignes, URLs datées) et la triangulation.

---

## 2. Les sept constats qui décident

### 2.1 Les P0 sont localisés, pas reproduits

Quatre lectures indépendantes confirment, au SHA `5408d9b3` :

- `write-tree` **avant** les hooks (`git_writer.rs:841-844`), `commit-tree` réutilisant l'arbre (`:868`) ;
- ordre `prepare-commit-msg` **avant** `pre-commit` (`:1221-1228`) — et un en-tête qui revendique pourtant « in Git's order » (`:37-43`), donc **auto-contradictoire** ;
- hooks exécutés dans le **workspace réel** (`current_dir(&self.repo)`, `:1344`) ;
- environnement hérité (`:85-94`, `:1353-1355` ; `common/mod.rs:367-372,442-450`) ;
- `ravum-store` = **base SQLite de l'app M4 Todo**, pas un CAS (`lib.rs:1-16`).

**Aucune perte, corruption ou exfiltration n'est reproduite.** `CODE_READ ≠ bug reproduit`. Les tests sont spécifiés (`TREE-01`, `HOOK-02a/b/c`, `ENV-04a/b/c`, `FENCE-07a-d`, `FILE-09`, `PATH-08`).

### 2.2 Le noyau revendiqué existe déjà dans Ravum

PR **#85** fusionnée (81 744 lignes, 329 fichiers, « publication Git transactionnelle, 15 tests de crash, CI par impact »), PR **#87** fusionnée (overlays transactionnels), `mvp.yml` + `mvp-impact.yml` présents, `dev` protégée. **Flow est devant un trilemme** : dupliquer le coordinateur (double orchestration interdite), dépendre de crates sans licence et instables, ou réécrire from scratch. Rien ne prouve encore que ce noyau soit **utilisable hors Ravum** — c'est un test de 2–3 jours, pas une décision d'architecture.

### 2.3 Le seul chiffre disponible ne prouve pas ce qu'on lui fait dire

Artefact S8 (`6b0ac7b6`), lu directement : **n = 3** par bras ; A 100,7–208,7 s, B 126,2–286,3 s, C 269,6–611,0 s → **plages A et B chevauchantes**. Le « +114 % » est un **rapport de médianes** ; l'effet apparié médian A→C est **+193 %**, pas +376 % ; test du signe 3/3 : **p = 0,125**. `total_ms` inclut préparation et orchestration, avec un résidu de **2 % à 82,5 %** : S8 mesure surtout l'orchestration LLM, pas la mécanique VCS. Et S8 ne mesure **aucun `git worktree`** — la prémisse du produit n'a jamais été mesurée.

### 2.4 Aucune licence, des deux côtés

Ravum : `LICENSE`/`LICENSE.md`/`COPYING` → **404**, `license: null`, et le dépôt **le déclare** (`README.md:216-218`). Flow : **aucun** `LICENSE`. Conséquence factuelle : **aucun grant** de copie/distribution ; H2 est un **bloqueur réel**, pas une formalité. Le clean-room est **affaibli** (les agents ont lu le code).

### 2.5 Windows n'a pas de frontière de sécurité crédible

Job Objects = quotas, pas une sandbox. Windows Sandbox = Pro/Ent/Edu, mono-instance. AppContainer = seul candidat non-VM, compatibilité `rustc/cargo/node` **non prouvée**. Sur Linux, userns + bubblewrap donnent une frontière conditionnelle. **Conséquence : O3 est indisponible sur l'OS de référence**, et toute promesse symétrique Windows est fausse. Position tenable : `UNSUPPORTED`/`MANUAL_REQUIRED`.

### 2.6 Le backlog est mesurablement défaillant, et réparable sans perdre d'ID

**14/26 RF sans aucune US** (54 %) ; **5 US sans RF** ; **9/26 RF à deux phases** ; **8/15 personas hors cible** (rôles d'organisation) ; 5 familles de doublons. Proposition : **10 fusions, 4 kills, 4 defers, 3 splits** → ≈ 17 objets de travail et ≈ 8 stories produit, sans supprimer ni renuméroter un seul ID.

### 2.7 La phase documentaire n'a pas de règle d'arrêt

V6 : « on ne lance pas une sixième contre-expertise par défaut » (l. 457). Le même jour, ce GOAL en lance une septième. Aucun budget, aucune date, aucun critère de sortie. **H8 proposé** : geler après quatre corrections de forme (`X-05`, `X-06`, `X-07`, `X-11`) et n'ouvrir une nouvelle revue que sur un **fait empirique nouveau**.

---

## 3. Verdict par gate

| Gate | Verdict | Justification |
|---|---|---|
| **D0** provenance | **PARTIAL** | SHA et inventaire établis ; **droits non établis** ; aucune décision humaine enregistrée |
| **D1** revue contradictoire | **REVIEW RECEIVED / DISSENSUS OUVERT** | 8 lectures, 13 désaccords dont 7 non tranchés ; pas de faux consensus |
| **D2** contrats sûreté/transaction | **PROPOSED** | contrats écrits, **aucun test** ; 2 trous structurels non résolus (gitdir partagé, complétude du contexte) |
| **D3** protocole S9 | **DRAFT** | pré-enregistrable ; **il manque deux chiffres humains** (seuil, budget) et une autorisation |
| **D4** décision humaine | **NOT GIVEN** | aucune réponse à H1–H6 |

---

## 4. Décisions à soumettre (aucune n'est présumée)

| ID | Décision | Recommandation non contraignante |
|---|---|---|
| **H1** | option provisoire | **O0 instrumenté par défaut** ; O2-mince en hypothèse non financée ; pas d'O3 avant H3 |
| **H2** | licence/droits | **aucune extraction** sans grant écrit ; `from scratch` par défaut ; choisir une licence Flow (ou assumer « interne ») |
| **H3** | sandbox | **fail-closed par OS** ; Windows = `UNSUPPORTED`/`MANUAL_REQUIRED` ; Linux = bwrap ou refus |
| **H4** | seuil et budget S9 | **≥ 20 % apparié + plancher absolu + heures/an** ; `δ_eq` fixé avant les résultats ; budget plafonné |
| **H5** | `GO_R0_FIXTURES` | **oui, borné** à R0-a/b/c, fixtures jetables, une campagne à la fois |
| **H6** | handoff P0 | **oui, signalement seulement** ; aucun patch émis par Flow |
| **H7** *(proposé)* | interne vs produit | **interne d'abord** ; produit après un second utilisateur réel |
| **H8** *(proposé)* | arrêt de la phase documentaire | **geler** après 4 corrections ; aucune revue sans fait nouveau |

**Deux décisions débloquent tout : H2 et H4.** H5 est le meilleur rapport décision/coût. H1 peut attendre : la recommandation par défaut est réversible et n'engage aucune dépense.

---

## 5. Prochaine autorisation unique proposée

**`GO_R0_FIXTURES`** — et rien d'autre — avec trois lots :

1. **R0-a** : harnais neutre + fixtures jetables, SHA épinglés (1–2 j) ;
2. **R0-b** : reproduction P0 sur fixtures (2–3 j) → `REPRODUCED / DISPROVED / NOT_REPRODUCED` ;
3. **R0-c** : **S9-baseline** A/B/C/D/E (+F), `n = 8` exploratoire → `σ_d`, `T_coord/T_total`, coût réel des worktrees.

**Budget indicatif** (`ESTIMATE`, à figer par H4/H5) : 3–5 j d'outillage, 15–30 h humaines, 7–20 h machine, une seule campagne à la fois sur 16 Go, **aucun run concurrent avec la session Ravum**.
**Interdit dans ce GO** : code Flow, modification Ravum, CI Flow, publication, licence, PR, dépôt réel non copié.
**Sortie** : `construire / réduire / arrêter`, en deux pages, avec les chiffres.

Le mandat complet, pré-rédigé et **non lancé**, est dans [`DEEPSEEK-EXECUTION-GOAL-DRAFT.md`](DEEPSEEK-EXECUTION-GOAL-DRAFT.md).

---

## 6. Ce que ce dossier ne fait pas

- Il **ne ratifie** aucun ADR/RFC et **ne recommande aucune architecture comme définitive**.
- Il **ne prouve** aucune garantie de sécurité ; il localise des chemins et spécifie des tests.
- Il **ne code rien**, **ne mesure rien** (sauf la lecture d'artefacts existants), **ne modifie pas Ravum**.
- Il **ne tranche pas** la demande : le test de demande (une semaine, coût quasi nul) reste à faire.
- Il **ne remplace pas** une décision humaine : les huit points ci-dessus restent ouverts.

---

## 7. Handoff — ce que l'humain doit faire, dans l'ordre

1. **Lire** `DEEPSEEK-06` (décisions) puis `DEEPSEEK-07` (désaccords). Ne pas lire le dossier par le résumé seul : sept points sont volontairement non tranchés.
2. **Décider H2** (licence/droits) — sans cela, rien d'autre n'est légalement exécutable.
3. **Décider H4** (seuil, marge, budget) — sans cela, le protocole n'est pas falsifiable.
4. **Accorder ou refuser H5** — c'est la seule dépense proposée, et elle est bornée.
5. **Consigner** chaque décision avec acteur, date, preuve, périmètre, dans `DECISION_REGISTER.md`.
6. **Corriger** les quatre défauts de forme (`X-05` état machine périmé, `X-06` lettres S9, `X-07` IDs de tests, `X-11` préfixes `RAVUM-ADR`/`FLOW-ADR`). Une heure, aucun impact sur les décisions.
7. **Transmettre H6** à la session Ravum — comme signalement, pas comme ordre.
8. **Ne pas lancer R1.** Le prochain GOAL n'est pas un GOAL de code.

---

## 8. Confiance et limites

**Confiance élevée** sur : les SHA et l'inventaire ; les constats P0 localisés ; l'absence de licence ; l'échec de S8 comme preuve ; l'incompatibilité Windows/sandbox ; les défauts du backlog.
**Confiance moyenne** sur : le coût réel des worktrees (modèle, non mesuré) ; l'ampleur de la demande ; la réutilisabilité du coordinateur Ravum ; la valeur du portail P2.
**Non vérifié** (`NOT_REVIEWED`) : Scion « Anthropic » ; licences tierces de `Cargo.lock` ; 39/42 ADR non comparés à l'octet ; WDAC/minifilters/cgroups v2 ; comportement Git sur FS réseau ; toute mesure de performance.

---

*Dossier produit en lecture seule sur Ravum (`5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96`) et sur `Rwanbt/Ravum-flow@dev` (`1d5fddf`). Aucun test, aucun build, aucune publication. Les huit rapports bruts sont dans [`raw/`](raw/).*
