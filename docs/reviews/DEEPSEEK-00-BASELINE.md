# DEEPSEEK-00 — BASELINE PROVENANCE ET SOURCES (D0, partiel)

**Date d'observation :** 2026-11-09 (Europe/Paris), session Flow en **lecture seule** sur Ravum.
**Statut du document :** `PROPOSED / EVIDENCE-ONLY` — aucune décision, aucune ratification, aucun GO.
**Portée :** Lot 0 du mandat [`docs/DEEPSEEK_GOAL.md`](../DEEPSEEK_GOAL.md). Les autres livrables (`01` à `07`, `EXECUTION-GOAL-DRAFT`, `SUMMARY`) citent ce document par son ID.

Ce document ne consolide **pas** les avis des huit sous-agents : il fixe la base factuelle, les SHA, les droits observés, et les écarts documentaires déjà visibles avant leur travail. Les rapports bruts sont dans [`raw/`](raw/).

---

## 1. Dépôt de travail — `Rwanbt/Ravum-flow`

| Élément | Valeur observée | Preuve / étiquette |
|---|---|---|
| Branche de livraison | `docs/deepseek-d0-d4-review`, créée depuis `dev` | `git branch --show-current` ; `SOURCE_READ` |
| Base `dev` | `1d5fddf6efc5` — 12 commits, tous datés 2026-11-09 | `git log` ; `SOURCE_READ` |
| `main` | `5672087` (commit de bootstrap uniquement) | `gh api repos/Rwanbt/Ravum-flow/branches` ; `SOURCE_READ` |
| PR ouverte | **#33** `docs: initialize …`, `head=dev` → `base=main`, état `open` | API pulls ; `SOURCE_READ` |
| Issues | **47** ouvertes, **0 commentaire, 0 fermée** (H1–H6 #1–#6, RF-001..026 #7–#32, US-001..015 #34–#48) | API issues ; `SOURCE_READ` |
| Protection de branche | **aucune** sur `main` ni `dev` (404 « Branch not protected ») | API branches/protection ; `SOURCE_READ` |
| CI | **0 workflow** (`total_count: 0`), aucun fichier `.github/workflows` | API actions ; `SOURCE_READ` |
| Licence Flow | **aucun fichier `LICENSE`** | `Test-Path LICENSE` → faux ; `SOURCE_READ` |
| Code / build | **aucun** code source, aucun Cargo.toml, aucun artefact exécutable | inventaire racine ; `SOURCE_READ` |
| Gabarits | `.github/ISSUE_TEMPLATE/{decision,research,user_story,config}.yml`, `PULL_REQUEST_TEMPLATE.md` | inventaire ; `SOURCE_READ` |

### 1.1 Inventaire documentaire (comptages vérifiés)

| Ensemble | Attendu par le mandat | Observé | Emplacement |
|---|---|---|---|
| V6 consolidée | 1 fichier autonomme | 445 500 octets, **5 838 lignes**, CRLF, sans BOM, SHA-256 `0946bda5f08feb4a48c275ce00c95c3b2f68f8b01dbb40d227551301eb47f9d0` | [`RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md`](RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md) |
| ADR Ravum reproduits en V6 | 42/42 | **42 en-têtes `# ADR-0xx —`** détectés dans la V6 | V6 lignes ~3385–5533 |
| Flow ADR proposés | 12 | **12** fichiers `docs/adr/FLOW-ADR-000..011` | [`adr/`](../adr/README.md) |
| RFC Flow historiques | 12 | **12** fichiers `docs/rfc/RF-RFC-001..012` | [`rfc/`](../rfc/README.md) |
| Tâches RF | 26 | **26** lignes `RF-001..026` dans le backlog | [`BACKLOG.md`](../BACKLOG.md) |
| User stories | 15 | **15** sections `## US-0xx` | [`USER_STORIES.md`](../USER_STORIES.md) |
| Tests négatifs | matrice | 19 lignes dans la V6 §9 (TREE-01…OS-18) ; **16** lignes dans `SECURITY_TESTS.md` | V6 ~407–428 ; [`SECURITY_TESTS.md`](../SECURITY_TESTS.md) |

**Note de provenance :** la V6 affirme que l'archive V4 intégrée a pour SHA-256 `700c604d…` ; ce hash porte sur le texte source V4, pas sur le fichier V6 courant, et **n'a pas pu être recalculé** → `NOT_REVIEWED`. Le commit `495332e` revendique la préservation à l'octet près de la référence V6 ; le contenu est lisible et complet, mais l'antériorité exacte reste `NOT_REVIEWED`.

---

## 2. Dépôt Ravum — lecture seule, SHA réépinglé

| Élément | Valeur observée le 2026-11-09 | Preuve / étiquette |
|---|---|---|
| Visibilité | public, non archivé | API repo ; `SOURCE_READ` |
| `dev` (ancre V6) | **`5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96`**, commit du 2026-11-09T12:46:36Z, « merge(mvp0d): … (#87) » | API branches/commits ; `SOURCE_READ` |
| `main` | `54491488bba78baca8df93e14742150ad2bd9b28` (2026-11-09T02:43:45Z) | API branches ; `SOURCE_READ` |
| Branches | 10 au total, dont `feat/mvp0b-hardening`, `feat/mvp0c-hardening`, `feat/mvp0d-continuous-collaboration`, `integration/mvp0{a,b,c}-clean`, `docs/dev-integration-status` | API branches ; `SOURCE_READ` |
| PR ouvertes | **1** — `#101` **brouillon** `feat/lang-core-1.0-rc` → `dev`, créée 2026-11-09 ~15:08 | API pulls (authentifiée) ; `SOURCE_READ` |
| Issues ouvertes | **90** (`search/issues?q=…type:issue+state:open` → `total_count`) | API search ; `MEASURED` |
| Protection de branche | **`dev` protégée** : check requis `mvp`, conversation résolue requise, force-push et suppression interdits, 0 review approuvante exigée, `enforce_admins=false`, signature non requise | API `branches/dev/protection` ; `SOURCE_READ` |
| Licence | **aucun fichier `LICENSE`** à la racine au SHA épinglé (API contents → 404) | API contents ; `SOURCE_READ` |
| ADR upstream | **42** fichiers `docs/adr/ADR-000…ADR-041` (noms vérifiés) | API contents ; `SOURCE_READ` |
| Clone local | un clone local de Ravum existe, **arbre sale**, appartient à l'autre session : **non touché, non compilé, aucun git** | `git status` lecture seule, puis abstention ; `SOURCE_READ` |

**Fait notable :** l'ancre historique de la V6 (`5408d9b3…`) est **toujours** la tête de `dev` à la date de cette revue (le dépôt a été poussé à 12:46Z ; la PR #101 est un brouillon non fusionné). Les constats P0 de la V6 restent donc localisés sur le code courant, pas sur un état périmé — mais ils restent des constats de lecture, non des bugs reproduits.

**Correction de mesure (transparence) :** une première requête de cette session comptait les issues avec `per_page=80`, ce qui plafonnait le résultat et a produit « 80 ». La valeur correcte, mesurée par `search/issues` et recoupée par l'agent D, est **90 issues ouvertes**. De même, l'affirmation « aucune PR ouverte » était fausse : la PR brouillon **#101** existait déjà à 15:08. Toute prose non datée sur ces compteurs est proscrite (voir `DEEPSEEK-02`, R8 de l'agent D).

### 2.1 Blob SHAs des fichiers cités (SHA épinglé)

| Fichier | Blob SHA | Taille |
|---|---|---|
| `crates/ravum-workspace-coordinator/src/git_writer.rs` | `0194caadac4c695bdc758f2f0f17c27d1d961c3c` | 75 434 o |
| `crates/ravum-workspace-coordinator/src/coordinator.rs` | `a56f0ed88dae7e08872360cb2932e557d2bb65f0` | 87 825 o |
| `crates/ravum-workspace-coordinator/src/bin/common/mod.rs` | `783b87cfbacee3d95a502ee6006197c870d270d8` | 29 490 o |
| `crates/ravum-workspace-coordinator/src/bin/ravum-collab.rs` | `968f03bc2107a6542f42dc98d56ba6e919a4f736` | 18 271 o |
| `crates/ravum-store/src/lib.rs` | `d6ab6e926d8adb4317850f986921263260c0a55a` | 11 893 o |

---

## 3. Vérification indépendante des constats P0 (Lead, lecture de code)

Méthode : téléchargement des fichiers au SHA épinglé vers `%TEMP%`, lecture seule, numérotation de lignes du fichier upstream. **Aucune exécution, aucun test, aucun hook lancé.** Ces vérifications ne sont pas une reproduction : elles confirment la **localisation** des chemins de risque allégués.

| ID | Constat vérifié | Preuve exacte | Étiquette |
|---|---|---|---|
| `P0-HOOK-ORDER-1` | `write-tree` est calculé **avant** l'exécution des hooks, et `commit-tree` réutilise cet arbre | `git_writer.rs:841-844` (`write-tree`), `:856` (`run_commit_hooks`), `:868` (`commit-tree &tree`) | `CODE_READ` |
| `P0-HOOK-ORDER-2` | Ordre inversé : `prepare-commit-msg` déclaré avant `pre-commit` | `git_writer.rs:1221-1228` (`steps` array) | `CODE_READ` |
| `P0-HOOK-ORDER-3` | Hooks exécutés avec `current_dir(&self.repo)`, donc dans le workspace réel | `git_writer.rs:1343-1344` | `CODE_READ` |
| `P0-HOOK-ORDER-4` | Le message est relu après hooks et un message vide est refusé (partie correcte) | `git_writer.rs:857-863` | `CODE_READ` |
| `P0-ENV-1` | Environnement hérité : seules quelques variables sont retirées ; pas de `env_clear` | `git_writer.rs:1353-1355` + `REDIRECTING_GIT_ENV` ; `common/mod.rs:371-372,445-447` (`RUSTFLAGS`, `RUSTC_WRAPPER`, `RUSTC_WORKSPACE_WRAPPER`) | `CODE_READ` |
| `P0-LEGACY-1` | Protocole par défaut = `V1CollabCompat` ; sous V1, `AUTO_SAFE` = applicabilité textuelle, même sans vérification | `coordinator.rs:180` (`protocol: EvaluationProtocol::V1CollabCompat`), `:164-168`, `:330`, `:362` | `CODE_READ` |
| `P1-STORE-1` | `ravum-store` est la frontière SQLite de l'application M4 Todo, **pas** un CAS générique | `ravum-store/src/lib.rs:1` (« SQLite boundary … M4 reference application »), `:10` (`use ravum_todo_core::{…}`), `:11` (`rusqlite`) | `CODE_READ` |
| `P1-FILES-1` | Le writer manipule `String`/`Option<String>` pour chemins et contenus → modèle UTF-8 | `git_writer.rs:122` (`pub path: String`), `:124` (`pub content: Option<String>`) | `CODE_READ` |
| `P1-FILES-2` | Un index isolé **est** utilisé (`GIT_INDEX_FILE`), donc l'isolation d'index existe — mais après le `write-tree` | `git_writer.rs:15,38,88` | `CODE_READ` |
| `P0-REPRO` | Aucune perte, corruption ou exfiltration reproduite | — | `PLAUSIBLE_NOT_REPRODUCED` |

**Nuances à conserver (elles comptent pour la décision) :**
1. ~~Le défaut d'ordre des hooks n'est pas nié par le code : l'en-tête du module **documente** `prepare-commit-msg` avant `pre-commit` (`git_writer.rs:41-43`). C'est un écart assumé au comportement Git, pas une omission cachée.~~
   **Correction après contradiction de l'agent A (retenue) :** l'en-tête est **auto-contradictoire**, pas « assumé ». `git_writer.rs:37-38` affirme que le writer exécute les hooks « **in Git's order** », puis `:41-43` liste un ordre qui n'est pas celui de Git. C'est une affirmation fausse, plus grave pour un auditeur qu'un écart documenté, parce qu'elle décourage la vérification. Voir [`DEEPSEEK-07`](DEEPSEEK-07-ADVERSARIAL-DISSENT.md) §2.
2. `V1CollabCompat` est un **défaut de compatibilité** explicitement documenté, et `Coordinator::v2` existe avec des statuts sémantiques (`coordinator.rs:211,314`). Le risque réel est la **traduction** d'un `AUTO_SAFE` V1 vers un reçu Flow, pas l'absence de mode sûr.
3. L'ordre des hooks et le `write-tree` anticipé n'ont d'effet que si un hook **modifie l'index ou les fichiers**. Un hook en lecture seule ne casse rien. C'est une condition de déclenchement, non une fatalité.
4. Aucun de ces constats ne constitue une autorisation pour Flow de corriger Ravum : ils restent un **handoff** (H6), à traiter par l'autre session selon ses priorités.

---

## 4. Droits, licence et provenance

| Fait | Étiquette | Conséquence documentaire |
|---|---|---|
| Ravum n'a **aucun** `LICENSE` à la racine au SHA épinglé | `SOURCE_READ` | Droit d'auteur par défaut : **aucun grant explicite** observé. Toute copie de code Ravum dans Flow est juridiquement risquée sans accord écrit du titulaire. |
| Flow n'a **aucun** `LICENSE` non plus | `SOURCE_READ` | Le projet Flow ne peut pas encore être qualifié d'open source ; H2 couvre **les deux** côtés, pas seulement Ravum. |
| `RF-RFC-002` propose « MIT OR Apache-2.0 » comme **étude** | `PROPOSED` | Ce n'est ni une décision, ni un choix par défaut. Aucun fichier de licence ne doit être créé sans H2. |
| GitButler est cité comme `FSL-1.1-MIT` dans le mandat | `SOURCE_READ` (mandat) / `NOT_REVIEWED` (licence primaire) | Une licence FSL n'est pas OSI ; toute intégration ou copie doit être vérifiée sur la source primaire avant usage (agent F). |
| `ravum-store` retiré de l'inventaire des CAS réutilisables | `CODE_READ` (§3) | Corrige une hypothèse historique erronée ; ne pas le réintroduire dans une architecture. |

**Conclusion D0 sur les droits :** le gate D0 est **PARTIAL**, non franchi. Les SHA sont épinglés et les textes inventoriés, mais l'absence de licence des deux côtés empêche toute réutilisation de code et toute publication sous licence. Aucune conclusion juridique définitive n'est tirée ici.

---

## 5. Décisions déjà prises par un humain vs propositions d'IA

C'est une exigence explicite du Lot 0 : distinguer les deux.

| Catégorie | Constat |
|---|---|
| Décisions humaines enregistrées | **Aucune.** 47 issues ouvertes, **0 commentaire**, aucun acteur humain nommé, aucune date de ratification. `DECISION_REGISTER.md` : H1–H6 = `OPEN`, D0 `PARTIAL`, D1 `DOCUMENT-CONDITIONAL`, D2 `PROPOSED`, D3 `DRAFT`, D4 `NOT GIVEN`. |
| Décisions qui *semblent* prises mais ne le sont pas | Le nom « Ravum Flow », la séparation des dépôts, la forme CLI, le Rust comme langage d'implémentation, l'absence de worktree obligatoire : ce sont des **hypothèses réversibles** issues du plan v0.1, pas des ratifications. |
| Propositions d'IA non ratifiées | 42 ADR Ravum (`PROPOSED`), 12 RFC Flow (`PENDING`), 12 `FLOW-ADR` (`PROPOSED`), 12 idées `RF-RFC-013..024`, la V6 elle-même (`DOCUMENT-CORRECTED-AFTER-CLAUDE`), et les recommandations de cette revue. |
| Signal externe | Le seul avis externe documenté est le rapport Claude sur la **V5** (`DOCUMENT-CONDITIONAL`), pas sur la V6 ; ses cinq corrections sont intégrées sans seconde signature. |

**Règle appliquée dans tout ce dossier :** aucune issue ouverte, aucun ADR proposé, aucun document présent ne vaut autorisation de développer.

---

## 6. Écarts documentaires détectés dès la baseline

Ces écarts sont **dans le dépôt Flow lui-même**, pas dans Ravum. Ils doivent être corrigés ou explicitement gelés avant toute ratification (voir `DEEPSEEK-02`, `DEEPSEEK-06`).

| ID | Écart | Preuve | Impact |
|---|---|---|---|
| `DOC-01` | La machine d'états de `ARCHITECTURE.md` utilise les **anciens alias** que la V6 déclare obsolètes (`APPROVED_HOOKS_APPLIED`, `CHECKS_PENDING`, `PUBLICATION_INTENT_DURABLE`, `REF_PUBLISHED`) | `ARCHITECTURE.md:40` vs V6 `:184-208` | Un implémenteur peut coder deux vocabulaires ; les reçus ne seront pas comparables. |
| `DOC-02` | `ARCHITECTURE.md` recommande **O3** pour l'évaluation ; `DECISION_REGISTER.md` recommande de **comparer O0/O1 d'abord** ; la V6 §10.1 dit « O3 comme hypothèse, pas vainqueur désigné » | `ARCHITECTURE.md:56`, `DECISION_REGISTER.md:6`, V6 `:444` | Biais de confirmation documentaire contraire au mandat d'impartialité. |
| `DOC-03` | Les **lettres des comparateurs S9 se contredisent** entre la V6 et `BENCHMARK_S9.md` : V6 `D=Git classique+coordinateur, E=Flow, F=outil existant, G=séquentiel` ; `BENCHMARK_S9.md` `D=séquentiel, E=jj/GitButler, F=O0 minimal, G=Flow` | V6 `:290` vs `BENCHMARK_S9.md:8` | Risque direct de mal étiqueter les résultats d'un futur run ; doit être figé **avant** H4. |
| `DOC-04` | **Deux numérotations** pour les tests négatifs : V6 `CONTEXT-03, SANDBOX-05, PATH-08, REF-10, CRASH-11, E2E-12, CACHE-13` vs `SECURITY_TESTS.md` `TREE-03, TRUST-15, PATH-16, GIT-10, CRASH-12, E2E-13, CACHE-14` | V6 `:407-427` vs `SECURITY_TESTS.md:5-22` | Traçabilité des preuves impossible si un test est cité par deux IDs. |
| `DOC-05` | `BENCHMARK_S9.md` liste **7 comparateurs** (A–G) alors que la V6 §6.2 en liste **7 autres** (A–G) de sens différent ; le mandat en cite 6 familles | 3 sources | Le protocole D3 n'est pas encore un objet unique et versionné. |
| `DOC-06` | `ROADMAP.md` et `BACKLOG.md` ne mappent pas les mêmes granularités (`RF-0..9` vs `RF-001..026`) ; plusieurs RF ont un mapping « R3/R5 » ou « R3/R4 » ambigu | `ROADMAP.md:24`, `BACKLOG.md:16-33` | Dépendances de phases floues ; voir `DEEPSEEK-05`. |
| `DOC-07` | La V6 fixe le vocabulaire canonique mais `DECISION_REGISTER.md` et `ARCHITECTURE.md` ne renvoient pas à la table d'alias | absence de renvoi | Dérive garantie à la prochaine édition. |
| `DOC-08` | `docs/INDEX.md` et le README ne listent pas les livrables de cette revue (normal avant livraison) ; à mettre à jour dans la PR finale | inventaire | Faible. |

---

## 7. Ce qui n'a pas été lu / vérifié

| Sujet | Statut | Raison |
|---|---|---|
| 42 textes ADR Ravum upstream intégralement | `SOURCE_READ` partiel (noms + V6), lecture exhaustive déléguée à l'agent D | volume ; la V6 les reproduit |
| Licences primaires jj / GitButler / Scion / Delta | `NOT_REVIEWED` ici | délégué à l'agent F (sources primaires + dates) |
| Docs primaires Git (`githooks`, `update-ref`, `reference-transaction`) | `NOT_REVIEWED` ici | délégué à l'agent A |
| Docs primaires sandbox Windows/Linux | `NOT_REVIEWED` ici | délégué à l'agent B |
| S8 réel (mesures brutes, scripts, logs) | `MEASURED_HISTORICAL` via V6 uniquement | les artefacts de mesure ne sont pas dans ce dépôt |
| Droits d'auteur complets (auteurs, CLA, historique) | `NOT_REVIEWED` | aucune conclusion juridique |
| SHA-256 de l'archive V4 intégrée | `NOT_REVIEWED` | hash du texte source, non recalculable ici |

---

## 8. Conclusion D0

- **Provenance :** établie et datée pour les deux dépôts ; les SHA de Ravum sont **identiques** à l'ancre V6, ce qui rend les constats P0 localisables sur le code courant.
- **Lecture :** les 5 constats P0/P1 majeurs sont **confirmés en lecture** avec numéros de ligne ; **aucun n'est reproduit dynamiquement**. `CODE_READ ≠ bug reproduit`.
- **Droits :** **aucune licence** dans Ravum ni dans Flow → `BLOCKER` pour toute copie de code et pour toute prétention open source ; H2 reste entièrement ouvert, des deux côtés.
- **Décisions :** **zéro** décision humaine enregistrée ; 47 issues sans commentaire ; D4 `NOT GIVEN`.
- **Qualité documentaire :** huit écarts internes (`DOC-01..08`) dont deux dangereux pour un futur benchmark (`DOC-03`, `DOC-05`) et un biais de recommandation (`DOC-02`).

**Verdict D0 : `PARTIAL` — non franchi.** Les preuves de provenance existent ; les droits, la ratification et l'unicité du vocabulaire manquent.

---

*Sources : API GitHub `Rwanbt/Ravum` et `Rwanbt/Ravum-flow` (consultées 2026-11-09), fichiers locaux du dépôt Flow, et lecture des blobs listés §2.1. Aucun test, aucun build, aucune exécution, aucune écriture sur Ravum.*
