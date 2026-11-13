# RAVUM FLOW — V6 CONSOLIDÉE APRÈS LA REVUE FINALE DE CLAUDE

**Date : 9 novembre 2026 — Europe/Paris.**  
**Statut : `DOCUMENT-CORRECTED-AFTER-CLAUDE / HUMAN-DECISIONS-PENDING`.**  
**Verdict de Claude sur la V5 : `DOCUMENT-CONDITIONAL` ; aucun blocage documentaire.**  
**Verdict d'exécution : `NO-GO-IMPLEMENTATION` ; `GO_R0_FIXTURES` seulement sur autorisation humaine explicite.**  
**Un seul Markdown autonome : V6 corrigée + archive V4 complète (42 ADR originaux) + rapport final Claude reproduit.**

> **CETTE V6 EST LA VERSION DE TRAVAIL PRIORITAIRE.** La V4 reproduite en annexe est une archive, pas une nouvelle instruction. Le rapport de Claude porte sur la **V5**, et non sur la présente V6 : ses observations ont été incorporées sans prétendre que Claude a validé ces changements une seconde fois. **Ne pas lancer un nouveau cycle de contre-expertise ni de développement implicitement.**

## 0. Rapport Claude reçu et corrections de clôture

### 0.1 Provenance, portée et verdict

Le responsable a transmis un rapport intitulé « Rapport final de revue — Ravum Flow V5 » (texte intégral à la fin du présent fichier). Claude rapporte une lecture **en lecture seule** de `git_writer.rs` et `bin/common/mod.rs` au SHA historique `5408d9b3…`. Il n'a exécuté aucun test et n'a modifié aucun dépôt. Son verdict : **`DOCUMENT-CONDITIONAL`**, sans bloqueur *documentaire*, avec **cinq corrections** et **six arbitrages humains**. La V6 intègre ces corrections dans les sections concernées ; ce travail est **documentaire exclusivement**.

Le même rapport classe `P0-TREE`, `P0-LEGACY`, `P1-STORE`, existence du dépôt Flow, licence GitButler, Scion d'Anthropic et pages SQLite `NOT_REVIEWED` **par Claude**. Les autres lectures/sources historiques conservent leurs propres niveaux de preuve ; une absence de lecture par Claude ne réfute pas les éléments précédents.

**P0-HOOK-ORDER — `CODE_READ`, sans reproduction dynamique** : `write-tree` est calculé **avant** les hooks ; le writer appelle `prepare-commit-msg` **avant** `pre-commit`, alors que Git exécute normalement `pre-commit` d'abord ; les hooks sont lancés avec `current_dir(&self.repo)`, soit le répertoire de travail réel. Un hook qui réécrit l'index isolé peut ne pas voir ses changements inclus dans le tree publié ; un hook qui écrit des fichiers peut modifier le workspace utilisateur. **Ce sont des chemins de risque observables, non des pertes reproduites**. Comparer au fonctionnement de Git : <https://git-scm.com/docs/githooks>.

**P0-ENV — `CODE_READ`** : l'environnement des sous-processus de test et hooks hérite de variables du parent à l'exception de suppressions ciblées ; ce n'est pas une sandbox. **P1-FILES — `CODE_READ`** : les changements actuels utilisent `String` UTF-8 et prennent `100644` pour les nouveaux chemins dans le parcours examiné. Les correctifs éventuels de Ravum appartiennent à **l'autre session Ravum**.

### 0.2 Registre des amendements issus de Claude (effectués dans cette V6)

| ID | Recommandation Claude | Emplacement corrigé | Qualification |
|---|---|---|---|
| `C6-01` | Hook order inversé, tree calculé avant hooks, hooks dans workspace réel | §2, §2.1, §4.1 et tests `HOOK-02a/b/c` | `CODE_READ` ; défaut de résultat non reproduit |
| `C6-02` | Fencing : epoch + CAS Git non atomiques sans coordination | §4.2, §4.3 et `FENCE-07` | `DESIGN-RECOMMENDED` ; à tester |
| `C6-03` | Hooks/builds requièrent une matérialisation isolée, non un seul chemin FS partagé | §3.1, §3.2 et §4.1 | `DESIGN-RECOMMENDED` |
| `C6-04` | L'élargissement d'`INV-01` ne doit pas être présenté comme une simple paraphrase | §1.2 | `AMENDMENT-PROPOSED`, jamais ratifié |
| `C6-05` | Nomenclature des états unifiée et traductions historiques | §4.1 et §7.1 | `DOCUMENTED`; aucune migration codée |
| `C6-06` | Tester explicitement fichiers binaires et métadonnées | §4.4, §9 `FILE-09` | `TEST-NOT-RUN` |

### 0.3 Ordre d'autorité et interdictions

Les exigences explicites de l'utilisateur, les contraintes P0-A..I, le code épinglé et les sources primaires vérifiables priment sur les propositions d'IA. Les 42 ADR Ravum historiques restent `PROPOSED`, les 12 RFC historiques Flow restent `PENDING/PROPOSED`, et les RFC 013–024 sont seulement des projets. **Aucun résultat de benchmark, correctif P0 ou essai d'isolation n'est revendiqué comme réalisé.**

**Interdit sans GO explicite** : création/modification de dépôt GitHub, changement de branche/PR, reprise de code Ravum, correction de son GitWriter, lancement de daemon, exécution de hooks ou de code candidat non fiable, tests lourds sur le workspace actif, mise en production ou changement de licence/ADR. L'autre session Ravum poursuit indépendamment. L'expérience `GO_R0_FIXTURES` et toute phase `GO_R1..R7` requièrent des autorisations distinctes.

### 0.4 Résumé des décisions humaines restantes

Six arbitrages sont encore attendus : **(H1)** choisir une option provisoire O0/O1/O2/O3 ; **(H2)** politique de licence/droits ou approche *from scratch* ; **(H3)** modèle de sandbox, notamment niveau supporté sur Windows ; **(H4)** seuil de gain et budget S9, figés avant essais ; **(H5)** autoriser ou non `GO_R0_FIXTURES` sur des fixtures jetables ; **(H6)** transmettre ou non le handoff P0 à la session Ravum. Aucun n'est présumé approuvé.

---

## 1. Vision, contraintes et définition du produit — V6 (héritée de V5, corrigée)

### 1.1 Pourquoi Flow existe

Le responsable construit **Ravum**, langage universel à développement continu, compilation/analyse incrémentale, sûreté mémoire par défaut, contrats et preuves bornées, runtime/hot swap sûr, audio DSP temps réel et cible multi-OS / multi-device. Ravum est encore expérimental : le code Rust actuel n'est pas un compilateur Ravum autonome en production. Les objectifs P0-A à P0-I sont rappelés dans l'archive et demeurent non négociables.

En parallèle, la personne développe des projets **polyglottes** (dont Unifia) dont les PR, builds et E2E s'accumulent, tandis que plusieurs agents IA peuvent créer des conflits ou des coûts d'isolation et compilation inutiles. Ravum Flow, **projet distinct**, vise à réduire le **temps jusqu'au livrable vérifié**, sans dégrader la correction ni exiger un worktree Git *persistant* par agent.

**Flow n'est pas :** un langage, un compilateur universel, un VCS remplaçant Git, une forge, un moteur de preuves, un PatchHost Ravum, une sandbox magique, un protocole CRDT obligatoire, un daemon monolithique, une plateforme SaaS obligatoire. Il n'invoque aucun LLM par nécessité architecturale et fonctionne offline si les toolchains sont déjà installées.

### 1.2 Invariants à préserver

- **P0-A..I Ravum** : feedback continu ; performance native ; sécurité mémoire ; contrats/vérification honnêtes ; diagnostics IA ; temps réel DSP ; polyvalence ; sobriété 16 Go ; productivité jusqu'à validation. Flow doit les respecter, **sans réimplémenter** leur partie langage.
- **INV-01 ORIGINAL (inchangé, `PROPOSED`)** : « Un seul répertoire utilisateur constitue l'état canonique visible. Les propositions sont des overlays de contenu, pas des copies Git physiques par agent. Les builds peuvent matérialiser des snapshots temporaires immuables quand l'outillage externe l'exige : ces sandbox ne sont pas des worktrees Git persistants. » Lire le texte complet dans l'annexe v0.1.
- **AMENDEMENT PROPOSÉ INV-01-A (non ratifié)** : étendre explicitement les matérialisations éphémères aux **hooks autorisés** et aux **agents/outils non intégrés** lorsqu'ils nécessitent un vrai système de fichiers. Ces matérialisations peuvent être des copies ou des `git worktree` temporaires selon la solution retenue, mais ne constituent ni un deuxième état canonique ni une obligation de worktrees permanents. Un espace physique unique ne donne pas plusieurs vues isolées à des processus arbitraires. Cet amendement doit recevoir une décision humaine séparée ; il ne remplace pas silencieusement INV-01.
- **INV-02/03** non-perte silencieuse / intégration et publication contrôlées ; rollback/recovery explicites, sans `git reset --hard` sur données utilisateurs.
- **INV-04/05** validation attachée au contenu et contexte exacts, statuts honnêtes et preuve bornée.
- **INV-06..08** adaptateurs polyglottes, Git classique et local-first offline.
- **INV-09/10** budgets, annulation, erreurs/E2E visibles ; pas de cache faux vert.
- **INV-11/12** désactivation et réversibilité Git ; performance end-to-end mesurée, pas de suppression dogmatique des worktrees.

**Sources canoniques des formulations complètes :** plan v0.1 et annexes de la V4 copiés intacts plus bas. Les autres lignes de cette section sont un index ; **INV-01-A est expressément un amendement soumis à ratification**, non un nouvel invariant accepté.

### 1.3 Frontières de propriété et anti-duplication

| Responsable | Autorité exclusive proposée | Interfaces fournies | Ce qui est interdit |
|---|---|---|---|
| **Ravum** | Compilateur/frontend/IR, sémantique, preuves du programme, PatchHost, runtime/activation, `ActiveVersionId`, contraintes audio RT | Analyse, dépendances, diagnostics, reçus sémantiques compatibles avec Flow | Lier le runtime à un service Flow obligatoire ; compilation, lock ou allocation Flow dans callback audio |
| **Flow** | Intentions/propositions multi-agents, versions candidates, provenance, journal, exécution ordonnancée des validations, vérification des reçus, propriétaire logique de publication Git | CLI/API/langage-neutral, `CandidateGitTreeOid`, `VerificationReceipt`, adapter protocol | Interpréter de lui-même la sémantique Ravum ; hot swap ; promettre un merge sémantique universel |
| **Adaptateurs** | Déclarer leurs capacités, découvrir les dépendances, invoquer les outils natifs et émettre les résultats qualifiés | Rust/Cargo, JS/TS, Python, C/C++, Go, Ravum | Assurer `PASSED` sur un outil manquant, un graphe incomplet ou une suite E2E requise non lancée |

Un seul **propriétaire logique par workspace** des écritures Git canoniques ; plusieurs **workers de validation** peuvent s'exécuter en parallèle dans des espaces isolés/annulables. Ravum peut rester autonome sans Flow ; Flow ne doit jamais dépendre obligatoirement du compilateur Ravum. Le mode `FLOW_DISABLED` doit laisser un Git standard utilisable.

**Migration :** ne pas copier automatiquement `ravum-workspace-coordinator`, `ravum-snapshot`, `ravum-protocol`, `ravum-daemon`, `ravum-unit-graph`, `ravum-rust-adapter` ou `ravum-collab-oracle` dans Flow. D'abord inventorier propriétaires, licences, API, versions, tests et évolutions de la session Ravum ; choisir réemploi légal, bibliothèque indépendante, adaptateur RPC/API ou réimplémentation limitée. `ravum-store` **n'est pas** un CAS réutilisable : c'est une base SQLite de l'application M4 Todo. Toute extraction devient une migration versionnée avec double-validation, pas un copier-coller.

---

## 2. Base factuelle réexaminée au 9 novembre 2026

**Ancre de code :** `Rwanbt/Ravum@dev` = `5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96` **à l'instant de lecture**, SHA susceptible de changer. Une PR d'intégration #85 a été observée fusionnée le 9 novembre 2026 ; cela ne valide pas automatiquement Flow. La recherche de `Rwanbt/Ravum-Flow` a renvoyé `404` depuis l'accès utilisé ; ne pas déduire publique/privée/inexistante avec certitude.

| ID | Observation vérifiée par lecture du code à ce SHA | Limite et correction V5 proposée |
|---|---|---|
| `P0-TREE` | `bin/ravum-collab.rs` vérifie une version `SourceSnapshot`, mais le writer prend une union filtrée de chemins et `git_writer.rs` prépare son index Git depuis `HEAD` ; aucun lien universel receipt `GitTreeOid` n'a été identifié dans le chemin inspecté. | Divergence **plausible non reproduite**. Construire tree candidat publiable exact puis vérifier sa matérialisation, lier le reçu à `GitTreeOid + BuildContextId + PolicyVersion`. Ne pas comparer `SourceSnapshotId` directement à l'OID Git. |
| `P0-HOOK-ORDER` | Dans `git_writer.rs`, `write-tree` (l. 841–844) précède `run_commit_hooks` (l. 856), puis `commit-tree` (l. 868) réutilise l'ancien OID. `run_commit_hooks` (l. 1221–1228) fait `prepare-commit-msg` **avant** `pre-commit` ; `run_hook_process` (l. 1344) exécute les hooks dans **le workspace réel**. | **Risque triple confirmé par lecture du chemin** : modifications de l'index temporaire non intégrées au tree déjà écrit, effets sur fichiers utilisateur dirty possibles, ordre Git inversé. Aucune perte/altération reproduite dynamiquement. Exiger l'ordre Git, un espace matérialisé isolé pour hooks approuvés, capture des effets souhaités dans l'index isolé, puis `write-tree` final **après** les hooks, gel et vérification de cet arbre avant `commit-tree`/`update-ref`. Tests `HOOK-02a/b/c`. |
| `P0-ENV` | `bin/common/mod.rs` lance rustc et le test harness avec environnement parent partiellement nettoyé ; `git_writer.rs` lance les hooks avec plusieurs variables Git enlevées, sans `env_clear()` général dans ces parcours. | Risque de transmission de secrets **confirmé en lecture**, exfiltration non démontrée ; séparer exécution trusted/untrusted ; environnement allowlist + sandbox OS réelle. |
| `P0-LEGACY` | `Coordinator::new()` et la CLI utilisent encore V1 `V1CollabCompat` par défaut ; `AUTO_SAFE` en V1 ne désigne que compatibilité/applicabilité textuelle sans vérification obligatoire. | Comportement historique documenté, **pas automatiquement une faille**. Ne jamais importer `AUTO_SAFE V1` comme `VALIDATED_FOR_SCOPE` ; V2 et receipts qualifiés explicites côté Flow. |
| `P1-FILES` | `PathUpdate` porte du `String` ou `None`; les nouveaux chemins prennent le mode `100644` par défaut dans le chemin examiné. La publication depuis CLI ne traite pas forcément toutes les suppressions/renames binaires. | Support universel non établi ; exiger chemins + octets bruts + modes + suppressions + renames + symlinks + submodules/LFS selon politique. Tests adversariaux. |
| `P1-STORE` | `ravum-store` importe `ravum-todo-core`, manipule SQLite et les tâches M4 Todo. | Retirer de l'inventaire des « moteurs CAS extrayables ». Stockage Flow à décider séparément. |

**Références code épinglées (GitHub, lecture seule) :**

- <https://github.com/Rwanbt/Ravum/blob/5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96/crates/ravum-workspace-coordinator/src/bin/ravum-collab.rs>
- <https://github.com/Rwanbt/Ravum/blob/5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96/crates/ravum-workspace-coordinator/src/coordinator.rs>
- <https://github.com/Rwanbt/Ravum/blob/5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96/crates/ravum-workspace-coordinator/src/git_writer.rs>
- <https://github.com/Rwanbt/Ravum/blob/5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96/crates/ravum-workspace-coordinator/src/bin/common/mod.rs>
- <https://github.com/Rwanbt/Ravum/blob/5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96/crates/ravum-store/src/lib.rs>

**Statuts de lecture :** `FACT_CODE_READ` pour ce qui est visible dans ces chemins ; `PLAUSIBLE_NOT_REPRODUCED` pour les conséquences de sécurité et la divergence ; **aucun `PASS`, `PROVED`, CVE ou correctif revendiqué**. Les constats doivent être transmis à **l'autre session Ravum** seulement comme demandes de tests sur fixtures et sous réserve de ses priorités.

### 2.1 Ordre réel et limites des hooks Git — correction Claude

**Source primaire :** <https://git-scm.com/docs/githooks>. Pour une création de commit ordinaire : **`pre-commit` → `prepare-commit-msg` → `commit-msg` → création du commit → `post-commit`**. Les arguments/contexte de chaque hook et les refus doivent être respectés. `reference-transaction` est invoqué nativement lors de la transaction de ref (`git update-ref`), pas comme un simple hook de message. Un échec `post-commit` après publication peut être remonté en avertissement sans effacer le commit (conforme à la sémantique Git).

**Lecture ciblée à `5408d9b3…` :** le prototype actuel construit le tree **avant** les hooks ; les appelle dans un ordre différent ; et exécute les scripts dans le working tree utilisateur avec un index isolé. Un `pre-commit` peut changer l'index ou un fichier : ces effets sont impossibles à qualifier correctement en gelant l'arbre avant. **Ce constat est `CODE_READ`, pas une reproduction de perte.**

**Contrat proposé pour Flow** : matérialiser un espace jetable distinct (une vue disque complète nécessaire aux outils ; un `git worktree` éphémère n'est qu'une implémentation possible), configurer l'index alternatif et des hooks explicites, exécuter `pre-commit` en premier et les hooks de message ensuite, relire l'index isolé, décider des changements de contenu réellement stagés, geler l'arbre final par `write-tree`, puis **valider ce tree exact**. Si un hook modifie seulement des fichiers non stagés, ne pas les inclure automatiquement : appliquer une politique explicite de refus, correction/staging ou redémarrage de préparation. Ne jamais rejouer des hooks non idempotents après coup sans règle de reprise. Les hooks nécessitant accès à des ressources interdites doivent bloquer ou demander approbation, et non être contournés silencieusement.

Lancer dans le dépôt réel des hooks aux effets non bornés contredirait la protection du workspace. `GIT_INDEX_FILE` seul protège l'index, **pas** les fichiers écrits directement par un script. Les actions réseau et systèmes externes restent hors transaction Git+FS+DB ; aucun protocole Flow ne les rend atomiques par simple ajout d'un journal.

## 3. Architecture candidate **minimale**, confrontée à l'option zéro

### 3.1 Produits et modes concurrents : pas de gagnant préjugé

| Option | Description | Critère de conservation |
|---|---|---|
| **O0 : ne pas construire Flow** | Git natif + copies privées ou worktrees éphémères + intégration sérialisée + CI impactée avec outils existants. | **Baseline prioritaire.** Si assez sûre/rapide au coût global inférieur, arrêter l'effort VCS spécifique. |
| **O1 : outil existant** | jj / GitButler / orchestrateur existant (Scion Google pour orchestration), relié à Git et à une CI native. | Licence, offline, Windows/Linux, API et garanties effectivement testées. |
| **O2 : Flow mince** | Propositions `MANAGED_PATCH`, versions candidates, reçus sur Git tree exact, writer Git contrôlé, journal durable, CLI. | Doit améliorer la qualité/coût vérifié par rapport à O0/O1, sans VFS universel. |
| **O3 : hybride pragmatique** | O2 pour agents intégrés ; **vue disque privée matérialisée éphémère** (copie, worktree ponctuel ou mécanisme équivalent qualifié) pour hooks, builds et outils ordinaires non intégrés ; aucun worktree persistant obligatoire. | Repli sûr, coût disque/CPU/RAM acceptable. **Pas de promesse d'absence de matérialisation.** |
| **O4 : VFS/CRDT/distribué** | Vues FS projetées, édition collaborative, cache distant, services multi-tenant. | **DEFER** jusqu'à preuve de besoin et projet de sûreté/portabilité séparé. |

**Important :** un CRDT de texte ne garantit pas la validité sémantique d'un programme. Les branches virtuelles dans un même working tree ne fournissent pas automatiquement deux systèmes de fichiers indépendants pour deux processus externes. L'architecture **local-first** interdit de faire de Scion Anthropic (toujours en ligne, serveur central) une dépendance du MVP. Il reste une référence à **contraintes différentes**, non une preuve que Flow doit reproduire sa pile.

### 3.2 Contrôle vs exécution

```text
          HUMAIN / IDE / AGENTS / CLI (aucun fournisseur LLM obligatoire)
                           │
                PROPOSITIONS / PATCHES (base versionnée)
                           │
               FLOW CONTROL PLANE LOCAL (léger)
                 ├── session/attribution/leases/fencing
                 ├── état logique candidats et journal durable
                 ├── politique d'obligations de vérification
                 └── unique autorité locale de publication Git
                           │
              CANDIDATE Git TREE + BUILD CONTEXT IMMUTABLE
                           │
       ┌───────────────────┴────────────────────┐
       │ workers isolés / annulables            │
       │ Cargo / npm/tsc / pytest / CMake ...   │
       │ Ravum adapter futur (opt-in)           │
       └───────────────────┬────────────────────┘
                           │
        RECEIPT lié au tree/context/policy et checks requis
                           │
                GIT REF CAS + RECOVERY
                           │
               GitHub / GitLab (OPTIONNEL)
```

**Exécution réelle :** le mode `MANAGED_PATCH` peut conserver des propositions purement logiques, mais **tout build, hook ou test exigeant des chemins de fichiers reçoit une matérialisation temporaire isolée**. Cela ne signifie pas obligatoirement une commande `git worktree add` : copie, projection VFS qualifiée ou worktree éphémère sont des implémentations à comparer. L'option « zéro worktree persistant » ne promet donc ni « zéro disque » ni « zéro workspace temporaire ». Un outil `UNMANAGED` qui écrit directement dans le répertoire utilisateur n'est pas rendu sûr par un watcher.

**Flow peut être une CLI sans daemon permanent** sur un projet solo. Si une session interactive a besoin d'une coordination permanente, un daemon léger peut être opt-in. Un seul **propriétaire logique** des ref updates, non un seul processus pour toutes les compilations. Le système garde les outils usuels compatibles et permet un retour à Git même si Flow ne tourne plus.

### 3.3 Modèle d'identités explicite

| Entité | Autorité et définition | Attention |
|---|---|---|
| `GitPublishedRef` | Une ref Git locale, commit OID et son tree OID, état déjà publié ; la forge a sa propre tête distante. | `HEAD` seul ne suffit pas si plusieurs refs ; branche ciblée explicitée. |
| `UserWorkingTree` | Fichiers observés sur disque, incluant dirty, ignored/untracked et édition concurrente. | **N'est pas** automatiquement un état validé ; ne pas l'écraser. |
| `ChangeProposal` | Proposition attribuée avec `BaseCommitOid`, `BaseCandidateId`, chemins/opérations, préconditions de contenu/mode, auteur et idempotency key. | Recevabilité ≠ validation. |
| `CandidateGitTreeOid` | Tree Git préparé, figé et exact, incluant bytes, modes, symlinks/submodules selon politique ; matière d'un commit candidat. | **Pas interchangeable** avec `SourceSnapshotId` de Ravum. |
| `SourceSnapshotId` | Identité interne de Ravum/Flow, algorithme et sérialisation versionnés. | Une passerelle explicite vers Git tree est requise ; pas de comparaison directe de hashes différents. |
| `BuildContextId` | Contexte versionné : toolchain digest, cible OS/arch, flags, lockfiles, deps externes, sandbox, policy d'environnement, fichiers générés et entrées nécessaires. | Git tree seul ne suffit pas ; secrets jamais journalisés en clair, identité privée HMAC/clé locale quand nécessaire. |
| `VerificationReceipt` | `(candidate_git_tree, base context, build context, policy version, obligations, checks, evidence)` + résultat `PASS/FAIL/UNKNOWN/STALE/...`. | `PASSED` pour obligations déclarées n'est jamais `PROVED` du programme. |
| `PublicationIntent/Outcome` | Transaction idempotente : expected old ref, commit/tree candidat, fencing epoch, journal sequence, statut de réconciliation. | Pas d'ACID Git+FS+DB+hooks+remote. |
| `ActiveVersionId` | Identité runtime Ravum uniquement. | Flow n'active, ne migre ni ne roll back le runtime lui-même. |

**Reçus :** les preuves sont liées non seulement aux bytes du tree, mais aussi aux **commandes effectivement exécutées** et à un `ExecutionBoundaryId` (identité/limites de sandbox) et au périmètre de confiance des outils. Des variables non-hermétiques, fichiers `ignored`, sous-modules, dépôt distant ou services externes non figés obligent à déclarer une portée limitée, invalider la cache, lancer des checks élargis ou `UNKNOWN` ; elles ne créent pas un résultat hermétique fictif.

---

## 4. Contrat transactionnel V5 : ordre exact et modes d'échec

### 4.1 Machine d'états **canonique V6** — seul vocabulaire normatif candidat

```text
RECEIVED
  -> ADMITTED                    [identité, périmètre, base, chemins, politique]
  -> CANDIDATE_ASSEMBLED         [index isolé + espace privé matérialisé au besoin]
  -> APPROVED_HOOKS_REPLAYED     [ordre Git, effets sur index/fichiers traités explicitement]
  -> CANDIDATE_FROZEN           [write-tree FINAL après les hooks ; tree OID immuable]
  -> VALIDATING_EXACT_TREE      [matérialisation propre du tree figé + BuildContextId]
  -> VALIDATED_FOR_SCOPE        [obligations de la politique réellement satisfaites]
  -> COMMIT_OBJECT_CREATED      [commit-tree, tree OID strictement identique au receipt]
  -> INTENT_DURABLE             [ancien ref attendu, nouvel OID, receipt, epoch, idempotency key]
  -> REF_CAS_PUBLISHED          [epoch recontrôlé + update-ref CAS sous verrou writer exclusif]
  -> RECONCILED                [index / fichiers utilisateur si et seulement si non destructif]
```

**États d'écart** : `CONFLICT`, `REJECTED`, `FAILED`, `TIMEOUT`, `UNSUPPORTED`, `UNKNOWN`, `STALE`, `MANUAL_REQUIRED`, `ABORTED`, `NEEDS_RECONCILIATION`, `RECOVERED_PUBLISHED`, `RECOVERED_NOT_PUBLISHED`. Aucun statut non positif ne satisfait implicitement une obligation de publication.

**Glossaire des alias archivés (non nouveaux états)** :

| Ancienne graphie trouvée dans V4/V5 | Graphie canonique V6 | Attention |
|---|---|---|
| `APPROVED_HOOKS_APPLIED` | `APPROVED_HOOKS_REPLAYED` | « replayed » ne donne pas le droit de rejouer des effets non idempotents ; exécution contrôlée une fois pour le candidat gelé |
| `PUBLICATION_INTENT_DURABLE` | `INTENT_DURABLE` | Condition de durabilité avant CAS de ref |
| `GIT_REF_CAS` / `REF_PUBLISHED` | `REF_CAS_PUBLISHED` | Publication locale de la ref seulement, pas CI/forge |
| `READY` / `PASSED` (non qualifiés) | `VALIDATED_FOR_SCOPE` | Valide uniquement les obligations déclarées dans un contexte borné |
| `AUTO_SAFE` de protocole Ravum V1 | `TEXTUALLY_APPLICABLE` | **N'est pas** un état de promotion ni un reçu de preuve |

**Conditions incontournables** : les hooks `pre-commit`, `prepare-commit-msg` et `commit-msg` sont exécutés uniquement selon politique, dans une matérialisation isolée du candidat et **dans l'ordre Git** (cf. §2.1). Les modifications de l'index par un hook sont intégrées ou refusées **avant** le gel final du tree ; les modifications non stagées ne sont pas importées implicitement. Le message final est relu avant `commit-tree`. Les hooks `post-commit`/`reference-transaction` conservent leur sémantique spécifique. Tout effet ultérieur modifiant le tree ou les entrées qualifiées rend le receipt `STALE` et impose une revalidation ou un refus.

**Tree exact et contexte** : créer le `CandidateGitTreeOid` par `write-tree` **final**, puis vérifier le Git tree dans un environnement de validation isolé. Associer le reçu à ce tree, à `BuildContextId`, `ExecutionBoundaryId`, `PolicyVersion`, à la base et aux checks requis. Ne jamais comparer directement `SourceSnapshotId` et `GitTreeOid` ; les deux ont des conventions de hachage différentes. Après gel, les hooks ne peuvent pas modifier le tree publié sans invalider la vérification.

**Publication** : le commit object est préparé avec le tree exact validé, les intents durables sont journalisés, puis la ref est modifiée conditionnellement **sous le verrou exclusif du writer Flow** : epoch durable relu, état ref contrôlé, `update-ref <new> <old>`, issue enregistrée, verrou libéré. `update-ref` compare la ref Git, **pas** le fencing epoch. Les écritures `git` externes peuvent contourner le verrou et l'epoch ; le CAS Git reste une protection contre les ref concurrentes si leurs anciens OID diffèrent, non une garantie contre les écritures hors Flow. Au besoin, récupération idempotente de l'issue après crash.

**Rebase/squash et CI** : une modification de Git tree, contexte, politique, obligations ou trust boundary rend le reçu périmé. À tree+contexte+politique **strictement identiques**, une réutilisation de vérification doit être permise seulement par une politique explicite ; une merge queue et une CI distante peuvent imposer une requalification du SHA final. Les actions de hooks purement externes ne sont pas transactionnelles.

### 4.2 Assertions pré-CAS obligatoires

1. `prepared_commit.tree == verification_receipt.candidate_git_tree_oid` — **même modèle d'OID Git**, pas `SourceSnapshotId` comparé à un OID Git.
2. `verification_receipt.build_context_id == candidate_build_context_id` ; `PolicyVersion` et `ExecutionBoundaryId` conformes et toujours valides.
3. Toutes les obligations de la politique sont satisfaites sur le bon arbre. `SKIPPED`, `FAILED`, `TIMEOUT`, `UNKNOWN`, `UNSUPPORTED`, `STALE` ne satisfont jamais une obligation **requise**.
4. **Sous un verrou exclusif inter-processus du writer Flow pour la ref/workspace ciblée**, relire l'`epoch` durable et la ref, vérifier `publication_epoch == current_fencing_epoch` et `current_ref_oid == prepared_expected_old_oid`, puis exécuter `git update-ref <ref> <new> <old>` **sans relâcher ce verrou**. Enregistrer l'issue avant libération, avec reprise prévue si journalisation impossible après CAS. **Git compare uniquement l'OID attendu, pas l'epoch** ; les outils Git externes ne respectent pas nécessairement le verrou Flow.
5. Aucun fichier utilisateur dirty/untracked n'est effacé par la **réconciliation** ; les modifications externes sont re-checkées (TOCTOU) avec résultat observable, pas `reset --hard`.
6. Modes, chemins et opérations de fichiers sont validés comme octets et entrées Git, non uniquement sous forme `String` UTF-8 ; en cas de cas non supporté, `UNSUPPORTED` plutôt que conversion destructive.
7. Les entrées de compilation non présentes dans le Git tree sont soit figées dans `BuildContextId`, soit traitées par une validation conservatrice/hermétique, soit déclarées limites empêchant une promotion sûre.
8. Intent durable **avant** ref CAS ; replay idempotent capable de distinguer publication réalisée, jamais réalisée et situation ambiguë.
9. Un hook facultatif/obligatoire et sa policy font partie du contexte. Ne pas supprimer un hook exigé pour obtenir un verdict vert.
10. Une publication distante ou une merge queue est qualifiée selon son **nouveau tree/commit/context** si celui-ci diffère de l'état local précédemment vérifié ; status distant `PENDING/UNKNOWN` n'est jamais local `PASS`.

### 4.3 Conflits, leases et fencing — protocole corrigé

- **Managed patches :** préconditions sur contenu, modes et version des unités touchées ; sur base périmée, refuser ou rebaser explicitement **puis revalider**. Deux fichiers distincts peuvent être sémantiquement incompatibles.
- **Leases :** optionnelles et **différées** jusqu'à preuve de besoin par S9 ou collisions adversariales. Si implémentées, les epochs monotones doivent être attribuées et persistées par la même autorité d'écriture, avec nettoyage/reprise contrôlés ; expiry seule ne protège pas d'un ancien agent réveillé.
- **Fence contre race TOCTOU :** interdire la séquence « vérifier epoch → relâcher contrôle → update-ref ». Acquérir un **verrou exclusif inter-processus** couvrant : relecture epoch durable → vérification du nonce/lease/base/ref → `update-ref` Git avec `old_oid` attendu → enregistrement durable du résultat → libération. Un crash après la ref CAS et avant journal outcome est un état récupérable, **pas une preuve qu'il n'y a pas eu publication**. Documenter les comportements si fichier de lock stale, reentrant, ou si l'exécuteur est sur FS non supporté.
- **Limite de périmètre** : un `git update-ref` externe ne connaît ni epoch ni verrou Flow. Il peut modifier la ref ; la CAS `old_oid` détecte certaines races, mais **Flow ne fournit pas de fencing universel de Git externe**. Un IDE ou script écrivant directement dans le disque contourne aussi cette protection. Mode `OBSERVED_UNMANAGED` = détection best-effort seulement ; aucune prévention garantie.
- **Vérification négative à prévoir** : `FENCE-07a` ancien agent reprend après lease expiré ; `FENCE-07b` un writer tente de publier pendant que l'autre conserve le verrou ; `FENCE-07c` `git` externe met à jour la ref ; `FENCE-07d` crash entre update-ref et journal outcome. Vérifier que les protections annoncées correspondent aux résultats, sans blocage de l'autre session Ravum.
- **Sémantique** : `TEXTUALLY_APPLICABLE` n'est pas `VALIDATED_FOR_SCOPE`. Aucun oracle non borné ne peut promettre un `AUTO_SAFE` universel.

### 4.4 Opérations binaires et métadonnées de fichiers

**Constat code supplémentaire de Claude (`CODE_READ`)** : `content.as_bytes()` lit un `String` UTF-8 et le mode par défaut des nouveaux chemins est `100644`. L'essai `FILE-09` reste une spécification à exécuter seulement après GO sur fixture. Le noyau de transaction ne doit pas se limiter à `Option<String>` : modéliser des opérations `ADD/EDIT/DELETE/RENAME/COPY/CHMOD/SYMLINK/SUBMODULE` ou une abstraction Git *diff/tree* équivalente préservant les **octets arbitraires**, modes `100644`, `100755`, `120000`, `160000`, liens, Git LFS pointers, EOL/`gitattributes`, chemins non-UTF-8 là où la plateforme le permet, noms Windows interdits, jonctions, fichiers générés et casse Unicode. Pour les modes submodule/LFS, l'identité externe effective des contenus doit être liée au contexte d'analyse ; un `Git tree OID` ne contient qu'un gitlink/pointeur quand c'est le modèle du repo. Les `renames` Git étant souvent inférés par le diff, préciser si l'opération logique `RENAME` est une intention de l'agent ou une observation reconstruite et comment elle affecte le conflit.

### 4.5 Reprise après crash, WAL et GC

Établir les variantes **process kill, OS crash, power loss** séparément. Les jalons techniques minimaux sont : `COMMIT_OBJECT_CREATED`, `INTENT_DURABLE`, `REF_CAS_PUBLISHED`, `JOURNAL_OUTCOME_RECORDED`, `RECONCILED`, `RECOVERY_UNKNOWN` (le dernier est un état d'écart) ; ne pas transformer un état « ref publiée mais disque non réconcilié » en échec de commit. Après crash, inspecter le ref attendu et le journal sans réinitialiser les changements humains. Conserver les commits/blobs candidates nécessaires (références de rétention ou stockage durable dédié), prévoir GC, quotas et restauration ; ne pas affirmer que `git gc` protège des objets **inaccessibles** non épinglés.

SQLite WAL `synchronous=FULL` est une option de durabilité après coupure sous hypothèses VFS/FS réalistes ; `NORMAL` conserve les garanties de cohérence SQLite mais peut perdre des commits récents après coupure. WAL impose généralement un stockage local à la machine (pas une base WAL partagée sur un FS réseau multi-hôtes). **SQLite ne remplace pas à lui seul un magasin CAS de blobs, ni une transaction ACID inter-systèmes**. Comparer WAL + tables minimalistes, journal append-only sync, et stockage Git protégé : choisir le moins complexe répondant aux crash tests.

---

## 5. Contrat de confiance, isolation et CI honnête

### 5.1 Modèle de confiance des exécutables

| Source | Politique candidate | Garanties et limites |
|---|---|---|
| `TRUSTED_REPO` | Code/scripts explicitement approuvés, hooks éventuellement autorisés selon policy | Ne signifie pas que tout script est sûr pour une machine contenant des secrets ; limiter quand même privilèges. |
| `UNTRUSTED_CANDIDATE` | Compilation/tests/scripts dans sandbox jetable *vérifiée pour l'OS*, avec environnement allowlist, secrets hors mounts, réseau refusé selon policy, quotas CPU/RAM/process/temps | `env_clear` **ne suffit pas** sans contrôle FS/réseau ; si impossible, `UNSUPPORTED/MANUAL_REQUIRED`, jamais faux `PASSED`. |
| `HOST_TOOLING` | `git`, compilateurs, IDE, daemon sous compte utilisateur avec permissions et provenance connues | Boundary différente de `UNTRUSTED_CANDIDATE`. Les hooks natifs peuvent contourner un sandbox si exécution non contrôlée. |

Sous Linux : technologies de namespaces/seccomp/bubblewrap/etc **à tester** selon distribution et permissions réelles. Sous Windows : Job Objects/Windows Sandbox/AppContainer/etc **à évaluer**, pas à annoncer comme solution prête ni uniforme. La sécurité du sandbox inclut la prévention de lecture de faux `~/.ssh`, `.env`, tokens et sockets/montages sensibles, pas seulement le filtrage des sorties. Ne jamais utiliser de vrais secrets pendant les tests canaris.

### 5.2 Reçus, preuves et ancien protocole V1

`ADMITTED` = forme/permission/base recevable. `TEXTUALLY_APPLICABLE` = patch/hunk valide. `SEMANTICALLY_CHECKED` = vérification par outil spécifique réussie dans une portée précise. `TESTED` = obligations/tests réellement exécutés et passés. `PROVED` = uniquement si un moteur de preuve avec limites, hypothèses et trusted computing base déclarées a émis une preuve vérifiable. `VALIDATED_FOR_SCOPE` = **toutes** les obligations exigées pour une politique et un tree/context donnés ont réussi ; cela ne promet ni perfection, ni correctness générale.

**V1 de Ravum :** son `AUTO_SAFE` historique est une étiquette d'applicabilité textuelle, même quand `verification=SKIPPED` ; pour Flow traduire ce résultat en `TEXTUALLY_APPLICABLE` **uniquement**, sans promotion automatique. V2 peut transporter des évaluations plus précises, mais il faut toujours vérifier politiques, SHA, receipts et obligations, pas faire confiance au seul label.

### 5.3 Impact de compilation et E2E

**Principe :** invoquer les outils natifs (`cargo metadata`/Cargo, `tsc` + package scripts, `pytest`/outils Python, CMake/Ninja/Clang, Go tools), utiliser un graphe conservateur : **modifié + consommateurs transitifs inchangés** + scripts de build/config/lockfiles + données E2E pertinentes. Les imports dynamiques, reflection, ABI, headers, macros, générateurs, migrations de schémas et graphes incomplets déclenchent **validation élargie ou complète**. Un adaptateur qui ne peut pas trouver son toolchain répond `UNSUPPORTED`, pas `PASS`. Les E2E requis de la politique sont bloquants, même si les tests ciblés ont réussi.

Cache : clé `tree + context + toolchain + target + flags + dependencies + policies + execution boundary`; refus des résultats `UNKNOWN/STALE/SKIPPED` comme preuves ; vérification de l'intégrité des artefacts et protection contre poisoning. Une optimisation de test doit déclarer clairement les **tests réellement exécutés et ceux évités avec justification**, sans cacher l'échec d'un E2E connu. Salsa est une bibliothèque de calculs incrémentaux, pas un remplaçant universel des build systems.

### 5.4 OS / ressources / annulation

Mode solo offline sur Windows et Linux avec **16 Go de RAM de référence**, limites globales à l'ordonnanceur pour le nombre de compilers/test runners parallèles, préemption des travaux périmés, contrôle des descendants et rétention bornée des logs/artifacts. L'optimisation « plusieurs sous-agents » est une stratégie adaptative ; plus de parallélisme peut **augmenter** wall-clock en cas de contention et augmenter le nombre de conflits. Le modèle doit inclure le coût CPU/RAM des modèles agents **s'ils tournent localement**, et les ressources des applications IDE déjà présentes.

---

## 6. Benchmark S9 : méthode corrigée et falsifiable

### 6.1 Résultat historique, sans généralisation abusive

**S8 réel Ravum**, fixture Rust réduite, 27 sous-agents sur 9 campagnes (3 par stratégie) : médianes **A copies privées + intégration 111,110 s ; B dossier partagé + garde optimiste 237,674 s ; C séquentiel 528,481 s**. C'est une mesure historique, **pas** un test des vrais worktrees ni du Flow candidat. Les campagnes pilotes scriptées de quelques secondes n'ont ni le même coût ni les mêmes instruments, et ne doivent pas être agrégées. Les résultats S8 ne démontrent pas une supériorité universelle des copies privées, mais **interdisent** de présupposer que le dossier partagé serait plus rapide.

### 6.2 Pré-enregistrement avant toute expérimentation autorisée

**Comparateurs obligatoires :** A copies privées de fichiers ; B véritables `git worktree` éphémères ; C shared-directory garde optimiste ; D Git classique + coordinateur minimal (option zéro) ; E Flow transactionnel `MANAGED_PATCH`; F au moins un outil existant pertinent (`jj`, GitButler, etc. lorsque ses conditions permettent une comparaison honnête) ; G séquentiel témoin si coût justifié. **Ne pas confondre une modalité impossible sous une licence/plateforme avec un échec expérimental** : la marquer `NOT_AVAILABLE`.

**Workloads :** agents écrivant zones disjointes, même fichier, même symbole, renames, suppression, binaires, headers/API/ABI et consommateurs inchangés, broken build, E2E rouge, fichiers generated/untracked, changements de hooks, dirty, conflit de rebase, erreur réseau, caches froid/chaud, crash, sandbox, projet Rust multi-crate, TS/JS, Python et au moins un cas C/C++ si compatible budget.

**Design :** appariement des tâches et des seeds par variante, randomisation/blocage de l'ordre de passage et de la charge système, modèles/prompt/tokens/latences documentés, version/commit/toolchain hardware épinglés, CI+E2E de même politique, supervision humaine chronométrée, et profils 16 Go RAM. Le critère primaire est **le temps total d'attribution à un Git tree réellement qualifié et, lorsque la tâche l'exige, à la publication/CI distante conforme**. Mesures secondaires : p50/p95 si n suffisant, taux succès, temps de réparation, collisions, CPU/RAM/disque, coût tokens réellement observé, secrets canaris, réconciliation après crash, ergonomie.

**Statistique :** définir les paires indépendantes réelles avant le run. Ne pas demander que deux intervalles de confiance **marginaux** ne se chevauchent pas : analyser la **distribution des différences appariées**, bootstrap/IC selon protocole et pertinence opérationnelle ; un pilote sert à calibrer la taille d'échantillon. Un nombre initial `n >= 8` peut être utilisé **pour reconnaissance exploratoire**, pas comme puissance démontrée de validation. Un seuil de gain (p. ex. 15 %, 20 % ou 25 %) n'est **pas une vérité scientifique** : le responsable doit le ratifier et le **figer avant S9** ; pas de retouche a posteriori pour sauver une variante.

**Conditions éliminatoires quelle que soit la vitesse :** perte silencieuse d'une modification validée ou dirty, corruption/récupération non maîtrisée, publication d'un autre arbre que celui qualifié dans le contexte attendu, fuite de secrets fictifs en mode annoncé protégé, test requis E2E masqué, promotion d'un `UNKNOWN` comme `PASSED`. Ces conditions doivent être mesurées par adversarial corpus dédié, distinct des seuls essais de latence.

**Arrêt ou simplification :** si O0/O1 aboutit au même niveau de sûreté et de productivité à coût de maintenance/distribution plus faible, **ne pas construire** les couches transactionnelles avancées. Si O2/O3 apporte un gain net démontré sur une classe de tâches, le déployer **seulement** sur cette classe, avec fallback sûr ailleurs. Réversibilité exigée.

---

## 7. Roadmap V5, gates documentaires et autorisations explicites

### 7.1 Une seule nomenclature prioritaire

La V4 et les rapports historiques utilisent des `G0–G5`, `RF-0..9`, `R0..R7` parfois avec des définitions incompatibles. **Pour cette V5 :**

- `D0..D4` = **gates de préparation et décision documentaire**. Leur franchissement ne déclenche **aucune action** sans permission explicite de l'utilisateur.
- `R0..R7` = **phases futures d'exécution**, seulement après un `GO(scope, owner, branch/fixture, budget, tests, stop_conditions)` humain pour une phase précise. Aucun GO global implicite.
- `RF-0..RF-9`, `RF-001..RF-026` et `RF-RFC-001..012` = **identifiants historiques archivés**, jamais silencieusement renumérotés ni déclarés obsolètes. Les correspondances sont décrites ci-dessous.
- `DOCUMENT-CONDITIONAL` (avis Claude sur V5) et `DOCUMENT-CORRECTED-AFTER-CLAUDE` (statut V6) ne sont **ni** `GO_R0`, ni `GO_PRODUCTION`. Aucun statut Claude de la V5 n'est imputé rétroactivement à la V6.

### 7.2 Gates documentaires

| Gate | Preuve à fournir | Statut au moment de la V5 |
|---|---|---|
| **D0 — inventaire/provenance** | 42 ADR lus individuellement ou `NOT_REVIEWED`, 12 invariants, 12 RFC, code Ravum SHA, source complète des droits, licences et contributions, repo Flow vérifié si accessible | **PARTIAL** ; droits et repo distant inconnus |
| **D1 — critique indépendante** | Dossier multi-IA provenance exacte ; duplication Qwen/Gemini reconnue ; critiques contradictoires sans faux vote ; revue finale Claude du 9 novembre 2026 | **REVIEW RECEIVED / DOCUMENT-CONDITIONAL** ; ses cinq corrections sont incorporées en V6, sans seconde signature de Claude |
| **D2 — contrats sûreté/transactions** | Modèle Git tree/context/receipt, V1/V2, hook ordering, secrets, sandbox, fencing, file modes, recovery, cache et E2E ; RFC proposées | **PROPOSED**, non validé par tests |
| **D3 — protocole S9** | Comparateurs avec vrai worktree, tâches appariées, sources de mesure, seuils et coût pré-enregistrés, conditions stop ; propriétaire humain | **DRAFT**, seuil/budget non décidés |
| **D4 — décision humaine** | Option O0/O1/O2/O3 choisie provisoirement, licence/droits, frontière Ravum, budget, autorisation ciblée et non-objectifs | **NOT GIVEN** |

**Deux autorisations différentes après la revue :**

- `GO_R0_FIXTURES` : autorise uniquement des expériences synthétiques jetables, bornées en CPU/RAM/temps, **pas** des correctifs dans Ravum ni la publication Flow.
- `GO_R1..R7` : autorisation explicite distincte pour implémenter une phase et ses tests dans le **seul dépôt Flow** après validation des prérequis ; si un correctif Ravum s'avère nécessaire, il appartient à **l'autre session** et à sa propre décision.

### 7.3 Roadmap candidate, conditionnelle

| Phase | Objectif minimal et limites | Preuve de sortie |
|---|---|---|
| **R0** | Sur fixtures isolées, tester les risques P0, qualifier vrai Git worktree/copies/jj/GitButler si possible, fixer baseline O0 ; audit droits | `REPRODUCED/DISPROVED/NOT_REPRODUCED` par test, benchmarks honnêtes ; zéro mutation Ravum |
| **R1** | Schémas versionnés de proposition, identité Git tree, receipt, policy, journal minimal, CLI offline basique ; ni nouveau CAS général ni VFS | Parsing/fuzz, migrations, crash/replay à périmètre défini, pas de faux status vert |
| **R2** | `MANAGED_PATCH` avec MVCC optimiste, fallback `ISOLATED_EXEC`, suivi `UNMANAGED`, leases/fencing si justifiés | Concurrence, renames, modes binaires, dirty sans perte, cas refus expliqués |
| **R3** | Tree exact post-hooks gelé, builds/tests sur ce tree, sandbox OS qualifiée, receipts, adaptateur Rust natif ; V1 textuel exclu | Tous P0 TREE/ENV/LEGACY + E2E/consommateurs tests négatifs satisfaits ou mode bloqué |
| **R4** | Git writer avec CAS ref, intent durable/recovery, protection des objets candidats, réconciliation non destructive | Crash injection à chaque frontière, no lost writes, ref atomique, Git utilisable sans Flow |
| **R5** | CLI/API agents neutres LLM, TS/JS/Python/C++/Go par adaptateurs **conditionnels** et fallback conservateur | Scénarios 3+ agents, build natif, API breaking, E2E required, budgets 16 Go |
| **R6** | Publication forge optionnelle, merge queue/trains au bon SHA, intégration Ravum opt-in **via API stable**, sans double coordinator | E2E au SHA final ; deux dépôts validés indépendamment ; aucun hot swap déclenché par Flow |
| **R7** | Distribution, VFS, CRDT, remote cache, multi-tenant **séparément gated** ; abandon possible | Bénéfice empirique S9 et modèle menace, licence/support OS, no regression solo |

**Mapping des anciens jalons :** RF-0 → D0/D3/R0 ; RF-1 → R1 ; RF-2 → R2 ; RF-3 → R4 (mais **R3 vérification préalable obligatoire**) ; RF-4 → R3/R5 ; RF-5 → R5 ; RF-6 → R5 ; RF-7 → R6 ; RF-8 → R7 ; RF-9 → R6. Les tickets `RF-001..026` restent dans l'archive et doivent obtenir des tags de mapping dans le backlog, **sans modification de leur formulation historique**. Les RFC `001..012` demeurent `PROPOSED`; propositions `013..024` en V3/V4 sont des **idées documentaires, pas des fichiers acceptés**.

### 7.4 Conditions non négociables de passage entre phases

- Les RFC/droits nécessaires à une extraction de code Ravum doivent être explicitement ratifiés **avant** toute copie inter-dépôts ; option from-scratch disponible.
- Aucun résultat `NOT_TESTED` ne donne le droit d'affirmer qu'une garantie est prouvée ou mise en production.
- `R2` peut intégrer une proposition logique sans tests obligatoires **uniquement** si le résultat reste candidat non publiable ; promotion Git conditionnée à `R3` puis `R4`.
- L'exécution de code candidat non fiable ne se fait **jamais sur le workspace Ravum actif** ; validation en sandbox selon OS ou `MANUAL_REQUIRED`.
- Avant autorisation d'une nouvelle phase, le responsable doit recevoir l'état des échecs réels, coûts, résiduels, CI/E2E et un ordre de correction, sans cachage d'alerte.

---

## 8. Registre des corrections V4 → V5 et traçabilité

### 8.1 Corrections substantielles appliquées

| ID V5 | Problème de la V4 ou des rapports historiques | Correction incorporée dans la présente V5 |
|---|---|---|
| `COR-01` | Confusion d'autorité et de hashes | Tree Git candidat / receipt / commit tree comparés comme Git OIDs ; SourceSnapshotId séparé ; contexte build étendu. |
| `COR-02` | Ordre exact des hooks insuffisamment audité | Gel **après** hooks autorisés, vérification ensuite ; matrice de hooks et refus. |
| `COR-03` | `env_clear()` parfois présenté comme protection suffisante | Trust classes, contrôle des mounts et du réseau, sandbox réelle selon OS, défaut fail-closed. |
| `COR-04` | Double sens `AUTO_SAFE` | Migration explicite V1 textuel → `TEXTUALLY_APPLICABLE`, sans promotion. |
| `COR-05` | `fencing token` insuffisamment opérationnalisé | Epoch durable vérifié à la publication ; limitation pour outils non intégrés explicitée. |
| `COR-06` | CAS de blobs et SQLite WAL conflés | Choix indépendant du stockage des objets et du journal ; GC/pinning durables. |
| `COR-07` | Métadonnées fichier et UTF-8 | Opérations binaires, modes, symlinks, submodules, renames, LFS et CRLF dans modèle et tests. |
| `COR-08` | Bench Qwen/Claude trop rapide à conclure | Différences appariées, n exploratoire non probant, seuil préalable décidé par l'utilisateur, stop O0. |
| `COR-09` | Gates G0–G5 contradictoires entre versions | Nomenclature D0..D4 / R0..R7 séparant étude, GO fixtures et GO code. |
| `COR-10` | « 5 IA indépendantes » sans provenance | Qwen/Gemini identiques reconnus ; Kimi réponse légère ; Claude final demandé comme vraie revue distincte. |
| `COR-11` | Reprendre `ravum-store` comme CAS | Retiré explicitement de l'inventaire de modules réutilisables. |
| `COR-12` | Le plan ne posait pas assez d'alternatives de non-construction | O0/O1 baselines officielles ; VFS/CRDT reportés, pas condamnés définitivement. |
| `COR-13` | Double orchestration Ravum/Flow | Autorités exclusives, API opt-in, pas de cycle, pas d'extraction sans droits. |
| `COR-14` | Risque de cache faux vert et E2E | Contextes exacts, consommateurs transitifs, gate E2E conservateur et statuts honnêtes. |
| `COR-15` | Confusion revue vs implémentation | Explicitement review-only ; deux GO distincts futures pour fixtures / code, aucun implicite. |
| `COR-16 (V6)` | Claude : ordre Git et contexte du hook erronés dans prototype | §2/2.1, §4.1, `HOOK-02a/b/c`, handoff Ravum ; risques non reproduits. |
| `COR-17 (V6)` | Claude : fencing epoch non atomique avec ref CAS | §4.2/4.3 : verrou writer exclusif autour de relecture epoch + update-ref + outcome, limites Git externe. |
| `COR-18 (V6)` | Claude : absence de matérialisation pour hooks / outils ordinaires | §3.1/3.2/4.1 : vue disque privée éphémère ; pas forcément `git worktree`, mais pas « zéro matérialisation ». |
| `COR-19 (V6)` | Reformulation INV-01 présentée comme simple index | §1.2 : INV-01-A proposé, formulation historique intacte et non ratifiée. |
| `COR-20 (V6)` | Alias d'états V4/V5 qui s'entrechoquent | §4.1 : machine d'états V6 canonique et table d'alias historiques. |
| `COR-21 (V6)` | Fichiers non texte et provenance du dernier avis | §4.4/§9 : `FILE-09`, annexe intégrale du rapport Claude, niveaux de preuve conservés. |

### 8.2 Index de couverture après la revue Claude

| Ensemble historique | Couverture documentée | Emplacement de référence |
|---|---|---|
| **P0-A..P0-I** | Les neuf objectifs Ravum, conservés | §1.2 V5 et annexe V4 dossier maître |
| **INV-01..INV-12 Flow** | Douze invariants de Flow | §1.2 V5 + annexe V4 plan v0.1 complet |
| **ADR-000..ADR-041** | **42 textes complets originaux** | Archive V4, section `ANNEXE Z` ; pas de statut changé |
| **RF-RFC-001..012** | Douze RFC d'origine | Archive V4, registre et annexe décisions |
| **RF-RFC-013..024** | Propositions ultérieures non ratifiées | Archive V4/V3, spécifiquement §§ V3 registres |
| **RF-0..RF-9** | 10 jalons historiques | Archive V4 plan v0.1, mapping §7.3 V5 |
| **RF-001..RF-026** | 26 tâches historiques | Archive V4 backlog original, mapping par phase à établir explicitement avant R1 |
| **S8 réel / pilotes** | Séparés sans agrégation | §§2/6 V5 + archive V4 dossier expérimental |
| **P0 TREE/ENV/LEGACY/HOOK** | Constats, reproductibilité limitée | §2 V5, plan tests §9 |

**Claude a rapporté** avoir trouvé les 42 ADR, les 12 RFC, INV-01..12, RF-0..9 et RF-001..026 dans la V5 ; cela ne ratifie aucun de ces textes. La présente V6 conserve l'archive V4 sans modification et vérifie automatiquement la présence des 42 en-têtes ADR. Les tags de mapping individuel des 26 tickets restent à produire avant R1.

---

## 9. Matrice de tests négatifs et livrables futurs (spécification seulement)

**NE PAS EXÉCUTER PENDANT LA REVUE.** Toutes les expériences doivent, après autorisation `GO_R0_FIXTURES`, tourner sur dépôts synthétiques jetables avec des secrets **fictifs**, quota mémoire/CPU et supervision de l'autre session Ravum pour éviter compétition de ressources.

| Test | Scénario d'échec injecté | Comportement attendu |
|---|---|---|
| `TREE-01` | B dirty locale hors patch rend A compilable ; commit Git prévu ne contient que A | Aucun reçu réutilisé ; build clean du tree exact échoue ou publication refusée. |
| `HOOK-02a` | `pre-commit` approuvé modifie l'index alternatif après `write-tree` | **Reproduire d'abord** le défaut Ravum sur fixture ; pour Flow, la modification staged est intégrée au tree final ou publication bloquée, jamais ignorée silencieusement. |
| `HOOK-02b` | `pre-commit` modifie un fichier dirty du workspace utilisateur dans l'implémentation historique | **Reproduire sans données réelles** ; pour Flow, hooks dans une matérialisation privée, aucune écriture au workspace utilisateur avant réconciliation sûre. |
| `HOOK-02c` | Hooks journalisent leur ordre et leurs arguments | Comparer `pre-commit → prepare-commit-msg → commit-msg → post-commit` à Git ; `reference-transaction` est géré séparément/nativement. |
| `HOOK-02d` | Hook modifie seulement un fichier non stagé ou échoue, hook post-commit échoue après CAS | Refus ou traitement explicite de la modification non stagée ; refus avant commit si pre-hook rouge ; après publication, warning/recovery visible sans prétendre annuler le commit. |
| `CONTEXT-03` | Fichier ignored, gitlink/submodule ou blob LFS externe influence le build | Provenance déclarée et requalification, ou `UNKNOWN/MANUAL_REQUIRED`. |
| `ENV-04` | Variable `FLOW_FAKE_SECRET` présente dans environnement parent | Absent de subprocess untrusted, des logs et artefacts. |
| `SANDBOX-05` | Test tente de lire faux `~/.ssh` et d'ouvrir réseau sortant | Refus effectif **si mode sandbox annoncé**, sinon code non exécuté. |
| `LEGACY-06` | Résultat V1 `AUTO_SAFE` avec check skipped | Promotion refusée, étiquette `TEXTUALLY_APPLICABLE` uniquement. |
| `FENCE-07` | Ancien agent reprend après expiry ; race entre lecture epoch et `update-ref` ; `git` externe ; crash avant outcome | Lock writer couvrant epoch+ref-CAS+outcome, rejet contrôlé du stale ; races externes et crash documentés honnêtement. |
| `PATH-08` | `../`, symlink/junction, Unicode/case, Windows reserved paths, rebases/renames | Aucune écriture hors cible ou écrasement silencieux ; erreur bornée. |
| `FILE-09` | Binaire non-UTF8, `100755`, `120000`, suppression, rename, `160000` | Identité, modes et opérations préservés ou `UNSUPPORTED` explicite. |
| `REF-10` | Une autre publication modifie expected old Git ref avant update-ref | CAS échoue ; aucune publication avec receipt obsolète. |
| `CRASH-11` | Arrêt forcé avant/après intent/commit object/ref/reconciliation | Replay idempotent, refus de reset destructif, état publié identifiable. |
| `E2E-12` | API change, consommateur inchangé, E2E requis rouge | Gate final rouge, pas de « impacted green » injustifié. |
| `CACHE-13` | Flags/lockfile/toolchain/platform/sandbox/policy changent | Cache miss ou revalidation ; pas de faux hit. |
| `RESOURCE-14` | RAM 16 Go, compilations concurrentes, annulation, child processes | Budgets cumulatifs, pas de deadlock/fuite de processus. |
| `FORGE-15` | Merge queue/squash/rebase produit nouveau tree/context | Qualification au commit/tree final ou statut bloquant. |
| `RECOVERY-16` | SQLite WAL corrompu/plein, Git objet unreachable, `git gc`, journal tronqué | Diagnostic honnête et reprise non destructive, aucune publication inventée. |
| `UNMANAGED-17` | Éditeur externe écrase un fichier pendant soumission | Pas de promesse anti-écrasement ; détection best-effort, pas d'attribution fausse, restauration uniquement si copie existe. |
| `OS-18` | Windows path/case, antivirus, lockfiles ; Linux permissions/FUSE indispo | Modes supportés correctement annoncés, aucun fallback destructif. |

**Livrables post-GO :** suite synthétique adversariale, provenance SHAs, résultats exacts, journal d'exécution, snapshot diff, reçus, rapport p50/p95 selon n, coût humain/tokens/RAM/CPU et constat d'échec sans masquage. Aucun fichier source Ravum actif n'est modifié par la future équipe Flow.

---

## 10. Verdict de clôture documentaire **après** la revue Claude

- **Claude sur V5** : `DOCUMENT-CONDITIONAL`, aucun blocage documentaire, cinq corrections ponctuelles et six décisions humaines. Ce rapport est reproduit intégralement en annexe et ne doit pas être rebaptisé « DOCUMENT-READY ».
- **V6 présente** : `DOCUMENT-CORRECTED-AFTER-CLAUDE / HUMAN-DECISIONS-PENDING`. Les cinq corrections ont été intégrées ; il n'y a **pas de nouvelle signature indépendante sur V6**, ni d'essai de code.
- **Produit** : `NO-GO-IMPLEMENTATION` pour R1..R7 et tout code productif ; `GO_R0_FIXTURES` seulement si l'utilisateur l'autorise explicitement sur des fixtures jetables. Le dépôt GitHub Flow n'est toujours pas prouvé accessible ; aucune publication ni changement Git n'a été faite.
- **Position technique** : O0/O1 constituent de vraies baselines de sortie ; O2/O3 restent candidats expérimentaux ; VFS/CRDT/CAS général/daemon permanent/cache distant sont différés et peuvent être abandonnés si les mesures ne les justifient pas.

### 10.1 Décisions humaines requises — aucun choix implicite

| Décision | Options pertinentes | Recommandation documentaire non contraignante |
|---|---|---|
| H1 — architecture provisoire | O0, O1, O2, O3 ; O4 différée | **Comparer O0/O1 d'abord** ; O3 comme hypothèse à éprouver, pas vainqueur désigné |
| H2 — licence/droits | audit provenance puis extraction ; réimplémentation originale limitée | Garder **from scratch** disponible si droits incertains ; aucune licence créée automatiquement |
| H3 — sandbox | exécution trusted contrôlée ; untrusted isolée selon OS ; blocage si indisponible | Sur Windows/Linux, annoncer précisément les protections établies ; sans sandbox démontrée : `UNSUPPORTED/MANUAL_REQUIRED` |
| H4 — S9 | seuil de gain, taille/complexité des campagnes, temps & RAM, coût humain | Pré-enregistrer le critère principal et les **différences appariées** ; pas de seuil arbitraire prétendu scientifique |
| H5 — expérimentations | Autoriser ou refuser `GO_R0_FIXTURES` | **Pas d'autorisation présumée** ; uniquement fixtures synthétiques, jamais repo Ravum actif |
| H6 — Ravum | transmettre ou non le handoff P0 | Transmettre le signalement, mais laisser tests/correctifs et priorités à l'autre session |

### 10.2 Handoff autonome pour la session Ravum — **signalement, pas ordre d'intervenir**

> **Contexte :** Flow est en lecture seule sur `Rwanbt/Ravum@5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96`. Claude a confirmé en lecture : (i) `git_writer.rs` écrit l'arbre avant les hooks (841–856), (ii) le hook `prepare-commit-msg` est appelé avant `pre-commit` (1221–1228), (iii) les hooks ont `current_dir(&self.repo)` (1344), (iv) les subprocess conservent la plupart des variables d'environnement, (v) le writer traite du texte `String` et attribue `100644` aux nouveaux chemins. **Aucun défaut dynamique n'est reproduit.** Vérifier d'abord le SHA et le chemin actuel, puis, *si cette session l'autorise et suivant ses priorités*, exécuter `HOOK-02a/b/c`, `TREE-01`, `ENV-04`, `LEGACY-06`, `FILE-09` sur **fixtures jetables** avec secrets canaris fictifs. En cas de reproduction, proposer le correctif minimal et tests de non-régression, sans affaiblir les gates CI/E2E ; rapporter `REPRODUCED / DISPROVED / NOT_REPRODUCED` et les SHAs. Ne pas changer la branche Ravum depuis la session Flow.

### 10.3 Clôture

**Fin de la revue documentaire :** on ne lance pas une sixième contre-expertise par défaut. La suite est un arbitrage humain des six décisions, puis, uniquement s'il y a un `GO_R0_FIXTURES` explicite, une étude empirique sur fixtures. La V6 n'est ni du code implémenté, ni une preuve de sûreté, ni une autorisation à l'agent de créer/merger des PR.

---

## ANNEXE UNIQUE — ARCHIVE V4 INTÉGRALE, NON MODIFIÉE

**Provenance :** `RAVUM-FLOW-V4-CONTRE-EXPERTISE-UNIQUE.md`, SHA-256 UTF-8 : `700c604d118295c53e11aacf3b59589dff8622a301b46d396c6db62b133d529c`. Le texte ci-dessous est reproduit à l’identique pour préserver tous les ADR, RFC, textes de modèles et variantes historiques. **Seul le mandat V5 ci-dessus est opérationnel** ; les demandes d’action contradictoires dans l’archive ne sont pas des autorisations.

---

# RAVUM FLOW — DOSSIER UNIQUE V4 DE CONTRE-EXPERTISE ET ARBITRAGE

**Date : 9 novembre 2026. Statut : `REVIEW-ONLY` — aucune implémentation autorisée.**

**Mode d'emploi :** ce fichier Markdown constitue **la seule pièce nécessaire** pour transférer le contexte à une nouvelle discussion IA. Copier/coller son intégralité (ou le joindre tel quel si la limite de contexte empêche le collage). Il contient ci-dessous l'arbitrage V4, le GOAL autonome de revue, puis l'archive intégrale du texte reçu, comprenant les revues multi-IA, la V3 et **les 42 ADR originaux complets**. Les sections anciennes de l'archive sont des pièces historiques, non des instructions opérationnelles prioritaires.

## A. MANDAT PRIORITAIRE POUR LA NOUVELLE DISCUSSION

Tu agis en **auditeur critique indépendant**, idéalement avec sous-agents réellement séparés, pour vérifier et améliorer le plan de **Ravum Flow** avant toute mise en œuvre. **Ne code rien, ne crée aucun dépôt, ne publie aucune PR, ne change aucun ADR, fichier, licence, CI ni branche.** Une autre session poursuit le développement de **Ravum**, qui doit rester intégralement en lecture seule ici. N'exécute aucun benchmark lourd sur sa machine ou son workspace. Ne confonds jamais `GO` documentaire et autorisation d'implémenter.

### A.1 But utilisateur non négociable

Ravum Flow est envisagé comme une couche **universelle**, indépendante du langage Ravum et compatible avec Rust, TypeScript/JS, Python, C/C++, Go et les langages à venir :

- une source de référence unique et des modifications continues par plusieurs agents humains/IA ; **pas de `git worktree` persistant obligatoire par agent**, mais isolation éphémère permise quand nécessaire pour la fiabilité ;
- provenance des patches, autorité claire de la publication, absence d'écrasement silencieux en mode contrôlé, reprise idempotente après crash ;
- compatibilité Git local, GitHub et GitLab, utilisable hors ligne, `local-first`, sans service cloud ni LLM obligatoire ;
- analyses/compilations/tests incrémentaux fondés sur l'état exact et les dépendances ; **jamais de faux vert E2E ni de preuve simulée** ;
- sobriété PC Windows/Linux 16 Go RAM ; évolutivité future vers équipes, monorepos et distribution ;
- mesure primaire : **temps total jusqu'au livrable vérifié** avec coûts humains, agents, CPU/RAM, conflits, essais E2E et reprise ;
- Ravum conserve la sémantique, le compilateur, les preuves de propriétés du programme, le runtime, le PatchHost/hot swap et les contraintes DSP temps réel ; Flow n'active aucun code dans le runtime et n'intervient jamais dans le callback audio.

L'architecture proposée, les ADR et RFC originaux du dossier restent **des propositions, jamais ratifiées**. Les modèles sont autorisés à recommander l'option zéro (ne pas développer un moteur VCS nouveau).

### A.2 Sources et états à revérifier

- Dépôt Ravum : <https://github.com/Rwanbt/Ravum>. Lors de cette vérification **en lecture seule** du 9 novembre 2026, `dev` était à `5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96`. Ce SHA peut évoluer : l'épingler à nouveau avant tout avis technique.
- Dépôt cible Flow : <https://github.com/Rwanbt/Ravum-Flow>. L'API GitHub répondait `404` avec l'accès utilisé. **Cela indique inaccessible/absent/ou privé, pas une preuve certaine d'inexistence** ; ne pas le déclarer créé.
- La PR Ravum #85 est indiquée fusionnée le 9 novembre 2026 à 11:26:56 UTC, mais toute suite doit vérifier l'état courant.
- Source historique primaire du risque : `crates/ravum-workspace-coordinator/src/{git_writer.rs,coordinator.rs,bin/ravum-collab.rs,bin/common/mod.rs}` et `crates/ravum-store/src/lib.rs` au SHA épinglé.
- Deux sources **Scion distinctes et réelles** : GoogleCloudPlatform/scion (orchestrateur open source Apache-2.0 : <https://github.com/GoogleCloudPlatform/scion>) **et** Scion interne d'Anthropic (VCS serveur décrit publiquement le 13 octobre 2026 : <https://www.anthropic.com/research/redesigning-source-control>). Ne jamais mélanger leurs fonctionnalités, licences ou modèle online/offline. Le Scion Anthropic n'est pas une base réutilisable local-first.
- Licence GitButler actuelle à vérifier au fichier exact : <https://github.com/gitbutlerapp/gitbutler/blob/master/LICENSE.md>, historiquement `FSL-1.1-MIT`, **pas MIT pur pour les versions récentes**.
- SQLite WAL/durabilité : <https://www.sqlite.org/pragma.html>. SQLite WAL est un journal transactionnel ; il **ne remplace pas sémantiquement** un magasin CAS de blobs.
- Les 42 ADR Ravum figurent **intégralement** dans l'annexe de l'archive ci-dessous : `ADR-000` à `ADR-041` tous présents, statut historique `PROPOSED`.

### A.3 Fiabilité et indépendance des revues

**Correction matérielle vérifiée sur les octets du document utilisateur reçu : les sections « Qwen » et « Gemini » sont strictement identiques, caractère pour caractère (11 357 caractères chacune après extraction). Elles ne constituent qu'une seule analyse documentaire pour la synthèse.** Ne pas confondre cela avec l'affirmation historique « Kimi et Gemini identiques » qui concernait possiblement une autre version ou un autre corpus. Ici, la section Kimi, courte, est différente mais ne donne pas d'audit approfondi. Le texte ChatGPT V3 fait partie des analyses transmises et ne constitue pas un nouveau réviseur indépendant du présent dossier.

Toute affirmation « cinq IA ont confirmé » doit être remplacée par une **matrice de provenance** : modèle, corpus lu, accès code oui/non, sources externes vérifiées oui/non, autonomie réelle du raisonnement, identités textuelles, tests exécutés ou non. Ne pas compter des copies comme votes indépendants.

## B. ARBITRAGE TECHNIQUE V4 — FAITS, RISQUES ET POSITIONS

### B.1 Constats vérifiés par lecture seulement

| ID | Constat observé sur Ravum `dev` au SHA du 09/11/2026 | État et priorité |
|---|---|---|
| P0-TREE | `ravum-collab` assemble/valide un `SourceSnapshot`; `GitWriter` génère un tree Git à partir de `HEAD` et des chemins sélectionnés. Aucun contrôle d'identité systématique `verified_git_tree_oid == candidate_git_tree_oid` n'a été identifié dans ce parcours. | `FACT_CODE_READ`, divergence **plausible mais non reproduite**, test P0 |
| P0-ENV | Le verifier et les hooks lancent des processus avec environnement hérité et quelques variables supprimées ; pas d'`env_clear()` dans les chemins inspectés. | `FACT_CODE_READ`, exposition potentielle, exfiltration **non démontrée**, test P0 |
| P0-LEGACY | La CLI `ravum-collab` conserve `--protocol v1` par défaut et `Coordinator::new()` est V1 ; `AUTO_SAFE` y décrit l'applicabilité **textuelle**, pas une qualification sémantique. | Comportement intentionnel historique ; **ne pas** importer ce statut comme reçu de promotion dans Flow |
| P1-FILES | `PathUpdate` porte `Option<String>` ; nouvelles entrées prennent par défaut le mode Git `100644` ; dans la CLI, les chemins acceptés absents du snapshot conduisent à une erreur. | Vérifier avec tests les cas **binaires, executable bit, suppression et rename** ; ne pas présumer compatibilité universelle |
| P1-STORE | `ravum-store` gère la base SQLite de l'exemple **M4 Todo** et non un CAS générique. | Ne pas compter comme bloc de stockage prêt à extraire |

Les constats de lecture précédents se retrouvent sur le SHA `5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96`, mais **ceci n'est ni la reproduction de défauts, ni la vérification exhaustive des garanties du système**. Faire intervenir la session Ravum pour des reproductions/correctifs dans ses propres priorités ; Flow reste lecture seule.

### B.2 Invariant essentiel : l'identité de ce qui est vraiment vérifié

La comparaison **correcte** est entre **deux OID Git tree du même modèle** : l'arbre immuable sur lequel les vérifications ont été effectuées et `commit^{tree}` du commit préparé pour publication. Un `SourceSnapshotId` propriétaire **n'est pas** interchangeable avec un Git tree OID. Un tree OID n'est pas non plus suffisant pour capturer toutes les entrées de build (toolchain, OS, flags, fichiers ignorés, modules, secrets, processus externes). Le reçu doit inclure `CandidateGitTreeOid`, `BuildContextId`, `PolicyVersion`, les obligations requises et leurs verdicts.

Machine d'états cible **proposée** :

```text
RECEIVED -> ADMITTED -> CANDIDATE_ASSEMBLED
 -> APPROVED_HOOKS_APPLIED -> CANDIDATE_FROZEN
 -> VALIDATING_EXACT_TREE -> VALIDATED_FOR_SCOPE
 -> PUBLICATION_INTENT_DURABLE -> GIT_REF_CAS
 -> RECONCILED
```

États latéraux : `CONFLICT`, `UNMANAGED`, `FAILED`, `UNKNOWN`, `TIMEOUT`, `STALE`, `MANUAL_REQUIRED`, `NEEDS_RECONCILIATION`. Un changement après gel (hook, base Git, entrée externe significative) impose **re-préparation ou revalidation** ; aucune promotion d'un reçu périmé. Git est l'autorité des états publiés ; le journal des intentions en cours ; les snapshots/trees des candidats et des entrées de vérification ; le runtime Ravum de sa version active.

### B.3 Contrôle d'accès, secrets, hooks et isolation

Distinguer `TRUSTED_REPOSITORY_CODE` et `UNTRUSTED_CANDIDATE`. `env_clear()` et une allowlist minimisent une fuite d'environnement mais **ne suffisent pas** à isoler un exécutable qui peut lire `$HOME`, `.ssh`, fichiers de tokens ou utiliser le réseau. Pour candidat non fiable : sandbox OS réellement qualifiée, volumes contrôlés, réseau restreint, quotas, descendants annulables, faux secrets de tests et logs redigés. Si aucune sandbox suffisante n'est disponible sur un OS, déclarer `UNSUPPORTED` / `MANUAL_REQUIRED` ; ne jamais inventer une garantie universelle. Les hooks Git doivent avoir une politique séparée (approuvés, exécutés avec limites, non silencieusement contournés).

### B.4 Transactions et concurrence

Un seul **propriétaire logique de la publication Git** par workspace, mais plusieurs vérificateurs isolés/annulables. `git update-ref <new> <old>` réalise un CAS de référence, **pas** une transaction ACID globale Git + filesystem + SQLite + hooks + forge. Définir intent durable, idempotence, récupération après crash avant/après CAS, et réconciliation non destructive des modifications humaines. Si un lease expire, utiliser un **fencing token monotone** contrôlé à l'admission et à la publication : un processus réactivé ne doit pas retrouver un droit d'écrire sur un ancien epoch.

**SQLite WAL et CAS sont des fonctions distinctes.** Choisir journal append-only ou SQLite WAL `synchronous=FULL` selon garanties de crash et coût ; réutiliser le stockage objet de Git pour les trees/blobs quand c'est sûr, ou conserver les patches dans un stockage durable séparé et versionné. Garantir que `git gc` ne supprime pas des objets candidats encore nécessaires, et que le nettoyage n'efface jamais des propositions en cours. **Ne pas créer une pile CAS/VFS générale au MVP sans preuve de nécessité.**

### B.5 Les modes ne garantissent pas la même chose

| Mode | Garantie plausible | Limite explicite |
|---|---|---|
| `MANAGED_PATCH` | Patches sur base exacte, CAS, attribution, refus des collisions et vérification de tree. | Un agent avec accès disque direct peut contourner le protocole. |
| `ISOLATED_EXEC` | Exécution dans espace jetable privé, puis proposition réconciliée ; fallback fiable pour outils classiques. | Temps, mémoire, stockage et sécurité réelle de sandbox à mesurer. |
| `OBSERVED_UNMANAGED` | Détection best effort et signalement des écritures hors protocole. | Impossible de garantir la prévention d'un overwrite déjà fait. |
| `VIRTUALIZED` | Vues de fichiers par agent potentiellement transparentes aux outils traditionnels. | Coûts, portabilité et sécurité OS non démontrés ; **DEFER**, pas « inutile à jamais ». |

La préférence utilisateur reste une source physique canonique unique et l'absence de worktree **obligatoire**. Le fallback éphémère n'est pas une remise en cause de cette vision ; l'absence totale d'isolation ne doit pas devenir un dogme.

### B.6 CI et caches

Les adaptateurs doivent invoquer les **outils natifs** de chaque langage avant d'envisager un build engine universel : `cargo metadata`/Cargo pour Rust, gestionnaires de packages/tsconfig/test runners pour TS, pytest et outils Python pertinents, CMake/Ninja/Clang pour C++. Une dépendance d'API changée requiert la vérification des **consommateurs inchangés**. Graphe incomplet/imports dynamiques/script non hermétique => validation conservative élargie. `Salsa` est un framework de calculs incrémentaux, **pas** un remplaçant transparent de Cargo, Bazel, Nx ou Pants. Les suites E2E requises restent bloquantes ; pas de faux `PASSED` par cache `STALE`, `UNKNOWN`, `SKIPPED`, `TIMEOUT` ou non exécution.

### B.7 Benchmark et règle d'arrêt

Le benchmark S8 historique a mesuré : copies privées + intégration **111,110 s**, répertoire partagé + garde simple **237,674 s**, séquentiel **528,481 s**, trois répétitions par stratégie et 27 sous-agents sur fixture réduite. **Ce n'est pas** une comparaison de vrais worktrees Git ni une mesure de Flow transactionnel. Les pilotes scriptés de quelques secondes ne doivent pas être agrégés au S8 multi-agents.

Pré-enregistrer pour le S9 : même tâches, même matériel 16 Go, mêmes modèles/prompt/budgets, ordre randomisé/alterné, caches chauds/froids, effets de réseau et tests E2E identiques ; distinguer A copies de fichiers, B vrais Git worktrees, C dossier partagé gardé, D option zéro Git + intégration légère, E Managed Flow, F solution existante (GitButler/jj) quand réalisable. Mesurer : temps total jusqu'au livrable vérifié, p50/p95 quand échantillon suffisant, taux de réussite, coûts humains, tokens réellement comptabilisés, RAM/CPU/disque, collisions et reprises.

**Erreur méthodologique à éviter :** exiger que deux intervalles de confiance marginaux ne se chevauchent pas n'est pas le test statistique approprié pour des campagnes **appariées**. Estimer plutôt la **distribution de la différence appariée**, son intervalle de confiance et sa pertinence opérationnelle ; calibrer la taille de l'échantillon après pilote ou analyse de puissance. Les valeurs de gain **15 %, 20 %, 25 %** proposées ailleurs sont des options à arbitrer **avant** le benchmark, non des normes scientifiques. Conditions éliminatoires : perte silencieuse, écrasement de dirty, publication d'arbre non vérifié, secret exposé, E2E requis caché. Réaliser un vrai test d'arrêt : si l'option zéro ou un outil existant est aussi sûre et productive à coût total moindre, ne pas construire le moteur supplémentaire.

### B.8 Niveaux de décision et phases (numérotation unifiée proposée)

**D0–D4 : gates documentaires, lecture seule et sans exécution lourde** :

| Gate | Preuve documentaire attendue | État de référence |
|---|---|---|
| `D0` Sources/provenance | Document maître, 42 ADR complets, 12 RFC, 12 invariants, SHAs, contributions/licences, rapports sources | PARTIAL ; droits non déterminés |
| `D1` Contre-expertise indépendante | Rapports A–H réellement séparés, option zéro, désaccords et doublons identifiés | PARTIAL ; Qwen/Gemini dupliqués |
| `D2` Contrat d'états et menaces | tree OID/reçu/context, statuts V1/V2, hooks, secrets, crash, fencing, offline | PROPOSED / NON RATIFIÉ |
| `D3` Protocole S9 | corpus, groupes de contrôle, mesures, taille d'échantillon et seuils pré-enregistrés | DRAFT |
| `D4` Arbitrage humain | choix d'architecture réversible, licences, droits, budget, OWNERS Ravum/Flow | NOT GIVEN |

**R0–R7 : phases futures conditionnées par GO explicite**, sur fixtures isolées (jamais le dépôt Ravum actif) :

| Phase | Objet | Gate technique |
|---|---|---|
| `R0` | Reproduire P0 et baseline option zéro | Sur fixtures, rapports avec `REPRODUCED/DISPROVED/NOT_REPRODUCED` |
| `R1` | API minimale de propositions/receipts | Identités/version, migration, tests négatifs, sans CAS/VFS général |
| `R2` | Patch contrôlé + fallback exécution isolée | Pas de perte silencieuse, conflits, leases/fencing et annulation |
| `R3` | Préparer tree exact + sandbox + tests | Un reçu pour le même tree/context qui sera publié |
| `R4` | Publication Git CAS + recovery | Power-loss/process crash simulés, hooks, dirty, idempotence |
| `R5` | CLI/agents + adaptateurs polyglottes | Rust puis TS/Python/C++; E2E et consommateurs inchangés |
| `R6` | GitHub/GitLab optionnels + intégration Ravum | Qualification SHA, pas de cycle, pas de modification du runtime par Flow |
| `R7` | Options avancées conditionnelles | Preuve du gain avant VFS, distribution, cache distant ou CRDT |

**Cette numérotation sépare clairement autorisation de concevoir, d'expérimenter et de coder**, alors que plusieurs versions précédentes utilisaient `G0–G5` pour des objets différents. Les noms historiques RF-0…RF-9 restent dans l'archive ci-dessous ; aucun de leurs statuts n'est changé par ce tableau.

### B.9 Suite de tests adversariaux proposée, pas exécutée

`TREE-01` dirty dépendance non sélectionnée ; `TREE-02` hook modifiant l'index/le contenu ; `TREE-03` ignored/untracked/LFS/submodule/build script ; `ENV-04` variable canari, faux home/SSH et tentative réseau ; `LEGACY-05` V1 `AUTO_SAFE` + `SKIPPED` ne devient pas vérifié ; `CONC-06` lease expiré + ancien writer + fencing token ; `FS-07` symlink/junction, casse Unicode, rename et chemins hors racine ; `FILE-08` blob non UTF-8, fichier supprimé, executable bit, fichiers binaires ; `GIT-09` ref concurrent avant CAS ; `CRASH-10` kill aux frontières durable/commit/ref/reconcile ; `E2E-11` API cassée, consommateur inchangé et suite requise rouge ; `CACHE-12` toolchain/env/lockfile changés ; `RESOURCE-13` plusieurs agents/compilations sur 16 Go, annulation et absence de deadlock.

Aucun test ne doit utiliser de véritable secret ni le repo Ravum vivant. L'autre session Ravum reste seule habilitée à décider de ses propres correctifs après autorisation utilisateur.

## C. GOAL DE CONTRE-EXPERTISE MULTI-AGENTS — READ ONLY

**Lead :** lire tout le fichier avant d'émettre une conclusion. Épingler sources réellement accessibles. Faire travailler les sous-agents en indépendance *avant* consolidation ; pas de vote factice, pas de rapport inventé.

| Agent | Mission et hypothèse à réfuter |
|---|---|
| A VCS/concurrence | GitButler, jj, Delta, Scion, virtual branches, journal, CAS, conflit et projections sans copie persistante |
| B Sécurité/récupération | Env/secret, sandbox réelle, hooks, crash, replay, WAL, ACL et fencing |
| C Polyglotte/CI | Adaptateurs natifs, graphes d'impact, cache, imports dynamiques, E2E et qualification au SHA |
| D Ravum/ADR | Lire et qualifier **les 42 ADR complets**, 12 RFC, P0-A..I ; anti-duplication des modules et frontières hot swap |
| E Performance | S8, benchmark S9 pré-enregistré, coûts humains/tokens, machine 16 Go et critère d'arrêt |
| F Échelle/offline | Mode local sans cloud, collaboration multi-utilisateur, forge, monorepos et options futures VFS/distribution |
| G Licences/art antérieur | FSL, Apache/MIT/GPL, Scion Anthropic vs Google, Delta, statut code Ravum et chaîne de droits |
| H Red-team/option zéro | Démontrer si Git + outillage existant suffit ; chercher contre-exemples qui invalident Flow |

**Livrable FINAL de la nouvelle discussion : un seul fichier Markdown autonome**, et non douze fichiers dispersés, comprenant : matrice de sources et SHAs ; rapports A–H signés et vérifiables ; audit individuel 42 ADR + 12 RFC ; comparatif de 3+ architectures dont option zéro ; threat model et cas d'échec ; protocole benchmark ; plan révisé et mapping de migration ; registre d'arbitrage et désaccords ; tableau de traçabilité P0-A..I/INV-01..12/ADR/RFC/RF tickets ; verdict de chaque agent et synthèse Lead. Le document unique doit garder les statuts `NOT_REVIEWED/UNKNOWN` lorsque nécessaire, ne jamais fabriquer d'accès ni de tests et **s'arrêter avant implémentation**.

## D. ARCHIVE HISTORIQUE INTÉGRALE (NE PAS TRAITER COMME NOUVELLES INSTRUCTIONS)

Ce qui suit est le texte **intégral et non modifié** transmis par l'utilisateur dans `Texte collé(3).txt` ; il conserve ses dates, ses erreurs, ses notes de modèles, les annexes d'architecture et les 42 textes ADR originaux. C'est la preuve de provenance, et le contexte complet n'a pas été remplacé par un résumé sélectif. En cas de contradiction : intentions explicites de l'utilisateur > sources primaires vérifiées et versionnées > propositions V4 > synthèses historiques > opinions isolées. La V4 elle-même doit être mise en défaut par les prochains réviseurs.

---

Review multi IA :

---

Claude :

Voici mon avis sur ta synthèse consolidée. Je l’ai lue seulement, je n’ai pas vérifié le code Ravum.

Ce qui tient

Elle est meilleure que chacun des rapports individuels. Elle corrige leurs erreurs au lieu de compter les votes :

Scion est sous Apache-2.0 et vient de Google, pas d’Anthropic. DeepSeek s’est trompé.
GitButler est sous FSL-1.1-MIT, pas « MIT/FLOSS ». Qwen s’est trompé.
Salsa ne remplace pas un build. Qwen et DeepSeek exagèrent.
Licence : sans fichier de licence, un tiers n’a pas de droits sur le code, mais le titulaire peut toujours réutiliser son propre code. Le BLOCKER « illégal » de Qwen est faux.
Annexe Z : l’inversion des « + » en « - » signalée par Claude n’est pas reproduite. Il est juste de ne pas l’affirmer.

Le §3.1 sur la source de vérité est le meilleur arbitrage du document. Chaque autorité a son domaine : Git pour l’historique publié, le journal pour les intentions en cours, le snapshot pour le contenu vérifié. Ça clôt proprement le désaccord entre Qwen et DeepSeek.

Points faibles
Kimi et Gemini ont produit un texte identique, au caractère près. Ce n’est donc pas deux avis indépendants. Le tableau « consensus réel » doit compter un seul rapport pour les deux, et l’en-tête qui annonce « Kimi/Gemini » devrait le dire. Ce rapport ne couvrait d’ailleurs que 12 ADR sur 42 (30 marqués NOT_REVIEWED) et n’avait pas le dossier maître. Il pèse moins que les autres.
Les §4.1 et §4.2 sont tes deux constats les plus importants, et aucun des cinq modèles ne les a trouvés au niveau du code, Claude n’ayant fait qu’en suggérer l’idée. Pour l’instant, ce sont des lectures de code, pas des reproductions. Il faut les faire passer en tests P0 avant toute autre chose :
pour le §4.1 : vérifier l’égalité verified_tree_oid == published_tree_oid avant update-ref ;
pour le §4.2 : un test qui lit une variable sentinelle (par exemple FLOW_CANARY_SECRET) depuis un test candidat.

C’est le travail de la session Ravum, pas de celle-ci.

Les fencing tokens sont absents de l’architecture recommandée (§5). Claude et DeepSeek l’ont soulevé : un agent figé qui reprend la main après l’expiration de son verrou. Ça devrait figurer au G2.
Le §3.5 sur le seuil du benchmark reste trop ouvert. Je recommanderais une règle simple, à figer avant la mesure : n ≥ 8 par stratégie, critère principal = temps médian jusqu’au livrable vérifié, intervalles de confiance qui ne se chevauchent pas, et zéro perte comme condition éliminatoire. Le seuil de gain lui-même (0 %, 20 % ou 25 %) reste ta décision. Mais il faut l’écrire dans le protocole avant le G3, sinon le G5 ne sera pas falsifiable.
L’Option 0 de Claude (6 à 10 jours) n’est pas reprise explicitement. Ton G3 et ton G4 y correspondent en pratique. La nommer comme baseline « ne rien construire » rendrait le critère d’arrêt plus net.
Verdict

Je suis d’accord avec CONDITIONAL-GO incrémental et NO-GO pour le VFS/CAS. L’ordre des gates G0 → G5 est cohérent. La seule chose que je remonterais en priorité, c’est la reproduction des deux défauts P0 dans Ravum. Elle ne dépend d’aucune décision de licence et elle débloque le G2.

Veux-tu que j’intègre ces corrections directement dans le document ? Ou que je rédige le protocole de benchmark figé (pour le G3) ?

---

DeepSeek :

Cette synthèse critique est globalement solide et nettement plus fiable que les contre-expertises initiales. Elle corrige plusieurs erreurs factuelles importantes et identifie des risques architecturaux concrets dans le code existant. Elle mérite cependant quelques nuances et renforcements avant d’être considérée comme une base de décision définitive.

1. Ce que la synthèse établit correctement (vérifié)
1.1 Correction de l’erreur sur Scion
La synthèse a raison de distinguer deux entités portant le même nom :

Scion (Google Cloud Platform) est bien un projet open source sous Apache-2.0, une plateforme d’orchestration multi-agents qui exécute chaque agent dans son propre conteneur avec son propre workspace ou worktree. Il ne s’agit pas d’un système propriétaire d’Anthropic.

Scion (Anthropic) est le système interne de contrôle de source côté serveur, avec CAS et caches hiérarchisés, qui a effectivement remplacé Git dans le monorepo d’Anthropic face à l’explosion des merges (2 500 → 30 000/jour). Ce système n’est pas publié et n’est pas réutilisable par Flow.

Cette distinction est cruciale : elle évite de disqualifier à tort le Scion de Google comme option d’orchestration, tout en confirmant que le Scion d’Anthropic n’est qu’une validation industrielle du problème, pas une solution transférable.

1.2 GitButler : isolation partielle, pas de garantie transactionnelle
La documentation officielle de GitButler confirme les limites pointées par la synthèse :

Les branches parallèles ne sont pas une isolation d’exécution : les agents partagent un seul système de fichiers, une seule installation de dépendances et un seul état applicatif.

GitButler ne résout pas les collisions au niveau de la forge : les « lanes » sont une abstraction locale, et les conflits entre branches virtuelles restent à traiter manuellement.

La licence est bien FSL-1.1-MIT (Fair Source License), avec des restrictions de concurrence commerciale et une conversion en MIT après deux ans par version — ce n’est pas du MIT pur.

La synthèse a donc raison de recommander GitButler comme candidat à tester, pas comme garantie d’isolation.

1.3 Jujutsu : workspaces = copies de travail distinctes
La synthèse corrige une erreur répandue : les workspaces jj ne sont pas plusieurs vues isolées dans un même répertoire physique. Chaque workspace est une copie de travail supplémentaire, avec son propre commit de working-copy, souvent matérialisée comme un worktree Git réel (jj ≥ 0.46 avec --colocate). Cela confirme que jj ne résout pas le problème du répertoire partagé ; il fournit une isolation par copies distinctes, ce qui est précisément ce que Flow cherche à éviter par défaut.

1.4 Sapling/EdenFS/Mononoke : non réutilisables
Les sources primaires confirment que Mononoke et EdenFS sont explicitement non supportés pour un usage externe, malgré leur usage en production chez Meta. La licence principale est GPL-2.0, ce qui peut poser des problèmes de compatibilité pour une intégration dans un outil sous licence permissive. L’inspiration architecturale (VFS paresseux, CAS serveur) reste valable, mais aucune brique de code n’est directement réutilisable.

1.5 SQLite WAL : synchronous=FULL nécessaire pour la durabilité
La synthèse a raison de distinguer cohérence et durabilité. En mode WAL, synchronous=NORMAL préserve la cohérence (pas de corruption) mais peut perdre des transactions récentes après un crash OS ou une coupure de courant. Pour un journal qui protège une publication, synchronous=FULL est le réglage approprié, au prix de fsync supplémentaires. Ce point doit être explicitement intégré au protocole de reprise de Flow.

1.6 Risques concrets identifiés dans le code Ravum
Les deux risques P0 pointés par la synthèse sont plausibles et non reproduits, ce qui justifie pleinement leur statut de tests prioritaires :

Divergence arbre vérifié / arbre publié : le fait que ravum-collab vérifie un snapshot mais publie via un index Git isolé qui n’inclut que les chemins sélectionnés crée un écart structurel si des modifications non sélectionnées ont influencé les tests. Le test recommandé (verified_tree_oid == published_tree_oid avant update-ref) est le bon garde-fou.

Fuite de secrets via l’environnement des tests et hooks : l’absence de env_clear() et de liste autorisée signifie que les tests et hooks héritent des variables sensibles de l’utilisateur. La redaction des logs ne suffit pas : un test malveillant peut exfiltrer par le réseau. Un environnement à liste autorisée, sans réseau par défaut, est nécessaire avant toute publication contrôlée.

1.7 ravum-store n’est pas un CAS
La synthèse a raison : ravum-store est une frontière SQLite pour l’application de tâches M4, pas un moteur CAS générique. Le compter comme brique réutilisable serait une erreur d’architecture.

2. Points où la synthèse mérite d’être renforcée ou nuancée
2.1 Le verdict « NO-GO pour l’architecture v0.1 » est justifié, mais la formulation gagnerait à être plus opérationnelle
La synthèse dit « NO-GO pour l’architecture v0.1 telle quelle » et « CONDITIONAL-GO pour une trajectoire incrémentale ». C’est cohérent. Mais il manque une règle d’arrêt explicite pour le mode transactionnel : à partir de quel résultat de benchmark considère-t-on que RF-3 (CAS + snapshots) est justifié ? La synthèse mentionne des seuils (20 %, 25 %) mais ne tranche pas. Il faut fixer le seuil avant l’expérience, comme le recommande d’ailleurs la synthèse elle-même (§3.5). Une proposition concrète :

Si le mode transactionnel Flow ne réduit pas le temps jusqu’au livrable vérifié d’au moins 15 % par rapport à la stratégie A (copies privées) sur au moins 9 campagnes avec 27 agents, il n’est pas justifié de le généraliser. Le seuil de 15 % est arbitraire mais doit être gelé avant le benchmark ; il pourra être révisé uniquement par une nouvelle version du protocole.

2.2 La recommandation sur Salsa est trop prudente
La synthèse dit : « n’introduire Salsa qu’en réponse à un besoin d’invalidation qu’aucun outil existant ne couvre correctement ». C’est raisonnable, mais Salsa n’est pas la seule option pour le graphe d’impact. Pour un outil polyglotte, il peut être plus simple de commencer par un graphe conservateur de dépendances par adaptateur (Cargo metadata pour Rust, tsconfig/package.json pour TS, imports pour Python) et de n’introduire un moteur incrémental générique que si les adaptateurs ne suffisent pas. Le risque de Salsa est de complexifier prématurément le noyau pour un bénéfice non démontré sur le corpus cible.

2.3 Le « coordinateur unique par dépôt » est une bonne cible, mais il faut distinguer les plans de contrôle et de données
La synthèse recommande un coordinateur d’intégration unique par dépôt. C’est correct pour le plan de contrôle (qui décide de l’ordre des transactions et publie sur Git). Mais il ne faut pas en déduire un processus unique pour tout. Les compilateurs et exécuteurs de tests doivent rester isolés et annulables, comme la synthèse le reconnaît elle-même (§3.4). La nuance importante est : un seul écrivain Git, plusieurs workers de validation. Cette distinction doit être explicite dans l’architecture cible.

2.4 La correction sur la licence est juste, mais la conclusion opérationnelle reste bloquante
La synthèse a raison : l’absence de licence open source n’interdit pas au titulaire des droits de réutiliser son propre code. Mais elle omet une difficulté pratique : si des contributions ont été faites par des tiers (agents ou humains) sans CLA/DCO clair, le titulaire peut ne pas détenir l’intégralité des droits. La synthèse mentionne ce point mais le minimise. La décision de licence (RF-RFC-002) reste un BLOCKER, non pas parce que « tout est interdit », mais parce que l’inventaire des droits n’est pas établi. L’alternative from scratch sous licences permissives reste la voie la plus sûre si cet inventaire tarde.

2.5 Le benchmark S8 scripté (A=2,65 s, B=3,06 s, C=2,91 s) ne doit pas être utilisé pour relativiser le S8 réel
La synthèse mentionne une « autre campagne scriptée » avec des temps de l’ordre de quelques secondes, en précisant qu’elle n’a pas le même profil temporel. C’est exact, mais ce chiffre ne doit pas être utilisé pour atténuer la portée du S8 réel (111 s / 237 s / 528 s). Le S8 réel, avec 27 sous-agents et 9 campagnes, reste la seule mesure multi-agents disponible, avec ses limites (petite fixture, copies privées, garde simple). La campagne scriptée mesure autre chose (probablement des opérations unitaires) et ne doit pas servir de contre-argument pour minimiser l’écart entre partage direct et copies privées.

3. Recommandation finale consolidée
La synthèse critique converge vers la bonne conclusion : NO-GO pour l’architecture v0.1 complète, CONDITIONAL-GO pour une trajectoire incrémentale mesurée. Je la renforce avec les conditions suivantes, qui doivent être satisfaites avant tout développement du mode transactionnel :

Condition	Statut	Preuve attendue
G0 — Dépôt Flow accessible et RFC publiées	BLOCKER	URL publique, SHA épinglé, 12 RFC et invariants lisibles
G1 — Licence ou décision from scratch	BLOCKER	Licence approuvée ou décision écrite de construire sans code Ravum
G2 — Frontière daemon Ravum/Flow clarifiée	HIGH	RFC dédiée, aucune duplication de coordinateur
G3 — Correctifs P0 sécurité	BLOCKER pour publication contrôlée	env_clear(), liste autorisée, pas de réseau par défaut, test verified_tree_oid == published_tree_oid
G4 — Benchmark pré-enregistré avec seuil gelé	HIGH	Seuil de gain (ex. ≥ 15 %) fixé avant mesure ; 9 campagnes, 27 agents
G5 — Mode par défaut = copies privées + intégration	KEEP	Implémenté et benchmarké avant RF-3 (CAS + snapshots)
La voie recommandée est donc :

RF-0 : publier le dépôt, les RFC, les invariants ; trancher la licence ou décider from scratch.

RF-1 : implémenter le mode copies privées + intégration comme défaut, avec CLI minimale et reçus liés au snapshot exact.

RF-2 : ajouter un coordinateur léger (verrous par fichier/symbole, journal SQLite WAL synchronous=FULL, détection UNMANAGED).

RF-3 (conditionnel) : n’ajouter CAS + snapshots + publication transactionnelle que si RF-1/RF-2 ne satisfont pas l’objectif P0-I et qu’un benchmark impartial démontre un gain gelé à l’avance.

Le plan initial tendait à sur-engineer avant d’avoir prouvé que la simplicité ne suffit pas. La synthèse a raison de le dire. La seule correction de cap que j’ajouterais est de fixer le seuil de décision maintenant, afin d’éviter que le benchmark soit interprété a posteriori pour justifier une architecture déjà choisie.

---

Qwen :

CONTRE-EXPERTISE MULTI-IA — RAVUM FLOW
Lead de la contre-expertise | Date de consolidation : 9 novembre 2026 | Mode : READ-ONLY / RESEARCH & ARCHITECTURE REVIEW
00-SOURCES-AND-BASELINE.md
État des dépôts et références épinglées (au 9 novembre 2026)
Dépôt / Branche
SHA / État
Observations
Licence
Ravum (main)
54491488bba78baca8df93e14742150ad2bd9b28
Stable.
AUCUNE (BLOCKER)
Ravum (dev)
HEAD MOVING (PR #85 fusionnée le 09/11/2026 à 11:26 UTC)
L'intégration du MVP-0c a modifié la tête.
AUCUNE
Ravum (feat/mvp0c-hardening)
2f92c2410ef2c7bb33f67db6737c92fddf2cefa6
Contient les 42 ADR et prototypes.
AUCUNE
Ravum (integration/mvp0c-clean)
ef6b38cab0a062a5b97f862bdcdbb1ab13ec17c3
Consolidation documentaire.
AUCUNE
Ravum-Flow (GitHub)
404 / Non accessible
Le dépôt distant n'existe pas ou est privé.
N/A
Sources externes et corrections factuelles
GitButler : Licence FSL-1.1-MIT (Functional Source License), et non MIT pur. Restriction de concurrence commerciale pendant 2 ans avant conversion MIT. Impacte le choix d'une dépendance "Virtual Branches".
Scion : Projet GoogleCloudPlatform (Apache-2.0), et non un système propriétaire d'Anthropic.
SQLite WAL : synchronous=FULL requis pour la durabilité après coupure ; NORMAL préserve la cohérence mais risque de perte de transaction récente.
Jujutsu (jj) : Les jj workspaces partagent le stockage Git mais créent des copies de travail physiques distinctes, et non des vues virtuelles isolées dans un seul répertoire.
01-PRIOR-ART-2026.md
Veille internationale et art antérieur
Domaine
Solution
Licence / État
Pertinence Flow
Verdict
VCS / Workspaces
GitButler
FSL-1.1-MIT
Branches virtuelles, UX cible.
CONDITIONAL (Attention FSL).
VCS / Workspaces
Jujutsu (jj)
Apache-2.0
Oplog, workspaces multiples.
HAUTE (Mais copies physiques).
VCS / Monorepo
Sapling / EdenFS
GPL-2.0 / MIT
VFS (FUSE/ProjFS).
REJETÉ (Trop lourd pour P0-H 16Go).
Build / Cache
Salsa (Rust)
MIT / Apache-2.0
Graphe incrémental.
HAUTE (Mais n'est pas un build system).
Stockage
SQLite WAL
Domaine Public
Journal transactionnel local.
HAUTE (Remplace le CAS custom).
Conclusion de la veille
L'option "Ne rien reconstruire" (Option 0/1) est validée. Le plan initial (Option 2 : CAS/VFS custom) réinvente des solutions existantes avec une dette technique massive. Salsa est une bibliothèque de calculs incrémentaux, pas un remplacement transparent de Cargo ou npm.
02-ADR-REVIEW-42-plus-12.md
Matrice des 42 ADR de Ravum (Tous PROPOSED)
L'audit ligne-à-ligne des 42 textes (Annexe Z) confirme qu'aucun n'est ACCEPTED. Flow n'a aucune autorité pour modifier ou valider ces ADR.
ADR-000 à 009 (Périmètre, Mémoire, Contrats) : Impact Flow = INDIRECT. Flow orchestre les snapshots, Ravum vérifie la sémantique.
ADR-010 à 018 (IR, Compilation, Backends) : Impact Flow = INDIRECT. Flow ne doit pas dupliquer le cache de compilation de Ravum.
ADR-019 à 024 (Daemon, Hot Swap, DSP) : CRITIQUE. Flow propose, le PatchHost de Ravum dispose. Flow ne fait PAS de hot swap.
ADR-025 à 041 (Full-stack, CI, Gouvernance) : Impact Flow = INDIRECT. La gouvernance (ADR-041) exige une licence claire avant toute extraction de code.
Revue des 12 RFC de Ravum Flow (Synthèse multi-IA)
RFC
Sujet
Verdict Consolidé
Justification
RF-RFC-002
Licence
BLOCKER
Faux "interdit" absolu : le titulaire peut réutiliser son code, mais les droits des contributions/agents IA doivent être clarifiés.
RF-RFC-003
Source de vérité
CHANGE
Distinguer les états : Git (publié), Journal (intentions), Snapshot (vérifié). Pas d'autorité concurrente.
RF-RFC-007
Journal/CAS
REJECT
ravum-store est une base de tâches (M4), pas un CAS générique. Utiliser SQLite WAL + Git.
RF-RFC-008
Build
CHANGE
Réutiliser le build natif. Salsa uniquement pour l'invalidation fine non couverte.
03-ALTERNATIVE-ARCHITECTURES.md
Option 0 : Git + Worktrees Éphémères (Baseline Sûre)
Description : Isolation physique temporaire, file d'intégration unique.
Verdict : MVP par défaut. Coût mesuré, zéro risque d'écrasement silencieux par des outils UNMANAGED.
Option 1 : Orchestrateur de Branches Virtuelles (GitButler-style)
Description : Routage logique des diffs via watcher ou API.
Verdict : CANDIDAT. Ne garantit pas l'isolation contre les écritures disque directes (IDE, scripts). Dépendance à la licence FSL-1.1-MIT si intégration profonde.
Option 2 : Flow Transactionnel avec CAS/VFS Propriétaire (Plan v0.1)
Description : Overlay FUSE/ProjFS, CAS local.
Verdict : NO-GO. Complexité OS, performances I/O désastreuses sur Windows, contournement trivial par les outils UNMANAGED.
Architecture Cible Recommandée (Incrémentale)
Coordinateur unique par dépôt (gère les reçus et l'attribution).
Journal SQLite WAL pour les intentions en cours.
GitWriter atomique pour la publication (read-tree, write-tree, update-ref).
Adaptateurs exécutant les builds sur l'arbre exact qui sera publié.
04-THREAT-MODEL-RECOVERY.md
Défauts P0 identifiés dans les prototypes Ravum (MVP-0c)
Ces risques doivent être corrigés dans Ravum ou isolés dans Flow avant toute extraction.
Menace
Scénario d'échec (Code observé)
Mitigation Requise
Gravité
Divergence Arbre Vérifié/Publié
ravum-collab vérifie un SourceSnapshot incluant des overlays, mais GitWriter publie un arbre filtré. Les tests passent sur un état qui n'est pas celui du commit.
Exiger verified_tree_oid == published_tree_oid avant update-ref. Fail-closed si divergence.
BLOCKER
Fuite de Secrets (Tests/Hooks)
RustcMergeVerifier lance std::process::Command sans env_clear(). Les hooks Git héritent de l'environnement. Exfiltration possible par test malveillant.
Environnement à liste autorisée (allowlist), réseau désactivé, logs redigés.
BLOCKER
Faux CAS
ravum-store utilisé comme CAS alors qu'il est conçu pour les tâches M4.
Migrer vers SQLite WAL avec schéma dédié aux blobs et intentions.
HIGH
Écriture UNMANAGED
Un IDE écrit sur le disque, contournant l'API de patches.
Détection via watcher, marquage STALE/UNMANAGED, restauration testée. Ne jamais promettre de prévention absolue sans VFS OS.
HIGH
05-BENCHMARK-PROTOCOL.md
Protocole S9 (Extension de S8)
Objectif : Mesurer le temps jusqu'au livrable vérifié (P0-I) sur machine 16 Go RAM.
Stratégies à comparer :
S9-A : Copies privées + fusion (Baseline S8 = 111,1s).
S9-B : Répertoire partagé + garde optimiste (Baseline S8 = 237,7s).
S9-C : Vrais Worktrees Git éphémères + Coordinateur.
S9-D : Flow Transactionnel (API Patches + SQLite WAL).
Métriques obligatoires :
Wall-clock jusqu'à CI_GREEN (E2E inclus).
Pic RAM : Daemon + Build ne doivent pas dépasser les budgets P0-H.
Scan de secrets : Vérification qu'aucun token n'a fui dans les logs ou artefacts.
Taux de divergence : verified_tree_oid vs published_tree_oid.
Gate de décision : Le seuil de gain (ex: ≥20% ou équivalent avec moins de ressources) doit être fixé avant l'expérience. Aucun ajustement post-hoc n'est autorisé.
06-REVISED-PLAN.md
Roadmap Corrigée et Gates (G0 à G5)
Gate
Livrable vérifiable
Décision / Action
G0
Provenance, licence Flow choisie, droits contributions clarifiés.
Pas d'extraction de code Ravum.
G1
RFC précisant autorité Git / Journal / Snapshots.
Pas de duplication de coordinateur.
G2
Tests de fuite de secrets, divergence arbre, crash recovery.
Défauts P0 corrigés ou fail-closed.
G3
Baseline S9 (Copies vs Worktrees vs Flow).
Sélection du mode initial.
G4
Prototype mince (Attribution + GitWriter + E2E).
Aucun changement perdu.
G5
Benchmark pré-enregistré du candidat "sans worktree".
Décision d'architecture avancée.
Ordre de travail :
Traiter les risques P0 dans la session Ravum dédiée (sans modifier Ravum depuis Flow).
Réviser l'architecture Flow et préparer les benchmarks dans un dépôt indépendant.
Développer le noyau minimal (Coordinateur + GitWriter).
07-DECISION-REGISTER.md
ID
Sujet
Décision Originale
Verdict Multi-IA
Risque
RF-RFC-002
Licence
Implicite
BLOCKER : Clarifier droits et choisir licence.
BLOCKER
RF-RFC-003
Source de vérité
Journal Flow > Git
CHANGE : Git = publié, Journal = intention.
HIGH
RF-RFC-007
Stockage CAS
CAS Propriétaire
REJECT : ravum-store inadapté, utiliser SQLite WAL.
BLOCKER
INV-01
Pas de worktree
VFS / Overlays
CHANGE : Worktrees éphémères par défaut, API patches ensuite.
HIGH
P0-H
Sobriété 16 Go
VFS FUSE
REJECT : FUSE trop lourd.
HIGH
ADR-019
Daemon/Hot Swap
Géré par Ravum
KEEP : Flow orchestre, Ravum exécute.
LOW
08-EXECUTION-GOAL-DRAFT.md
Projet de GOAL de développement (Non exécutable tant qu'un humain ne donne pas GO)
Mission : Développer un coordinateur léger d'agents et d'intégration Git pour Ravum Flow, produisant des reçus de validation liés à l'arbre exact, sans VFS ni CAS propriétaire au MVP.
Contraintes absolues :
Aucune modification du dépôt Rwanbt/Ravum.
Aucune extraction de code avant validation du Gate G0 (Licence/Droits).
Aucun test E2E ne doit être masqué ou désactivé pour passer au vert.
L'environnement d'exécution des tests candidats doit être sanitisé (env_clear()).
Première tâche autorisée (post-GO) :
Créer le dépôt Ravum-Flow (si G0 validé).
Implémenter le GitWriter atomique avec vérification stricte verified_tree_oid == published_tree_oid.
REVIEW-SUMMARY.md
Synthèse et Contradictions des Sous-Agents
L'analyse multi-IA (Claude, DeepSeek, Mistral, Qwen, Kimi/Gemini) converge vers un constat sans appel : l'architecture v0.1 monolithique (VFS/CAS complet) est un NO-GO. Elle tente de résoudre un problème d'ergonomie (worktrees) par une surcouche système coûteuse, impossible à sécuriser contre les outils UNMANAGED, et viole les principes de sobriété (P0-H).
Cependant, le besoin d'un coordinateur d'intégration et de traçabilité est validé. Le consensus s'établit sur une trajectoire incrémentale :
Git reste canonique pour les états publiés.
SQLite WAL remplace le CAS propriétaire pour le journal des intentions.
Les worktrees éphémères restent le mode par défaut sûr, les "branches virtuelles" n'étant qu'une candidate expérimentale nécessitant des preuves (S9).
Corrections factuelles majeures
GitButler est sous licence FSL-1.1-MIT (restriction commerciale temporaire), ce qui impacte son adoption comme dépendance directe.
Scion est un projet Google (Apache-2.0), non Anthropic.
ravum-store n'est pas un CAS générique mais une base de tâches M4.
SQLite WAL nécessite synchronous=FULL pour la durabilité stricte.
Défauts P0 découverts (Audit statique)
Divergence de publication : ravum-collab et GitWriter peuvent valider un snapshot différent de l'arbre Git publié.
Fuite de secrets : RustcMergeVerifier n'isole pas l'environnement (env_clear() manquant).
Matrice de Conformité Globale
Domaine
Couverture
Verdict
P0-A à P0-I
Aligné si on abandonne le VFS custom (P0-H sauvé).
CONDITIONAL
42 ADR Ravum
Respectés. Flow reste subordonné logistique.
KEEP
12 RFC Flow
3 RFC critiques (002, 003, 007) rejetées ou modifiées.
CHANGE
RF-0 à RF-9
Roadmap valide, mais gates G0-G5 imposés.
CHANGE
Verdict Final : CONDITIONAL-GO (Trajectoire Incrémentale)
NO-GO pour le VFS/CAS complet construit immédiatement.
CONDITIONAL-GO pour Ravum Flow comme produit autonome, commençant par un coordinateur léger, des reçus exacts et une publication Git contrôlée.
Prochaine étape attendue du responsable humain :
Valider le pivot architectural (Abandon du VFS/CAS).
Trancher la question de la licence et des droits sur le code généré/agent (Gate G0).
Autoriser la création du dépôt Ravum-Flow et le lancement du Gate G4 (Prototype mince).

---

Gemini :

CONTRE-EXPERTISE MULTI-IA — RAVUM FLOW
Lead de la contre-expertise | Date de consolidation : 9 novembre 2026 | Mode : READ-ONLY / RESEARCH & ARCHITECTURE REVIEW
00-SOURCES-AND-BASELINE.md
État des dépôts et références épinglées (au 9 novembre 2026)
Dépôt / Branche
SHA / État
Observations
Licence
Ravum (main)
54491488bba78baca8df93e14742150ad2bd9b28
Stable.
AUCUNE (BLOCKER)
Ravum (dev)
HEAD MOVING (PR #85 fusionnée le 09/11/2026 à 11:26 UTC)
L'intégration du MVP-0c a modifié la tête.
AUCUNE
Ravum (feat/mvp0c-hardening)
2f92c2410ef2c7bb33f67db6737c92fddf2cefa6
Contient les 42 ADR et prototypes.
AUCUNE
Ravum (integration/mvp0c-clean)
ef6b38cab0a062a5b97f862bdcdbb1ab13ec17c3
Consolidation documentaire.
AUCUNE
Ravum-Flow (GitHub)
404 / Non accessible
Le dépôt distant n'existe pas ou est privé.
N/A
Sources externes et corrections factuelles
GitButler : Licence FSL-1.1-MIT (Functional Source License), et non MIT pur. Restriction de concurrence commerciale pendant 2 ans avant conversion MIT. Impacte le choix d'une dépendance "Virtual Branches".
Scion : Projet GoogleCloudPlatform (Apache-2.0), et non un système propriétaire d'Anthropic.
SQLite WAL : synchronous=FULL requis pour la durabilité après coupure ; NORMAL préserve la cohérence mais risque de perte de transaction récente.
Jujutsu (jj) : Les jj workspaces partagent le stockage Git mais créent des copies de travail physiques distinctes, et non des vues virtuelles isolées dans un seul répertoire.
01-PRIOR-ART-2026.md
Veille internationale et art antérieur
Domaine
Solution
Licence / État
Pertinence Flow
Verdict
VCS / Workspaces
GitButler
FSL-1.1-MIT
Branches virtuelles, UX cible.
CONDITIONAL (Attention FSL).
VCS / Workspaces
Jujutsu (jj)
Apache-2.0
Oplog, workspaces multiples.
HAUTE (Mais copies physiques).
VCS / Monorepo
Sapling / EdenFS
GPL-2.0 / MIT
VFS (FUSE/ProjFS).
REJETÉ (Trop lourd pour P0-H 16Go).
Build / Cache
Salsa (Rust)
MIT / Apache-2.0
Graphe incrémental.
HAUTE (Mais n'est pas un build system).
Stockage
SQLite WAL
Domaine Public
Journal transactionnel local.
HAUTE (Remplace le CAS custom).
Conclusion de la veille
L'option "Ne rien reconstruire" (Option 0/1) est validée. Le plan initial (Option 2 : CAS/VFS custom) réinvente des solutions existantes avec une dette technique massive. Salsa est une bibliothèque de calculs incrémentaux, pas un remplacement transparent de Cargo ou npm.
02-ADR-REVIEW-42-plus-12.md
Matrice des 42 ADR de Ravum (Tous PROPOSED)
L'audit ligne-à-ligne des 42 textes (Annexe Z) confirme qu'aucun n'est ACCEPTED. Flow n'a aucune autorité pour modifier ou valider ces ADR.
ADR-000 à 009 (Périmètre, Mémoire, Contrats) : Impact Flow = INDIRECT. Flow orchestre les snapshots, Ravum vérifie la sémantique.
ADR-010 à 018 (IR, Compilation, Backends) : Impact Flow = INDIRECT. Flow ne doit pas dupliquer le cache de compilation de Ravum.
ADR-019 à 024 (Daemon, Hot Swap, DSP) : CRITIQUE. Flow propose, le PatchHost de Ravum dispose. Flow ne fait PAS de hot swap.
ADR-025 à 041 (Full-stack, CI, Gouvernance) : Impact Flow = INDIRECT. La gouvernance (ADR-041) exige une licence claire avant toute extraction de code.
Revue des 12 RFC de Ravum Flow (Synthèse multi-IA)
RFC
Sujet
Verdict Consolidé
Justification
RF-RFC-002
Licence
BLOCKER
Faux "interdit" absolu : le titulaire peut réutiliser son code, mais les droits des contributions/agents IA doivent être clarifiés.
RF-RFC-003
Source de vérité
CHANGE
Distinguer les états : Git (publié), Journal (intentions), Snapshot (vérifié). Pas d'autorité concurrente.
RF-RFC-007
Journal/CAS
REJECT
ravum-store est une base de tâches (M4), pas un CAS générique. Utiliser SQLite WAL + Git.
RF-RFC-008
Build
CHANGE
Réutiliser le build natif. Salsa uniquement pour l'invalidation fine non couverte.
03-ALTERNATIVE-ARCHITECTURES.md
Option 0 : Git + Worktrees Éphémères (Baseline Sûre)
Description : Isolation physique temporaire, file d'intégration unique.
Verdict : MVP par défaut. Coût mesuré, zéro risque d'écrasement silencieux par des outils UNMANAGED.
Option 1 : Orchestrateur de Branches Virtuelles (GitButler-style)
Description : Routage logique des diffs via watcher ou API.
Verdict : CANDIDAT. Ne garantit pas l'isolation contre les écritures disque directes (IDE, scripts). Dépendance à la licence FSL-1.1-MIT si intégration profonde.
Option 2 : Flow Transactionnel avec CAS/VFS Propriétaire (Plan v0.1)
Description : Overlay FUSE/ProjFS, CAS local.
Verdict : NO-GO. Complexité OS, performances I/O désastreuses sur Windows, contournement trivial par les outils UNMANAGED.
Architecture Cible Recommandée (Incrémentale)
Coordinateur unique par dépôt (gère les reçus et l'attribution).
Journal SQLite WAL pour les intentions en cours.
GitWriter atomique pour la publication (read-tree, write-tree, update-ref).
Adaptateurs exécutant les builds sur l'arbre exact qui sera publié.
04-THREAT-MODEL-RECOVERY.md
Défauts P0 identifiés dans les prototypes Ravum (MVP-0c)
Ces risques doivent être corrigés dans Ravum ou isolés dans Flow avant toute extraction.
Menace
Scénario d'échec (Code observé)
Mitigation Requise
Gravité
Divergence Arbre Vérifié/Publié
ravum-collab vérifie un SourceSnapshot incluant des overlays, mais GitWriter publie un arbre filtré. Les tests passent sur un état qui n'est pas celui du commit.
Exiger verified_tree_oid == published_tree_oid avant update-ref. Fail-closed si divergence.
BLOCKER
Fuite de Secrets (Tests/Hooks)
RustcMergeVerifier lance std::process::Command sans env_clear(). Les hooks Git héritent de l'environnement. Exfiltration possible par test malveillant.
Environnement à liste autorisée (allowlist), réseau désactivé, logs redigés.
BLOCKER
Faux CAS
ravum-store utilisé comme CAS alors qu'il est conçu pour les tâches M4.
Migrer vers SQLite WAL avec schéma dédié aux blobs et intentions.
HIGH
Écriture UNMANAGED
Un IDE écrit sur le disque, contournant l'API de patches.
Détection via watcher, marquage STALE/UNMANAGED, restauration testée. Ne jamais promettre de prévention absolue sans VFS OS.
HIGH
05-BENCHMARK-PROTOCOL.md
Protocole S9 (Extension de S8)
Objectif : Mesurer le temps jusqu'au livrable vérifié (P0-I) sur machine 16 Go RAM.
Stratégies à comparer :
S9-A : Copies privées + fusion (Baseline S8 = 111,1s).
S9-B : Répertoire partagé + garde optimiste (Baseline S8 = 237,7s).
S9-C : Vrais Worktrees Git éphémères + Coordinateur.
S9-D : Flow Transactionnel (API Patches + SQLite WAL).
Métriques obligatoires :
Wall-clock jusqu'à CI_GREEN (E2E inclus).
Pic RAM : Daemon + Build ne doivent pas dépasser les budgets P0-H.
Scan de secrets : Vérification qu'aucun token n'a fui dans les logs ou artefacts.
Taux de divergence : verified_tree_oid vs published_tree_oid.
Gate de décision : Le seuil de gain (ex: ≥20% ou équivalent avec moins de ressources) doit être fixé avant l'expérience. Aucun ajustement post-hoc n'est autorisé.
06-REVISED-PLAN.md
Roadmap Corrigée et Gates (G0 à G5)
Gate
Livrable vérifiable
Décision / Action
G0
Provenance, licence Flow choisie, droits contributions clarifiés.
Pas d'extraction de code Ravum.
G1
RFC précisant autorité Git / Journal / Snapshots.
Pas de duplication de coordinateur.
G2
Tests de fuite de secrets, divergence arbre, crash recovery.
Défauts P0 corrigés ou fail-closed.
G3
Baseline S9 (Copies vs Worktrees vs Flow).
Sélection du mode initial.
G4
Prototype mince (Attribution + GitWriter + E2E).
Aucun changement perdu.
G5
Benchmark pré-enregistré du candidat "sans worktree".
Décision d'architecture avancée.
Ordre de travail :
Traiter les risques P0 dans la session Ravum dédiée (sans modifier Ravum depuis Flow).
Réviser l'architecture Flow et préparer les benchmarks dans un dépôt indépendant.
Développer le noyau minimal (Coordinateur + GitWriter).
07-DECISION-REGISTER.md
ID
Sujet
Décision Originale
Verdict Multi-IA
Risque
RF-RFC-002
Licence
Implicite
BLOCKER : Clarifier droits et choisir licence.
BLOCKER
RF-RFC-003
Source de vérité
Journal Flow > Git
CHANGE : Git = publié, Journal = intention.
HIGH
RF-RFC-007
Stockage CAS
CAS Propriétaire
REJECT : ravum-store inadapté, utiliser SQLite WAL.
BLOCKER
INV-01
Pas de worktree
VFS / Overlays
CHANGE : Worktrees éphémères par défaut, API patches ensuite.
HIGH
P0-H
Sobriété 16 Go
VFS FUSE
REJECT : FUSE trop lourd.
HIGH
ADR-019
Daemon/Hot Swap
Géré par Ravum
KEEP : Flow orchestre, Ravum exécute.
LOW
08-EXECUTION-GOAL-DRAFT.md
Projet de GOAL de développement (Non exécutable tant qu'un humain ne donne pas GO)
Mission : Développer un coordinateur léger d'agents et d'intégration Git pour Ravum Flow, produisant des reçus de validation liés à l'arbre exact, sans VFS ni CAS propriétaire au MVP.
Contraintes absolues :
Aucune modification du dépôt Rwanbt/Ravum.
Aucune extraction de code avant validation du Gate G0 (Licence/Droits).
Aucun test E2E ne doit être masqué ou désactivé pour passer au vert.
L'environnement d'exécution des tests candidats doit être sanitisé (env_clear()).
Première tâche autorisée (post-GO) :
Créer le dépôt Ravum-Flow (si G0 validé).
Implémenter le GitWriter atomique avec vérification stricte verified_tree_oid == published_tree_oid.
REVIEW-SUMMARY.md
Synthèse et Contradictions des Sous-Agents
L'analyse multi-IA (Claude, DeepSeek, Mistral, Qwen, Kimi/Gemini) converge vers un constat sans appel : l'architecture v0.1 monolithique (VFS/CAS complet) est un NO-GO. Elle tente de résoudre un problème d'ergonomie (worktrees) par une surcouche système coûteuse, impossible à sécuriser contre les outils UNMANAGED, et viole les principes de sobriété (P0-H).
Cependant, le besoin d'un coordinateur d'intégration et de traçabilité est validé. Le consensus s'établit sur une trajectoire incrémentale :
Git reste canonique pour les états publiés.
SQLite WAL remplace le CAS propriétaire pour le journal des intentions.
Les worktrees éphémères restent le mode par défaut sûr, les "branches virtuelles" n'étant qu'une candidate expérimentale nécessitant des preuves (S9).
Corrections factuelles majeures
GitButler est sous licence FSL-1.1-MIT (restriction commerciale temporaire), ce qui impacte son adoption comme dépendance directe.
Scion est un projet Google (Apache-2.0), non Anthropic.
ravum-store n'est pas un CAS générique mais une base de tâches M4.
SQLite WAL nécessite synchronous=FULL pour la durabilité stricte.
Défauts P0 découverts (Audit statique)
Divergence de publication : ravum-collab et GitWriter peuvent valider un snapshot différent de l'arbre Git publié.
Fuite de secrets : RustcMergeVerifier n'isole pas l'environnement (env_clear() manquant).
Matrice de Conformité Globale
Domaine
Couverture
Verdict
P0-A à P0-I
Aligné si on abandonne le VFS custom (P0-H sauvé).
CONDITIONAL
42 ADR Ravum
Respectés. Flow reste subordonné logistique.
KEEP
12 RFC Flow
3 RFC critiques (002, 003, 007) rejetées ou modifiées.
CHANGE
RF-0 à RF-9
Roadmap valide, mais gates G0-G5 imposés.
CHANGE
Verdict Final : CONDITIONAL-GO (Trajectoire Incrémentale)
NO-GO pour le VFS/CAS complet construit immédiatement.
CONDITIONAL-GO pour Ravum Flow comme produit autonome, commençant par un coordinateur léger, des reçus exacts et une publication Git contrôlée.
Prochaine étape attendue du responsable humain :
Valider le pivot architectural (Abandon du VFS/CAS).
Trancher la question de la licence et des droits sur le code généré/agent (Gate G0).
Autoriser la création du dépôt Ravum-Flow et le lancement du Gate G4 (Prototype mince).

---

Kimi :

J'ai bien lu la synthèse consolidée du 9 novembre 2026. Le verdict est clair : NO-GO sur l'architecture v0.1 (CAS/VFS/ordonnanceur complets), CONDITIONAL-GO sur un coordinateur léger incrémental, NOT_REVIEWED sur les implémentations non testées et les droits non établis. Les gates G0–G5 et les risques P0 (divergence arbre vérifié/publié, fuite de secrets via RustcMergeVerifier, confusion ravum-store/CAS) sont bien posés.

Que souhaitez-vous que je fasse à partir de cette synthèse ? Par exemple :

Mettre en forme le document en pdf ou docx livrable (avec tableaux des consensus, schéma d'architecture, matrice des gates).
Rédiger les RFC des gates G0–G1 (frontières d'autorité Git/journal/snapshots, API Ravum–Flow) en tant que documents séparés.
Préparer les protocoles de test P0 : fixtures et scénarios pour les risques 4.1 et 4.2.
Critiquer certains arbitrages (3.1–3.5) ou le choix des seuils.
Traduire en anglais pour diffusion.
Précisez l'objectif et le format attendu, et j'engage le travail avec la skill correspondante.

--- 

Mistral :

Merci pour cette consolidation. Elle converge avec ma contre-expertise sur l'essentiel (CONDITIONAL-GO incrémental, NO-GO sur VFS/CAS complet immédiat, architecture « coordinateur » ≈ mon A6), et elle apporte des corrections factuelles que j'accepte — dont deux qui corrigent directement mes livrables : le cadrage juridique de l'absence de licence (trop absolu chez moi) et les constats de lecture de code (verified_tree_oid, fuite d'environnement) que ma session avait laissés NOT_REVIEWED. J'intègre ces amendements dans le canvas.




Modifié un Canvas














Mis à jour Contre-expertise multi-IA — Ravum Flow (9 novembre 2026)











Mis à jour Contre-expertise multi-IA — Ravum Flow (9 novembre 2026)











Mis à jour Contre-expertise multi-IA — Ravum Flow (9 novembre 2026)









Réfléchi pendant 1s
Now the remaining edits: 00.2 PR timestamp + 404 nuance, 00.4 prototypes, 04.1 two new rows, 04.2 SQLite, REVIEW-SUMMARY addendum.













Premier lot d'amendements appliqué. Je poursuis avec les constats de lecture de code et l'addendum de synthèse.




Modifié un Canvas














Mis à jour Contre-expertise multi-IA — Ravum Flow (9 novembre 2026)











Mis à jour Contre-expertise multi-IA — Ravum Flow (9 novembre 2026)











Mis à jour Contre-expertise multi-IA — Ravum Flow (9 novembre 2026)











Mis à jour Contre-expertise multi-IA — Ravum Flow (9 novembre 2026)











Mis à jour Contre-expertise multi-IA — Ravum Flow (9 novembre 2026)









Réfléchi pendant 1s
The first edit failed — the yolo row string didn't match. Let me check the exact text in the file around that table.













Un amendement n'a pas trouvé sa cible ; je relis la ligne exacte dans le fichier.




Réfléchi pendant 1s












Réfléchi pendant 1s
The canvas renderer apparently converted search-result://... URIs into their resolved https URLs when displaying/saving? Actually the file now contains the full https URLs instead of search-result:// URIs. Interesting — the system must have rewritten them on save. So my old_string with the search-result URI didn't match. I need to use the exact current text with the https URL.
Let me redo the failed edit with the correct old_string (line 295): "| Outil non intégré (mode yolo) | ZCode --prompt en mode yolo écrase le working tree (Flavio Copes) | UNMANAGED observé + restauration depuis snapshot ; jamais qualifié « sûr » (RF-RFC-004) | Présent mais sous-spécifié (overlay non spécifié) |"













La cible utilisait une URL résolue ; je refois l'amendement avec le texte exact.




Modifié un Canvas














Mis à jour Contre-expertise multi-IA — Ravum Flow (9 novembre 2026)









Mis à jour Contre-expertise multi-IA — Ravum Flow (9 novembre 2026)









Contre-expertise multi-IA — Ravum Flow (9 novembre 2026)ModifierCopier


[data-radix-scroll-area-viewport]{scrollbar-width:none;-ms-overflow-style:none;-webkit-overflow-scrolling:touch;}[data-radix-scroll-area-viewport]::-webkit-scrollbar{display:none}Contre-expertise documentaire multi-IA — Ravum Flow
Mode : RESEARCH + ARCHITECTURE REVIEW — READ ONLY. Aucune implémentation, extraction, migration, modification Git, fusion de PR ou ratification d'ADR n'a été effectuée pendant cette mission. Aucun ADR n'a été passé à ACCEPTED.
Date de référence : 9 novembre 2026. Verdict exécutif : CONDITIONAL-GO (conditions G0–G5, voir REVIEW-SUMMARY). Un GO n'est pas une autorisation de développer.
Honnêteté méthodologique. Les huit perspectives de sous-agents (A–H) du mandat ont été couvertes en passes séquentielles distinctes par une seule session d'IA, avec relances de recherche indépendantes entre les passes, puis consolidation contradictoire. Il ne s'agit pas de huit instances réellement indépendantes : les rapports ci-dessous le documentent ainsi et aucun consensus artificiel n'a été fabriqué. Les désaccords non résolus sont conservés dans REVIEW-SUMMARY.
Navigation
#
Livrable
Contenu
00
SOURCES-AND-BASELINE
Sources lues, SHA épinglés, documents absents
01
PRIOR-ART-2026
Veille écosystème 2026, comparatif sourcé
02
ADR-REVIEW-42-plus-12
Revue des 42 ADR Ravum et des 12 RFC Flow
03
ALTERNATIVE-ARCHITECTURES
Six architectures comparées, compromis
04
THREAT-MODEL-RECOVERY
Sûreté, races, crash, confidentialité
05
BENCHMARK-PROTOCOL
Protocole S8 étendu, pré-enregistrement
06
REVISED-PLAN
Architecture recommandée, roadmap corrigée
07
DECISION-REGISTER
Décisions classées, risques, gates humaines
08
EXECUTION-GOAL-DRAFT
Plan d'exécution futur — non autorisé
—
REVIEW-SUMMARY
Synthèse, contradictions, verdict, matrice

00 — SOURCES AND BASELINE
00.1 Documents fournis et réellement lus
Source
Nature
État de lecture
Dossier maître Ravum-Flow-Contre-Expertise-Multi-IA-COMPLETE.md
Fourni par le responsable, 2051 lignes, ~182 Ko
Lu intégralement, y compris Annexe A (plan Flow v0.1) et Annexe Z (42 textes ADR complets)
Annexe Z : ADR-000 à ADR-041
Textes intégraux des 42 ADR
Tous lus. Spot-check de fidélité effectué : ADR-000 et ADR-012 comparés au dépôt → identiques (bonne foi du dossier établie sur échantillon)
Annexe A : plan Flow v0.1
Invariants, RFC, roadmap, backlog
Lu intégralement

00.2 Dépôt Ravum — état vérifié le 9 novembre 2026 (lecture seule via API GitHub)
Dépôt principal : github.com/Rwanbt/Ravum

Registre ADR : docs/adr (feat/mvp0c-hardening)

Consolidation multi-IA du 8 nov. 2026 : docs/reviews/2026-11-08-consolidation.md

Branche
SHA (référence dossier)
SHA vérifié 9 nov. 2026
Divergence
main
54491488
54491488bba78baca8df93e14742150ad2bd9b28
Identique
dev
ece8133d
6b0ac7b (avancé)
Divergente — la référence du dossier est périmée
feat/mvp0c-hardening
2f92c241
2f92c2410ef2c7bb33f67db6737c92fddf2cefa6
Identique
integration/mvp0c-clean
ef6b38ca
ef6b38cab0a062a5b97f862bdcdbb1ab13ec17c3
Identique

PR : #78, #80, #82 fermées (remplacées) ; #83, #84, #85 fusionnées (#85 dans dev le 9 nov. 2026 à 11:26:56 UTC — horodatage fourni par la consolidation) ; #86 ouverte (docs dev-integration-status → dev). L'annexe Z a été comparée à feat/mvp0c-hardening (SHA 2f92c24) ; la RFC partagée a été lue sur dev (6b0ac7b).
Dépôt Ravum-Flow : non accessible (404). L'API GitHub répond 404 pour Rwanbt/Ravum-Flow — absent ou non public ; ne pas le déclarer créé. Le bootstrap documentaire local ne prouve aucune création distante. Conséquence : tout le plan Flow est non publié, sans licence ratifiée et sans historique Git vérifiable.
00.3 Sources primaires Ravum lues intégralement
Document
Branche/SHA
Constat clé
docs/rfc/RFC-SHARED-WORKSPACE-MULTIAGENT.md (~79 Ko)
dev/6b0ac7b
Invariants SW-01..SW-10, taxonomie C1..C7, niveaux AUTO_SAFE/REBASE_REVALIDATE/CONFLICT/REJECT/MANUAL_REQUIRED, alternatives §7 (CRDT/OT/MVCC/CAS/event sourcing/AST merge/jj), modèle Git plumbing (commit-tree + update-ref CAS + GIT_INDEX_FILE), tests T01–T15, étapes COLLAB-0..4, rejets R-01..R-10, limites §15
docs/architecture/PROJECT_INVARIANTS.md
feat/mvp0c-hardening
P0-A..P0-I + INV-01..INV-13 (rédigés par le responsable humain)
docs/mvp/MVP-0b-licence-analysis.md
idem
Aucun fichier LICENSE dans Ravum ; 41 dépendances toutes permissives ; recommandation « MIT OR Apache-2.0 » non ratifiée ; le script licence-audit.ps1 refuse une licence implicite
docs/mvp/MVP-0c-s8.md
idem
Protocole S8 pré-enregistré ; pilote scripté ; campagne réelle 27 sous-agents (voir 05)

00.4 Documents annoncés mais non trouvés / non lus
Ravum-Flow/DECISIONS.md, BACKLOG.md, ARCHITECTURE.md, BENCHMARKS.md, MIGRATION_RAVUM.md, GITHUB_SETUP.md : uniquement en annexe A du dossier maître (copies), pas de source distante vérifiable (dépôt 404).

Codes sources des prototypes ravum-snapshot, ravum-workspace-coordinator, ravum-daemon, ravum-unit-graph, ravum-collab-oracle, ravum-batch, ravum-rust-adapter, ravum-protocol : non audités ligne par ligne dans cette mission (seuls les documents et le RFC ont été lus). Leurs API, dépendances et tests restent NOT_REVIEWED de la part de cette session — partiellement levé par la consolidation du 9 nov., qui a lu ravum-collab.rs, git_writer.rs, common/mod.rs et ravum-store/src/lib.rs : constats repris en 04.1/04.2 ; « ravum-store » n'est pas un CAS générique mais la frontière SQLite de l'application M4 de tâches (ne pas le réutiliser par son nom).

Le détail des runs S8 bruts (logs par campagne) : non reçus ; seules les médianes agrégées du document MVP-0c-s8 ont été lues.

00.5 Veille externe (recherches web, 9 novembre 2026)
Recherches primaires et secondaires en anglais et en français sur : GitButler, Jujutsu, Zed Delta/DeltaDB, Sapling, Scalar, gitoxide, libgit2, partial clone/sparse checkout, Git LFS, CRDT (Automerge, Yjs, Loro, Diamond Types), Claude Code Agent Teams, Scion, ZCode/GLM, OpenCode, MCP, ai-crew-sync/AgentSync/agentsync, Salsa, Buck2, Bazel, Nx, Turborepo, sccache/ccache, GitHub merge queue, GitLab merge trains, Graphite/gh-stack, fencing tokens. Chaque constat du livrable 01 cite sa source avec date d'accès. Les affirmations non sourcées restent marquées UNKNOWN ou signalées comme connaissance générale à revérifier.
01 — PRIOR ART 2026
Comparatif au 9 novembre 2026. « Pertinence Flow » = capacité à répondre au besoin multi-agents sans worktree obligatoire, local-first.
01.1 Versionnement et espaces de travail
Outil
Version/état (nov. 2026)
Licence
Local/offline
Modèle d'isolation
Pertinence Flow
Git
Standard du secteur
GPLv2 (CLI)
Oui
Worktrees physiques
Base obligatoire (INV-07)
GitButler
Actif, Series A 17 M$ (avril 2026)
Fair Source (MIT après 2 ans)
Oui
Branches virtuelles sans worktrees dans le working tree
Élevée — démonstration qu'un VCS au-dessus de Git peut éviter les worktrees, mais licence Fair Source non réutilisable telle quelle
Jujutsu (jj)
v0.46 courant, ~31,5 k étoiles
Apache-2.0
Oui (colocated avec Git)
Changes/commits de première classe, op log, undo, conflits matérialisés
Élevée — le modèle « opération + undo + conflit de première classe » est réutilisable en amont de Flow
Zed Delta / DeltaDB
App bêta publique 16-09-2026 ; DeltaDB annoncé 11-06-2026
Closed source (pas de release OSS de DeltaDB), cloud (Cloudflare), gratuit en bêta
Non (cloud)
CRDT diamond-types ; chaque édition (humaine ou agent) journalisée
Conceptuelle — la thèse « échapper à la tyrannie des worktrees » est validée par le marché, mais non réutilisable : code fermé, cloud obligatoire
Sapling
Actif (~228 auteurs sur 12 mois)
GPL-2.0
Oui
Client Meta ; Mononoke/EdenFS non supportés en externe
Moyenne — UX intéressante ; backend distribué indisponible
Scalar
Produit migré dans microsoft/git ; le code produit a été supprimé du dépôt microsoft/scalar (PR #511)
MIT
Oui
Fine couche de configuration Git (partial clone, sparse, commit-graph)
Moyenne — les recettes (blobless + cone sparse + sparse index + maintenance) sont directement applicables aux gros monorepos
gitoxide
0.61.0 (oct. 2026), très actif
Apache-2.0 OR MIT
Oui
Bibliothèque Rust Git
Élevée pour un futur moteur Flow en Rust (plumbing CAS refs), au lieu d'appeler le CLI Git
libgit2 / Git LFS / partial clone
Maintenus
GPLv2 avec exception / MIT / natif Git
Oui
—
Support

Fait de marché marquant : GitButler lui-même a ajouté but worktree et but agent setup (2026) — l'éditeur des branches virtuelles revient aux worktrees pour les agents IA, signe que l'isolation physique reste la solution de robustesse privilégiée (TAMradar).
01.2 Concurrence et collaboration
Claude Code Agent Teams (v2.1, 2026) : un lead et des teammates, chacun dans son propre worktree Git, fusion un à un ; l'attribut isolation: worktree provisionne un worktree frais par invocation d'agent parallèle (Kimi.ai, tmux.app). Anthropic, acteur dominant, valide le modèle « un agent = un worktree », pas le répertoire partagé.

Scion (Google Cloud Platform, open source 7 avril 2026, Apache-2.0, Go) :





Contenu tronqué. Cliquez sur « Agrandir » pour voir le document complet.






Agrandir


Réfléchi pendant 1s












Réfléchi pendant 1s
All amendments applied. Final chat message: concise French summary of convergence, corrections accepted, points maintained, next steps. No file paths.













Le canvas est mis à jour avec un addendum qui intègre votre consolidation. Voici ma position de relecteur :
Convergence sur l'essentiel :
Verdict identique : votre CONDITIONAL-GO incrémental / NO-GO sur l'architecture v0.1 « telle quelle » correspond à ma recommandation A6 (« Flow coordinateur, pas Flow VCS ») — les couches CAS/VFS/ordonnanceur général ne doivent être ajoutées qu'après preuve par benchmark adversarial.
Source de vérité (§3.1) : votre distinction à trois états (HEAD Git = historique publié ; journal = intentions ; snapshot = contenu vérifié) est plus précise que ma formulation initiale ; je l'adopte.
Daemons (§3.4) : votre correction est juste — c'est l'unicité du propriétaire des écritures Git et du budget qui est exigée, pas un processus monolithique ; les compilateurs doivent rester isolables et annulables séparément. J'ai nuancé D-07 en ce sens.
Corrections factuelles que j'accepte et ai intégrées :
Licence : mon « juridiquement impossible » était trop absolu — l'absence de licence ne bloque que les tiers et la distribution, pas le titulaire. D-01 et RF-RFC-002 sont amendés ; le vrai sujet est la clarification des droits sur les contributions « Ravum Agent » et le code généré par IA.
Lecture de code : les constats verified_tree_oid == published_tree_oid (divergence arbre vérifié/publié) et l'absence d'env_clear() dans RustcMergeVerifier sont intégrés au modèle de menaces (04.1, gate G2), marqués « rapportés par la consolidation, non reproduits par cette session » — ma session avait laissé ces prototypes NOT_REVIEWED.
ravum-store : noté comme frontière SQLite de l'application M4, pas un CAS extractible.
SQLite : synchronous=FULL en WAL ajouté aux exigences de durabilité du journal.
Seuil de benchmark : mon 15 % était une proposition parmi d'autres (20–25 % ailleurs) — le point valide est de le figer avant la mesure, pas sa valeur ; reformulé en « proposition à ratifier à G2 ».
Aucune divergence bloquante résiduelle. Le seul point que je maintiens explicitement : le mode partagé transactionnel reste non prouvé jusqu'à exécution d'un S9 avec worktrees Git réels et solution existante — votre gate G3/G5 l'encadre correctement.
L'ordre de travail final est cohérent : risques P0 de Ravum traités dans sa session dédiée (pas depuis la session Flow), benchmarks préparés dans un dépôt Flow indépendant, noyau minimal ensuite. Aucune implémentation lancée ; les gates G0–G5 restent votre décision.

---

Chatgpt :

Analyse de la nouvelle contre-expertise
Cette synthèse apporte une amélioration importante au dossier V2 : elle transforme plusieurs risques théoriques en points de contrôle précis dans le code existant de Ravum.

J'ai vérifié les trois constats principaux sur Rwanbt/Ravum, à la révision 6b0ac7b de dev :
Constat	Vérification	Priorité proposée
Risque de différence entre l'arbre testé et l'arbre Git publié	Le vérificateur travaille sur un snapshot candidat, tandis que GitWriter reconstruit un arbre à partir de HEAD et des chemins sélectionnés. Je n'ai pas identifié de contrôle systématique d'égalité entre les deux arbres dans ce parcours.	P0 — test de reproduction
Transmission de secrets aux processus de test	Les commandes retirent certaines variables Rust, mais héritent du reste de l'environnement. Les hooks Git suivent également ce modèle.	P0 — frontière de confiance
ravum-store réutilisable comme stockage CAS générique	Non : le module correspond au stockage SQLite d'une application de tâches.	Correction de l'inventaire

Ces observations sont confirmées par lecture du code, pas par exploitation ou test de défaillance exécuté.

Le point le plus important pour Ravum Flow est de ne jamais confondre snapshot validé, arbre Git préparé et commit publié. Ils doivent être reliés par une preuve explicite de l'identité exacte du contenu, y compris après l'exécution des hooks.
Dossier Ravum Flow V3 finalisé
J’ai intégré cette nouvelle synthèse au dossier précédent et révisé le plan sur quatre points : validation du tree Git exact avant publication, séparation stricte des statuts V1/V2, isolation des processus de test et des hooks, et comparaison réelle des solutions existantes avant de construire un nouveau moteur.
Contre-expertise Ravum Flow — V3

Archive contenant les nouvelles analyses, le plan révisé, le GOAL multi-agents et les rapports V2 historiques


Télécharger le dossier complet V3 (.zip)

Télécharger le Markdown autonome avec le contexte complet

J’ai également préparé deux documents directement réutilisables : le GOAL de contre-expertise pour DeepSeek et ses sous-agents, ainsi que la note P0 destinée à l'autre session Ravum.

Le dossier conserve les 42 ADR originaux sans modification, ainsi que la synthèse multi-IA reçue. Les rapports indépendants bruts des autres modèles ne sont pas joints à cette synthèse ; je n'ai pas relancé ces agents et aucun défaut P0 n'a été reproduit dynamiquement.

Décision recommandée : CONDITIONAL-GO pour poursuivre la contre-expertise, NO-GO pour implémenter immédiatement l'architecture v0.1. Aucun dépôt GitHub, branche, PR, code ou CI n'a été modifié.

--

# RAVUM FLOW — CONTRE-EXPERTISE V3 CONSOLIDÉE, DOCUMENT AUTONOME

**9 novembre 2026. STRICTEMENT REVIEW-ONLY.** Ce document est destiné à une nouvelle conversation IA. **NE RIEN CODER, NE RIEN MODIFIER DANS LES DÉPÔTS RAVUM OU FLOW, NE CRÉER AUCUN REPO, BRANCHE, PR, LICENCE, CI OU MIGRATION ; NE PAS LANCER DE TESTS LOURDS SUR LE WORKSPACE UTILISATEUR.** Une autre session développe Ravum. L'équipe Flow est en lecture seule et ne doit pas la gêner. La décision GO/NO-GO est humaine.

## Ordre d'autorité et lecture

1. **Intentions et invariants du responsable utilisateur dans le dossier maître original** (annexe ci-dessous), P0-A..P0-I, 42 ADR originaux `PROPOSED` ; le code réel vérifié au SHA domine les déclarations de progrès obsolètes.
2. Données factuelles documentées dans la nouvelle synthèse multi-IA **et** vérifiées dans le code ou la documentation officielle ; les avis non sourcés restent des hypothèses.
3. Notes V3 du présent dossier — elles constituent une analyse et des **propositions contestables**, jamais des ADR `ACCEPTED`.
4. Opinions des modèles antérieurs et notes V2 — elles ne valent ni vote indépendant vérifié ni autorisation d'implémentation.

**Verdict provisoire : `NO-GO` architecture v0.1 telle quelle ; `CONDITIONAL-GO` poursuite de la contre-expertise ; `NO-GO-IMPLEMENTATION`.** Les sous-agents A–H n'ont pas été relancés dans cette session ; aucune validation empirique de Flow n'a été obtenue.

**Avertissement d'archive :** les sections historiques reproduisent *mot pour mot* les décisions et observations telles qu'elles ont été rédigées au moment de leur production ; certaines dates, SHAs, statuts PR et interprétations ont pu être corrigés par les sections V3. Ne pas traiter ces annexes comme état courant du repo. Dans ce dossier, une référence code épinglée est un constat de sa révision, jamais du dernier HEAD sans revalidation.

---


---

# PARTIE : ARBITRAGE CRITIQUE V3

# Ravum Flow — arbitrage critique V3 de la synthèse multi-IA

**Date : 2026-11-09 — DÉCISION DOCUMENTAIRE SEULEMENT.** **Aucun GO d'implémentation ; aucun ADR/RFC ratifié ; aucun dépôt modifié.**

## 1. Périmètre, provenance et fiabilité

Cette note confronte :
1. `SOURCE-MASTER-RAVUM-FLOW.md` (dossier maître original, 42 ADR intégraux, plan v0.1, 12 invariants, 12 RFC, RF-0…9 et RF-001…026), SHA-256 `2994d36de4f0ee2acecbd7d5a70bae8ae4ee669a775e3574e5b3b32f4206b8ff`.
2. La synthèse apportée le 9 novembre `SYNTHESE-MULTI-IA-FOURNIE.md`, SHA-256 `853f6a77766633b4bc582a26acad23e4dea61c234f8c904dd7676ef84ed5a68c`.
3. Les rapports de la revue documentaire V2, qui déclarent eux-mêmes ne **pas** correspondre à huit experts indépendants.
4. Une nouvelle vérification **en lecture seule** via le connecteur GitHub du code de `Rwanbt/Ravum@6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf` (branche `dev` observée le 09/11/2026) et de la révision historiquement intégrée `ef6b38cab0a062a5b97f862bdcdbb1ab13ec17c3` ; les fichiers examinés ci-dessous ont des blobs identiques dans ces deux révisions.
5. Documents primaires de GitButler, Jujutsu, Delta, Scion et SQLite consultés sur le web.

**Limite :** la synthèse cite des modèles nommés mais ne fournit pas les *rapports bruts individuels*, leurs contextes ni leur provenance. Elle est `USER_SUPPLIED_MULTI_IA_SYNTHESIS`, pas une preuve que nous avons conduit ces cinq/six revues indépendantes. **Aucune nouvelle sous-équipe A–H n'a été exécutée ici.** Les conclusions de tiers sont évaluées comme hypothèses, pas comme votes.

**États utilisés :** `FACT_SOURCE`, `FACT_CODE_READ`, `MEASURED_HISTORICAL`, `PLAUSIBLE_NOT_REPRODUCED`, `RECOMMENDED`, `NOT_TESTED`, `BLOCKED`, `UNKNOWN` ; jamais `PROVED` sans portée explicite.

## 2. Conclusion stratégique et correction du V2

| Thème | Appréciation V3 | Justification |
|---|---|---|
| Architecture v0.1 **telle quelle** | **NO-GO** | VFS/CAS/journal/cache universels avant preuve = complexité injustifiée ; RF-2 pouvait précéder un gate de vérification crédible. |
| Ravum Flow comme outil indépendant | **CONDITIONAL-GO pour l'étude, non pour développer** | Une coordination sobre + publication Git exacte peut être utile, mais l'option zéro doit rester falsifiable. |
| « Sans worktrees obligatoires » | **KEEP comme objectif, non absolu** | Les outils non intégrés ne sont pas isolables par simple watcher d'un répertoire partagé. |
| Sources de vérité | **CLARIFY** | `HEAD` = publié, candidat Git tree = contenu à qualifier, journal = transactions en cours, working tree utilisateur = état observé potentiellement dirty, runtime Ravum = version active. |
| Architecture initiale | **Option 0/5 en concurrence** | Git existant + isolation éphémère privée, file d'intégration, receipts exacts ; API patches managed en expérimentation. |
| Salsa / build engine universel | **REJECT comme présupposé** | Adaptateurs des outils natifs et fallback conservateur avant toute couche de compilation nouvelle. |
| Nombre de processus | **CLARIFY** | Un seul propriétaire logique de la publication Git par workspace ; sous-processus de build/test isolables. Un daemon unique monolithique n'est pas requis. |
| Licence Ravum | **CLARIFY — droits** | Pas de licence ouverte ≠ interdiction pour le véritable titulaire de réutiliser son propre code ; vérifier titulaires, contributeurs, dépendances et distribution. |
| Stockage | **UNDECIDED** | SQLite WAL et journal append-only doivent être comparés, sans prétendre à une transaction ACID globale Git+FS+DB. |

**Révision importante de V2 :** le contrat `verified_tree_oid == published_tree_oid` doit être vérifié **sur le même type d'identité Git tree**, pas en comparant directement le hash propriétaire `SourceSnapshotId` avec un OID Git. Préparer l'arbre définitif (avec hooks éventuels autorisés), le figer, puis vérifier cet arbre et **seulement ensuite** autoriser `update-ref` si le parent/ref et tous les prérequis sont toujours valides. Toute préparation qui modifie l'arbre invalide le reçu précédent.

## 3. Constat technique 1 — arbre vérifié ≠ arbre publié (P0 candidat)

### Ce qui est effectivement observé

- Dans [`ravum-collab.rs`](https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/crates/ravum-workspace-coordinator/src/bin/ravum-collab.rs#L322-L429), l'admission/validation porte sur `coordinator.current()` et ses snapshots ; l'écriture Git récupère **seulement** l'union des `integrated_paths`, en en ignorant le reste.
- Dans [`git_writer.rs`](https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/crates/ravum-workspace-coordinator/src/git_writer.rs#L769-L875), l'index isolé est initialisé depuis `HEAD`, puis les changements sélectionnés sont appliqués. Le tree est écrit par `git write-tree`, puis associé à un `commit-tree`.
- Aucun contrôle systématique d'équivalence entre **l'ensemble du snapshot candidat vérifié** et **l'arbre Git effectivement créé** n'a été repéré dans ce parcours de code. Ceci n'est **pas** un exploit reproduit, ni une preuve que toute publication diverge.

### Contre-exemple à tenter (sur fixture synthétique après GO spécifique)

`HEAD` contient un module A nécessitant B. Un utilisateur a modifié B localement sans l'intégrer à la proposition. Le vérificateur voit A+B et conclut vert. Le writer publie seulement A depuis `HEAD` et produit un arbre sans B. Test de contrôle : reconstruire le commit dans une vue propre et montrer que la validation ne s'y transpose pas. Autres variantes : fichier non suivi consulté par le build, script de build modifié, symlink, mode exécutable, CRLF, hook modifiant un index ou un fichier et rebase de la référence entre préparation et publication.

### Recommandation

Créer une **identité d'arbre publiable immuable** depuis `expected_old` + changements attribués ; appliquer les hooks autorisés avant gel du tree ; reconstruire un dossier de vérification depuis **ce tree exact** ; lier le reçu `CandidateGitTreeOid`, `BaseCommitOid`, `BuildContextId`, `PolicyVersion`, `VerificationReceiptId` et `checks[]` ; refuser publication si l'arbre a changé ou si un contrôle obligatoire est STALE/UNKNOWN/FAILED. Les changements utilisateurs non sélectionnés sont préservés, jamais effacés.

## 4. Constat technique 2 — environnement des tests/hooks (P0 candidat)

- [`common/mod.rs`](https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/crates/ravum-workspace-coordinator/src/bin/common/mod.rs#L367-L447) lance le test harness et `rustc` via `Command`, avec quelques `env_remove`, sans `env_clear()` ni allowlist de toutes les variables de processus.
- [`git_writer.rs`](https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/crates/ravum-workspace-coordinator/src/git_writer.rs#L1329-L1357) lance les hooks avec l'environnement du processus parent, à quelques suppressions Git près.
- Le fichier lui-même rappelle que le test s'exécute avec le **même utilisateur / même machine et n'est pas une sandbox**. `FACT_CODE_READ`. Ce n'est pas une exfiltration démontrée.

**Proposition :** deux modes de confiance explicites (code/hook approuvé vs code candidat non fiable). Pour le non-fiable : sandbox réelle adaptée à l'OS, réseau refusé par défaut si requis, secrets absents de l'environnement et des volumes montés, limitation des processus enfants et logs redigés avant export. `env_clear` seule n'est pas une sandbox ; le refus des hooks implicites ne doit pas violer silencieusement les conventions utilisateur. Mesurer les coûts sur Windows et Linux, éviter toute promesse de garantie parfaite cross-OS avant tests.

## 5. Constat technique 3 — `AUTO_SAFE` V1, statut de compatibilité à ne jamais importer (P0 de politique)

- [`Coordinator::new()`](https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/crates/ravum-workspace-coordinator/src/coordinator.rs#L165-L201) démarre en `V1CollabCompat`. En V1, `AUTO_SAFE` désigne une applicabilité et un non-chevauchement **textuels**, et le module décrit explicitement cette limitation ; `Coordinator::v2()` exige une validation déclarée.
- [`ravum-collab.rs`](https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/crates/ravum-workspace-coordinator/src/bin/ravum-collab.rs#L97-L144) choisit `--protocol v1` par défaut et n'active `--verify` ou `--verify-tests` qu'à la demande.
- Le [test de compatibilité](https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/crates/ravum-workspace-coordinator/src/coordinator.rs#L1733-L1751) documente `AutoSafe` avec vérification `Skipped` et `verification_required=false`.

**Nuance :** ce comportement historique est déclaré, et non un défaut d'implémentation démontré. Mais importer cette définition dans Flow violerait le gate de sécurité souhaité. Recommandation : `TEXTUALLY_APPLICABLE` pour V1 ; `VALIDATED_FOR_SCOPE` uniquement après une politique de validation passée ; pas de `AUTO_SAFE` v1 exposé comme promesse de publication saine.

## 6. Constat technique 4 — faux réemploi de `ravum-store`

[`ravum-store/src/lib.rs`](https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/crates/ravum-store/src/lib.rs) est une frontière SQLite pour la **référence M4 Todo** (`ravum-todo-core`, table `tasks`). `FACT_CODE_READ`. Il n'est pas un magasin CAS générique réutilisable tel quel. Ne pas réintroduire ce raccourci dans les plans de migration.

## 7. Vérification des benchmarks S8 et divergence des pilotes

**Campagne réelle (preuve JSON de `dev`)** : [`evidence/mvp0c-s8-live-summary-2026-11-09.json`](https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/evidence/mvp0c-s8-live-summary-2026-11-09.json) : trois répétitions par scénario ; médiane A=**111 110 ms = 111,110 s** ; B=**237 674 ms = 237,674 s** ; C=**528 481 ms = 528,481 s**. `MEASURED_HISTORICAL`, non re-mesuré ici. A = **copies privées** et fusion de contenu, **pas Git worktrees**. B = disque partagé + garde optimiste, pas Flow transactionnel.

**Deux campagnes pilotes scriptées distinctes** : la synthèse fournie rapporte A=2,650 s / B=3,061 s / C=2,914 s. Le fichier primaire [`docs/mvp/MVP-0c-s8.md`](https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/docs/mvp/MVP-0c-s8.md#L143-L206) confirme ces valeurs pour le run initial, et précise que le JSON versionné provient d'une seconde exécution indépendante dont les valeurs diffèrent de 5 à 10 % sur la machine partagée. Le V2 cite les valeurs du **second** pilote (~2,434 / 2,825 / 2,657 s). **Ne pas fusionner les deux séries, ni déclarer l'une erronée.** Les pilotes ne mesurent pas de tokens LLM.

**Conclusion S8 :** il manque toujours l'expérience décisive des vrais `git worktree`, de jj/GitButler, de l'orchestration minimale et du candidate Flow, avec tâches de conflit réalistes et mesures de temps jusqu'au livrable vérifié. Le critère du succès doit être défini **avant** le run, sans seuil de gain arbitraire rétrospectif.

## 8. Art antérieur : nuances vérifiées

- [GitButler Desktop](https://docs.gitbutler.com/overview) : branches virtuelles dans un dossier, mais pas isolation de deux écrivains externes sur le même chemin. Licence [FSL-1.1-MIT](https://github.com/gitbutlerapp/gitbutler/blob/master/LICENSE.md), restriction spécifique des usages commerciaux concurrents. Utilisation comme outil distincte d'une extraction du code.
- [Jujutsu Git compatibility](https://docs.jj-vcs.dev/latest/git-compatibility/) : compatibilité avec Git et working copies distinctes quand on veut des vues indépendantes ; pas de prévention magique d'écriture concurrente sur un fichier unique.
- [Scion](https://github.com/GoogleCloudPlatform/scion) : Apache-2.0, pré-1.0, plusieurs modes d'exécution ; ne pas confondre avec un produit Anthropic.
- [Delta public beta](https://zed.dev/blog/delta-public-beta) : bêta septembre 2026, vraie piste de comparaison ; ne pas assimiler produit accessible et DeltaDB sous licence permettant le réemploi.
- [SQLite synchronous](https://www.sqlite.org/pragma.html) : mode WAL `FULL` vise la durabilité power-loss ; `NORMAL` conserve la cohérence mais peut perdre la dernière transaction après coupure. Aucune de ces options ne rend atomique Git+FS+DB.

## 9. Désaccords et inconnues maintenus ouverts

1. Option zéro (Git + verrous d'intégration et worktrees facultatifs) vs outil Flow indépendant : **benchmark nécessaire**.
2. Worktrees éphémères par défaut vs overlay managed dès MVP : **choix de première stratégie encore conditionnel** ; la préférence utilisateur pour la copie canonique est un objectif à atteindre, pas une raison de forcer un mode dangereux.
3. Journal minimal vs SQLite WAL, qui détient l'autorité sur la revision non publiée : **RFC de modèle d'état obligatoire**.
4. Licence du code Ravum : **titulaires de droits à établir** ; le propriétaire légitime peut disposer de droits propres, mais une licence publique ne se déduit pas d'une relation au repo.
5. Conformité E2E/CI, facturation des tokens, cache hostile, performances réelles sur 16 Go : `NOT_TESTED` pour Flow.
6. Rapports bruts des modèles de la synthèse : `NOT_PROVIDED`, à obtenir si l'on veut vérifier un consensus indépendant.

## 10. Verdict V3

- **`NO-GO` pour l'architecture v0.1 adoptée sans preuve** (stockage lourd, VFS/CRDT, moteur de build universel, promesses d'isolation non managed).
- **`CONDITIONAL-GO` pour une trajectoire d'étude et une expérimentation future autorisée sur fixtures isolées**.
- **`NO-GO-IMPLEMENTATION` aujourd'hui**, car l'autorisation humaine de développer, la politique de droits, les essais contradictoires et la qualification des risques P0 manquent.
- Aucun rapport ou résumé externe ne doit être interprété comme décision normative automatique.



---

# PARTIE : CONTRAT TRANSACTIONNEL ET SECURITE

# Ravum Flow V3 — contrat candidat d'états, de validation et de sécurité

**Statut : PROPOSED / REVIEW-ONLY.** Aucun élément de ce document n'est une API implémentée ni une décision humaine.

## 1. Vocabulaire de base

- `GitPublishedRef`: la référence Git locale **effectivement publiée**, avec `commit_oid` et `tree_oid` ; les branches de forge en sont des projections synchronisées distinctes.
- `UserWorkingDirectory`: état physique utilisateur, y compris dirty, untracked et changements simultanés ; **ne constitue pas une preuve de build**.
- `ChangeProposal`: contribution attribuée avec base exacte et chemins/symboles concernés ; pas encore acceptée.
- `CandidateGitTreeOid`: identité Git du **tree exact** préparé pour la publication, après prise en compte des hooks autorisés ayant des effets sur l'index ou les fichiers associés.
- `SourceSnapshotId`: identité de contenu propre à Flow ou Ravum, avec format/version et exclusions déclarés ; un hash de snapshot **n'est pas** un `GitTreeOid`.
- `BuildContextId`: digest versionné des outils, flags, dépendances, plateforme, entrées externes, environnement pertinent, politique de sandbox ; un secret peut contribuer à l'identité sans apparaître en clair dans les journaux (attention aux HMAC/empreintes prévenant les attaques de dictionnaire).
- `VerificationReceipt`: résultat positif/négatif/indéterminé, lié à `CandidateGitTreeOid`, `BuildContextId`, `PolicyVersion`, liste des obligations/tests exigés et preuve non falsifiée ; `PASSED` ne signifie pas `PROVED`.
- `PublicationIntent` / `PublicationOutcome`: transaction idempotente permettant après crash de constater si l'update de la ref s'est produit, sans écraser le travail utilisateur.
- `ActiveVersionId`: propriété exclusive du runtime Ravum ; aucune activation, rollback ou migration n'est décidée par Flow.

Ne pas affirmer qu'un OID Git suffit à capturer toutes les entrées de compilation : fichiers ignorés, sous-modules, pointeurs Git LFS, services externes, lockfiles, environnement et scripts peuvent modifier le résultat. Si une entrée ne peut pas être figée ou inventoriée, la validation doit être restreinte/UNKNOWN ou s'appuyer sur une politique plus conservative ; elle n'est pas librement réutilisable.

## 2. Machine d'états candidate — le tree est préparé avant la validation

```text
RECEIVED
  -> ADMITTED                      (auteur, base, chemins, anti-traversal, quotas)
  -> CANDIDATE_ASSEMBLED           (Git old commit + patches sélectionnés, index isolé)
  -> HOOKS_REVIEWED_APPLIED        (uniquement ceux autorisés par politique)
  -> CANDIDATE_FROZEN             (CandidateGitTreeOid exact + contexte)
  -> CHECKS_PENDING               (validation sur matérialisation DE CE TREE)
  -> VALIDATED_FOR_SCOPE          (toutes obligations requises PASS)
  -> PUBLICATION_INTENT_DURABLE   (old ref attendu + tree + receipts + idempotency key)
  -> REF_PUBLISHED                (git update-ref old -> new conditionnel)
  -> RECONCILED                   (index/working tree seulement sur chemins sûrs)
```

**Transitions latérales :** `CONFLICT`, `MANUAL_REQUIRED`, `FAILED`, `TIMEOUT`, `UNKNOWN`, `STALE`, `ABORTED`, `NEEDS_RECONCILIATION`. Le refus laisse intacte la version Git publiée. Un `VALIDATED_FOR_SCOPE` sur un arbre ne valide pas un arbre « rebasé automatiquement » : rebase => nouvel arbre => nouvelle validation. Une CI distante au SHA exact peut être requise en gate supplémentaire avant fusion en branche protégée.

**Deux étapes souvent confondues :** `ADMITTED` signifie qu'une proposition est recevable en tant que donnée et applicable suivant ses préconditions ; elle ne signifie ni acceptation sémantique, ni merge, ni publication. `VALIDATED_FOR_SCOPE` est un reçu positif dans une politique explicite (tests/analyses requis), pas une garantie universelle de correction.

**Cas hooks Git :** un hook peut lire l'index, éditer le message, modifier des fichiers, lancer des processus. Choisir explicitement : hooks approuvés exécutés dans une zone contrôlée **avant gel du tree** et relecture du tree ; hooks non sûrs bloquent ou demandent approbation. Un hook de `reference-transaction` appelé lors de `update-ref` doit être traité comme un effet séparé, avec risque de refus/délais et surveillance du ref ; ne pas promettre qu'une sandbox de builds neutralise les hooks Git natifs. Les hooks doivent conserver la sémantique exigée par le dépôt, pas être contournés silencieusement.

## 3. Assertions minimales avant la publication d'une ref

1. `current_ref_oid == prepared_expected_old_oid` juste avant la CAS Git.
2. `verification_receipt.git_tree_oid == prepared_commit.tree_oid` et `receipt.build_context`/`policy_version` restent valides.
3. `required_checks - passed_checks == ∅` pour les obligations correspondant à la politique ; `UNKNOWN`, `SKIPPED`, `TIMEOUT`, `STALE`, `UNSUPPORTED`, `FAILED` ne satisfont **jamais** silencieusement un test requis.
4. Aucune modification humaine externe non attribuée n'est susceptible d'être effacée par la réconciliation ; les fichiers dirty/symlinks/renames sont recontrôlés avant toute écriture et toute divergence produit un état de reprise visible.
5. Les données d'un agent non fiable n'ont pas été utilisées comme preuve « trusted » sans sandbox et chaîne de provenance conforme à la politique.
6. Les entrées non suivies/ignorées nécessaires à la compilation sont soit prises en compte dans un contexte hermétique, soit explicitement exclues avec une portée de validation limitée.
7. `PublicationIntent` idempotent suffisamment durable pour satisfaire le niveau de panne promis ; distinction entre crash de processus, crash OS, coupure d'alimentation.
8. Le tree final est **recalculé** si une étape le modifie ; aucune assertion ne compare directement des algorithmes de hachage ou modèles d'identité différents.

À la publication distante, requalification **au SHA de la tête de merge** (merge queue GitHub/merge train GitLab) si celui-ci n'est pas celui de l'arbre déjà validé. Pour un squash/rebase, la différence de commit ne suffit pas à invalider automatiquement le contenu si le tree/context sont identiques, mais les politiques de provenance/signature et base importent ; préciser ces règles dans les RFC.

## 4. Modèles d'exécution et promesses par mode

| Mode | Agent peut modifier | Garantie raisonnable | Non-garantie |
|---|---|---|---|
| `MANAGED_PATCH` | propositions via CLI/API uniquement | base versionnée, admission CAS, merge contrôlé, receipts | sécurité d'un processus doté aussi d'accès direct FS |
| `ISOLATED_EXEC` | sandbox privée, clone/copier/snapshot éphémère | conflits confinés au contenu attribué ; intégration contrôlée | vitesse supérieure sans mesure ; sandbox forte sans preuve OS |
| `OBSERVED_UNMANAGED` | workspace utilisateur partagé | détection best-effort et traces après événement | prévention d'un overwrite externe ou d'un `reset --hard` |
| `VIRTUALIZED` (futur) | vue par agent réellement projetée/interceptée | à définir par implémentation et OS | garanties universelles Windows/Linux/macOS par défaut |

**Recommandation initiale :** `ISOLATED_EXEC` est le **fallback sûr** pour agents qui veulent lancer des outils ordinaires ; `MANAGED_PATCH` peut fournir un mode « pas de worktree persistant » sur les modifications soumises via API. Toute couche VFS ou CRDT requiert son propre projet de preuve et un avantage démontré.

## 5. Trust model processus et secrets

**Deux classes de source :** `TRUSTED_REPOSITORY_CODE` (approuvé par l'utilisateur, droit d'exécuter éventuellement hooks spécifiques) et `UNTRUSTED_CANDIDATE` (PR, patch IA ou dépendance nouvelle non approuvés). Éviter une unique politique globale qui exécuterait tout avec les tokens d'accès de l'utilisateur.

- Utiliser un environnement de processus explicitement borné et un profil FS/network/process selon OS et classe de confiance.
- `env_clear()` / liste autorisée et redaction des logs sont nécessaires **mais insuffisants** pour isoler code malveillant : il peut lire secrets dans les fichiers et utiliser des sockets ou des processus enfants.
- Isoler le compilateur et les tests dans des workers jetables; quotas de RAM/CPU, annulation et limite de processus descendant.
- Ne jamais transmettre par défaut un token GitHub/GitLab, clé SSH, `.env` ou vault à un test non fiable.
- Distinguer données confidentielles de diagnostics techniques ; aucun contenu source, chemin personnel ou payload secret dans le journal partagé, la CI ou les reçus d'agent.
- Sur Windows/Linux, tester honnêtement le niveau de sandbox disponible ; si absent, refuser l'exécution non fiable ou demander une décision humaine explicite.

## 6. Cas d'échec reproductibles à documenter (aucun exécuté ici)

| Test | Précondition / injection | Attendu |
|---|---|---|
| `P0-TREE-01` | dépendance locale dirty non sélectionnée ; suite verte sur workspace | build du tree Git candidat échoue ou refuse la publication ; jamais reçu vert recyclé |
| `P0-TREE-02` | hook autorisé change un contenu préparé | tree re-gelé puis re-testé, ou refus ; pas de receipt sur ancien tree |
| `P0-TREE-03` | fichier ignoré modifie résultat build | contexte invalide/portée limitée/fallback ; pas validation hermétique fictive |
| `P0-LEGACY-04` | protocol V1 `AUTO_SAFE`, verification `SKIPPED` | Flow classe `TEXTUALLY_APPLICABLE`, refuse promotion gated |
| `P0-SECRET-05` | variable canari `FLOW_TEST_SECRET_CANARY` présente chez parent | secret absent de l'environnement enfant et de ses logs pour untrusted |
| `P0-SECRET-06` | test ou hook tente lecture de faux `~/.ssh` et connexion sortante | isolation réseau/FS effective ou interdiction de lancement ; journal redigé |
| `P0-GIT-07` | HEAD change avant `update-ref` | CAS échoue sans mise à jour ; re-préparer + revalider avant retry |
| `P0-FS-08` | humain édite un chemin avant réconciliation | état `NEEDS_RECONCILIATION`, aucune perte ni reset destructif |
| `P0-CRASH-09` | kill avant/après update-ref et après journal INTENT | replay idempotent avec état exact, pas double commit/publication |
| `P0-PATH-10` | junction/symlink/casse Unicode/rename, chemin hors racine | refus de toute écriture hors workspace |
| `P0-E2E-11` | API modifiée avec consommateur inchangé et E2E rouge | fallback suite complète ou validation bloquante, jamais vert par omission |
| `P0-CACHE-12` | cache utilisé sous flags/env/toolchain/base différents | miss ou invalidation ; pas de faux PASS |

Exécuter uniquement sur **fixtures synthétiques jetables**, autorisation et budget séparés, jamais sur le dépôt Ravum vivant dans la session Flow.

## 7. Durabilité, récupération, choix de stockage

**Les ressources ne partagent pas une transaction ACID** : SQLite + Git ref + FS + processus de hooks + forge. Il faut formaliser un ordre d'opérations, une intention idempotente et des états de replay plutôt qu'affirmer « tout ou rien ».

| Interruption | État publiable observé | Reprise non-destructive |
|---|---|---|
| avant intent | ancien ref | abandon sans marquage publié |
| intent durable, aucun commit objet | ancien ref | abandon/reconstruction selon politique, pas publication silencieuse |
| objet commit écrit, avant CAS ref | ancien ref | revalider bases ; commit inaccessible possible, nettoyage ultérieur |
| CAS ref réussi, avant outcome | nouveau ref | détecter ref actuel + journal, noter `RECOVERED_PUBLISHED` |
| ref publié, reconciliation échoue | nouveau ref, dirty FS/index | garder `NEEDS_RECONCILIATION`, avertir, ne pas `reset --hard` |
| journal tronqué/disque plein | inconnu jusqu'à inspection | fail-closed, pas `PUBLISHED` automatique |

Comparer WAL SQLite `synchronous=FULL` et journal append-only + sync des répertoires selon les garanties testables par OS. `synchronous=NORMAL` en WAL peut perdre une transaction validée lors d'une panne de courant malgré la cohérence (SQLite officiel : https://www.sqlite.org/pragma.html). `FULL` ne résout pas les effets des hooks ni la cohérence Git/FS externe.

## 8. Contrat pour Ravum

Flow orchestre snapshots/propositions/recettes et publication. Ravum conserve le compilateur, l'IR, les preuves de programmes, le PatchHost, le runtime/hot swap et les contraintes audio temps réel. **Aucun code Flow ne tourne dans le callback audio.** Les composants existants du dépôt Ravum ne doivent pas être dupliqués sous de nouvelles API sans gouvernance, tests de migration, légalité de réutilisation et absence de cycles de dépendances.



---

# PARTIE : ROADMAP ET REGISTRE DECISIONS V3

# Ravum Flow — plan d'étude et d'implémentation candidat V3

**Statut : REVIEW-ONLY.** Phases décrites = **hypothèses de roadmap après GO humain** ; aucune création de dépôt/branche/CI ni exécution de benchmarks n'est autorisée par ce texte. Le projet Ravum continue dans une autre session ; la revue Flow ne touche pas son workspace.

## 1. Problème produit à préserver

Développement humain+multi-IA sur Windows/Linux, machine ~16 Go, local-first et hors ligne, Git/GitHub/GitLab compatibles, polyglotte, sans worktree **obligatoire**, avec propositions attribuées et conflits observables, tests E2E maintenus, crash recovery et provenance. Le critère principal est le **temps jusqu'à un livrable effectivement vérifié**, coût humain/agents inclus. Aucun outil existant ne doit être exclu parce qu'il n'a pas le nom Flow.

Ravum reste indépendant de Flow et propriétaire de ses choix langage/runtime/hot swap ; Flow ne doit pas forcer Ravum à rester un frontend rustc ni devenir un second coordinateur divergent.

## 2. Options concurrentes avant construction

| Option | Contenu minimum | Avantages à vérifier | Limites/arrêt |
|---|---|---|---|
| 0. **Ne pas créer Flow** | Git + vrais worktrees facultatifs / copies privées, intégration sérialisée, validation CI par impact | faible coût de maintenance, outils matures | worktree/copie non évitable selon tâches, UX multi-IA à concevoir |
| 1. **Utiliser outil existant** | GitButler ou jj/Scion, adaptateurs et orchestration légère | virtual branches, op logs, ergonomie déjà disponibles | licence et limites, isolation FS des agents non garantie dans une seule vue |
| 2. **Flow mince** | file d'attribution, API patch controlled, frozen Git tree + receipts, CAS ref, recovery | source canonique + propositions logiques pour clients intégrés | nécessite sécurité des processus et preuve que la complexité apporte un gain |
| 3. **Hybride** | Flow mince + sandbox privées éphémères pour outils non intégrés | isolation pratique et absence de worktrees persistants pour tâches managed | coût disque/RAM et fusion à mesurer |
| 4. **VFS/CRDT avancé** | vues overlay transparentes par processus/OS ou réplication | potentiel confort et multi-utilisateur | **DEFER** : coûts OS, sémantique, sécurité et maintenabilité élevés |

Mesurer option 0 en premier, sans chercher à la rendre artificiellement mauvaise.

## 3. Gates de recherche avant autorisation de développer

- **G0 — provenance et état** : maître V3 complet lu, 42 ADR/12 RFC/12 invariants/26 tickets documentés, SHAs et releases vérifiés, fichiers modifiés par autre session identifiés ; droits/auteurs/source tiers inventoriés. `BLOCKED` tant que droits non établis si extraction envisagée.
- **G1 — avis adversariaux indépendants** : rapports A–H séparés, résultats bruts sourcés, au moins 3 architectures et option zéro, consensus et désaccords documentés. `BLOCKED` si seuls résumés d'autres modèles accessibles.
- **G2 — contrat de sûreté** : source de vérité par état, tree exact, trust model pour subprocess/hooks, rollback/recovery sans perte, distinction V1/V2, cas d'échec et mitigations. `DESIGN_ONLY` tant que les tests de reproduction n'ont pas été autorisés/réalisés.
- **G3 — protocole benchmark gelé** : fixtures réalistes, hypothèses, coûts, seuils et stop rules approuvés *avant* les runs ; comparaison copies / vrais worktrees / shared guard / GitButler/jj / mince Flow après prototypes autorisés. `DRAFT`.
- **G4 — approbation des RFC/choix réversibles** : modèle de données, publication, modes, stockage/standard, CI et relation Ravum–Flow proposés à l'humain ; pas de licence implicitement adoptée.
- **G5 — GO humain explicite** : budget, périmètre autorisé, risques acceptés, propriété du code, stratégie de repli, et tests requis écrits. **NOT GIVEN**.

**Interdit :** traiter le simple « CONDITIONAL-GO pour la recherche » comme un « GO pour développer ».

## 4. Roadmap conditionnelle APRÈS autorisation explicite

### RF-R0 — Baseline juste, droits, non-objectifs

**Dépendances** : G0/G3/G5. Inventorier code Ravum sans le modifier. Fixtures jetables Rust multi-crate, TS/Python, C/C++ selon moyens, projets synthétiques avec E2E rouges, dirty/untracked, changements API, environnements secrets-canaris et collisions. Comparer isolation A copies privées, A2 vrais Git worktrees, B shared direct+guard, C séquentiel, E GitButler/jj si possible ; aucune trace du repo vivant mutée. **Gate** : rapport reproductible, coûts CPU/RAM/disque, temps wall-clock, échecs visibles, politique d'arrêt et seuils pré-enregistrés.

### RF-R1 — Modèle d'identité et de recettes

**Dépendances** : R0. Contrats minimaux `ChangeProposal`, `BaseCommitOid`, `CandidateGitTreeOid`, `BuildContextId`, `VerificationReceipt`, `PublicationIntent`/`Outcome`, états et idempotence ; prix des migrations. Conserver Git comme format publié, journal minimal tant que besoin non prouvé ; pas de stockage CAS généralisé prématuré. **Gate** : schéma, lecture/écriture/reprise documentées et tests de données incorrectes, aucun faux `PROVED`.

### RF-R2 — Admission contrôlée et espaces d'exécution

**Dépendances** : R1. `MANAGED_PATCH` validé structurellement avec base exacte ; `ISOLATED_EXEC` en fallback pour outils ordinaires, espaces éphémères, zéro écrasement utilisateur, pas de branches/worktrees permanents imposés ; `OBSERVED` sans garantie de prévention. Gestion de chemin, symlink, refus/diff/undo attribué. **Gate** : pas de perte silencieuse sur corpus adversarial et sources externes non altérées.

### RF-R3 — Préparation de l'arbre, vérification obligatoire et environnement sûr

**Dépendances** : R2. Geler **l'arbre Git réellement publiable**, vérifier depuis sa matérialisation isolée après tous hooks autorisés modifiant l'index, associer receipt tree+build context+checks; mode v1 textuel jamais utilisé comme preuve. Adapter Rust natif d'abord : Cargo/rustc + tests transitoires, propagation des dépendances, E2E requis ; aucun doublon de compiler/build engine. Sandbox de code candidat, logs sans secrets, CPU/RAM/timeout. **Gate** : P0-TREE-01/02, P0-LEGACY-04, P0-SECRET-05/06, P0-E2E-11, P0-CACHE-12 ; si obligation UNKNOWN/FAILED -> stop.

### RF-R4 — Publication Git à référence CAS et recovery

**Dépendances** : R3. Git index isolé, arbre préparé, `commit-tree`, vérification tree OID, `update-ref old_oid` + journal durable idempotent ; interruptions à chaque point, protection dirty/untracked, hooks et réconciliation. Aucune promesse d'ACID multi-ressources ; choisir stockage selon protocole de reprise et mesures. **Gate** : résultat cohérent après crash sur Windows et Linux, aucune écriture externe écrasée, Git lisible sans Flow.

### RF-R5 — Agents et CLI local-first

**Dépendances** : R4. CLI et JSONL versionnés ; clients indépendants de DeepSeek, Claude, OpenCode/ChatGPT/Codex ou LLM, pas de daemon requis en solo ; attribution de tâches, API patches, diff/review, diagnostics/provenance, canaux d'annulation. **Gate** : trois clients hétérogènes avec scopes disjoints et une collision explicite, correction et reprise sans perte ; UI non encombrante, version affichée.

### RF-R6 — Polyglotte et impact conservateur

**Dépendances** : R3/R5. Adapter TS/JS via gestionnaire/tsc/tests natifs, Python via pytest/Ruff/typing déclaratif, C/C++ via CMake/Clang/Ninja, Rust via Cargo ; outils absents=UNSUPPORTED. Scripts/imports dynamiques/flags ou graph incomplet => test conservateur complet. **Gate** : rupture d'API dans un module consommateur inchangé détectée, E2E requis bloquants, cache miss sûr et absence de faux positifs de validation.

### RF-R7 — Publication forge GitHub/GitLab facultative

**Dépendances** : R4/R6. Pousser uniquement contenu/commits authorisés ; merge queue/merge trains qualifiés au SHA exact ; permissions, protections, refus de hook, signatures lorsque politique l'exige ; pas d'obligation SaaS. **Gate** : merge et rebase revalidés au tree/context, refus des checks rouges, source privée non exportée.

### RF-R8 — Fonctionnalités entreprise avancées (DEFER)

**Dépendances** : gain réel démontré + budget séparé. Services multi-utilisateur, distributed CAS, VFS Windows/Linux, CRDT, ACLs, remote cache : uniquement si besoin commercial/technique prouvé, threat model et compatibilité OS. **Gate** : bénéfice mesuré sur charges grandes ; ne pas dégrader mode solo.

### RF-R9 — Ravum consommateur opt-in

**Dépendances** : R4 et contrat interopérable stable, gouvernance droits. Adapter sans cycle vers snapshots/protocoles de Ravum ; pas de duplication durable de coordinator/snapshot/daemon. Ravum conserve décisions de patch actif, certificats de langage, DSP RT et rollback. **Gate** : double-suite reproductible sur SHAs spécifiques, last-known-good intact si compilation/refus/activation échoue, confidentialité.

## 5. Responsabilités et concurrence de travail (après GO seulement)

- **Lead** : maintien de `Decision Register`, merges, gates, sources et budget ; un seul propriétaire d'écriture Git par workspace.
- **Pôle sécurité** : tests adversariaux et menace/process/FS, sans modifier de code dans les autres scopes.
- **Pôle performance** : fixture, échantillonnage, reproductibilité, coûts agents/CPU/RAM/disque, aucun réglage opportuniste post-run.
- **Pôle adaptateurs** : contrats de vérification Rust puis polyglottes, ne touche pas writer/journal sans validation de migration.
- **Pôle intégration Ravum** : lecture seule sur Ravum durant l'étude, coordination de ses éventuels correctifs avec la session Ravum uniquement.

Une machine 16 Go ne doit pas lancer plusieurs Cargo lourds concurrents par défaut : planifier ressources cumulées et temps interactif. L'orchestrateur unique est une **autorité logique sur Git**, pas nécessairement un seul processus.

## 6. Mesure falsifiable et règles d'arrêt

- `time-to-verified-deliverable`: de l'assignation des tâches à l'obtention d'un tree validé selon politique + publication Git/forge souhaitée, y compris ré-essais, code review, supervision humaine, E2E et CI.
- Mesures complémentaires : lost writes, temps de traitement de conflit, taux d'erreur, p50/p95 (p99 uniquement si échantillon suffisant), RAM pic globale/enfants, CPU, disque, cache froid/chaud, tokens réellement attribués et coût fournisseur si présent.
- Échantillons, tailles, seuils de gain, budgets, distribution des tâches, ordre alterné et critères d'arrêt **pré-enregistrés par l'humain** ; ne pas inventer seuil 20/25 % comme s'il était universel.
- Ajouter test d'abandon : si option Git / existant donne le même livrable sûr à coût global égal ou inférieur, recommander la **non-création** du moteur Flow transactionnel.
- Toute solution qui masque E2E rouge, perd modification dirty, republie un tree différent de celui qualifié, ou exécute code non fiable avec secrets à portée sans garde est `NO-GO`, indépendamment de sa vitesse.

## 7. Registre d'arbitrage RFC — toutes PROPOSED

| RFC | Proposition V3 | Action humaine demandée |
|---|---|---|
| 001–012 originales | restent OPEN / PENDING, voir maître ; revue V2 des conflits conservée | pas de ratification implicite |
| 013 Admission vs promotion | séparer application texte, vérification obligatoire et publication | arbitrer vocabulaire et compatibilité |
| 014 Ownership Ravum–Flow | versions, droits, single owner de chaque responsabilité | choisir adaptateur vs extraction |
| 015 Exécutions managed/unmanaged/isolated | promesse par mode affichée, repli sûr | adopter contrat honnête |
| 016 VFS | différer au-delà MVP sans preuve de besoin | décision de non-objectif provisoire |
| 017 Cache trust | digest contexte, limite de preuve et tenant | éviter faux vert |
| 018 Budgets | priorité interactivité et 16 Go, processus enfants | politique ressources |
| 019 CI/E2E | invalidation conservatrice, full gate obligatoire | choisir scope et cadence |
| 020 Worktrees facultatifs | mesurer, pas interdire une méthode plus sûre | règle de repli |
| **021 NOUVELLE** Tree/receipt/hook | vérifier le tree Git **après** hooks modifiants et avant CAS de ref | BLOCKER avant publisher |
| **022 NOUVELLE** Trust des subprocess | politique sandbox/env/secrets et hooks de confiance différenciés | BLOCKER avant tests candidats |
| **023 NOUVELLE** V1 AutoSafe | bannir le sens textuel V1 de toute recette de promotion Flow | BLOCKER sémantique |
| **024 NOUVELLE** Entrées de build & FS | Git tree vs ignored/untracked/deps/permissions/modes | définir exactitude et fallback |

Les identifiants RF-RFC-013, RF-RFC-014, RF-RFC-015, RF-RFC-016, RF-RFC-017, RF-RFC-018, RF-RFC-019, RF-RFC-020, RF-RFC-021, RF-RFC-022, RF-RFC-023 et RF-RFC-024 sont **candidats documentaires**, pas des RFC créées dans un dépôt.

## 8. Verdict et chaîne d'autorisation

`REVIEW-ONLY` -> clôturer G0–G4 -> obtenir rapport contradictoire et verdict signé par responsable -> **GO explicite** pour une étape bornée ou **NO-GO**. À ce jour : `CONDITIONAL-GO pour étude`, `NO-GO pour construction v0.1 telle quelle`, `NO-GO-IMPLEMENTATION` pour tous les jalons ci-dessus. Les agents ne déduisent aucune permission de ce plan.



---

# PARTIE : GOAL DE REVUE MULTI-IA V3

# GOAL — Contre-expertise indépendante de Ravum Flow V3 (COPIER-COLLER)

> **RÔLE : Lead de revue multi-IA, strictement READ-ONLY.** Tu reçois la V3, le dossier maître historique reproduit en annexe et une synthèse externe multi-modèles apportée par l'utilisateur. **Tu dois produire une contre-expertise, PAS développer Flow.** Toute publication, modification des dépôts, PR, branche, hook, CI, licence, migration, test lourd ou action Git mutante est interdite. Le développement de Ravum est actif dans une AUTRE session ; ne touche jamais à ses fichiers ou processus. Tu n'as aucune autorisation de créer, pousser ou corriger quoi que ce soit. La décision finale appartient à l'humain.

## MISSION

Établir, à partir des sources primaires consultées réellement, si l'objectif **local-first, polyglotte, Git-compatible, multi-agents sans worktrees obligatoires, fiable sur machine 16 Go, CI/E2E honnête** mérite le développement d'un nouveau Ravum Flow, ou s'il est mieux satisfait avec des outils existants. Donner un verdict `GO pour une étape bornée | CONDITIONAL-GO étude | NO-GO` qui ne soit jamais interprété comme autorisation automatique.

## SOURCES IMPÉRATIVES — ORDRE DE LECTURE

1. Lis intégralement le dossier maître original, joint au paquet et reproduit en annexe dans la version autonome : objectifs P0-A..I, **42 ADR originaux**, 12 invariants Flow, **12 RFC RF-RFC-001..012**, 10 phases RF-0..9, 26 tickets RF-001..026, annexes de plan/architecture/CI, interdictions ; note `PROPOSED` partout où c'est indiqué.
2. Lis intégralement la synthèse externe multi-IA fournie (jointe au paquet et reproduite en annexe de la version autonome après le maître), mais **ne présume ni indépendance de tous les avis ni correction de leurs citations**. Demande si possible les rapports sources individuels ; sinon marque `NOT_PROVIDED`.
3. Lis les **trois notes V3** : `00-ARBITRAGE-CRITIQUE-V3.md`, `01-CONTRAT-TRANSACTIONNEL-ET-SECURITE.md`, `02-PLAN-REVISE-ET-DECISIONS.md` (dans le paquet, et reproduites au début de la version autonome) ; elles sont des **propositions à réfuter**, pas la réponse attendue.
4. Re-vérifie le dépôt GitHub `Rwanbt/Ravum` avec le SHA **réel au moment de ton accès**, en lecture seule. Repère la tête `dev`, `main`, les PR et branches. Pour les observations V3, l'ancre historique est `6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf` sur `dev`; `ef6b38cab0a062a5b97f862bdcdbb1ab13ec17c3` sur la révision intégrée. Si les fichiers ont changé, recommence leur vérification sur les nouveaux blobs et distingue l'historique du présent.
5. Vérifie les docs officielles/releases/licences de Git, GitButler (licence **FSL-1.1-MIT** à examiner en détail), Jujutsu, Scion (GoogleCloudPlatform, Apache-2.0), Sapling, Delta/DeltaDB, Salsa, SQLite WAL, Nx/Bazel/Pants/Buck2, Github Actions/merge queues, GitLab merge trains et outils multiagents pertinents. Les descriptions marketing ne sont pas des tests.
6. L'accès public à `Rwanbt/Ravum-Flow` renvoyait 404 lors des revues ; cela veut dire **non accessible aux outils**, pas nécessairement inexistant ou privé. **Ne le crée pas.**

## CONSTATS À REPROUVER OU CONFIRMER PAR LECTURE DU CODE

- **P0-TREE** : `ravum-collab` valide un `SourceSnapshot`, puis `GitWriter` construit l'arbre Git depuis HEAD avec seulement les chemins intégrés : le contenu qualifié peut-il différer du contenu effectivement publié ? Exige une preuve de correspondance `CandidateGitTreeOid` ↔ `VerificationReceipt` ↔ `PublishedCommit.tree`, avec hooks, dirty, ignored, submodules, symlinks, configurations et scripts de build. Vérifier si ce risque a été corrigé depuis le SHA historique. *À ce stade plausible, PAS reproduit*.
- **P0-ENV** : test harness `RustcMergeVerifier` et hooks Git héritent-ils d'un environnement hôte potentiellement secret ? Le code consulté supprimait quelques variables mais pas tout l'environnement. **Ne jamais exécuter un canari contenant un véritable secret**. Contre-mesures : allowlist, suppression du réseau, isolation OS, policy hooks, logs, limites des protections ; *exploitation non reproduite*.
- **P0-V1** : `Coordinator::new()` et le CLI `ravum-collab` conservent par défaut le protocole V1 où `AUTO_SAFE` est textuel et verification `SKIPPED`. V2 est plus strict. Établir contrat de compatibilité ; **Flow ne doit jamais transformer ce marqueur V1 en preuve sémantique implicite**.
- **P0-STORE** : `ravum-store` est l'application M4 todo avec SQLite, pas un CAS générique exploitable sans conception ; revoir l'inventaire d'extraction.
- **P0-S8** : ne pas confondre A copies privées avec Git worktrees, B shared+guard avec un moteur Flow. Le S8 réel médiane A 111,110 s, B 237,674 s, C 528,481 s ; valeurs de pilotes ~2,650/3,061/2,914 s pour première exécution, et une seconde différente dans JSON. Ne pas les agréger.

Sources code historiques :
- https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/crates/ravum-workspace-coordinator/src/bin/ravum-collab.rs
- https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/crates/ravum-workspace-coordinator/src/coordinator.rs
- https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/crates/ravum-workspace-coordinator/src/bin/common/mod.rs
- https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/crates/ravum-workspace-coordinator/src/git_writer.rs
- https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/crates/ravum-store/src/lib.rs
- https://github.com/Rwanbt/Ravum/blob/6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf/docs/mvp/MVP-0c-s8.md

## ORGANISATION MULTI-AGENTS — AUCUN FAUX CONSENSUS

Lancer **huit analyses indépendantes A–H** lorsque l'environnement offre réellement plusieurs agents, en assignant des périmètres non concurrents en **lecture seule** ; tous lisent la mission et produisent leur conclusion avant de consulter celles des autres. Le Lead **n'impose pas** l'architecture V3 comme réponse. Si le système n'offre que 3–4 agents, regrouper A+B, C+E, D+G et F+H sans perdre les objections. Si le lancement échoue, déclarer `BLOCKED` et produire uniquement une analyse individuelle en distinguant cette limite.

| Agent | Mission indépendante | Hypothèse à attaquer et preuve indispensable |
|---|---|---|
| A — VCS / transaction | Git/jj/GitButler, version logique, refs, branches virtuelles, Git index, CAS et recovery | Des propositions managed remplacent-elles utilement des worktrees ? Mode d'échec d'une vue disque partagée. |
| B — Sécurité | untrusted tests/hooks, env, sandbox Windows/Linux, secrets, traversal, crash/power-loss et SQLite WAL | La validation pourrait-elle exfiltrer un secret ou publier un arbre non qualifié ? |
| C — Build/CI | Cargo/Rust, TS, Python, C/C++, cache, graphes, ABI/exports/dépendants, E2E, merge queue/train | Comment garantir zéro faux vert tout en réduisant réellement le temps ? |
| D — Ravum | **les 42 ADR intégraux**, invariants P0, modules existants, V1/V2, identité, PatchHost, droit de réemploi | Flow double-t-il un coordinateur déjà existant ? Quel adaptateur sans cycles et sans toucher Ravum ? |
| E — Mesure | S8 complet, pilotes distincts, protocole benchmark pré-enregistré, charge 16 Go, UX supervision agents | Le scénario « sans worktree » est-il intrinsèquement gagnant ? Comment le réfuter ? |
| F — Entreprise | offline-first, grandes bases de code, forge, sécurité multiutilisateur, local/remote, VFS vs quotas | Le mode solo est-il réellement indépendant du cloud et d'un daemon global ? |
| G — Licences/art antérieur | licences primaires, disponibilité/frais GitButler/Delta/jj/Scion/Sapling, code Ravum, governance | Utiliser vs copier vs forker : quelle option a les droits et le meilleur TCO ? |
| H — Red team / zéro | essaie de **tuer le produit** : option 0 et 1 suffisantes, complexité et coût, scénarios non gérés | Quel cas utilisateur justifie une nouvelle plateforme ? Que supprimer sans perdre de valeur ? |

**Chaque rapport indépendant doit contenir :** sources primaires avec SHA/tag/version + date, faits observés, assertions non vérifiées, scénarios de contre-exemple, décisions `KEEP/CHANGE/DEFER/REJECT`, severities, impacts Ravum/Flow, un verdict propre et au moins trois objections sérieuses. Pas d'approbation automatique par majorité.

## MATRICE DES ARCHITECTURES, GATES ET CONTRADICTIONS

Comparer **Option 0** Git natif + copies/worktrees facultatifs + CI/coordination légère, **Option 1** solution existante GitButler/jj/Scion, **Option 2** coordination patch/receipts Git tree avec commit CAS, **Option 3** hybride avec isolated exec éphémère, **Option 4** VFS/CRDT avancé différé. Chaque option doit donner : machine d'états, impossibilité/possibilité des vues isolées, crash consistency, permissions, coût de maintenance, Windows/Linux, RAM 16 Go, E2E, build native, rebase/forge, sortie Git et critères d'abandon.

**Gates documentaires sans développement :**
- G0 sources originales complètes, SHAs à jour et droits identifiés.
- G1 rapports A–H indépendants et désaccords non effacés.
- G2 modèle exact des états et sécurité/processus/Git/FS avec tests de reproduction prévus, sans faux `PROVED`.
- G3 benchmark pré-enregistré, impartial, avec de vrais worktrees et contexte polyglotte, qui **ne sera exécuté que dans des fixtures et après permission séparée**.
- G4 RFC et migration Ravum/Flow proposées, toutes encore ouvertes.
- G5 approbation humaine écrite avant toute phase d'exécution.

## LIVRABLES DOCUMENTAIRES REQUIS

1. `00-SOURCES-AND-BASELINE.md` : inventaire réel, SHA, logs de lecture, NOT_REVIEWED.
2. `01-PRIOR-ART-2026.md` : solutions, versions, licences, pricing, contraintes locales/entreprise.
3. `02-ADR-REVIEW-42-plus-12.md` : **42 ADR individuellement** + 12 RFC ; projet de modifications non ratifiées.
4. `03-ALTERNATIVE-ARCHITECTURES.md` : ≥3 modèles détaillés dont aucun nouveau moteur.
5. `04-THREAT-MODEL-RECOVERY.md` : tree mismatch, subprocess, WAL, hooks, cache, crash et filesystem.
6. `05-BENCHMARK-PROTOCOL.md` : pré-enregistrement, vrai contrôle worktrees et sans worktrees, mesures justes.
7. `06-REVISED-PLAN.md` : jalons bloqués jusqu'au GO, anti-duplication de Ravum.
8. `07-DECISION-REGISTER.md` : décisions `KEEP/CHANGE/DEFER/REJECT` à arbitrer.
9. `08-EXECUTION-GOAL-DRAFT.md` : **brouillon non exécutable** ; aucun agent ne doit coder.
10. `REVIEW-SUMMARY.md` : verdict et contradictions, erreurs factuelles, doutes.
11. `TRACEABILITY.md` : P0-A..I, 12 invariants, 12 RFC, 42 ADR, RF-0..9, RF-001..026 et décisions nouvelles (021..024 proposées).
12. Rapports indépendants A–H et matrice de désaccords, **uniquement si ces agents ont effectivement travaillé**.

## FORMAT DE PREUVE ET INTERDICTIONS

Toujours étiqueter `FACT_SOURCE`, `FACT_CODE_READ`, `MEASURED`, `ASSUMED`, `PLAUSIBLE_NOT_REPRODUCED`, `NOT_TESTED`, `NOT_REVIEWED`, `BLOCKED`. Citation primaire+SHA+ligne quand possible. Aucune PR/branche/repo/LICENSE/code/build/tests exécutés ni outil d'inspection mutateur. Aucun secret divulgué. Ne jamais déclarer qu'un test vert prouve un programme correct. Ne pas masquer E2E requis. **Ne pas accepter le plan V3 par réflexe : le mettre en défaut.**

## FIN DE MISSION

Conclure par **un verdict motivé**, une table `GO / CONDITIONAL-GO / NO-GO`, liste des désaccords ouverts, preuve indépendante disponible vs manquante, décisions humaines requises, et plan de recherche contrôlé. **Arrêter avant toute implémentation.**



---

# PARTIE : HANDOFF DE RISQUES POUR LA SESSION RAVUM

# Handoff lecture seule pour la session Ravum — risques P0 issus de Flow

**Destinataire :** responsable ou session de développement Ravum déjà active. **Cette note ne lance aucun correctif et ne donne pas le GO d'en faire.** La session Flow n'a pas le droit de changer Ravum ; toute action éventuelle dépend de l'ordre de priorité et de la validation du responsable dans la session Ravum.

## Baseline lue

`Rwanbt/Ravum@6b0ac7b69b7e2ca518e3153f15ffef9a9bce3eaf` (dev observée 2026-11-09) ; fichiers `ravum-collab.rs` blob `968f03bc...`, `coordinator.rs` blob `a56f0ed8...`, `common/mod.rs` blob `783b87cf...`, `git_writer.rs` blob `0194caad...`, `ravum-store/lib.rs` blob `d6ab6e92...` ; source brute dans `00-ARBITRAGE-CRITIQUE-V3.md`.

## P0-A — Décalage entre arbre qualifié et tree Git publiable : PLAUSIBLE, NON REPRODUIT

`ravum-collab` fait vérifier un snapshot candidat, puis publie uniquement certains chemins depuis l'ancien HEAD avec un index isolé. Chercher un scénario synthétique où une dépendance locale dirty/non sélectionnée fait passer la validation, mais où un clone propre du tree publié échoue. Tester modes, hook, symlink, untracked, script de build, changement API et rebase. **N'affirmer ni exploitation ni correctif acquis avant démonstration.** Proposition de contrat futur : préparer et geler le tree Git exact, exécuter les vérifications sur ce tree, lier receipts à cet OID, refuser si tree ou contexte divergent avant le ref-CAS.

## P0-B — Environnement hérité des tests et hooks : CONFIRMÉ EN LECTURE, PAS D'EXFILTRATION DÉMONTRÉE

Le verifier lance `rustc`/binaire test avec environnement hérité moins quelques variables ; le writer exécute les hooks avec un schéma analogue. Tester uniquement avec **canaris secrets fictifs** et un faux répertoire SSH de fixture. Distinguer `trusted`/`untrusted` et vérifier que les processus enfants n'accèdent pas aux secrets, au réseau et aux volumes privés non autorisés. `env_clear()` seul ne constitue pas une sandbox ; aucune mesure de sécurité complète sans test OS Windows/Linux.

## P0-C — `AUTO_SAFE` historique V1 : COMPORTEMENT INTENTIONNEL DOCUMENTÉ

Le protocole V1 par défaut accepte un patch non chevauchant textuellement ; un test documente un `AUTO_SAFE` avec validation `Skipped`. En protocole V2, les gates sont plus stricts. **Ne pas présenter ce cas comme une faille absolue du système** : la portée V1 est explicitée dans le code. En revanche, la sortie `AUTO_SAFE` V1 ne doit pas être réinterprétée comme un reçu sémantique ni permettre une promotion dans Flow ; préciser les outils/versions/protocoles/flags utilisés par les clients.

## Écart documentaire — `ravum-store`

Module SQLite de l'application M4 Todo, **pas** un CAS générique. Enlever toute hypothèse de transfert direct depuis les futurs plans, sans toucher au module lui-même pendant la contre-expertise.

## Sortie attendue de la session Ravum, si elle accepte de traiter ces sujets

Pour chaque risque : confirmer SHA actuel et flags par défaut, relire le parcours complet, reproduire sur fixture jetable, classer `REPRODUCED / DISPROVED / NOT_REPRODUCED`, évaluer régression et coûts, puis demander autorisation selon priorités de cette session avant toute correction. Respecter CI/E2E, tests négatifs, secrets fictifs et limites 16 Go. Transmettre uniquement les faits, les SHA, les tests et les conclusions à la revue Flow. **Ne pas provoquer de concurrence avec les branches/PR de l'autre session depuis Flow.**



---

# PARTIE : DOSSIER MAITRE ORIGINAL INTEGRAL — REPRODUCTION ARCHIVE

# RAVUM FLOW — DOSSIER MAÎTRE DE CONTRE-EXPERTISE MULTI-IA

**Édition : 2026-11-09 — REVIEW-ONLY / AUCUNE IMPLÉMENTATION AUTORISÉE**

> Ce document est conçu pour être transmis intégralement à une nouvelle conversation DeepSeek, Claude, GPT, Gemini, Mistral ou autre. Il inclut le contexte décisionnel, la mission, le registre vérifié des 42 ADR de Ravum, le registre des RFC de Flow et l'intégralité du plan provisoire et des pièces préparatoires. Les 42 textes intégraux des ADR historiques sont inclus dans l’annexe Z de ce document, avec leurs liens de provenance GitHub. Les modèles doivent les lire réellement ; si un original est inaccessible pour vérifier un passage litigieux, signaler cette limite plutôt que d’inventer.

## 0. Ordre de mission — lire avant toute action

**Mission unique : effectuer une recherche internationale à jour et une contre-expertise indépendante, contradictoire et sourcée de Ravum Flow AVANT le développement.** Ne pas coder, déplacer des fichiers, ouvrir des PR, créer des branches, merger, modifier le dépôt Ravum ou le dépôt Flow, changer des ADR, lancer des migrations, modifier la CI, déplacer des fichiers de licence ou publier quoi que ce soit. Les commandes d'inspection en lecture seule sont acceptables ; si la seule méthode d'accès induit des changements locaux, s'abstenir.

Ce n'est ni une demande de validation de principe, ni un plaidoyer pour la solution proposée. Il faut être capable de recommander `NO-GO`, une architecture radicalement plus simple, des worktrees facultatifs, voire l'usage direct de produits existants si cela satisfait mieux le besoin. Un changement d'architecture substantiel doit être justifié par preuves, coût et contre-exemples. Les décisions finales reviennent exclusivement au responsable humain.

### Priorité des sources

1. Les intentions explicites du responsable, récapitulées dans la présente section.
2. Les principes Ravum P0-A à P0-I (source originale) et les ADR historiques **tels qu'écrits**, tous `PROPOSED`.
3. Le code, les tests, SHA et preuves réellement consultés dans les branches Ravum datées ; les observations contredisent les README anciens.
4. Le plan Flow v0.1 `PROPOSED`, fourni en annexe, à critiquer sans complaisance.
5. Les sources externes primaires vérifiées à la date de la revue (docs officielles, dépôts, licences, releases et commits).
6. Les avis des modèles et synthèses précédentes, qui sont des hypothèses et non des décisions ratifiées.

Toujours distinguer `FACT` (source reproductible et SHA), `MEASURED` (mesure limitée + protocole), `PROPOSED`, `ASSUMED`, `UNKNOWN`, `CONFLICTING`, `NOT_REVIEWED`, `BLOCKED`, `DISPROVED` et `PROVED` **seulement dans un périmètre de preuve explicite**.

## 1. Contexte complet : origine du besoin, décisions et contraintes

### 1.1 Pourquoi ce projet existe

Le responsable développe localement et sur GitHub/GitLab, généralement sur Windows/Linux, avec un PC de référence **16 Go de RAM**. Il utilise plusieurs agents IA en parallèle. Son workflow actuel subit un fort ralentissement : multiplication des worktrees, collisions, corrections simultanées, merges, PR et CI/E2E longs, compilation redondante, tests reconduits sans invalidation correcte. Il veut réduire le temps total jusqu'à un **livrable réellement vérifié**, sans diminuer les garanties.

La demande initiale : **une version canonique du code** accessible aux agents, chacun travaillant sur sa partie ; ne plus exiger un worktree Git par agent. Cela ne signifie pas que toutes les opérations doivent écrire directement sur le même fichier sans contrôle. La bonne représentation des propositions peut reposer sur overlays immuables, MVCC/CAS, vues virtuelles ou snapshots temporaires. Les agents humains et IA doivent partager le progrès sans s'écraser.

Le système doit : être léger et local-first ; fonctionner hors ligne sans serveur ni LLM obligatoire ; s'intégrer aux outils et Git classiques ; rester compatible GitHub et GitLab ; gérer les changements simultanés, récupération après crash, rebase et publication propre ; fonctionner avec **Ravum mais aussi Rust, TypeScript/JavaScript, Python, C/C++ et d'autres langages** ; permettre plus tard multi-utilisateurs et grandes entreprises ; favoriser compilation et tests impactés, **sans cacher aucun échec E2E pertinent** ; protéger les secrets, budgets CPU/RAM/disque et provenance.

La recherche exploratoire précédente a examiné **GitButler, Delta/DeltaDB de Zed, Jujutsu, Sapling/EdenFS, Scion de Google CloudPlatform, Anthropic Claude Code Agent Teams, Z.ai/ZCode**, et des projets d'orchestration/locks. Ces outils ne doivent pas être rejetés ni adoptés sans réexamen de leurs versions, licences, limitations, prix, confidentialité, écosystèmes et modalités réelles d'intégration.

### 1.2 Décision de séparation des dépôts (hypothèse réversible)

Le projet de langage **Ravum** est distinct du projet d'outillage universel **Ravum Flow** (nom de travail ; « Ravem Flow » est traité comme une coquille). Dépôt Ravum : [https://github.com/Rwanbt/Ravum](https://github.com/Rwanbt/Ravum). Dépôt cible envisagé : [https://github.com/Rwanbt/Ravum-Flow](https://github.com/Rwanbt/Ravum-Flow). **Au moment de la vérification, le dépôt `Ravum-Flow` n'était pas accessible (404)** ; un paquet bootstrap local a été préparé, ce qui ne prouve pas une publication GitHub. Revalider son existence au début de la revue.

**Frontière imposée :** Flow, s'il est développé, est un outil polyvalent autonome ; il ne doit pas compiler en dépendance obligatoire du compilateur Ravum. Ravum reste propriétaire de sa sémantique, de son runtime, de ses preuves de langage et de sa décision de hot swap ; Flow est candidat pour les snapshots, journal, orchestration de modifications, Git, cache générique, graphes et reçus de validation. Les adaptateurs font le lien, sans cycles de dépendances.

**Contrainte critique de concurrence avec une autre session :** une autre session poursuit le développement de Ravum. L'équipe Flow est **strictement lecture seule sur Ravum**, et **lecture seule sur Flow pendant cette contre-expertise** ; aucune modification des branches, PR, historiques, hooks, fichiers, vaults, systèmes CI ou processus de l'autre session. Ne pas considérer qu'une branche reste figée. Épingler le SHA au début de l'audit, et expliquer ce qui a changé si la tête se déplace.

### 1.3 Vision Ravum — cadre à ne pas dénaturer

Ravum explore un langage universel à développement continu, auto-mise-à-jour sûre, sûr en mémoire par défaut, ownership/borrow, sans GC obligatoire, performant natif/multi-OS/multi-device ; DSP temps réel, backend, frontend, BDD/SQL, WebAssembly, desktop, mobile, embarqué et GPU. L'édition humaine ou IA doit déclencher analyse, diagnostics structurés, compilation incrémentale ciblée, tests et vérifications déterministes, puis une prévisualisation/hot swap si réellement compatible. La dernière version **valide** reste active quand une édition est incorrecte. Aucun code exécuté en callback audio temps réel ne doit allouer, bloquer, compiler, migrer ni détruire de façon coûteuse. Les preuves annoncées doivent rester bornées et honnêtes.

Le protocole de décision du langage reste conditionnel : **F**, plateforme Rust existante qui démontre l'utilité immédiate, puis **D**, frontend/IR/langage Ravum autonome seulement si le gate comparatif M5 le justifie. Flow n'a pas le droit de devenir, par accident, un obstacle ou un double orchestrateur contraignant Ravum à Rust ou à une API privée de rustc.

### 1.4 Les neuf objectifs non négociables Ravum

| ID   | Intention normative utilisateur                                                                            |
| ---- | ---------------------------------------------------------------------------------------------------------- |
| P0-A | Feedback continu, analyse à la frappe et compilation/preview incrémentales.                                |
| P0-B | Performance native, faible overhead, SIMD/multicœur, sans GC obligatoire.                                  |
| P0-C | Sécurité mémoire par défaut, ownership/emprunts et frontières `unsafe` explicites.                         |
| P0-D | Contrats, vérifications et preuves avec bases de confiance et statuts honnêtes.                            |
| P0-E | Diagnostics structurés pour IA et correction sans affaiblissement silencieux des contrats.                 |
| P0-F | Profils temps réel et contraintes d'effets/ressources, en particulier DSP/audio.                           |
| P0-G | Universalité des domaines : web, backend, bases de données, desktop/mobile, embarqué, DSP, GPU, streaming. |
| P0-H | Faibles budgets CPU/RAM ; référence initiale de 16 Go de mémoire.                                          |
| P0-I | Productivité mesurée sur le temps **jusqu'au livrable vérifié**, humain+IA inclus.                         |

Charte : [https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/architecture/PROJECT_INVARIANTS.md](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/architecture/PROJECT_INVARIANTS.md) ; elle n'est pas ratifiée comme texte normatif autonome. Les objectifs utilisateurs ont priorité.

### 1.5 État réel de Ravum à la dernière observation (9 novembre 2026)

- `main` SHA `54491488bba78baca8df93e14742150ad2bd9b28` ; `dev` SHA `ece8133d5a97004e29a356b9014ab532aa6771da` ; `feat/mvp0c-hardening` SHA `2f92c2410ef2c7bb33f67db6737c92fddf2cefa6` ; `integration/mvp0c-clean` SHA `ef6b38cab0a062a5b97f862bdcdbb1ab13ec17c3` au moment de la consultation. **Valeurs historiques à revalider, ne jamais pousser dessus sans confirmation.**
- PR ouvertes observées : [#78](https://github.com/Rwanbt/Ravum/pull/78) MVP-0a vers dev ; [#80](https://github.com/Rwanbt/Ravum/pull/80) MVP-0b empilée ; [#82](https://github.com/Rwanbt/Ravum/pull/82) MVP-0c brouillon ; [#85](https://github.com/Rwanbt/Ravum/pull/85) intégration propre du MVP-0c vers dev. Aucune de ces PR ne doit être modifiée.
- Les rapports expérimentaux de `feat/mvp0c-hardening` indiquent snapshots/overlays, coordination multi-agents, Git writer avec index isolé / CAS / recovery, daemon Rust, graphe inter-crates, oracle d'interface Rust, impact CI et tests E2E. Ils documentent 491 tests locaux verts au jalon, mais cela ne qualifie ni un compilateur Ravum ni le projet Flow. Le code exact de la branche et les preuves doivent être revérifiés.
- **S8 réel multi-agents :** stratégie A copies privées + intégration médiane **111,1 s**, stratégie B répertoire partagé + garde **237,7 s**, stratégie C séquentiel **528,5 s** ; 27 sous-agents, 9 campagnes, 3 répétitions/stratégie, fixture réduite, 0 perte constatée. **Ce n'est ni une preuve universelle ni un benchmark du coordinateur transactionnel cible.** La stratégie A n'utilisait pas des worktrees Git ; ne pas conclure que les worktrees sont plus rapides.
- MVP-0c : `ravum-workspace-coordinator`, `ravum-snapshot`, `ravum-daemon`, `ravum-unit-graph`, `ravum-collab-oracle`, `ravum-batch`, `ravum-rust-adapter`, `ravum-protocol` sont des **candidats d'audit**, pas automatiquement des modules à copier. `ravum-patch-host`, runtime, preuve du langage et DSP restent côté Ravum. Les droits de réutilisation doivent être établis : **Ravum n'a pas de licence de code ratifiée** dans les documents consultés.
- La consolidation multi-IA [https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/reviews/2026-11-08-consolidation.md](https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/reviews/2026-11-08-consolidation.md) rappelle que l'annexe A contient les 42 ADR originaux et que les avis de modèles ne valent ni tests ni votes indépendants. Les analyses Mistral/Gemini historiques y étaient en partie identiques : préserver la provenance avant de compter des avis.

Références prioritaires :

1. [https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/architecture/STATUS.md](https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/architecture/STATUS.md)
2. [https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/mvp/MVP-0c-REPORT.md](https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/mvp/MVP-0c-REPORT.md)
3. [https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/mvp/MVP-0c-s8.md](https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/mvp/MVP-0c-s8.md)
4. [https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/mvp/MVP-0c-collaboration.md](https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/mvp/MVP-0c-collaboration.md)
5. [https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/mvp/MVP-0c-publication-robustness.md](https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/mvp/MVP-0c-publication-robustness.md)
6. [https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/mvp/MVP-0c-cli.md](https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/mvp/MVP-0c-cli.md)
7. [https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/mvp/MVP-0b-licence-analysis.md](https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/docs/mvp/MVP-0b-licence-analysis.md)

## 2. Statut des décisions et piège de revue

Tous les **ADR-000 à ADR-041 de Ravum sont `PROPOSED`**, textes historiques du 23 octobre 2026. Aucun n'est automatiquement ratifié par cette revue. Lire chaque texte intégral, y compris « Décision proposée », alternatives, conséquences, preuves exigées, critères d'échec et dépendances. Ne pas proposer de « nouvelle » solution déjà décrite dans l'original. L'annexe A de la consolidation Ravum contient des copies intégrales de ces 42 ADR ; les fichiers originaux référencés ci-dessous sont la source primaire.

### Registre exhaustif ADR Ravum (42/42)

| ADR (texte intégral)                                                                                                                                  | Intitulé                                                      | Dépendances originales | Statut   |
| ----------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- | ---------------------- | -------- |
| [ADR-000](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-000-perimetre-universel-et-garanties-conditionnelles.md)             | Périmètre universel et garanties conditionnelles              | Aucune                 | PROPOSED |
| [ADR-001](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-001-implementation-initiale-du-compilateur-en-rust.md)               | Implémentation initiale du compilateur en Rust                | 000                    | PROPOSED |
| [ADR-002](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-002-modele-memoire-ownership-emprunts-et-regions.md)                 | Modèle mémoire : ownership, emprunts et régions               | 001                    | PROPOSED |
| [ADR-003](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-003-erreurs-panic-et-defaillance-a-lexecution.md)                    | Erreurs, panic et défaillance à l’exécution                   | 002                    | PROPOSED |
| [ADR-004](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-004-contrats-raffinements-et-invariants.md)                          | Contrats, raffinements et invariants                          | 003                    | PROPOSED |
| [ADR-005](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-005-systeme-deffets-et-capacites.md)                                 | Système d’effets et capacités                                 | 002,004                | PROPOSED |
| [ADR-006](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-006-classes-temps-reel-et-certification-de-delai.md)                 | Classes temps réel et certification de délai                  | 005                    | PROPOSED |
| [ADR-007](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-007-arithmetique-entiers-flottants-et-fast-math.md)                  | Arithmétique : entiers, flottants et fast-math                | 003,004                | PROPOSED |
| [ADR-008](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-008-concurrence-synchronisation-et-annulation.md)                    | Concurrence, synchronisation et annulation                    | 002,005                | PROPOSED |
| [ADR-009](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-009-frontieres-unsafe-ffi-et-code-de-confiance.md)                   | Frontières unsafe, FFI et code de confiance                   | 001,004                | PROPOSED |
| [ADR-010](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-010-frontend-et-ir-semantique-multi-niveaux.md)                      | Frontend et IR sémantique multi-niveaux                       | 002,003,004            | PROPOSED |
| [ADR-011](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-011-syntaxe-progressive-et-parsing-dedition.md)                      | Syntaxe progressive et parsing d’édition                      | 010                    | PROPOSED |
| [ADR-012](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-012-compilation-incrementale-a-requetes.md)                          | Compilation incrémentale à requêtes                           | 010,011                | PROPOSED |
| [ADR-013](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-013-cache-de-preuves-et-identite-de-contexte.md)                     | Cache de preuves et identité de contexte                      | 012                    | PROPOSED |
| [ADR-014](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-014-protocole-de-preuve-statuts-et-confiance.md)                     | Protocole de preuve, statuts et confiance                     | 004,013                | PROPOSED |
| [ADR-015](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-015-solveurs-et-moteurs-de-verification.md)                          | Solveurs et moteurs de vérification                           | 010,014                | PROPOSED |
| [ADR-016](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-016-model-checking-fuzzing-et-simulation-deterministe.md)            | Model checking, fuzzing et simulation déterministe            | 008,015                | PROPOSED |
| [ADR-017](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-017-codegen-initial-rust-sur-comme-backend-de-confiance-limite.md)   | Codegen initial : Rust sûr comme backend de confiance limité  | 010,012                | PROPOSED |
| [ADR-018](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-018-backends-natifs-cranelift-et-llvm.md)                            | Backends natifs : Cranelift et LLVM                           | 017                    | PROPOSED |
| [ADR-019](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-019-daemon-permanent-isolation-et-budgets.md)                        | Daemon permanent, isolation et budgets                        | 012                    | PROPOSED |
| [ADR-020](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-020-compilations-a-la-frappe-et-source-incomplete.md)                | Compilations à la frappe et source incomplète                 | 011,019                | PROPOSED |
| [ADR-021](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-021-hot-swap-atomique-et-versionnement-du-code.md)                   | Hot swap atomique et versionnement du code                    | 019,020                | PROPOSED |
| [ADR-022](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-022-migration-detat-rollback-et-effets-irreversibles.md)             | Migration d’état, rollback et effets irréversibles            | 021                    | PROPOSED |
| [ADR-023](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-023-runtime-minimal-et-profils-sans-runtime.md)                      | Runtime minimal et profils sans runtime                       | 000,002                | PROPOSED |
| [ADR-024](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-024-thread-audio-et-bascule-dsp-aux-frontieres-de-bloc.md)           | Thread audio et bascule DSP aux frontières de bloc            | 006,021,022            | PROPOSED |
| [ADR-025](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-025-frontend-full-stack-et-composants-webassembly.md)                | Frontend full-stack et composants WebAssembly                 | 023,005                | PROPOSED |
| [ADR-026](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-026-schemas-et-migrations-de-bases-de-donnees.md)                    | Schémas et migrations de bases de données                     | 025                    | PROPOSED |
| [ADR-027](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-027-api-machine-pour-les-agents-et-les-ide.md)                       | API machine pour les agents et les IDE                        | 014,019                | PROPOSED |
| [ADR-028](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-028-autorite-des-contrats-et-changement-ia-verifie.md)               | Autorité des contrats et changement IA vérifié                | 027                    | PROPOSED |
| [ADR-029](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-029-security-by-default-et-sandbox-du-build.md)                      | Security-by-default et sandbox du build                       | 005,009                | PROPOSED |
| [ADR-030](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-030-reproductibilite-provenance-et-artefacts.md)                     | Reproductibilité, provenance et artefacts                     | 029                    | PROPOSED |
| [ADR-031](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-031-ci-en-gates-et-qualification-exacte-au-sha.md)                   | CI en gates et qualification exacte au SHA                    | 014,029                | PROPOSED |
| [ADR-032](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-032-performance-et-budgets-de-construction.md)                       | Performance et budgets de construction                        | 012,018                | PROPOSED |
| [ADR-033](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-033-telemetrie-confidentialite-et-donnees-locales.md)                | Télémétrie, confidentialité et données locales                | 027,029                | PROPOSED |
| [ADR-034](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-034-ergonomie-lsp-debug-et-navigation-des-preuves.md)                | Ergonomie, LSP, debug et navigation des preuves               | 011,027                | PROPOSED |
| [ADR-035](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-035-certificats-de-preuve-et-trusted-computing-base.md)              | Certificats de preuve et Trusted Computing Base               | 014,018,030            | PROPOSED |
| [ADR-036](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-036-interop-gpu-et-accelerateurs-specialises.md)                     | Interop GPU et accélérateurs spécialisés                      | 007,024                | PROPOSED |
| [ADR-037](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-037-gestion-des-paquets-plugins-macros-et-ecosysteme.md)             | Gestion des paquets, plugins, macros et écosystème            | 028,029                | PROPOSED |
| [ADR-038](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-038-ordre-des-plateformes-et-cout-de-portabilite.md)                 | Ordre des plateformes et coût de portabilité                  | 000,023                | PROPOSED |
| [ADR-039](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-039-validation-de-loptimisation-et-compatibilite-de-versions.md)     | Validation de l’optimisation et compatibilité de versions     | 007,018,035            | PROPOSED |
| [ADR-040](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-040-diagnostics-structures-audit-et-confidentialite-du-retour-ia.md) | Diagnostics structurés, audit et confidentialité du retour IA | 027,032                | PROPOSED |
| [ADR-041](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/adr/ADR-041-gouvernance-open-source-et-gestion-des-changements.md)           | Gouvernance open source et gestion des changements            | 030                    | PROPOSED |

### Avis requis par ADR

Pour CHAQUE ADR : `ID`, `SOURCE_SHA`, `REVIEWED|NOT_REVIEWED`, résumé fidèle en une phrase de la décision originale, impact Flow (direct/indirect/aucun), dépendance entre dépôts, conflit ou duplication potentielle, pertinence technique 2026, critique, preuve/benchmark manquant, action recommandée (`KEEP|CLARIFY|AMEND_VIA_RFC|SUPERSEDE_PROPOSAL|NOT_REVIEWED`), gravité et citation de paragraphe exact. **Aucun ADR n'est approuvé ou modifié par une IA**. Distinguer review de Ravum et ADR propres à Flow.

### RFC Ravum Flow à contre-expertiser (12/12, statut PENDING)

| RFC        | Sujet actuel                  | Hypothèse initiale à contester                             |
| ---------- | ----------------------------- | ---------------------------------------------------------- |
| RF-RFC-001 | Nom / binaires                | `Ravum Flow` / `ravum-flow`                                |
| RF-RFC-002 | Licence / droits d'extraction | Étudier MIT OR Apache-2.0, sans rien adopter implicitement |
| RF-RFC-003 | Source de vérité              | Git publication + journal Flow interne                     |
| RF-RFC-004 | Outils non intégrés           | Mode observé, pas de garantie d'isolation                  |
| RF-RFC-005 | Granularité des conflits      | MVCC optimiste fichier/hunk d'abord                        |
| RF-RFC-006 | Protocole externe             | API Rust + JSONL stdio, MCP éventuel                       |
| RF-RFC-007 | Journal/CAS                   | CAS local + append-only transactionnel                     |
| RF-RFC-008 | CI et build                   | Cargo natif puis adaptateurs polyglottes                   |
| RF-RFC-009 | Packaging                     | CLI Rust Windows/Linux/macOS, daemon optionnel             |
| RF-RFC-010 | Statuts de vérification       | Recettes versionnées, `PROVED` borné si réellement établi  |
| RF-RFC-011 | Distribution                  | Sync distante hors MVP, TLS/RBAC avant exposition          |
| RF-RFC-012 | IDE                           | CLI/JSON d'abord, extension plus tard                      |

Source : annexe `docs/DECISIONS.md` ci-dessous. Ces RFC ne sont pas des ADR acceptés.

## 3. Points faibles du plan initial à challenger impérativement

1. **Un seul répertoire physique vs vues différentes simultanées :** démontrer la faisabilité, notamment quand des outils traditionnels écrivent directement dans le fichier. Décrire clairement la différence `managed`, `unmanaged`, VFS/overlay et copies éphémères ; ne jamais annoncer l'absence de conflits pour les outils non interceptés.
2. **Sens de « source canonique unique » :** quelle autorité entre `HEAD`, arbre de travail, revision logique Flow, overlays, journal et version active de Ravum ? Définir invariants exacts et événements d'intégration indépendamment d'un commit Git.
3. **Atomicité Git vs système de fichiers :** `update-ref` CAS protège une référence Git, mais le working tree/index peuvent rester en reconciliation après crash. Spécifier recovery, non-perte des changements externes et hooks, sans fausse promesse d'atomicité de toute la machine.
4. **Concurrence semantic-aware :** non-chevauchement textuel ne suffit pas ; collisions même fonction, changement API sans dépendant modifié, rename/delete, fichiers générés, macros, erreurs de rebase ; refus et revalidation sur le snapshot exact.
5. **Validation avant intégration :** le plan RF-2 précède RF-4 ; quelle vérification minimale permet d'accepter une proposition sans réintroduire `AUTO_SAFE` textuel ? Définir deux gates distincts (pré-acceptation logique vs promotion validée).
6. **Intégration de Ravum existant :** plan concret pour ne pas conserver deux implémentations concurrentes de coordinateur, snapshot, oracle, protocole ou daemon ; versioning, ownership, dépréciation, tests double-repo et extraction seulement après licence/droits.
7. **Sécurité du cache et des preuves :** identité exacte des builds, variables/flags/exports, toolchain, chemins absolus, nondéterminisme, sandbox et cache poisoning ; méfiance avec l'optimisation et le partage multi-tenant.
8. **Taille et coût :** ne pas remplacer un worktree trivial par un daemon gourmand, une base de données trop lourde ou un protocole démesuré. Confirmer gain du temps jusqu'au livrable vérifié plutôt qu'ergonomie déclarée.
9. **Exécution de code non fiable :** agents, hooks Git, build scripts, compilateurs, tests, extensions IDE ; permissions OS, sandbox et secrets. Aucune isolation métier ne vaut sandbox de sécurité.
10. **GitHub/GitLab réel :** rebases, merge queues/trains, checks au SHA exact, branches protégées, commits signés, politiques d'entreprise, hooks, divergence et offline.
11. **Local/enterprise/huge monorepos :** monter en charge sans imposer une infrastructure cloud aux utilisateurs solos ; éviter de mélanger les modèles de cohérence locale et distribuée sans protocole et preuves.
12. **Ravum hot reload :** Flow ne doit jamais activer un patch lui-même ; ownership des identités `SourceSnapshotId`, `BuildContextId`, `VerificationKey`, `ArtifactId`, `ActiveVersionId`, règles RT, rollback et dernière version saine.
13. **Compatibilité multi-langage :** découverte des dépendances dynamique TS/Python, C++/headers/ABI, configs et lockfiles, plugins non fiables, limites des analyses de code non enregistré.
14. **Données privées :** provenance et logs ne doivent pas exposer sources privées, secrets, chemins, tokens, messages agents ou fichiers .env ; confidentialité local-first prouvable.
15. **Licence :** ne pas confondre référence conceptuelle et copie de GitButler (licence à vérifier), Zed/DeltaDB, Sapling, Scion ou code du dépôt Ravum sans autorisation explicite.
16. **Faux « 100 % no-worktree » :** prévoir repli contrôlé (sandbox éphémère / clone léger / worktree ponctuel) si cette option gagne sur une classe de tâches ; ne pas sacrifier correction et vitesse pour une préférence ergonomique.

Chaque point doit être classé `BLOCKER`, `HIGH`, `MEDIUM`, `LOW`, avec scénarios d'échec reproductibles, estimation d'effort et correctif minimal crédible. Le review doit aussi rechercher les oublis non présents dans cette liste.

## 4. Veille internationale à jour : sources, compétiteurs et art antérieur

Avant d'affirmer ce qui existe, ce qui est gratuit, open source, fonctionnel ou compatible Windows/Linux, **vérifier au moment de la revue** : site officiel, dépôt GitHub, documentation technique, licence effective, activité récente, statut des releases, limitations, usage cloud/compte obligatoire, coût, maintenabilité, benchmark reproductible, sécurité et possibilité d'intégration. Ne jamais faire d'affirmation de disponibilité en se fiant uniquement au marketing ou à un ancien résultat de recherche.

Périmètres minimum :

- **Versionnement et espaces collaboratifs :** Git et index alternatifs, GitButler virtual branches, Jujutsu (`jj`), Sapling/Mononoke/EdenFS/Scalar, DeltaDB / Delta de Zed (CRDT et checkouts), Gitoxide/libgit2, sparse checkout, partial clone, Git LFS.
- **Concurrency/collaboration :** Automerge, Yjs, Diamond Types, OT/CRDT, MVCC, journaux WAL, verrous/fencing, réservations de fichiers/symboles, gestion des opérations concurrentes et politiques de conflit.
- **Orchestration agents :** Scion (vérifier « Sion »), Claude Code Agent Teams, Z.ai/ZCode/GLM, OpenCode/DeepSeek Harness, agentsync, claude-collab, ai-crew-sync, MCP et mécanismes équivalents. Étudier limites des outils qui modifient directement les fichiers.
- **Builds/CI :** Bazel, Pants, Buck2, Nx, Turborepo, Ninja/CMake, Cargo incrémental, Gradle, caches distants et graphes de dépendances ; runners GitHub/GitLab ; E2E par impact + gates globaux.
- **Développement continu/hot reload :** Salsa, Deno/TS, rust-analyzer, Flutter/Metro et projets de hot reload/hot patch pertinents ; ne pas confondre la validation générale Flow et l'activation Ravum.
- **Stockage, rétention et distribution :** CAS, SQLite WAL, stockage objet, stratégies GC, réplication/reconnexion, sécurité tenant, observabilité, entrées/sorties d'une session agent.
- **Licences et gouvernance :** SPDX, droits d'auteurs, compatibilité des codes repris, alternatives build-vs-buy-vs-fork-vs-wrap, données privées et dépendance fournisseur.

Pour chaque solution : lien primaire, version et date, licence vérifiée, architecture détaillée, capacité exacte (pas adjectifs), exigences de ressources, mode offline, multi-OS, multi-langue, multi-agents, précision des garanties concurrentes, maintenance et limites ; preuve ou mention `NOT_VERIFIED`. Comparer également l'option **ne développer aucun moteur nouveau**.

## 5. Organisation de la revue multi-agents (parallélisme indépendant)

Désigner un **Lead/Synthèse** qui ne crée pas de consensus avant la réception des rapports indépendants. Les sous-agents ne doivent pas partager des conclusions prématurées. Tous effectuent une revue **lecture seule**, avec périmètres distincts :

- **A — VCS et concurrence :** Git/jj/Delta/GitButler/CRDT, modèle de transactions, espaces logiques, merge, undo, invariants ; comparer au minimum trois modèles réellement concurrents.
- **B — Sécurité/récupération :** crash consistency, WAL/CAS/idempotence, sandbox, secrets, threat model, permissions, path traversal et effets des hooks ; chercher des contre-exemples.
- **C — Compilation/cache polyglotte :** graphe d'impact, adaptateurs Rust/TS/Python/C++, tests E2E, cache invalidation, snapshots en entrée, frontières sémantiques.
- **D — Architecture Ravum & migration :** lire les 42 ADR **intégralement** ; cartographier P0 et 12 RFC, code existant, interfaces et dépendances, hot swap, licences, compatibilité avec l'autre session.
- **E — Performance/S8/UX :** définir protocole impartial, métriques end-to-end et workload, profiler les frais de coordination, qualifier mémoire 16 Go et scénarios multi-agents hétérogènes.
- **F — Entreprise et distribution :** offline-first, forge GitHub/GitLab, sécurité multi-tenant, grosse volumétrie, VFS/projection, réseau partitionné, rôle des services optionnels.
- **G — Licences et écosystème :** veille récente, maintenance, disponibilité, prix, SaaS, brevets/licences, possibilité de réemploi vs fork, coûts de dépendances.
- **H — Adversarial / option zéro :** agir comme red team du plan, démontrer pourquoi le système ne serait PAS nécessaire, proposer une solution plus petite reposant sur Git existant, et identifier les non-objectifs à préserver.

Si le système ne permet que 3–4 sous-agents, conserver l'indépendance, regrouper A+B, C+E, D+G, F+H. Tous les sujets doivent être couverts. Les conclusions doivent être émises individuellement **avant** consolidation, avec sources/dates et aveux de non-lecture.

## 6. Méthode de décision et preuves exigées

### Architectures à comparer sans favori présumé

**Option 0 :** Git+journaux/CI optimisée et coordination minimale ; worktrees si utiles. **Option 1 :** GitButler/jj ou outil préexistant + adaptateurs et locks. **Option 2 :** petit gestionnaire transactionnel de patches sur snapshots immuables + Git publication (proposition Flow). **Option 3 :** VFS/overlays par agent permettant de transparenter des outils non intégrés, avec coût OS/sécurité. **Option 4 :** système CRDT/DeltaDB et projection Git. **Option 5 :** combinaison hybride pragmatique. Proposer d'autres variantes si elles sont mieux fondées.

Pour chaque option, fournir : diagramme état/transactions, complexité minimale, danger des modifications concurrentes, compatibilité outils existants, dépendance fournisseur, charge 16 Go, OS, maintenance, risque sémantique, réversibilité, coût local et SaaS, chemin de migration de Ravum, plafond potentiel entreprise. Ne pas déguiser un « protocole désiré » en fonctionnalité déjà disponible.

### Protocole expérimental à remettre, sans l'exécuter sur les dépôts utilisateur

Comparer **A** copies privées/worktrees (séparer sous-variantes), **B** dossier partagé brut + garde, **C** séquentiel, **D** système transactionnel Flow, éventuellement **E** solution existante. Workloads : agents écrivant mêmes/différents fichiers et symboles ; renommages, API cassée, E2E rouge, erreurs de tests, crash, fichiers générés, hooks, rebases, cache changeant, large monorepo et code secret synthétique. Mesurer wall-clock **jusqu'au livrable vérifié**, tokens réels et attribution, heures de supervision, conflits/réessais, CPU/RAM/disque, perf cold/hot, p50/p95/p99, stabilité, tests non exécutés et pourquoi. Pré-enregistrer SHA, versions, seed, ordre alterné, répétitions et critères de décision ; ne pas donner une valeur seuil sortie du chapeau.

### Format des preuves

Chaque affirmation technique importante : URL/source primaire, SHA/tag/version, date de vérification, observations, niveau de confiance, contre-exemple et limites. Les conseils juridiques ne sont pas des avis juridiques. Les benchmarks publiés par des vendeurs sont à traiter comme données externes non reproductibles tant que le protocole n'a pas été vérifié.

### Gates d'entrée en développement

- `G0` **Documents lus et statut gelé** : 42 ADR audités (ou explicitement NOT_REVIEWED), RFC Flow, branches/SHAs, plan initial et non-objectifs sourcés.
- `G1` **Comparaison indépendante** : au moins 3 architectures concurrentes analysées en profondeur ; option « rien construire » évaluée sérieusement.
- `G2` **Architecture sûre et réversible** : modèle d'état, atomicité, crash recovery, confidentialité et frontières Ravum/Flow sans contradiction non traitée.
- `G3` **Licence et droits** : aucun code emprunté avant la décision humaine et l'inventaire des titulaires/licences ; la proposition permissive n'est pas une autorisation.
- `G4` **Mesure falsifiable** : corpus, bench, coût, critères Go/No-Go gelés avant implémentation.
- `G5` **Validation humaine** : plan révisé, ADR/RFC proposés, risques résiduels et stratégie de migration soumis au responsable ; **aucun lancement automatique**.

## 7. Livrables attendus de DeepSeek (uniquement texte et documents de revue)

1. `00-SOURCES-AND-BASELINE.md` — inventaire des sources réellement lues, SHA, état de repo et matrices `NOT_REVIEWED`.
2. `01-PRIOR-ART-2026.md` — veille des solutions et coûts/licences, avec recommandations sourcées et classement, sans marketing non vérifié.
3. `02-ADR-REVIEW-42-plus-12.md` — tableau **42 ADR Ravum + 12 RFC Flow** et propositions de nouveaux ADR spécifiques, sans changement unilatéral de statut.
4. `03-ALTERNATIVE-ARCHITECTURES.md` — au moins 3 architectures détaillées, scénarios de race, coût, limites et critères de choix.
5. `04-THREAT-MODEL-RECOVERY.md` — sécurité, secrets, provenance, écriture concurrente, crash, recovery, supply chain et distributeur.
6. `05-BENCHMARK-PROTOCOL.md` — plan S8 étendu et impartial, sans inventer de mesures et sans masquer E2E/CI.
7. `06-REVISED-PLAN.md` — roadmap révisée sans dette volontaire, dépendances, lots parallèles disjoints, objectifs vérifiables et cas de repli.
8. `07-DECISION-REGISTER.md` — à **faire approuver** : `KEEP|CHANGE|DEFER|REJECT`, avantages/risques/sources et impact Ravum/Flow.
9. `08-EXECUTION-GOAL-DRAFT.md` — projet de GOAL de développement, **non exécutable tant qu'un humain ne donne pas GO**.
10. `REVIEW-SUMMARY.md` — synthèse contradictoire, désaccords non résolus, erreurs du plan initial, recommandations classées et verdict `GO|CONDITIONAL-GO|NO-GO` motivé.

Inclure une matrice de conformité couvrant **P0-A à P0-I, les 12 invariants Flow, les 12 RFC, les 42 ADR, RF-0 à RF-9 et le backlog RF-001 à RF-026**. N'inventer ni mesures, ni références, ni statuts de tests. Chaque équipe livre sa synthèse indépendante et une liste d'objections majeures ; le Lead conserve les désaccords et les sources, ne fait pas de vote de majorité artificiel.

## 8. Interdictions absolues pendant ce cycle

- **Pas de code, dépôt, commit, PR, merge, `git checkout` risqué, extraction Ravum, migration, suppression, modification de tests, changement de seuil, création de LICENSE ou publication sur GitHub/GitLab**.
- **Pas de modification du dépôt Ravum ni du workspace/branches de l'autre session** ; pas de démarrage de builds lourds pouvant pénaliser la session active (préférer lecture seule et analyse des preuves existantes).
- **Pas de « PROVED » sur la base d'un test vert ; pas de `PASS` si un E2E requis est échoué, absent ou inconnu**.
- **Pas de décisions ADR `ACCEPTED` sans autorisation explicite du responsable**.
- **Pas de conclusion « un dossier partagé est forcément plus rapide »** : les mesures historiques de Ravum contredisent cette supposition sur leur corpus.
- **Pas d'oubli du cas multi-langage, ni de serveur/cloud comme prérequis au mode solo**.

**Répondre en français. Commencer par un audit documentaire et l'assignation des sous-agents. Remettre la contre-expertise avant tout GOAL d'implémentation.**

---

# ANNEXE A — Plan d’implémentation original Ravum Flow v0.1 (reproduit intégralement ; PROPOSED)

# Ravum Flow — plan d'implémentation complet

**Version :** `0.1 / PLAN-ONLY` · **Date :** 9 novembre 2026 · **Statut :** PROPOSED, soumis à validation humaine.

**Mission :** développer un moteur de travail sur le code, **local-first**, compatible Git/GitHub/GitLab et indépendant des langages, qui permette à plusieurs humains et agents de produire des modifications en continu **sans exiger un worktree Git par agent**, tout en conservant la vérifiabilité, le versionnement, la récupération et les performances. Ravum doit en être un consommateur privilégié, pas une dépendance.

Les actions décrites ci-dessous ne sont pas encore implémentées dans ce dépôt. Les composants réels existants dans le dépôt Ravum devront être audités avant toute extraction. Les critères de performance chiffrés sont des **cibles candidates à valider contre des baselines**, jamais des résultats acquis.

---

## 0. Contexte vérifié et raison d'être

Ravum étudie une boucle édition → analyse → vérification → compilation ciblée → patch compatible → activation sûre. La branche `Rwanbt/Ravum@feat/mvp0c-hardening` comporte des prototypes : snapshot/overlays, coordinateur, publication Git transactionnelle, daemon Rust, graphe inter-crates, oracle sémantique et CI par impact. D'après les rapports de ce jalon, les suites locales étaient vertes et des essais de crash et de collaboration ont été réalisés. **Ce n'est pas un Flow utilisable de façon autonome, multi-langages, ni une release.**

Le benchmark S8 documente une limite fondamentale. Sur 3 petites campagnes par stratégie, la médiane avec 27 sous-agents DSH était environ **111,1 s** en copies privées + intégration, **237,7 s** en répertoire partagé + garde, **528,5 s** en séquentiel. Cette expérience compare une fixture précise et une garde optimiste simple, et **non** le futur modèle transactionnel. Elle interdit de prendre « un dossier partagé = plus rapide » comme postulat.

### Résultat recherché

Un humain/agent modifie des zones de code depuis un contexte exact ; le moteur enregistre une proposition attribuée, détecte les collisions avant publication, vérifie seulement les dépendances réellement touchées et produit un état publiable Git. Les autres agents peuvent continuer sur une base saine. Tout fonctionne **hors ligne** en mode solo ; la collaboration distante, les caches partagés et l'intégration forge sont optionnels.

### Non-objectifs initiaux

- Remplacer le format Git, créer une forge, créer un langage, un compilateur, un linker ou un IDE.
- Garantir « zéro bug », « zéro crash », « preuve formelle générale » ou hot swap arbitraire.
- Fusionner automatiquement deux algorithmes sémantiquement incompatibles au seul motif que les lignes sont distinctes.
- Supprimer les isolations de tests nécessaires pour exécuter du code non fiable, ou supprimer les tests E2E.
- Émettre des garanties sur des agents non intégrés qui écrivent directement dans le système de fichiers.
- Déployer PostgreSQL, Kubernetes ou un service cloud pour un usage mono-PC.

---

## 1. Invariants produit

**INV-01 — Source canonique unique.** Un seul répertoire utilisateur constitue l'état canonique visible. Les propositions sont des overlays de contenu, pas des copies Git physiques par agent. Les builds peuvent matérialiser **des snapshots temporaires immuables** quand l'outillage externe l'exige : ces sandbox ne sont pas des worktrees Git persistants.

**INV-02 — Aucune écriture silencieusement perdue.** Une proposition porte un `base_snapshot_id` ; sa validation et sa publication font une comparaison de version (CAS/MVCC). Si la base diverge, rejeter, rebaser explicitement et revalider.

**INV-03 — Pas de mutation non transactionnelle de l'état validé.** Chaque changement canonique est atomiquement publiable ou absent. Le journal durable sait distinguer intention, commit et réconciliation après crash.

**INV-04 — Compilations et validations rattachées au snapshot exact.** Une compilation réussie sur l'union accidentelle de modifications de plusieurs agents **ne qualifie pas** un patch individuel. Les reçus portent une empreinte complète.

**INV-05 — Sémantique honnête.** Niveaux distincts : textuellement applicable, vérifié sémantiquement par un oracle déclaratif, testé, prouvé dans une portée bornée, incertain, refusé. Aucun `PROVED` inféré par défaut.

**INV-06 — Langages substituables.** Le noyau ne dépend pas de `rustc`, `tsc`, Python, Ravum ou d'un protocole propriétaire. Le niveau de vérification accessible est déclaré par adaptateur.

**INV-07 — Compatibilité Git.** L'utilisateur peut toujours cloner, ouvrir une PR, auditer les commits, revoir des diff et utiliser GitHub/GitLab sans serveur Flow.

**INV-08 — Système local utilisable hors ligne.** Le chemin principal (édition, snapshots, analyse déjà installée, historique, vérification locale et commit Git) ne demande aucun compte externe.

**INV-09 — Budget et annulation.** CPU/RAM/disque/temps de compilation bornés et configurables. Les analyses périmées sont interrompues ou ignorées. Machine de référence initiale : Windows, 16 Go RAM.

**INV-10 — Échecs observables.** Tests cassés, E2E en échec, limites d'oracle, cache miss, timeouts et erreurs de reprise sont visibles et ne deviennent jamais des succès par omission.

**INV-11 — Réversibilité.** On peut désactiver Flow, récupérer un dépôt Git propre et relire le journal indépendamment d'une instance cloud. Le moteur ne doit pas prendre le dépôt en otage.

**INV-12 — Performance end-to-end.** Juger d'abord le temps jusqu'à un livrable vérifié, avec coût humain, agents, conflits, revues, CPU/RAM, E2E et CI. L'absence de worktree est une propriété ergonomique, pas un score de performance.

---

## 2. Frontières et sous-systèmes

```
Human / IDE / agent (Claude, DeepSeek, Z.ai, CLI, etc.)
          │ intents, base_snapshot, proposed patches
          ▼
  flow-cli + protocol (JSON-lines / SDK)
          │
   flow-coordinator ───── journal durable / revision CAS
          │                       │
   flow-snapshot (CAS)  ◄─────────┘
          │
   flow-impact / scheduler / cache
          ├── adapter-rust   (Cargo/rustc)
          ├── adapter-ts     (TypeScript/Javascript)
          ├── adapter-python (Python)
          ├── adapter-cpp    (CMake/Clang)
          └── adapter-ravum  (future compiler; optional)
          │
   verification receipts / provenance
          │
   flow-vcs (Git local) → GitHub / GitLab (optional)
          │
   optional: sync service / distributed cache / remote runners
```

**Bornes d'ownership :** Flow possède coordination, état des propositions, stockage de contenu, scheduler, publication et preuves de test *comme métadonnées*. Chaque adaptateur de langage possède son analyse/compilation et rapporte précisément son domaine de validité. Ravum conserve son propre compilateur, runtime, moteur de preuve du langage et PatchHost. Aucun cycle de dépendances binaires entre dépôts.

### API logique candidate (non implémentée)

1. `flow init` : inventorier dépôt local et capacités des adaptateurs, sans mutation destructive.
2. `flow snapshot` : produire un snapshot immuable et son identifiant.
3. `flow task begin --base <id>` : ouvrir un contexte de travail attribué.
4. `flow propose --task <id> --patch <file>` : soumettre une modification sur sa base exacte.
5. `flow verify --proposal <id>` : exécuter les vérifications déclarées sur un état candidat immuable.
6. `flow integrate --proposal <id>` : CAS sur l'état logique et journalisation ; refus si périmé.
7. `flow publish --proposal <id> --git` : créer le commit Git atomique lié au reçu, avec hooks.
8. `flow recover` / `flow status --json` : reprise, inspection et preuves.
9. `flow adapter list`, `flow cache stats/gc`, `flow doctor` : outils de diagnostic.

Commandes purement indicatives ; leur forme et compatibilité seront décidées par RFC avant exposition publique.

### Mode sûr vs mode compatible

- **Managed (P0) :** l'agent utilise API/CLI ; le moteur valide chaque patch et son identité. Il ne peut pas écrire directement le canonique via cette interface.
- **Observed/unmanaged (P0 compatibilité) :** un outil écrit déjà dans un fichier partagé. Le watcher détecte le changement et le signale comme `UNMANAGED`, mais **ne prétend pas prévenir** une écriture déjà écrasée. Ne pas attribuer ce mode aux garanties d'INV-02.
- **Virtualized (P2+) :** système de fichiers projeté par agent (ex. ProjFS/FUSE ou adaptateur IDE) avec vues overlay transparentes ; à qualifier par OS et sécurité avant toute promesse.

---

## 3. Modèle de données et garanties de concurrence

Les identités de contenu doivent inclure chemins normalisés, octets, contexte d'analyse, dépendances, toolchain, flags, environnement pertinent et plateforme ; **ne pas réutiliser** un cache dont le contexte est incertain. Les identités ne constituent pas, seules, une signature cryptographique de confiance.

| Entité                 | Champs minimum                                                                                      |
| ---------------------- | --------------------------------------------------------------------------------------------------- |
| `Workspace`            | `workspace_id`, `root`, `policy_version`, `git_base`, `schema_version`                              |
| `SourceSnapshot`       | `snapshot_id`, map `path → content_id`, parents, exclusions explicites                              |
| `ChangeProposal`       | `proposal_id`, `task_id`, `actor_id`, `base_snapshot_id`, patches, préconditions, timestamp logique |
| `WriteLease` optionnel | `scope`, propriétaire, expiry, fencing token, nonce de session                                      |
| `MergeDecision`        | `base`, `candidate`, `conflicts[]`, `evaluation_level`, `reasons[]`                                 |
| `VerificationReceipt`  | `candidate_id`, `adapter_id/version`, context digest, toolchain, tests, verdicts, evidence IDs      |
| `PublicationIntent`    | `expected_ref`, commit/tree candidate, journal_seq, idempotency key                                 |
| `PublicationReceipt`   | commit SHA, new ref, status, journal_seq, recovery status                                           |

**CAS :** accepter une proposition seulement si ses préconditions et la version des zones qu'elle cible sont toujours valides. Deux patches sur fichiers disjoints peuvent être intégrés dans un même snapshot ; deux patches sur un même symbole ne sont pas considérés compatibles par simple non-chevauchement textuel.

**Verrous :** préférer contrôle optimiste comme mode normal, avec leases/fencing pour opérations à forte collision. Un lease expiré ne doit pas autoriser un ancien processus à publier (fencing token). Tester TOCTOU, renames et modifications de fichiers générés.

**Journal :** append-only avec checksum, flush et ordre explicite. Crash avant CAS ⇒ ancien état canonique ; crash après CAS ⇒ commit complet et reprise idempotente, même si la réconciliation disque/index reste nécessaire. Cas négatifs en `docs/BENCHMARKS.md`.

**Git :** index temporaire, création des blobs/tree/commit et `update-ref` avec expected SHA ; respecter les hooks utilisateurs et leurs refus. Les agents ne font pas simultanément `git add -A` dans l'index partagé. Ne pas supprimer de modifications non attribuées pendant la réconciliation.

---

## 4. Stratégie incrémentale et multi-langages

### 4.1 Contrat d'un adaptateur

Un adaptateur déclare : `language_ids`, outils requis, versions, plateformes, capacités (`discover_graph`, `diagnostics`, `check`, `build`, `test`, `test_impact`, `format`, `hot_reload`), périmètre des entrées et sorties, et limites connues. Chaque résultat porte la liste des fichiers et dépendances observés et un `confidence` explicite. L'absence d'une capacité est `UNSUPPORTED`, non une réussite.

**Ordre :** adaptateur Rust initial (existant dans Ravum à auditer) → TypeScript/JavaScript → Python → C/C++ → Ravum natif au moment où le compilateur/daemon existera. D'autres langages sont des plugins, pas des responsabilités du noyau.

### 4.2 Planification et analyse d'impact

- Construire un graphe conservateur des unités/tâches (source → interface → dépendants → vérifications).
- À chaque patch, invalider les unités réellement impactées **et leurs dépendants transitifs** ; si graphe incomplet, retomber sur validation plus large.
- Dédupliquer les requêtes identiques sur le même `candidate_id`. Permettre coalescence de patches indépendants uniquement si le coût prédit est inférieur au risque de reprise : **désactivé par défaut** tant que S8 et tests de conflit ne valident pas l'heuristique.
- Priorité à la prévisualisation interactive, puis tests ciblés, puis vérification complète ; préemption/annulation des tâches obsolètes.
- Ne jamais valider un patch sur un mélange accidentel de révisions. Les jobs reçoivent des entrées immuables.
- Les tests E2E affectés sont inclus. Les suites complètes restent exigées sur cadence définie et aux gates d'intégration.

### 4.3 Cache

Cache CAS local et journal indépendant du cache. Toute entrée est liée à son toolchain/config/OS/arch/flags/deps et au digest de la commande. Politique d'éviction LRU/budget disque ; le GC ne peut supprimer un artefact épinglé par une tâche active, un journal non résolu ou un reçu publié. Attaques de cache poisoning, collisions, corruptions et éditions pendant lecture testées avant cache distant.

### 4.4 Hot reload

Flow expose des changements vérifiés et un manifeste ; il **n'impose pas** le hot swap. Chaque runtime décide s'il peut activer un patch. Pour Ravum : conserver le dernier état sain, tester compatibilité ABI/état/effets, préparer hors thread RT, puis activation fail-closed. Pour Python/JS, un adaptateur peut lancer un reload existant ; C++ peut demander redémarrage. Le mode temps réel audio ne permet aucune compilation/allocation/lock/destruction coûteuse sur callback.

---

## 5. Roadmap de livraison (gates par résultats, pas par dates inventées)

### RF-0 — Bootstrap, gouvernance et benchmark de référence

**Dépendances : aucune. Priorité : bloquante.**

- Décider explicitement du nom, du statut public/privé et de la licence. Vérifier droits/auteurs pour toute extraction depuis Ravum et dépendances tierces. Documenter DCO/CLA si contributions externes.
- Définir `AGENTS.md`, ADR/RFC, conventions de versions et schémas, sécurité des branches `main`/\`dev\` (PR et CI bloquants), interdiction de force push.
- Établir trois fixtures indépendantes : Rust multi-crates, TypeScript mini-app, corpus concurrent à 3 agents ; inclure changements rouges, signatures cassées, fichiers générés et conflits.
- Capturer baseline **A** copies privées + intégration, **B** répertoire partagé simple, **C** séquentiel et **D** future stratégie Flow, avec même corpus, hardware, modèles, tests, caches et charge.
- Définir politique de collecte des tokens et secrets. Ne pas exporter de source confidentielle vers le cloud.

**Gate RF-0 :** droits déterminés, rapport de baseline reproductible, fixtures rouges→vertes, critères de mesure gelés ; pas de code tiers copié avant décision licence.

### RF-1 — Contrats et noyau minimal autonome

**Dépendances : RF-0.**

- Contrats versionnés de snapshot/proposition/décision/reçu/journal ; sérialisation stable et migration v1→v2 testée.
- CLI minimale `init`, `snapshot`, `status`, `doctor`, `propose` sur fichiers texte ; API Rust interne, JSONL sur stdio pour outils externes.
- CAS local de blobs par SHA-256 ; chemins relatifs normalisés et sandbox d'accès (traversal, symlinks, casse Windows).
- Journal durable redémarrable, écritures atomiques, migration de schéma et garbage collection qui protège les références vivantes.
- Récupérer ou réimplémenter de manière conforme les tests de `ravum-snapshot`, sans recopier de code sans droits.

**Gate RF-1 :** CLI fonctionne hors ligne Windows/Linux ; snapshot stable ; corruption et journal tronqué détectés ; zéro modification de source utilisateur par `snapshot` ; fuzz de parseur.

### RF-2 — Transactions multi-agents sans worktrees obligatoires

**Dépendances : RF-1. Premier MVP de la vision.**

- `begin → propose → reconcile → integrate` avec `base_snapshot_id` et CAS ; contexte d'agent basé sur overlays copy-on-write.
- Hunk conflicts, rename/delete/edit, collisions de même fichier, changements indépendants du même fichier, propositions simultanées.
- Ownership et lease facultatifs par fichier ; contrôle optimistic default, fencing pour leases et reprise après crash.
- Identité par agent/tâche, receipts, audit, undo ciblé sur une transaction et non `git reset --hard` global.
- Mode `UNMANAGED` détecté ; agents non intégrés n'obtiennent aucune garantie d'exclusivité.
- Harnais concurrent : N agents, fuzz de l'ordre d'arrivée et suppression forcée de processus.

**Gate RF-2 :** 0 perte silencieuse dans un corpus adversarial et reprise complète ; les changements indépendants coexistent ; aucun `AUTO_SAFE` sans vérification déclarée ; tout refus explique son motif.

### RF-3 — Publication Git transactionnelle, sans forge obligatoire

**Dépendances : RF-2.**

- Adapter le writer transactionnel Git du MVP-0c : index isolé, hooks préservés, `commit-tree`, `update-ref` CAS, journal INTENT/OUTCOME et recover.
- Commits attribués par tâche avec liens vers reçus ; options de commits locaux indépendants, sans faire reposer l'isolation sur une branche physique par agent.
- Rebase/revalidation si `HEAD` change ; tests sur index sale, modification externe, hooks qui refusent, permissions et crash entre étapes.
- Export/import de patches et retour au Git classique, sans verrouillage propriétaire du dépôt.

**Gate RF-3 :** tests de crash réels, idempotence et hooks réussis sur Windows/Linux ; aucune régression des tests Ravum portés ; Git reste lisible sans Flow.

### RF-4 — Validation continue et graphe d'impact universel

**Dépendances : RF-2 ; publication indépendante possible après RF-3.**

- Extraire le **contrat** du moteur de graphe ; conserver la découverte Cargo dans `adapter-rust`.
- Scheduler annulable, dedup, priorités, limites de compilations concurrentes et de RAM. Même snapshot exact pour build et test.
- Reçus `TEXTUALLY_APPLICABLE`, `SEMANTICALLY_CHECKED`, `TESTED`, `PROVED` borné, `STALE`, `UNKNOWN`, `TIMEOUT`, `UNSUPPORTED` ; aucun succès implicite.
- Cache local et invalidation inter-crates ; diagnostics d'erreur sur dépendant aux sources inchangées (test de rupture d'API).
- Adapter les tests ciblés **avec E2E impactés**, plus suite intégrale au gate de merge et campagne périodique.
- Détecter outillage absent, flags variables, dépendances dynamiques, build scripts, renvoi de validation complète conservateur.

**Gate RF-4 :** aucun cache stale accepté, validateur correct sur changements directs/transitifs ; couverture des E2E pertinente, budgets respectés ; temps/ressources publiés face à la baseline.

### RF-5 — Agents et IDE, expérience local-first

**Dépendances : RF-2 et RF-4.**

- Clients CLI/JSONL pour agents indépendants du fournisseur (DeepSeek, Claude, OpenCode, Codex, autres) ; ne jamais imposer un LLM pour le noyau.
- SDK minimal ; métadonnées de scope/provenance ; protocole versionné de notification et demande de patch.
- Mode humain : accepter/rejeter/cherry-pick d'une proposition avec diff et impact visibles, inspection de la validation.
- VS Code extension légère en lecture/interaction uniquement après preuve du protocole CLI ; pas d'IDE complet.
- Adapter l'accès à l'éditeur non sauvegardé en overlay sans muter la source utilisateur ; résoudre le statut `UNMANAGED` de façon UX claire.

**Gate RF-5 :** trois agents hétérogènes présentent des patches sur une même base et publient sans perte ; l'intégrité reste vraie en cas de crash de l'agent ; aucune donnée secrète dans logs.

### RF-6 — Polyglotte réel

**Dépendances : RF-4/5.**

- TypeScript/JavaScript : découverte package graph + `tsc`/test runner configurables ; invalidation conservatrice de configs et lockfile.
- Python : import graph conservateur + Ruff/pytest/typing optionnel ; imports dynamiques ⇒ fallback global, pas faux négatif.
- C/C++ : `compile_commands.json`, Clang/CMake/Ninja ; ABI/headers/flags/targets inclus dans les identités.
- Rust : démontrer propagation inter-crates et tests Cargo avec dépendances externes.
- Matrice OS/versions et fixtures rouge→vert pour chaque langage ; mode sans build explicite pour dépôts documentation.

**Gate RF-6 :** exécution d'au moins 3 langages non Ravum ; tests de modifications de config, dépendances, scripts et fichiers générés ; tous les fallbacks documentés.

### RF-7 — Distribution GitHub/GitLab et CI

**Dépendances : RF-3/4.**

- Forge adapters indépendants : authentification par jetons propres à la forge et secret store de l'OS, jamais en clair dans le journal.
- PR/MR attachées à commit SHA + receipt ID ; commentaires de checks et annotations sans divulguer des sources confidentielles.
- Checks de merge sur état intégré, merge queue/train selon les capacités de la forge ; branch protection/policy et dry-run local.
- Sync réseau **facultative** ; queues offline et reconnexion explicite, pas de commit publié deux fois.
- Cache partagé facultatif **uniquement après** qualification de l'identité et de l'isolation des locataires.

**Gate RF-7 :** PR GitHub + MR GitLab sur comptes/tests dédiés ; perte de réseau, conflits distants, révocation de token, rebase et backpressure testés ; aucun secret dans artefacts.

### RF-8 — Synchronisation multi-machines et entreprise

**Dépendances : RF-5/7, hors MVP local.**

- Service de coordination optionnel pour `actor_id`, lease/fencing, journal répliqué et contrôle d'accès ; consensus/leader pour publication canonique.
- TLS, authentification, autorisation par workspace/projet, audit, effacement et politique de rétention ; cache distant optionnel et tenant isolation.
- Exécuteurs de tests/build distants sur snapshots immuables, résultat lié à l'identité complète ; quotas CPU/RAM/bande passante.
- Grands monorepos : sparse checkout/partial clone, CAS projeté, graphe incrémental partitionné, chargement à la demande et GC borné.
- Test réseau : partition, duplicate/reorder, reconnexion, horloges divergentes, latence, désaccord sur état canonique.

**Gate RF-8 :** aucune divergence canonique ni fuite inter-tenant sous fautes simulées ; mêmes sémantiques de transaction qu'en solo ; pas de serveur requis pour le mode local.

### RF-9 — Intégration privilégiée avec Ravum

**Dépendances : RF-4 ; progressive et parallèle à RF-6/7.**

- Adaptateur `adapter-ravum` basé sur une interface stable, non un lien binaire cyclique entre dépôts.
- Réutiliser les diagnostics structurés et identités de contrat/proof du futur compilateur Ravum ; différencier résultats testés, bornés et inconnus.
- Recevoir `candidate_id` validé pour la boucle continue; le PatchHost Ravum garde le pouvoir d'accepter/refuser le hot swap.
- DSP temps réel : préparation hors callback, contrôle effets/ressources, activation fail-closed et maintien dernière version saine.
- Qualification comparative Rust/Flow seul vs Ravum+Flow, p50/p95/p99 et coûts d'intégration ; aucune preuve automatique implicite.

**Gate RF-9 :** Ravum continue à fonctionner en mode sans Flow ; Flow continue à fonctionner sans compilateur Ravum ; hot swap refusé proprement en cas d'incompatibilité.

---

## 6. Découpage en livrables publiables

| Livrable               | Phases minimales    | Public visé                       | Pourquoi il est utile                                                                              |
| ---------------------- | ------------------- | --------------------------------- | -------------------------------------------------------------------------------------------------- |
| **Prototype contrôlé** | RF-0→RF-2           | Recherche et tests                | Comparer une coordination transactionnelle à des copies privées sans faire de promesse commerciale |
| **Alpha locale Git**   | RF-0→RF-4           | Développeur solo et agents locaux | Modifications attribuées + validations Rust + commits sûrs, sans worktrees obligatoires            |
| **Beta polyglotte**    | + RF-5/6/7          | Solo et petites équipes           | Agents hétérogènes, plusieurs langages, GitHub/GitLab, CI sélective                                |
| **Version distribuée** | + RF-8              | Organisations                     | Gouvernance multi-machines, caches et exécuteurs partagés                                          |
| **Ravum intégré**      | RF-9 selon maturité | Développeurs Ravum                | Analyse et activation de code Ravum sur états vérifiés                                             |

Aucune durée ni date de release n'est présumée. Les phases sans valeur autonome peuvent être re-scopées après les résultats de RF-0, et les abstractions sans consommateur réel doivent être retirées.

---

## 7. Qualité, sécurité, CI et observabilité

### Gates CI requis

1. **Chaque PR** : format/lint ciblé, unit tests impactés + dépendants, checks d'intégration nécessaires, tests de protocole/journal et assertions d'invalidation ; fail closed sur graphe incomplet.
2. **PR d'intégration / merge queue** : validation de l'état combiné exact, suite complète pertinente incluant E2E, tests de concurrence et publication Git quand le core est touché.
3. **Nightly/periodic** : tests complets multi-OS et polyglottes, fuzz, crash-injection, tests de rétention/GC, matrice de toolchains.
4. **Release candidate** : reproductibilité, S8 humain+agents avec alternatives comparées, audits dépendances/licences, secrets, régression perf/ram/disk, migration de schéma, recovery, rollback.

**Jamais** neutraliser un E2E rouge sous prétexte qu'il ralentit. Les tests non exécutables doivent apparaître explicitement `BLOCKED` ou `UNSUPPORTED` selon leur vraie cause et le gate défini ; pas `PASS`.

### Sécurité

- Confinement à une racine allowlist ; rejeter chemins traversants, symlinks hors racine, collisions de casse Unicode/Windows et noms réservés.
- Journal et diagnostic nettoyés : pas de secrets, clés, corps de `.env`, tokens OAuth, variables sensibles ; fichiers d'évidence chiffrables si partage distant.
- Toute exécution de code de test est potentiellement non fiable : subprocess avec environnement borné ; sandbox/temp snapshot pour les tâches à risque, même si le **développement** n'utilise pas de worktrees.
- Audit de provenance, SBOM et dépendances avant release ; refus automatique des artefacts non cohérents avec leurs digests.
- TLS/RBAC/logging d'accès obligatoires avant tout service réseau partagé ; ne pas confondre un hub multi-agent avec une authentification implicite.

### Observabilité

Reçus JSON stables : `snapshot_id`, `proposal_id`, `actor_id` pseudonymisé, temps par phase, cache hits/misses, cancellations, conflicts, protected paths, test inventory, `toolchain_id`, `git_sha`, erreurs, limites et statut. Les ressources mesurées indiquent matériel, environnement, protocole et dispersion. Capturer **supervision humaine et tokens réels** quand le harnais le permet, sans estimation imaginaire.

---

## 8. Benchmarks et objectifs de performance

**Protocole normatif proposé :** `docs/BENCHMARKS.md`. Comparer au minimum : A copies privées, B partage simple, C séquentiel, D Flow transactionnel. Ordre alterné et répétitions suffisantes pour stabiliser les intervalles ; corpus identique, mêmes agents/modèles, caches chauds/froids séparés, compilation et E2E inclus, machine de référence 16 Go puis Linux/macOS. Les coûts doivent être mesurés jusqu'au **livrable réellement vérifié**.

Cibles de sûreté : **0 changement perdu**, **0 publication partielle**, **0 résultat stale déclaré valide**, **0 secret exposé dans les artefacts de test**, **0 test obligatoire masqué**. Ces exigences sont des gates stricts, pas des taux de performance.

Cibles de vitesse : n'en fixer les valeurs qu'après RF-0 ; comparer **non-régression** (ou amélioration avec IC) par scénario, mémoire et disque plafonnés par configuration, et analyser les p50/p95/p99. Les cibles interactives Ravum (ex. p95 diagnostic <50 ms, patch <500 ms dans les anciens plans) ne sont **pas** des performances Flow déjà atteintes ; elles restent expérimentales et propres à un corpus/platforme.

Critère Go/No-Go principal : ne pas imposer un répertoire partagé si D (Flow) ne réduit pas le temps jusqu'au livrable vérifié ou ne compense pas son surcoût par une fiabilité ergonomique démontrée. Offrir la stratégie A en fallback plutôt que dégrader les utilisateurs.

---

## 9. Gouvernance technique et stratégie Git

- Un responsable d'intégration pour les premières phases ; plusieurs agents peuvent produire des patches, mais un seul coordinateur publie sur l'état canonique.
- Branches : `main` pour la référence revue et `dev` pour l'intégration ; branches courtes `feat/RF-...`, PR vers `dev`. Branches d'agent purement virtuelles **après** qualification, pas des refs Git obligatoires.
- Politique de migration : toutes les extractions depuis Ravum sont liées au SHA et chemin source, auteurs/droits et tests portés. Ne jamais déplacer la seule implémentation avant validation sur les deux repos.
- Décisions ADR séparées du backlog ; aucun agent n'accepte son propre ADR ni n'abaisse seuil/contrat.
- Versionner le protocole, le format du journal et les reçus ; migrations avant modifications incompatibles ; option d'export des données en JSONL.
- Livraisons courtes et indépendantes ; privilégier une CLI testable et des adaptateurs réels plutôt qu'un framework générique abstrait.

---

## 10. Ordre d'exécution immédiat proposé

**Semaine de travail technique n°1 (ordre, pas promesse de durée) :**

1. Vérifier le statut du dépôt Ravum, le SHA de la branche, les PR empilées, l'issue licence et les conditions de droits. Ne rien merger ni modifier sur Ravum.
2. Finaliser RF-RFC-001/002 (nom, licence) puis déposer la baseline documentée ; préparer une fixture Rust indépendante.
3. Implémenter le schéma minimal d'un `SourceSnapshot` et `ChangeProposal` **sans** migration de gros volumes de code.
4. Prototyper trois agents scriptés envoyant 3 patches : fichiers distincts ; fonctions distinctes même fichier ; même fonction. Enregistrer refus, rebase, reprise et identité.
5. Brancher un Git writer transactionnel sur un **dépôt jetable**, avec crash-injection avant/après CAS.
6. Rejouer S8 en conditions comparables, puis décider de l'orientation de RF-2/3 par les mesures.

**Prochaine décision d'ingénierie :** accepter ou rejeter la séparation du moteur de coordination et du compilateur Ravum, puis ratifier les contrats des premiers objets. Aucune extraction automatique en l'absence de ces gates.

---

## 11. Références sources du plan (consultées le 9 novembre 2026)

- [https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/architecture/STATUS.md](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/architecture/STATUS.md)
- [https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/mvp/MVP-0c-REPORT.md](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/mvp/MVP-0c-REPORT.md)
- [https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/mvp/MVP-0c-publication-robustness.md](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/mvp/MVP-0c-publication-robustness.md)
- [https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/mvp/MVP-0c-collaboration.md](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/mvp/MVP-0c-collaboration.md)
- [https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/mvp/MVP-0c-cross-crate-daemon.md](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/mvp/MVP-0c-cross-crate-daemon.md)
- [https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/mvp/MVP-0c-s8.md](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/mvp/MVP-0c-s8.md)
- [https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/mvp/MVP-0b-licence-analysis.md](https://github.com/Rwanbt/Ravum/blob/feat/mvp0c-hardening/docs/mvp/MVP-0b-licence-analysis.md)

Ces documents proviennent d'une **branche expérimentale non fusionnée**. Les dates, SHA et chiffres historiques y décrivent leurs expériences, pas la validation de Ravum Flow. Vérifier les PR et la source SHA à nouveau avant toute migration.

---

# ANNEXE — README initial (`README.md`, reproduction documentaire)

# Ravum Flow

> **Statut : cadrage et dépôt d'amorçage (9 novembre 2026).** Aucun moteur Ravum Flow n'est encore livré. Les API, jalons et critères de qualification ci-dessous sont des **propositions**, pas des garanties ni des résultats de tests.

**Ravum Flow** est un projet de moteur **local-first** pour le développement continu, multi-agents et multi-langages, avec publication Git/GitHub/GitLab. Il vise un **seul répertoire de code canonique**, des modifications logiquement isolées, des snapshots immuables, une vérification incrémentale et un historique durable.

Ravum Flow est autonome : on doit pouvoir l'utiliser sur un projet Rust, TypeScript, Python ou C++ **sans** installer le langage Ravum. Ravum sera son premier terrain de référence et pourra ajouter un adaptateur plus avancé (sémantique, preuves, hot-swap). Les traitements critiques ne supposent ni LLM ni connexion réseau.

## Principes

1. **Aucun worktree Git obligatoire par agent.** Les changements sont isolés logiquement (patches/overlays) plutôt que par copies complètes du dépôt.
2. **Aucune perte silencieuse.** Toute proposition porte sa version de base ; les conflits non résolus bloquent la publication.
3. **État sain préservé.** Un patch inachevé ou invalide n'écrase ni le snapshot canonique ni une version active.
4. **Vérification honnête.** Un patch applicable n'est pas nécessairement correct ; `TESTED`, `ASSUMED`, `UNKNOWN` ne sont jamais `PROVED`.
5. **Git reste interopérable.** La forge est une cible de publication, pas une dépendance d'exécution locale.
6. **Performance mesurée.** Le critère central est le temps humain + agents jusqu'à un **livrable vérifié**, comparé aux worktrees/copies privés.
7. **Sobriété et modularité.** Mono-PC 16 Go d'abord, services distribués facultatifs ensuite ; pas d'abstraction sans consommateur.

## Documentation

- [Plan d'implémentation complet]\(docs/IMPLEMENTATION_PLAN.md)
- [Architecture et invariants]\(docs/ARCHITECTURE.md)
- [Extraction et réutilisation depuis Ravum]\(docs/MIGRATION_RAVUM.md)
- [Backlog priorisé]\(docs/BACKLOG.md)
- [Protocole de benchmark et gates]\(docs/BENCHMARKS.md)
- [Décisions ouvertes]\(docs/DECISIONS.md)
- [Mandat pour agents]\(AGENTS.md)

## Point de départ réel

Des briques existent dans [Rwanbt/Ravum](https://github.com/Rwanbt/Ravum), sur la branche expérimentale `feat/mvp0c-hardening` : `ravum-workspace-coordinator`, `ravum-snapshot`, `ravum-unit-graph`, `ravum-daemon`, publication Git transactionnelle et validations. Elles **ne sont pas** déjà importées ici. La branche Ravum est en revue et ne doit pas être déplacée ni supprimée. Voir la [matrice de migration]\(docs/MIGRATION_RAVUM.md).

La campagne S8 récente rapporte une médiane de 111,1 s pour copies privées et 237,7 s pour un répertoire partagé avec garde (3 campagnes par mode, petite fixture). Il serait incorrect de prétendre que le partage d'un répertoire est déjà plus rapide ; c'est une hypothèse de produit à valider par nouveaux essais.

## Licence

**Aucune licence n'a été choisie pour ce dépôt.** Tant qu'une décision écrite et un audit des droits de réutilisation n'ont pas eu lieu, ce bootstrap documentaire ne contient **aucun code extrait de Ravum ou d'outils tiers**. La proposition `MIT OR Apache-2.0` est discutée dans [les décisions]\(docs/DECISIONS.md), sans l'adopter implicitement.

## Pour démarrer

L'ordre initial recommandé est : décision de licence et d'autorité → baseline instrumentée → première transaction isolée sur un petit projet Rust → publication Git atomique → tests de concurrence/crash → adaptateurs additionnels.

**Ce dépôt n'implémente pas encore une CLI exécutable.** Les exemples de commandes présentés dans la documentation sont des API cibles.

## Publication GitHub (bootstrap)

Le dépôt distant n'est **pas créé par cette archive**. Sur Windows, avec `git` et `gh` installés et un compte `Rwanbt` authentifié (`gh auth login`), ouvrir PowerShell dans le dossier extrait et exécuter :

```
.\bootstrap-github.ps1 -Visibility public
```

Le script refuse de recréer un dépôt déjà existant, effectue le premier commit puis publie `main` sur `Rwanbt/Ravum-Flow`. Choisir `-Visibility private` si nécessaire. Lire [GITHUB_SETUP.md]\(docs/GITHUB_SETUP.md) avant de créer les règles de branches et issues.

---

# ANNEXE — Mandat des agents initial (`AGENTS.md`, reproduction documentaire)

# AGENTS.md — Ravum Flow

Ravum Flow est un projet de R&D autonome. **Aucune fonctionnalité de code ne doit être déclarée implémentée par le seul fait qu'elle figure dans le plan.**

## Sources d'autorité

1. `README.md` et `docs/ARCHITECTURE.md` (invariants et limites).
2. `docs/DECISIONS.md` (les éléments `PENDING` ne sont pas des décisions).
3. `docs/IMPLEMENTATION_PLAN.md` et `docs/BACKLOG.md`.
4. Ticket effectivement attribué, et éventuel ADR accepté explicitement par le responsable.
5. Pour la migration seulement : dossiers/rapports réels du dépôt `Rwanbt/Ravum` ; ne pas inventer l'état des branches.

## Contrats non négociables

- Mesurer le **temps jusqu'au livrable vérifié**, et pas seulement le temps de compilation.
- Ne pas désactiver, masquer ni relâcher les assertions des tests E2E ou d'intégration pour passer au vert.
- Les résultats se rapportent au snapshot exact, à l'outil, aux flags, aux dépendances et au contexte ; toute incertitude est `UNKNOWN`/\`STALE\`, jamais `PASSED`/\`PROVED\`.
- Aucun écrasement silencieux des écritures concurrentes, aucune mutation directe d'un dépôt externe non autorisée.
- Pas de réseau requis pour le chemin local ; aucun token ni secret dans logs, artefacts ou messages inter-agents.
- Sur une machine 16 Go : compilation bornée, annulation réelle, quotas configurables, caches élagables.
- Les agents **ne contournent pas** le coordinateur pour écrire dans le snapshot canonique. Les outils non intégrés au protocole sont explicitement `UNMANAGED`.
- Ne pas copier du code Ravum dans Flow avant la décision sur licence/droits et la vérification de provenance.
- Pas de merge, tag, publication, fork tiers, acceptation d'ADR ou changement des branches protégées sans instruction explicite.

## Parallélisme

- Le responsable de campagne demeure **l'unique écrivain Git** tant que le coordinateur n'est pas qualifié.
- Chaque agent annonce son périmètre de fichiers. Les modifications de schéma/API sont revues avant travaux simultanés.
- Pour les premiers lots, utiliser des tâches disjointes et des fixtures jetables ; ne pas tester la sûreté directement sur le dépôt Ravum.
- Toute proposition concurrente rapporte `base_snapshot_id`, fichiers et symboles modifiés, préconditions et identifiant de proposition.

## Définition de terminé

Chaque tâche fournit : objectif, limite de périmètre, commits, commande et résultat des tests positifs/négatifs, source SHA, OS/toolchain, mesure des ressources si performance, risques, artefacts de preuve, régressions, et état `DONE`/\`PARTIAL\`/\`BLOCKED\`. Ne jamais appeler un résultat `PROVED` sans portée ni hypothèses formalisées.

---

# ANNEXE — Architecture initiale (`ARCHITECTURE.md`, reproduction documentaire)

# Architecture fonctionnelle proposée — Ravum Flow

**Statut : RFC, non implémenté.** Le core doit rester autonome, offline et multi-langages. Les noms de crates et interfaces sont des candidats, pas une API figée.

## 1. Organisation cible minimale

```
crates/
  flow-core/          # IDs, protocoles, politiques, réception d'événements
  flow-store/         # CAS, journal durable, snapshots, GC
  flow-coordinator/   # propositions, conflits, MVCC, validation et publication
  flow-vcs-git/       # Git atomique, reprise, hooks, Git API
  flow-engine/        # graphe d'impact, scheduler, adaptateurs et cache
apps/
  flow-cli/           # CLI/JSONL pour humains et agents
adapters/
  rust/               # premier adaptateur réel
  typescript/         # 2e adaptateur
  python/
  cpp/
  ravum/              # facultatif, quand le compilateur existe
tests/
  concurrency/
  crash-recovery/
  end-to-end/
  fixtures/
```

**Ne pas créer tous ces modules au bootstrap.** Au MVP-1, limiter le nombre de crates au strict nécessaire et créer une abstraction seulement quand deux consommateurs réels en justifient l'existence.

## 2. Flux de transaction

```
Agent/humain (base B)
  -> propose(P, base=B)
  -> store immuable : blob + patch + provenance
  -> validation des préconditions P sur B
  -> calcul candidat C = apply(B, P)
  -> analyse/test(C) dans environnement immuable
  -> receipt(C, toolchain/context)
  -> compare-and-swap état canonique B -> C
       | succès -> journal COMMITTED -> sync disque -> commit Git si demandé
       | divergence -> REBASE_REVALIDATE ou CONFLICT, jamais écrasement
  -> recover() idempotent après crash
```

**La publication logique et la publication Git sont deux étapes distinctes.** Elles doivent avoir des points de commit formels et des états de reprise définis. Un agent ne doit pas interpréter l'intégration logique comme une PR mergée.

## 3. Concurrence

| Cas                                      | Politique de base                                                                                                     |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Deux fichiers distincts                  | Fusion contrôlée si les deux bases sont compatibles, vérification sur état résultant                                  |
| Même fichier, régions distinctes         | Patches recomposés sous préconditions ; analyse obligatoire sur résultat, ne jamais présumer compatibilité sémantique |
| Même région/fonction                     | `CONFLICT`, résolution humaine ou agent après relecture/rebase explicite                                              |
| Renommage et modification                | Conflit structurel ou migration de référence vérifiée ; aucune heuristique aveugle                                    |
| Modification du lockfile ou build config | Invalidation transversale, fallback validation complète                                                               |
| Éditeur externe non géré                 | Marque `UNMANAGED`, nouveau snapshot détecté ; risque d'écrasement annoncé                                            |

Le système garantit la cohérence des **transactions gérées par lui**. Pour obtenir une garantie sur un agent qui utilise `writeFile` directement, il faut un filesystem virtuel/overlay, un proxy d'accès IDE ou un confinement OS. Un simple watcher *après* écriture ne suffit pas.

## 4. États de vérification

Ne pas confondre **verdict de fusion** (`AUTO_SAFE`, `MANUAL_REQUIRED`, `CONFLICT`, `REBASE_REVALIDATE`, `REJECT`) et **niveau de preuve**. Les mêmes niveaux s'appliquent à tous les langages avec une portée différente.

| Niveau                                | Signification                                                  | Action                                        |
| ------------------------------------- | -------------------------------------------------------------- | --------------------------------------------- |
| `TEXTUALLY_APPLICABLE`                | Le patch s'applique, aucune vérification sémantique reconnue   | Pas de publication automatique                |
| `SEMANTICALLY_CHECKED`                | Oracle déclaré et checks passés sur C exact                    | Publication sous politique projet explicite   |
| `TESTED`                              | Tests sélectionnés exécutés sur C exact                        | Publication selon policy et couverture connue |
| `PROVED`                              | Propriété bornée, modèle, hypothèses, outil, cible et evidence | Jamais généralisé hors de sa portée           |
| `STALE`                               | La source/context a évolué                                     | Revalider                                     |
| `UNKNOWN`/\`TIMEOUT\`/\`UNSUPPORTED\` | Capacité absente ou preuve incomplète                          | Fail closed selon gate                        |
| `REJECTED`                            | Conflit ou test/check requis en échec                          | Refuser                                       |

## 5. Identités et stockage

Trois identités à séparer strictement : `SourceSnapshotId` (octets des sources et chemins), `BuildContextId` (toolchain/flags/deps/platforme), `VerificationKey` (propriété ou suite, policy et environnement). Un SHA unique de `src/` ne suffit pas à qualifier un binaire.

Le CAS contient des blobs immuables et manifests. Le journal a au minimum `schema_version`, `sequence`, `intent`, `base_id`, `candidate_id`, `commit_point`, `outcome`, `checksum`. Le GC protège les snapshots actifs, les intents non résolues et les reçus nécessaires à l'audit. Les migrations sont monotones et testées sur des journaux historiques.

## 6. Adapter contract

```
{
  "schema_version": 1,
  "adapter_id": "rust-cargo",
  "capabilities": ["discover", "diagnostics", "check", "test"],
  "toolchain_fingerprint": "<digest>",
  "input_snapshot_id": "<digest>",
  "context_id": "<digest>",
  "affected_units": ["core", "api"],
  "checks": [{"id": "cargo-test", "status": "PASSED", "evidence_id": "<digest>"}],
  "limits": ["dynamic-build-scripts-fallback"]
}
```

Ce JSON illustre **un contrat candidat**, pas le résultat d'une exécution ni une preuve. Les adaptateurs n'injectent pas de code arbitraire dans le processus Flow core : exécuteurs séparés par défaut, capacités déclarées et limites CPU/RAM. Les options de déploiement doivent prévoir une ABI stable, SDK et protocole JSONL compatibles sans compiler Flow pour chaque langage.

## 7. Performance et cache

La vérité d'un résultat dépend de sa **clé complète** : entrée immuable, dépendances transitives, outil/version, flags, OS/cible, entrées externes et policy. En cas de doute : cache miss + validation conservatrice. Les annulations ne laissent jamais un verdict partiel marqué `PASSED`.

Ordonnancement : priorités interactive > vérifications rapides > tests pertinents > suite complète ; jobs coûteux soumis à sémaphore de mémoire/CPU ; heuristique de batching initialement OFF. Sur la référence 16 Go, mesurer mémoire peak et variations sous 2–3 agents ; ne pas fixer une concurrence globale sans benchmark.

## 8. Git et écosystème

Git reste le format de partage, avec writer atomique, hooks respectés et refs CAS. Une forge adapter GitHub/GitLab opère sur commits et checks, sans être requise pour le mode offline. Le même protocole permet progressivement CLI, VS Code et service distant. Aucun accès direct aux secrets du compte Git ne doit être journalisé.

## 9. Modes de défaillance à tester

Process kill à chaque phase, disque plein, permission refusée, corruption d'un blob, ref qui change entre prepare/publish, hook refusant, index sale, symlink hors root, modification externe concurrente, journal tronqué, cache falsifié, outil absent, tests E2E en échec, worker qui ne termine pas, réseau coupé, clé de token révoquée et versions incompatibles. Le résultat attendu est un état canonique connu et une erreur explicite, **pas** une réparation silencieuse.

---

# ANNEXE — Benchmark initial (`BENCHMARKS.md`, reproduction documentaire)

# Protocole S8 / stabilité / performance

**Statut :** protocole candidat. Les résultats historiques cités sont ceux du dépôt Ravum, pas des mesures de Flow. Ne changer aucun seuil après observation sans nouvelle version du protocole.

## Stratégies à comparer

- **A :** copies privées / worktrees ou fixtures isolées (préciser l'implémentation) + fusion finale.
- **B :** répertoire partagé + simple garde optimiste sur sauvegarde.
- **C :** édition séquentielle.
- **D :** Flow avec overlays de propositions et commit CAS, sans worktree Git par agent.

Ne pas comparer le protocole A « copies de fichiers » à des worktrees Git sans l'indiquer. Dans toute campagne, source et fixtures identiques, mêmes tâches et critères de sortie, même modèle IA/fournisseur/configuration, mêmes outils, mêmes ressources et même régime de cache chaud/froid. Alterner l'ordre des stratégies et collecter répétitions ≥3 pour pilote puis augmenter avant décision de production.

## Corpus minimal

1. Trois agents sur trois fichiers disjoints.
2. Deux agents modifient deux fonctions d'un même fichier.
3. Deux agents modifient **la même** fonction avec intents incompatibles.
4. Renommage + import/usage dépendant.
5. Modification d'API inter-crates sans modification des consommateurs.
6. Test E2E en échec réel ; aucun masquage.
7. Éditeur externe qui écrit sans protocole `UNMANAGED`.
8. Crash agent/coordinateur avant et après CAS, et redémarrage.
9. Hook Git qui refuse, ref avancée par une autre session, index sale.
10. Changement de lockfile, flags, toolchain et build script ; absence de fausse réutilisation de cache.
11. Multi-OS et chemins Unicode/casse/symlinks.
12. Réseau coupé pour mode offline puis pour sync distante (une fois implémentée).

## Métriques

| Mesure                          | Définition                                                                           |
| ------------------------------- | ------------------------------------------------------------------------------------ |
| Temps total (prioritaire)       | Affectation première tâche → dernier test/PR validé sur état intégré                 |
| Latence interactive             | Édition → diagnostics (p50/p95/p99)                                                  |
| Temps de compilation/validation | Par tâche et end-to-end, cold/warm distincts                                         |
| Conflits                        | Nombre, classification, retries, temps de résolution, résultat sémantique            |
| Modifications perdues           | Diff exact attendu vs final + vérification de provenance (gate strict = 0)           |
| CI/E2E                          | Tests exécutés/non exécutés et raisons, échecs visibles, temps de merge queue        |
| Mémoire/CPU/disque              | Maxima/processus, daemon idle, build peak, taille caches, IO                         |
| Coût des agents                 | Tokens **réellement** rapportés par fournisseur, tours, temps de supervision humaine |
| Stabilité                       | Crash/recovery, journal cohérent, ref Git et receipts cohérents                      |
| Confidentialité                 | Secret scan des logs, artefacts, journaux, traces des agents                         |

## Méthode

- Pré-enregistrer SHA de la fixture, du protocole, version du moteur et hypothèses ; conserver données brutes JSON et commande complète.
- Fixture **rouge avant** et verte après les corrections ; tests initialement verts seuls ne démontrent pas une correction.
- Marquer les enregistrements non attribuables à un agent et les mesures manquantes `UNKNOWN`, pas zéro.
- Aucune relance destinée à exclure arbitrairement une mauvaise performance ; publier les outliers et la charge machine.
- Les résultats d'un build ne comptent que si le snapshot et contexte exacts sont vérifiés ; rejeter les résultats `STALE`.
- Utiliser des dépôts temporaires pour les fautes destructives ; jamais le dépôt Ravum de travail comme cible de crash injection.
- Conserver un test complet périodique et les E2E affectés ; ne jamais améliorer artificiellement le temps en les retirant.

## Gate de comparaison

**Sûreté stricte :** 0 perte silencieuse, 0 commit partiel, 0 reçu faussement valide, 0 test obligatoire caché, 0 secret exposé dans les logs tests.

**Productivité :** analyser différences statistiques sur le temps total, tokens, CPU/RAM/IO et charge ; une seule campagne ne suffit pas. Si Flow est plus lent, revoir la stratégie ou proposer le fallback A. L'absence de worktrees reste un objectif ergonomique, pas une excuse pour dégrader le débit.

## Baseline externe existante dans Ravum

Le rapport `docs/mvp/MVP-0c-s8.md` de `Rwanbt/Ravum@feat/mvp0c-hardening` indique, en vrai multi-agents (27 subagents, 9 campagnes, 3 par stratégie), médianes A=111,110 s, B=237,674 s, C=528,481 s, tests 12/12 et 0 perte dans 9/9. Limites : petite fixture, copies privées au lieu de worktrees Git, garde simple au lieu de transactions Flow, charge machine variable. Ré-exécuter de façon indépendante après le MVP de Flow.

---

# ANNEXE — Registre des décisions initial (`DECISIONS.md`, reproduction documentaire)

# Décisions et limites — toutes ouvertes

> État initial au 9 novembre 2026. `PENDING` signifie **aucun choix ratifié**. Les recommandations ne doivent jamais être interprétées comme des ADR acceptés.

| Référence  | Décision à prendre                      | Option préférée pour étude                                                                | Statut  |
| ---------- | --------------------------------------- | ----------------------------------------------------------------------------------------- | ------- |
| RF-RFC-001 | Nom final du dépôt et des binaires      | Ravum Flow / `ravum-flow`                                                                 | PENDING |
| RF-RFC-002 | Licence du code et droits de migration  | Étudier `MIT OR Apache-2.0` et les droits de chaque contributeur                          | PENDING |
| RF-RFC-003 | Source de vérité                        | Git comme backend de publication ; journal Flow local pour propositions                   | PENDING |
| RF-RFC-004 | Mode par défaut des outils non intégrés | Lecture et diagnostic, ou isolation par copie temporaire ; jamais « sûr » par présomption | PENDING |
| RF-RFC-005 | Isolation et granularité de conflit     | MVCC optimiste par fichier/hunk d'abord ; symboles plus tard                              | PENDING |
| RF-RFC-006 | Format de protocole                     | API Rust interne + JSON Lines versionné sur stdio ; MCP comme adaptateur éventuel         | PENDING |
| RF-RFC-007 | Stockage local durable                  | CAS (hash de contenu) + journal append-only transactionnel                                | PENDING |
| RF-RFC-008 | Système de build/CI initial             | Cargo et tests Rust natifs, outils polyglottes via adaptateurs                            | PENDING |
| RF-RFC-009 | Packaging Windows/Linux/macOS           | CLI Rust sans service obligatoire, daemon facultatif                                      | PENDING |
| RF-RFC-010 | Politique des preuves                   | Niveau de validation versionné, `PROVED` borné et sourcé exclusivement                    | PENDING |
| RF-RFC-011 | Synchronisation distante                | Hors MVP ; TLS/auth/RBAC avant réseau partagé                                             | PENDING |
| RF-RFC-012 | Édition intégrée de l'IDE               | CLI/JSON d'abord ; extension VS Code/JetBrains ensuite                                    | PENDING |

## Questions préalables au transfert de code

- Ravum n'a pas de licence publique choisie ; sa documentation de licence indique que le responsable doit ratifier la décision.
- Les bibliothèques tierces et contributions non entièrement détenues par l'auteur ne peuvent pas être re-licenciées par simple copie.
- Examiner historique des fichiers, auteurs, licences des dépendances, signatures et conditions des forks ; enregistrer la provenance (commit SHA et chemin original) dans `docs/MIGRATION_RAVUM.md`.
- Aucun renommage massif ni extraction avant baseline reproductible et tests du code d'origine.

## Statuts possibles d'une décision

`PROPOSED`, `ACCEPTED` (avec date et auteur), `REJECTED`, `SUPERSEDED`. Ce document demeure un registre de propositions jusqu'à approbation humaine.

---

# ANNEXE — Migration Ravum → Flow initiale (`MIGRATION_RAVUM.md`, reproduction documentaire)

# Migration contrôlée des briques Ravum → Ravum Flow

**Source observée :** `Rwanbt/Ravum`, branche `feat/mvp0c-hardening`, documentation consultée le 9 novembre 2026. Branche de travail expérimentale empilée ; les PR d'intégration et la licence restent en attente. Le présent dépôt ne contient aucune copie de ce code.

| Source Ravum                         | Utilité actuelle                                         | Travail dans Flow                                                                   | Décision proposée                |
| ------------------------------------ | -------------------------------------------------------- | ----------------------------------------------------------------------------------- | -------------------------------- |
| `crates/ravum-protocol`              | Types de diagnostics, identités, résultats               | Conserver les contrats utiles, enlever les types spécifiques au compilateur         | ADAPTER/EXTRACT après inventaire |
| `crates/ravum-snapshot`              | Snapshots immuables et overlays par agent                | Candidat direct pour `flow-snapshot`, avec chemins et contextes génériques          | EXTRACT sous gate licence/tests  |
| `crates/ravum-workspace-coordinator` | Propositions, vérification, journal, publication Git CAS | Noyau de coordination `flow-coordinator`; séparer l'adaptateur Rust                 | EXTRACT sous gate licence/tests  |
| `crates/ravum-unit-graph`            | Dépendances inter-crates via Cargo                       | Garder comme **adaptateur Rust**, pas comme graphe universel tel quel               | ADAPT                            |
| `crates/ravum-daemon`                | Analyse rustc, snapshots, externs et cache               | Réutiliser les concepts d'ordonnancement/CAS ; isoler le pilote rustc               | SPLIT/ADAPT                      |
| `crates/ravum-batch`                 | Groupement d'édits à rentabilité conditionnelle          | Tester au niveau Flow seulement si pré-vol sémantique fiable                        | EVALUATE; OFF par défaut         |
| `crates/ravum-collab-oracle`         | Oracle d'interface Rust                                  | Plugin `adapter-rust`; ne pas élever un verdict textuel à preuve                    | ADAPT                            |
| `crates/ravum-cli`                   | CLI Ravum et démonstrations                              | Garder dans Ravum ; créer un CLI Flow indépendant                                   | KEEP + ADAPTER                   |
| `crates/ravum-patch-host`            | Hot swap natif et cycle RT                               | Laisser dans Ravum, recevoir événements validés de Flow                             | KEEP                             |
| `crates/ravum-dsp-harness`           | Qualification DSP                                        | Laisser dans Ravum, réutiliser harnais de test comme intégration                    | KEEP                             |
| `crates/ravum-proof`                 | Ledger/proofs bornées                                    | Garder la logique de preuves côté langage ; définir un protocole générique de reçus | KEEP + INTERFACE                 |
| `crates/ravum-rust-adapter`          | Diagnostics rustc et vérification                        | Candidat adaptateur de langage Rust, non dépendance du core                         | ADAPT                            |

## Séquence de migration, sans casser Ravum

1. **Audit légal/technique :** licence, propriétaires, DAG des dépendances, API publiques, comportements, branch HEAD, fixtures et tests négatifs. Geler la baseline des deux projets.
2. **Démonstration indépendante :** implémenter un petit coordinateur Flow en façade autour de modules extractibles ou d'un prototype original ; conserver toutes les sources dans Ravum.
3. **Contrats stables :** définir dans Flow des schémas versionnés pour `SourceSnapshot`, `ChangeProposal`, `MergeDecision`, `VerificationReceipt`, `PublicationReceipt`; convertir au bord Ravum avec un adaptateur.
4. **Extraction minimale :** migrer un composant à la fois avec provenance (chemin/SHA), tests originaux et fixtures portées. Éviter les copies durables divergentes : choisir une seule implémentation canonique après double validation.
5. **Intégration réversible :** Ravum utilise une version épinglée de Flow (Git/tag ou crate vendored conforme à la licence), tests de non-régression, possibilité de revenir au composant historique si une version Flow échoue.
6. **Suppression des doublons :** seulement après plusieurs cycles CI verts, aucun consommateur caché et décision humaine ; conserver les rapports et l'historique.

## Points de vigilance tirés du MVP-0c

- La publication transactionnelle existante a subi des injections de crash (hooks Git, CAS, réconciliation) ; ne pas la remplacer par un simple `git add && git commit`.
- Le protocole v2 refuse `AUTO_SAFE` sur la seule applicabilité textuelle ; ne pas réintroduire l'ancien comportement v1 comme défaut de Flow.
- Les `.rmeta` Rust dépendaient des chemins absolus : une clé de cache universelle doit intégrer la canonisation et documenter les limites de portabilité.
- Le groupement de patches n'était pas rentable dans une fixture conflictuelle : conserver l'option désactivée jusqu'à preuves positives.
- La campagne S8 avec agents réels la plus récente rapporte A copies privées < B partage < C séquentiel ; il faut une nouvelle campagne après implémentation du coordinateur transactionnel, pas généraliser depuis les outils d'édition simulés.
- Certains documents de statut plus anciens contiennent des éléments `reste à faire` devenus obsolètes (ex. campagne S8). Lire la dernière preuve et son SHA avant tout constat de progression.

---

# ANNEXE — Backlog initial (`BACKLOG.md`, reproduction documentaire)

# Backlog initial — exécutable par issues, non créé automatiquement

Les identifiants `RF-xxx` sont **locaux à ce plan** : aucune issue GitHub n'a encore été ouverte. Chaque tâche exige tests positifs et négatifs, provenance, et gate mesurable. Ordre conditionné par les dépendances, pas des dates.

| ID     | Priorité | Tâche / livrable                                                    | Dépendances    | Gate principal                              |
| ------ | -------- | ------------------------------------------------------------------- | -------------- | ------------------------------------------- |
| RF-001 | P0       | Décision de nom, visibilité et licence ; vérification droits auteur | —              | Décisions humaines consignées               |
| RF-002 | P0       | Inventaire API/dépendances/tests des composants Ravum transférables | RF-001         | Matrice + SHA + provenance                  |
| RF-003 | P0       | Benchmark S8 pré-enregistré : A/B/C, fixtures et charge             | —              | Baselines reproductibles, métriques brutes  |
| RF-004 | P0       | Protocole `SourceSnapshot` + `ChangeProposal` versionné             | RF-001         | Compat tests/cas négatifs                   |
| RF-005 | P0       | CAS local, journal durable et GC borné                              | RF-004         | Corruption/troncature détectées             |
| RF-006 | P0       | CLI `snapshot/status/doctor`, hors ligne                            | RF-004/005     | Pas de modification des sources utilisateur |
| RF-007 | P0       | Proposition patch sur base immuable + CAS                           | RF-005         | 0 écrasement silencieux                     |
| RF-008 | P0       | Conflits rename/hunk/same-symbol, rebase et undo                    | RF-007         | Corpus adversarial complet                  |
| RF-009 | P0       | Recovery crash journal/MVCC + chaos tests                           | RF-007         | Reprise idempotente                         |
| RF-010 | P0       | Writer Git transactionnel et hooks                                  | RF-007, RF-002 | CAS ref + crash tests multi-OS              |
| RF-011 | P0       | Protocole d'évaluation v2 (texte ≠ sémantique)                      | RF-004/007     | Pas de `AUTO_SAFE` textuel                  |
| RF-012 | P0       | Adapter Rust : `cargo metadata`, tests et diagnostics               | RF-011, RF-002 | Propagation inter-crates réelle             |
| RF-013 | P0       | Scheduler, annulation, budgets, coalescence OFF                     | RF-005/011     | Résultats stale jamais acceptés             |
| RF-014 | P0       | Cache et identity context complète                                  | RF-012/013     | Tests cache poison et changements externs   |
| RF-015 | P0       | CI conservatrice impact + E2E et merge suite                        | RF-012/014     | Tous les échecs pertinents visibles         |
| RF-016 | P1       | Protocol JSONL stable / SDK agents                                  | RF-006/011     | 3 agents indépendants                       |
| RF-017 | P1       | UI revue des patches et politique de publication                    | RF-016         | Décision humaine traçable                   |
| RF-018 | P1       | Adapter TypeScript/JavaScript                                       | RF-013/014     | TS rouge→vert + lockfile invalidation       |
| RF-019 | P1       | Adapter Python                                                      | RF-013/014     | Imports dynamiques fallback conservateur    |
| RF-020 | P1       | Adapter C/C++                                                       | RF-013/014     | Headers/options/ABI invalidations           |
| RF-021 | P1       | CI et publication GitHub/GitLab                                     | RF-010/015     | PR et MR avec reçu exact                    |
| RF-022 | P1       | Benchmark S8 D (Flow) vs alternatives avec vrais agents             | RF-016/015     | Preuves n≥3, 0 perte, coûts et variance     |
| RF-023 | P2       | Overlay VFS sécurisé / intégrations IDE                             | RF-007/016     | Tests OS et confinement                     |
| RF-024 | P2       | Sync multi-machines/auth/RBAC/leases                                | RF-010/016     | Aucune divergence sous partitions           |
| RF-025 | P2       | CAS distant, remote executors et mono-repo géant                    | RF-024         | Isolation et performances mesurées          |
| RF-026 | P2       | Adapter natif Ravum et PatchHost interface                          | RF-013/014     | Réversible ; fail-closed et RT              |

## Lots multi-agents recommandés

**Phase initiale :** un agent `protocol/storage` sur `docs/ARCHITECTURE.md` puis prototypes, un agent `bench/fixtures` sur `tests/fixtures` et mesure, un agent `migration-audit` **lecture seule** sur Ravum. Ne pas faire plusieurs agents réécrire `Cargo.toml`, le journal ou les primitives CAS en même temps avant qualification RF-007/009.

**Après RF-7 :** agents par adaptateur (Rust, TS, Python) sur périmètres disjoints, lead unique pour l'intégration. Tous les résultats d'agents doivent porter SHA, test log, cause des échecs et scope de fichiers. L'orchestrateur ne doit pas déclarer `DONE` en l'absence d'un gate passé.

## Format de ticket conseillé

- **Objectif mesurable :** résultat observable et source d'autorité.
- **Fichiers autorisés / interdits :** ownership et restrictions.
- **Contrat/API :** schéma et versions impactés, interactions amont.
- **Tests :** positif, négatif, crash/concurrence si pertinent, E2E impactés.
- **Gate :** commande(s), machine, seuils, résultats attendus et risques.
- **Restitution :** commit, diff, SHA, logs/evidence, statut `DONE/PARTIAL/BLOCKED`.

---

# ANNEXE — Mise en place GitHub initiale (`GITHUB_SETUP.md`, reproduction documentaire)

# Publication et configuration initiale GitHub

**État :** instructions à exécuter par le propriétaire ; aucun dépôt GitHub n'a été créé lors de la préparation de ce package.

## 1. Prérequis

- `git` et [GitHub CLI](https://cli.github.com/) installés sur le PC.
- Authentification explicite : `gh auth login`, puis `gh auth status`.
- Droits de création de dépôt sur le compte `Rwanbt` et nom `Ravum-Flow` libre dans ce compte.
- Décision de visibilité : public ou privé. Le nom et la visibilité restent modifiables selon les capacités de GitHub.

## 2. Publier

Décompresser l'archive dans un dossier dédié, ouvrir PowerShell 5.1+ dans `Ravum-Flow/` puis :

```
.\bootstrap-github.ps1 -Visibility public
```

Le script vérifie l'authentification, refuse les dépôts déjà existants, initialise Git (branche `main`), crée un commit initial et appelle `gh repo create ... --push`. Il ne copie aucun code de Ravum. S'il échoue, ne pas réexécuter aveuglément : examiner `git status`, `git remote -v` et `gh repo view Rwanbt/Ravum-Flow`. Ne jamais utiliser `--force` pour réparer.

## 3. Paramétrage après création (actions manuelles à valider)

1. Vérifier que README, docs, `AGENTS.md` et `.gitignore` s'affichent et que la branche par défaut est `main`.
2. Créer `dev` depuis `main`, puis règles de protection de `main` et `dev` (PR obligatoires, force-push et suppression interdits). La règle CI obligatoire ne doit être activée qu'une fois un workflow exécutable ajouté ; ne pas bloquer le dépôt sur un check inexistant.
3. Décider de la licence et de la politique de contributions **avant** toute importation de code d'autres dépôts.
4. Créer les issues RF-001 à RF-003 depuis `docs/BACKLOG.md`, puis les autres au fur et à mesure ; les identifiants RF sont indépendants des numéros d'issues GitHub.
5. Définir une CI documentaire minimale, puis les gates Rust dès qu'il existe un premier crate réel. Ne pas prétendre que la CI valide une implémentation absente.
6. Une fois le moteur local qualifié, tester une intégration GitLab sur un dépôt de test, sans changer la source de vérité Git.

## 4. Référence

- Dépôt cible : `https://github.com/Rwanbt/Ravum-Flow` (**URL prévue, à ne pas présenter comme accessible avant publication**).
- Dépôt expérimental source : [https://github.com/Rwanbt/Ravum](https://github.com/Rwanbt/Ravum)

---

# ANNEXE Z — TEXTES COMPLETS DES 42 ADR ORIGINAUX DE RAVUM

Les 42 textes ci-dessous sont intégrés dans ce même document afin que la revue puisse être conduite sans autre pièce jointe. Ils restent des **propositions non ratifiées**. Les liens de provenance dans le registre plus haut demeurent utiles pour contrôler les SHA et l'évolution des branches ; le contenu du présent document est un snapshot documentaire et ne doit pas être supposé refléter des changements intervenus après le 9 novembre 2026.

**Important :** la source de vérité normative reste le dépôt original si un écart de transcription est découvert. Tout écart doit être signalé, jamais corrigé silencieusement.

\<!-- Texte ADR-000 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-000 — Périmètre universel et garanties conditionnelles

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : Aucune.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Universel ne peut signifier simultanément prouvable sans hypothèse, sans runtime et optimisé pour tout matériel.

## Décision proposée

Un cœur sémantique commun et des profils cibles optionnels ; toute garantie est liée à un profil, un modèle, une version et un périmètre de confiance.

## Alternatives à comparer / écarter

Langage/runtimes distincts par domaine ; VM universelle obligatoire ; promesse générale sans exclusions.

## Conséquences et compromis

- Cohérence et sincérité des garanties. - Documentation des hypothèses et compatibilités plus exigeante.

## Preuves à produire avant acceptation

Matrice de cibles et contrats de domaine ; deux programmes (audio no_std et serveur+web) passent les critères de leur profil.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Un profil cible démontre qu’il ne peut respecter les propriétés du noyau commun.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-001 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-001 — Implémentation initiale du compilateur en Rust

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-000.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Le compilateur est un composant critique, exposé à du code adversarial, et doit rester maintenable.

## Décision proposée

Implémenter le frontend, service, runtime de développement et outils en Rust sûr, avec `unsafe` exceptionnel, audité et instrumenté.

## Alternatives à comparer / écarter

C++ ; OCaml ; Zig ; code compiler généré majoritairement par IA sans revue.

## Conséquences et compromis

- Écosystème, sûreté et portabilité. - Compilation du compilateur parfois lente ; borrows complexes.

## Preuves à produire avant acceptation

Parser fuzzé avec corpus hostile, Miri sur noyau, profiling pic RAM/CPU, cross-platform CI.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Rust constitue un blocage mesuré de productivité/performance impossible à contourner.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-002 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-002 — Modèle mémoire : ownership, emprunts et régions

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-001.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Sans GC obligatoire, sûreté, faible latence et `no_std` imposent contrôle précis des durées de vie.

## Décision proposée

Valeurs possédées, emprunts exclusifs/partagés, restrictions d’aliasing, régions/arènes optionnelles ; API explicite pour références partagées.

## Alternatives à comparer / écarter

GC tracing obligatoire ; simple refcount sur toutes les valeurs ; gestion manuelle C non vérifiée.

## Conséquences et compromis

- Sécurité et déterminisme de ressources. - Ergonomie et inférence potentiellement difficiles.

## Preuves à produire avant acceptation

Test compile-fail exhaustif des emprunts invalides, tests d’alias et génération différentielle vers code Rust sûr.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Une famille d’applications valides est impossible à modéliser sans `unsafe` disproportionné.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-003 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-003 — Erreurs, panic et défaillance à l’exécution

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-002.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Le langage doit minimiser les crashs sans confondre échec métier, exception et défaillance du processus.

## Décision proposée

`Result`/\`Option\` et propagation explicite, pas d’exception cachée dans le noyau sûr ; `panic` interdit dans les profils strict/realtime, gestion des défaillances externes explicite.

## Alternatives à comparer / écarter

Exceptions non vérifiées ; abort silencieux ; conversions implicites en valeurs par défaut.

## Conséquences et compromis

- Chemins d’erreur audités et contractuels. - Signatures parfois plus verbeuses.

## Preuves à produire avant acceptation

Cas négatifs overflow/index/null/IO/timeout ; vérification que strict/realtime n’introduisent aucun chemin de panic supporté.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Le coût de gestion explicite annule la fluidité sans améliorer le taux de défauts.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-004 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-004 — Contrats, raffinements et invariants

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-003.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

La sûreté mémoire n’établit pas la conformité au métier ni les bornes de ressources.

## Décision proposée

`requires/ensures/invariant` et types raffinés limités en premier ; preuves séparées du code exécutable, sans effacement des contrôles runtime tant que non prouvés.

## Alternatives à comparer / écarter

Tests uniquement ; types dépendants intégraux immédiatement ; annotations détachées du code.

## Conséquences et compromis

- Spécification vérifiable proche du code. - Complexité de langage/solveur et risque de vacuité.

## Preuves à produire avant acceptation

Cas de test avec préconditions impossibles, spécifications incomplètes, contre-exemples et mutants pour éviter preuves vacues.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Les preuves se révèlent majoritairement inexploitables ou trop coûteuses.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-005 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-005 — Système d’effets et capacités

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-002, ADR-004.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Sécuriser frontend/backend/secrets/audio suppose connaître les accès externes et les opérations dangereuses.

## Décision proposée

Inférer les effets internes, déclarer les capacités externes et le niveau d’autorité ; l’appel ne peut accroître des permissions implicites.

## Alternatives à comparer / écarter

Conventions de nommage ; permissions globales ; sandbox sans analyse de code.

## Conséquences et compromis

- Principe du moindre privilège et règles realtime réutilisées. - Annotation FFI, polymorphisme d’effets.

## Preuves à produire avant acceptation

Tests compile-fail sur fuite de secret vers frontend, effect escalation, appel bloquant indirect, chemins dynamiques.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Précision d’inférence insuffisante sur code générique ou polymorphe.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-006 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-006 — Classes temps réel et certification de délai

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-005.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Les besoins d’un audio callback et d’un contrôleur certifié ne sont pas les mêmes.

## Décision proposée

Trois classes : `rt_safe` (aucun effet interdit), `rt_bounded` (ressources/itérations bornées), `hard_rt` (analyse WCET + plateforme/configuration certifiée).

## Alternatives à comparer / écarter

Annotation `realtime` binaire unique ; imposer WCET sur tous les OS ; écarter le temps réel.

## Conséquences et compromis

- Garanties honnêtes et montée progressive. - Complexité de profils et dépendances matérielles.

## Preuves à produire avant acceptation

Callbacks audio instrumentés ; rejets d’alloc/lock même transitifs ; rapport des hypothèses WCET et du scheduler.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Confusion persistante du public entre RT-safe et délais garantis.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-007 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-007 — Arithmétique : entiers, flottants et fast-math

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-003, ADR-004.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

L’optimisation change parfois les résultats flottants ; les débordements peuvent provoquer des erreurs.

## Décision proposée

Entiers checked par défaut pour opérations à risque, wrapping/saturating explicites ; IEEE défini en mode strict, tolérance bornée explicite en mode approximation ; fast-math opt-in contractuel.

## Alternatives à comparer / écarter

Overflow implicite ; optimisations flottantes agressives par défaut ; interdiction totale SIMD.

## Conséquences et compromis

- Reproductibilité et contrats auditables. - Certaines optimisations exigent dérogation.

## Preuves à produire avant acceptation

Suites IEEE NaN/Inf/-0, différentiels multi-arch, contrats d’erreur en ULP/absolu/relatif, tests de profils SIMD.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Des benchmarks démontrent un coût prohibitif avec gains de sûreté marginaux.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-008 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-008 — Concurrence, synchronisation et annulation

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-002, ADR-005.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Les serveurs et moteurs média ont besoin de tâches concurrentes sans courses mémoire et de charge maîtrisée.

## Décision proposée

Tâches structurées, canaux typés bornés, possession des messages, annulation explicite, backpressure ; primitives atomiques de bas niveau auditées.

## Alternatives à comparer / écarter

Fibres non structurées ; verrous implicites omniprésents ; actor model exclusif.

## Conséquences et compromis

- Traçabilité des durées de vie et scénarios simulables. - Adaptateurs aux runtimes existants.

## Preuves à produire avant acceptation

Tests Loom/Shuttle et simulateur, leak/cancel storms, timeouts, canaux saturés, deadlock detection partielle correctement signalée.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Incompatibilité majeure avec environnements async réels.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-009 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-009 — Frontières unsafe, FFI et code de confiance

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-001, ADR-004.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Universalité systèmes/pilotes/GPU implique interopérabilité avec des ABI externes.

## Décision proposée

`unsafe` isolé dans modules spécialisés avec justification, invariants publics, lints, revue obligatoire ; capacités et manifestes de confiance explicites.

## Alternatives à comparer / écarter

Bannir `unsafe` ; autoriser `unsafe` arbitraire ; garantir tacitement que les FFI sont sûres.

## Conséquences et compromis

- Compatibilité système sans vider les promesses de sûreté. - Coût d’audit et maintenance des wrappers.

## Preuves à produire avant acceptation

Inventaire machine de tous les sites, tests Miri/sanitizers/fuzzing, adversarial ABI/mémoire et revues externes.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Le périmètre de confiance devient trop vaste pour être auditable.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-010 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-010 — Frontend et IR sémantique multi-niveaux

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-002, ADR-003, ADR-004.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Syntaxe incomplète, ownership, preuves et backends ne doivent pas être forcés dans une IR monolithique.

## Décision proposée

CST lossless → HIR typée → MIR ownership/effets → IR de vérification et IR de génération liées par contrats de raffinement.

## Alternatives à comparer / écarter

AST unique mutable ; LLVM IR pour toutes les étapes ; compilateur par regexp/macros.

## Conséquences et compromis

- Meilleure séparation sémantique/optimisation. - Complexité de mapping diagnostic et vérification de traduction.

## Preuves à produire avant acceptation

Tests de préservation de sémantique, round-trip parsing, source maps précis et differential testing avant/après lowering.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

La duplication des IR nuit fortement aux builds et au contrôle des preuves.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-011 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-011 — Syntaxe progressive et parsing d’édition

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-010.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Une IA et un éditeur produisent des fragments temporairement invalides.

## Décision proposée

Parser tolérant, arbres partiels et overlays non sauvegardés ; jamais générer du code exécutable issu d’un AST invalide.

## Alternatives à comparer / écarter

Parser batch strict uniquement ; exécuter des fragments au fil des tokens ; parser via outil externe opaque.

## Conséquences et compromis

- Diagnostics pendant frappe et stabilité de preview. - Sémantique des nœuds de récupération à formaliser.

## Preuves à produire avant acceptation

Fuzz éditions à chaque position, interruptions d’écriture, tests de synchronisation source/buffer et résilience parser.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Dérives permanentes AST/source ou complexité mémoire disproportionnée.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-012 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-012 — Compilation incrémentale à requêtes

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-010, ADR-011.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Recompiler depuis zéro après chaque token est incompatible avec fluidité et budgets CPU/RAM.

## Décision proposée

Graphe de requêtes dépendantes façon Salsa ; granularité fonction/type/contrat et invalidation consciente des interfaces publiques.

## Alternatives à comparer / écarter

Build intégral à chaque sauvegarde ; suivi de timestamp fichiers seulement ; cache manuel sans dépendances.

## Conséquences et compromis

- Réutilisation fine. - Bogues d’invalidation particulièrement dangereux.

## Preuves à produire avant acceptation

Oracle froid vs chaud bit-à-bit/semantic parity, invalidations adversariales, mutation de types et dépendances transversales, benchmarks RAM.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Le système ne se stabilise pas ou se révèle plus coûteux que des rebuilds simples.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-013 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-013 — Cache de preuves et identité de contexte

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-012.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Les preuves conservées doivent dépendre de toutes les hypothèses pertinentes, pas seulement du texte source.

## Décision proposée

Empreinte content-addressed sur IR canonique, contrats, axiomes, solveur, options, modèles, FFI, cibles, version compilateur et dépendances ; cache invalidé par défaut en cas d’incertitude.

## Alternatives à comparer / écarter

Cache par chemin/n° ligne ; cache partagé sans version de solveur ; pas de cache de preuves.

## Conséquences et compromis

- Preuves rapides à chaud sans régression sournoise. - Stockage, canonicalisation et coûts de hash.

## Preuves à produire avant acceptation

Mutation testing du cache : changer chaque composant du contexte et vérifier invalidation ; comparer preuves fresh/cached.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Une seule fausse réutilisation confirmée : blocage immédiat du cache de preuve.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-014 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-014 — Protocole de preuve, statuts et confiance

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-004, ADR-013.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

UNSAT solver, test qui passe, compilation et certificat de preuve ne portent pas la même certitude.

## Décision proposée

Status `PROVED/DISPROVED/UNKNOWN/TIMEOUT/UNSUPPORTED/ASSUMED/TESTED` ; chaque résultat mentionne formule, portée, hypothèses, moteur, version, budget, certificat ou base de confiance.

## Alternatives à comparer / écarter

Booléen pass/fail unique ; `UNKNOWN` assimilé au succès ; certificat inventé par un agent.

## Conséquences et compromis

- Audit honnête et CI stricte. - UX plus riche, qualification des moteurs nécessaire.

## Preuves à produire avant acceptation

Injecter timeouts, modèle partiel, invariants faux, précondition contradictoire, solver crash ; vérifier qu’aucun n’est marqué PROVED.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Un moteur de preuve fiable et plus simple devient disponible.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-015 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-015 — Solveurs et moteurs de vérification

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-010, ADR-014.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Les propriétés diffèrent : logique SMT, exploration bornée, preuves manuelles et concurrence.

## Décision proposée

Abstraction de backends (Z3/cvc5 + Kani/Verus/Why3 en spikes) ; IR de vérification propre, adaptateurs avec capacités déclarées ; pas de dépendance à un outil unique.

## Alternatives à comparer / écarter

Implémenter un solveur maison ; se lier exclusivement à Kani ; admettre les preuves sans moteur indépendant.

## Conséquences et compromis

- Flexibilité et comparaisons. - Risque de sémantiques divergentes entre adaptateurs.

## Preuves à produire avant acceptation

Mêmes propriétés sur au moins deux chemins quand possible ; corpus de faux positifs/négatifs et budget mémoire/temps.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Adaptateurs structurellement incapables de préserver le sens du langage.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-016 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-016 — Model checking, fuzzing et simulation déterministe

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-008, ADR-015.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Tests unitaires classiques échouent souvent à explorer concurrence et fautes.

## Décision proposée

Fixture runtime virtualisable (horloge, random, scheduling, réseau, stockage), seed et replay versionnés ; fuzz/property-based et model checking ciblés.

## Alternatives à comparer / écarter

Tests système non reproductibles seulement ; prétendre que le simulateur couvre tout ; RNG non enregistré.

## Conséquences et compromis

- Reproduction et triage automatisé. - Interfaces environnementales et couverture partielle.

## Preuves à produire avant acceptation

100 replays identiques sur système figé, fault matrix, crash traces identiques ; marquer les appels non virtualisés comme hors modèle.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Test réel et simulé divergent systématiquement sans explication.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-017 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-017 — Codegen initial : Rust sûr comme backend de confiance limité

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-010, ADR-012.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

La création immédiate d’un backend machine et d’un borrow checker complet multiplie les risques de sémantique.

## Décision proposée

Produire du Rust sûr dans la première chaîne exécutable ; `rustc` est un validateur secondaire ; effectuer vérifications sémantiques propres ; backend direct différé.

## Alternatives à comparer / écarter

Direct LLVM dès le premier jour ; seulement transpilateur sans IR ; compiler via C non typé.

## Conséquences et compromis

- Livraison expérimentale rapide, outillage mature. - Overhead de transpilation et double-compilation possible.

## Preuves à produire avant acceptation

Différentiels oracle entre évaluateur de référence, Rust généré et règles du langage ; mesurer builds froid/chaud.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Le pont Rust impose un coût structurel contraire aux SLA malgré cache, ou bloque des caractéristiques nécessaires.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-018 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-018 — Backends natifs : Cranelift et LLVM

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-017.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Souhait simultané de compilation dev rapide et optimisation release maximale.

## Décision proposée

Comparer Rust+rustc, Cranelift direct et LLVM sur kernels et projets ; backend direct uniquement si preuve de justesse/validation de traduction et gain mesuré.

## Alternatives à comparer / écarter

Tout LLVM dès le départ ; tout Cranelift même en release ; compilation JIT obligatoire.

## Conséquences et compromis

- Choix par mesures, pas slogans. - Maintenance de plusieurs codegens coûteuse.

## Preuves à produire avant acceptation

Bench compilation/mémoire/binaire/latence énergie + suite differential pour chaque transformation retenue.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Une chaîne unique atteint tous objectifs avec moins de maintenance.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-019 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-019 — Daemon permanent, isolation et budgets

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-012.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Une compilation permanente peut saturer le PC et affecter les IDE ou les agents.

## Décision proposée

Daemon par workspace/scope avec limites RAM/CPU/threads, annulation, priorité interaction, cache LRU borné et mode CLI batch sans service résidant.

## Alternatives à comparer / écarter

Daemon système global non isolé ; compilateur uniquement batch ; worker non interruptible.

## Conséquences et compromis

- Réactivité maîtrisée, ressources prévisibles. - Priorités et gestion de concurrence complexe.

## Preuves à produire avant acceptation

Stress multi-projets, kill/restart, mémoire forcée, restauration cache, mesure CPU idle et de latence p95.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Le daemon ajoute plus de coût que de gain sur travaux fréquents.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-020 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-020 — Compilations à la frappe et source incomplète

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-011, ADR-019.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Les préviews doivent suivre les buffers d’éditeur et pas seulement les fichiers sauvegardés.

## Décision proposée

Protocoles overlays versionnés ; syntaxe invalide analysable mais jamais activée ; compilation dès unités closes/valides, debouncing adaptatif et annulation du travail obsolète.

## Alternatives à comparer / écarter

Exécuter code partiel ; n’activer que sauvegarde disque ; interpréteur séparé sans sémantique commune.

## Conséquences et compromis

- Preview réellement continue et sûre. - Granularité de compilation et reconciling des offsets.

## Preuves à produire avant acceptation

Éditions concurrentes human+AI, commandes out-of-order, versions périmées, tests non activation d’AST invalide.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Overlays provoquent désynchronisation fréquente et bugs de compilation.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-021 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-021 — Hot swap atomique et versionnement du code

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-019, ADR-020.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Le code actif ne peut être remplacé en retirant des fonctions encore utilisées.

## Décision proposée

Points d’entrée indirects versionnés, activation atomique aux safepoints admissibles, conservation des anciens segments jusqu’à quiescence (epochs/refcount adapté), échec fermé.

## Alternatives à comparer / écarter

Patcher octets arbitraires en place ; reconstruire tout l’exécutable pour chaque modification ; supprimer immédiatement l’ancienne version.

## Conséquences et compromis

- Évolution active sans redémarrer. - Overhead indirection, collecte des versions, limites ABI.

## Preuves à produire avant acceptation

Stress multi-thread, appels in-flight, partage état, échecs patch, tests ASan/TSan/Miri où applicables.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Une classe fréquente de patchs valide ne peut préserver la sécurité ; opter pour redémarrage/sandbox.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-022 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-022 — Migration d’état, rollback et effets irréversibles

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-021.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Un changement de layout ou de données peut invalider les pointeurs et invariants existants.

## Décision proposée

Hot swap sans migration uniquement si contrats ABI/layout/état compatibles ; sinon migration typée préparée hors chemin critique, validation, commit/rollback. Effects externes ont compensation séparée, jamais rollback fictif.

## Alternatives à comparer / écarter

Réinterprétation de mémoire ; migration automatique supposée ; rollback de tout effet réseau/DB.

## Conséquences et compromis

- Gestion réaliste de l’état. - Tests de migrations, mémoire temporaire nécessaire.

## Preuves à produire avant acceptation

Chaînes upgrade/downgrade, migration échouée en milieu de parcours, idempotence et compatibilité d’interface.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Le coût de migration domine ; privilégier versions de process et redémarrage contrôlé.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-023 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-023 — Runtime minimal et profils sans runtime

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-000, ADR-002.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Binaire serveur, navigateur et microcontrôleur ne doivent pas partager une grosse VM obligatoire.

## Décision proposée

Runtime en couches opt-in : core no_std, alloc, async IO, patch-dev, OS, WASI ; fonctionnalités de développement exclues des releases sauf décision contraire.

## Alternatives à comparer / écarter

Runtime universel toujours lié ; VM obligatoire ; comportement caché par target.

## Conséquences et compromis

- Petites tailles et embarqué. - Matrice de compatibilité plus large.

## Preuves à produire avant acceptation

Build `no_std` minimal, tailles de binaire, hooks d’allocation, tests absence de daemon en prod.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Le coût de multiplicité des runtimes excède les gains.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-024 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-024 — Thread audio et bascule DSP aux frontières de bloc

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-006, ADR-021, ADR-022.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Un thread audio ne peut compiler, allouer, verrouiller ou attendre une migration.

## Décision proposée

Préparer patch, buffers et état sur thread non RT ; frontière de bloc avec swap non bloquant, anciennes ressources retirées ailleurs ; crossfade optionnel borné.

## Alternatives à comparer / écarter

Compilation dans callback ; mutex bloquant ; migration heap sur thread audio.

## Conséquences et compromis

- Latence audio prévisible. - Coût de double état et découplage complexe.

## Preuves à produire avant acceptation

Callback instrumenté, compteurs allocation/locks, stress hot swap, xruns sur device réel, analyse worst-case.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Décrochages persistants dus au mécanisme même de swap.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-025 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-025 — Frontend full-stack et composants WebAssembly

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-023, ADR-005.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Un seul langage doit partager types UI/API sans prétendre supprimer les frontières de sécurité.

## Décision proposée

Interfaces de composants typées, générateur WASM/JS interop, serveur SSR facultatif ; WIT/WASI quand prise en charge cible validée ; authentification/autorisation côté serveur obligatoires.

## Alternatives à comparer / écarter

Tout dans navigateur ; protocole RPC opaque ; une VM propriétaire client obligatoire.

## Conséquences et compromis

- Contrats de transport et portabilité. - APIs DOM et outils navigateur varient.

## Preuves à produire avant acceptation

E2E Chromium/Firefox/WebKit, hydration/SSR, requêtes interdites client, XSS/CSP/CSRF tests.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Le modèle composant existant ne couvre pas des besoins essentiels sans lourds polyfills.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-026 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-026 — Schémas et migrations de bases de données

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-025.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

La validation SQL sur une base de dev ne prouve pas l’état de la base prod.

## Décision proposée

Schémas versionnés, SQL statiquement vérifié contre snapshot attesté, migration expand/contract avec compatibilité n/n-1, contrôle runtime de version et transactions.

## Alternatives à comparer / écarter

ORM masquant SQL ; migration automatique destructive ; requêtes non typées.

## Conséquences et compromis

- Contrats de données détectables avant déploiement. - Spécificités SQL par backend.

## Preuves à produire avant acceptation

PostgreSQL+SQLite réels, migrations avant/après, rollback contrôlé, tests deadlock/isolation et requêtes paramétrées.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Incompatibilités SQL dialectes invalidant abstractions communes.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-027 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-027 — API machine pour les agents et les IDE

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-014, ADR-019.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Les diagnostics texte libres font gaspiller des tokens et compliquent les corrections.

## Décision proposée

API JSON/SARIF versionnée, IDs de diagnostics/obligations, liens code→contrat→preuve→test→binaire, diff et contre-exemple minimal, CLI/LSP indépendants IDE.

## Alternatives à comparer / écarter

Sortie CLI humaine seulement ; RPC propriétaire non versionné ; agent directement lié au compilateur.

## Conséquences et compromis

- Agent automation reproductible. - Versionner les schémas et compatibilité.

## Preuves à produire avant acceptation

Tests de contrat API, mutation des champs, compression de feedback mesurée et compatibilité OpenCode/Unifia/VS Code via clients.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

La taille et complexité des événements dépassent leur intérêt.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-028 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-028 — Autorité des contrats et changement IA vérifié

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-027.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Un agent IA peut faire passer ses tests en affaiblissant une spécification.

## Décision proposée

Contrats séparés par statut verrouillé : obligations normatives signées/contrôlées par humain ; agent peut proposer mais pas accepter suppressions, affaiblissement ou changement d’hypothèse.

## Alternatives à comparer / écarter

Agent propriétaire de toutes les preuves ; signatures de contrats non tracées ; autorisation au cas par cas ad hoc.

## Conséquences et compromis

- Régression contractuelle bloquée. - Plus de workflow pour changer légitimement un contrat.

## Preuves à produire avant acceptation

Tests injection d’agents modifiant préconditions/skip/CI, fail closed, diff sémantique et audits.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

La gouvernance devient trop lourde pour de petits projets : profils d’autorité à définir.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-029 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-029 — Security-by-default et sandbox du build

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-005, ADR-009.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Scripts de build, macros, dépendances et plugins peuvent accéder à des secrets et au système hôte.

## Décision proposée

Permission explicite de fichiers/réseau/process, sandbox build, dependencies lock, restrictions de plugins et manifeste d’effets ; analyse de flux secrets (taint).

## Alternatives à comparer / écarter

Exécution sans isolation ; confiance implicite crates ; sandbox complète impossible à désactiver.

## Conséquences et compromis

- Réduction supply-chain/SSRF/exfil. - Compatibilité et limitations OS.

## Preuves à produire avant acceptation

Fixtures malveillantes de script, exfiltration simulée, path traversal, symlink/junction, secrets redaction.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Des outils nécessaires exigent privilèges non compatibles avec le modèle.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-030 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-030 — Reproductibilité, provenance et artefacts

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-029.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Une preuve sans lien fiable avec le code machine et les dépendances est impossible à auditer.

## Décision proposée

Lockfiles complets, SHA exact, toolchain pin, artefacts identifiés, SBOM, provenance SLSA-compatible quand applicable et manifestes de preuve ; source date epoch contrôlé.

## Alternatives à comparer / écarter

`latest` implicite ; releases manuelles non traçables ; supposer bit-identique sur toutes machines.

## Conséquences et compromis

- Traçabilité et audit. - Rebuild reproductible multi-platform demande travail.

## Preuves à produire avant acceptation

Double build indépendant sur machines contrôlées, comparaison binaire/diffoscope et rapport d’écarts documentés.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Les plateformes ne permettent aucune reproducibilité acceptable.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-031 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-031 — CI en gates et qualification exacte au SHA

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-014, ADR-029.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Les garanties varient selon la cible et ne peuvent être inférées du simple succès d’un build.

## Décision proposée

Gates T0 spécification, T1 prototype, T2 safety+conformité, T3 E2E/RT/fullstack, T4 release auditée ; NO-GO si requis rouge, inconnu non justifié ou SHA divergent.

## Alternatives à comparer / écarter

Une seule CI verte ; tests désactivés pour merger ; déploiement à la validation textuelle.

## Conséquences et compromis

- Dossiers de preuves fiables. - Coût CI et maintenance.

## Preuves à produire avant acceptation

Tester CI négative, jobs requis réels, matrice cibles, SHA des artefacts, contrôle policy-as-code.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Coûts CI non tenables : réduire les variantes exploratoires, non les gates critiques.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-032 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-032 — Performance et budgets de construction

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-012, ADR-018.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

« Compilation instantanée » et « très légère » doivent être mesurables.

## Décision proposée

SLO exploratoires et reproductibles sur hardware de référence : diagnostics locaux p95 <50 ms ; petite UI patch p95 <150 ms ; petite fonction chaude p95 <500 ms ; budgets RAM/CPU profilés ; aucun engagement sans baseline.

## Alternatives à comparer / écarter

SLA marketing sans périmètre ; optimiser compilateur sans mesurer app ; ignorer coût énergétique.

## Conséquences et compromis

- Décisions fondées sur mesures. - Variance matériel et test harness complexe.

## Preuves à produire avant acceptation

Bench froid/chaud et p50/p95/p99, RSS/peak CPU/IO, taille, latence de preview et énergie si mesurable ; comparer Rust/Cranelift/rust-analyzer.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Targets impossible à atteindre sur matériel ciblé après enquête et budgets décidés.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-033 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-033 — Télémétrie, confidentialité et données locales

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-027, ADR-029.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Agents IA et preuves peuvent contenir du code et données confidentielles.

## Décision proposée

Off-line et local-first par défaut ; aucune télémétrie source ou compilation envoyée sans consentement ; redaction, journaux éphémères configurables, rétention limitée.

## Alternatives à comparer / écarter

Télémétrie obligatoire ; stockage cloud source par défaut ; aucune mesure locale possible.

## Conséquences et compromis

- Souveraineté et confidentialité. - Moins de données de diagnostic distant.

## Preuves à produire avant acceptation

Tests trafic réseau nul en mode offline, secrets dans logs, effacement caches et politique d’opt-in claire.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Un cas d’usage explicitement connecté nécessite partage contrôlé.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-034 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-034 — Ergonomie, LSP, debug et navigation des preuves

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-011, ADR-027.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

La vérification profonde ne doit pas envahir l’éditeur avec des erreurs peu utiles.

## Décision proposée

LSP standard, diagnostics agrégés/priorisés, code actions et statuts `PROVED`/unknown ; debugger et inspecteur de contrats indépendants de l’IDE.

## Alternatives à comparer / écarter

IDE propriétaire obligatoire ; console seule ; affichage de toutes les obligations à chaque frappe.

## Conséquences et compromis

- Adoption par humains et agents. - UI complexe et versionnement LSP.

## Preuves à produire avant acceptation

Évaluations tâches concrètes, accessibilité clavier, événements out-of-order, temps à comprendre un contre-exemple.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

La couche UI devient un goulot d’étranglement même avec daemon rapide.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-035 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-035 — Certificats de preuve et Trusted Computing Base

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-014, ADR-018, ADR-030.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Un résultat `UNSAT` dépend du solveur ; génération binaire dépend du compilateur.

## Décision proposée

Manifeste TCB détaillant solveur, backend et hypothèses ; à moyen terme certificats vérifiables par petit checker pour propriétés compatibles, validation de traductions critiques.

## Alternatives à comparer / écarter

Se proclamer end-to-end formally verified ; aucun traçage de confiance ; réécrire CompCert entier dès le départ.

## Conséquences et compromis

- Niveau de confiance explicite. - Certificats pas disponibles pour toutes obligations ; effort recherche.

## Preuves à produire avant acceptation

Checker indépendant sur preuve falsifiée et vraie, tests tampering, hash artefact, rapport de couverture TCB.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Aucun format de certificat adapté au moteur/à la propriété retenus.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-036 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-036 — Interop GPU et accélérateurs spécialisés

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-007, ADR-024.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

L’universalité suppose GPU/IA/SIMD mais les modèles de mémoire et d’effets diffèrent.

## Décision proposée

Extensions cibles versionnées (WebGPU/SPIR-V/CUDA selon besoin), noyaux purs et frontières de synchronisation vérifiables ; pas de GPU obligatoire pour le cœur.

## Alternatives à comparer / écarter

Backend GPU unique pour tout ; shader compiler universel garanti ; code CPU exécuté sans restriction sur GPU.

## Conséquences et compromis

- Extensions sans contaminer sûreté du noyau. - Complexité de cibles et preuve flottante.

## Preuves à produire avant acceptation

Bench kernels vectoriels et coût transfert, tests numériques multi-cibles, sécurité mémoire shader et comparaison oracle CPU.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Bénéfice nul vs bibliothèques existantes ou modèles de preuves irréconciliables.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-037 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-037 — Gestion des paquets, plugins, macros et écosystème

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-028, ADR-029.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

L’écosystème doit rester composable sans permettre des extensions de compilation arbitrairement privilégiées.

## Décision proposée

Paquets signables, lockfiles et politiques de capacités ; macros déclaratives pures d’abord, générateurs procéduraux sandboxés ensuite ; ABI composants stable/versionnée.

## Alternatives à comparer / écarter

Macros procédurales natives non restreintes ; tout code vendored ; registre central unique obligatoire.

## Conséquences et compromis

- Supply-chain mesurable, extensions auditables. - Limitations de métaprogrammation.

## Preuves à produire avant acceptation

Tests dépendances hostiles, résolution déterministe, compatibilité semver, signature/refus et modes air-gap.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Les bibliothèques critiques ne fonctionnent pas sans escapes non auditables.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-038 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-038 — Ordre des plateformes et coût de portabilité

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-000, ADR-023.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Toutes plateformes à la fois multiplient exponentiellement la matrice de tests.

## Décision proposée

Targets prioritaires : x86_64 Linux + Windows et WASM ; puis macOS/ARM, Android/iOS et no_std ; GPU/hard-RT par cible séparée. Décision à confirmer par bench.

## Alternatives à comparer / écarter

Tout cibler avant T1 ; Windows uniquement ; WASM comme runtime obligatoire.

## Conséquences et compromis

- Focus des tests et expérience réelle. - Universalisme retardé.

## Preuves à produire avant acceptation

Matrice par cible / fonctionnalités, cross-compile, smoke sur machines réelles, package reproductible.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Demande matérielle prioritaire change les contraintes initiales.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-039 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-039 — Validation de l’optimisation et compatibilité de versions

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-007, ADR-018, ADR-035.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Optimisations d’IA, LLVM et patch peuvent changer la sémantique.

## Décision proposée

Préservation des obligations contractuelles, differential testing et translation validation sur sous-ensembles critiques ; ne pas supposer équivalence totale de programmes arbitraires.

## Alternatives à comparer / écarter

Comparaison benchmark seule ; ignorer codegen ; exiger preuve complète de tout binaire instantanément.

## Conséquences et compromis

- Optimisation sous contraintes. - Verification très coûteuse sur transformations complexes.

## Preuves à produire avant acceptation

Tester mutants d’optimizer, overflow/NaN/concurrence, refuser transform où validation échoue ou devient UNKNOWN en profil strict.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

L’approche n’évolue pas au-delà de micro-transformations locales.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-040 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-040 — Diagnostics structurés, audit et confidentialité du retour IA

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-027, ADR-032.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Des rapports IA trop verbeux ou peu fiables peuvent augmenter les cycles de développement.

## Décision proposée

IDs stables, sorties typées JSON Schema versionné et SARIF, cause, trace, contre-exemple, scope preuve, coût, liens de source ; minimiser exposition secrets et répétitions.

## Alternatives à comparer / écarter

Chaînes de texte non structurées ; agent ayant accès à toute base de preuve ; IDs dépendant de position ligne.

## Conséquences et compromis

- Débogage et audit automatiques. - Maintenance des schémas et source maps.

## Preuves à produire avant acceptation

Tests de parsing strict, triage automatique, diagnostics re-localisés après édition, secrets de test jamais exposés.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

La structure imposée empêche des diagnostics essentiels ; proposer un champ extension borné.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.

---

\<!-- Texte ADR-041 reproduit depuis la branche Ravum feat/mvp0c-hardening, vérification documentaire du 9 novembre 2026. -->&#x20;

# ADR-041 — Gouvernance open source et gestion des changements

- **Statut** : `PROPOSED` — validation humaine et preuves en attente.
- **Date de proposition** : 2026-10-23.
- **Décideur** : à désigner.
- **Dépendances** : ADR-030.
- **Impact** : architecture du compilateur, profil cible, sécurité et/ou expérience développeur selon le domaine.

## Contexte et problème

Le compilateur sera une infrastructure de confiance avec contributions humaines et IA.

## Décision proposée

Décisions via ADR, RFC pour grande syntaxe, PR petites et auditées, CODEOWNERS selon périmètre, branche protégée, licences évaluées avant ajout d’outils.

## Alternatives à comparer / écarter

Merge automatique sans revue ; ADR réécrits sans historique ; adoption aveugle de code GPL/propriétaire.

## Conséquences et compromis

- Pérennité, révision facile. - Revue formelle et coût administratif.

## Preuves à produire avant acceptation

Test de protection main, reviewers indépendants sur code critique, politique licence/SBOM et audit des décisions.

**Critère d'acceptation** : expérience reproductible, SHA + plateforme + toolchain + résultats + limites documentées, sans fail critique inexpliquée. Mettre à jour l'ADR avec chiffres mesurés avant le passage à `ACCEPTED`.

## Échec, repli et conditions de réouverture

Administration trop lourde pour maintenir un noyau ouvert viable.

Si la preuve échoue : statut maintenu `PROPOSED` ou `REJECTED`, preuve jointe, alternative expérimentée ; **ne pas dégrader les contrats exigés** pour faire passer l'essai.

## Questions ouvertes

- Quelles hypothèses sont confiées à des outils ou runtimes tiers ?
- Quels cas limites doivent produire `UNSUPPORTED`/\`UNKNOWN\` plutôt qu'un succès ?
- Quelles mesures sur plateforme de référence pourraient réfuter ce choix ?
- Quels compromis de maintenance et d'ergonomie devront être revus à T1/T2 ?

## Historique

- 2026-10-23 — Proposition initiale à discuter.


---

# PARTIE : SYNTHESE CRITIQUE MULTI-IA FOURNIE — REPRODUCTION ARCHIVE

# Ravum Flow — synthèse critique des contre-expertises multi-IA

**Date de consolidation :** 9 novembre 2026\
**Documents analysés :** texte fourni « Review multi IA » (Claude, DeepSeek, Mistral, Qwen, Kimi/Gemini) ; plan et dossier maître de Ravum Flow ; sources primaires sélectionnées du dépôt Ravum ; documentation officielle GitButler, Scion, GitHub et SQLite.\
**Statut :** revue, pas autorisation de développement ; aucun ADR ratifié, aucune modification des dépôts.

## 1. Verdict consolidé

- **NO-GO pour l'architecture v0.1 telle quelle :** trop de systèmes coûteux présumés nécessaires avant mesure (CAS indépendant, journal général, VFS, ordonnanceur, moteur d'impact polyglotte) ; garanties impossibles à tenir pour les outils non intégrés qui écrivent directement sur un disque partagé.
- **CONDITIONAL-GO pour Ravum Flow comme produit autonome et incrémental :** commencer par un coordinateur léger d'agents et d'intégration Git ; produire des reçus de validation liés à l'arbre exact ; comparer aux méthodes existantes ; n'ajouter des couches transactionnelles / overlays que si elles sont justifiées par des tests adversariaux et des gains démontrés.
- **Le souhait « sans worktree obligatoire » demeure un objectif légitime, mais non une contrainte qui autorise la perte de données ou des régressions de débit.** Le premier prototype peut utiliser des espaces éphémères isolés et viser progressivement une représentation logique plus légère.

## 2. Consensus réel des modèles

| Sujet         | Consensus utilisable                                                                                          | Niveau de confiance |
| ------------- | ------------------------------------------------------------------------------------------------------------- | ------------------- |
| GitHub/GitLab | Conserver Git comme moyen de publication / interopérabilité                                                   | Élevé               |
| Worktrees     | Une unique vue physique n'isole pas deux agents non intégrés qui écrivent le même chemin                      | Élevé               |
| Sécurité      | Admission des patches, validation exacte, publication atomique et reprise doivent être explicitement séparées | Élevé               |
| Tests         | Les E2E et tests requis restent bloquants ; cache uniquement avec identité complète de contexte               | Élevé               |
| Polyglotte    | Le noyau ne doit pas réimplémenter Cargo, npm, Rust ou le compilateur Ravum                                   | Élevé               |
| Ravum         | Flow ne doit pas prendre l'autorité sur la sémantique, les contrats ou le hot swap de Ravum                   | Élevé               |
| Performance   | Mesurer le temps jusqu'au livrable vérifié, pas uniquement les temps d'édition ou le nombre de worktrees      | Élevé               |
| Stratégie     | Démarrer par une base simple et n'élargir le périmètre qu'après benchmarks                                    | Élevé               |

**Benchmark S8 historique :** A copies privées + fusion (médiane 111,1 s), B dossier partagé + garde (237,7 s), C séquentiel (528,5 s), sur trois campagnes réelles par stratégie (27 sessions d'agents en tout). Une autre campagne scriptée a mesuré A = 2,65 s, B = 3,06 s, C = 2,91 s ; ces résultats n'ont pas le même profil temporel. Ni les véritables worktrees Git ni un moteur transactionnel Flow cible n'ont été comparés directement dans le S8 réel. Échantillon restreint, machine partagée : aucun classement universel ne peut être déduit.

## 3. Divergences à arbitrer

### 3.1 Source de vérité : Git seul ou CAS + journal ?

- **Qwen :** Git unique source de vérité et journal opérationnel secondaire.
- **DeepSeek / Mistral / Gemini :** journal + snapshots comme autorité opérationnelle ; Git comme projection publiable.
- **Conclusion proposée :** distinguer les **états**, pas proclamer deux autorités concurrentes. La tête Git est l'autorité sur l'historique **publié** ; le journal est l'autorité sur les **intentions en cours** ; l'identité de snapshot est l'autorité sur le **contenu qui a été vérifié**. Le protocole décrit précisément la transition « proposition → intégration → validation exacte → publication Git » et la récupération après crash. Les différents états doivent rester traçables et reconciliables sans ambiguïté.

### 3.2 Branches virtuelles vs worktrees temporaires

- **Qwen :** adopter directement des branches virtuelles à la GitButler.
- **Claude :** commencer par de vrais worktrees éphémères et une file d'intégration.
- **DeepSeek/Mistral/Gemini :** hybride évolutif selon besoin.
- **Conclusion proposée :** mode initial à isolation sûre et coût mesuré ; tester des branches virtuelles **comme candidate**, pas comme garantie d'isolation. Router des diffs via un watcher **ne prévient pas** qu'une commande externe écrase un fichier avant que le watcher agisse. Une seule vue sur disque ne peut pas être une vue indépendante par agent sans interception réelle des écritures, ou sans passer exclusivement par une API de patches.

### 3.3 Graphe de dépendances

- Certains rapports suggèrent que **Salsa remplacerait un moteur de build**. C'est excessif : Salsa est une bibliothèque de calculs incrémentaux, pas un remplacement transparent de Cargo, Nx, Bazel, Python packaging ou des compilateurs. Un adaptateur doit fournir une identification correcte des entrées, dépendances, outils et modes de fallback.
- **Conclusion :** réutiliser le build natif et n'introduire Salsa qu'en réponse à un besoin d'invalidation qu'aucun outil existant ne couvre correctement, avec un oracle de comparaison build froid/build chaud.

### 3.4 Nombre de daemons

- Le besoin d'un unique **propriétaire des écritures Git** et d'un budget de ressources central par workspace est solide.
- Il ne découle pas nécessairement qu'un unique **processus** doive tout exécuter. Les compilateurs et processus de vérification doivent rester isolables, annulables, et parfois séparés pour éviter qu'un crash ne compromette le coordinateur. Éviter l'orchestration en double Ravum/Flow, pas tous les processus multiples.

### 3.5 Seuil du benchmark

- Les rapports proposent alternativement « pas plus lent avec moins de ressources », « ≥20 % » et « ≥25 % » de gain.
- Il n'y a pas de seuil scientifiquement établi ici. Le fixer **avant** l'expérience, distinguer bénéfice de vitesse, d'ergonomie et de fiabilité, définir une règle d'arrêt et un budget d'essai ; ne pas retoucher le seuil après observation des résultats.

## 4. Vérifications indépendantes et corrections des rapports

### 4.1 Risque de publier un arbre différent de l'arbre validé — confirmé comme écart architectural plausible, non reproduit

Le binaire `ravum-collab` vérifie des `SourceSnapshot` construits depuis l'espace de travail et les overlays. Lors de la publication, il sélectionne les chemins intégrés puis appelle `GitWriter`. Celui-ci amorce un index isolé à partir de `HEAD` (`read-tree`) et applique uniquement les chemins du commit proposé (`write-tree`, `commit-tree`, `update-ref`).

**Risque à tester :** la vérification peut s'exécuter avec des modifications non intégrées présentes dans le snapshot, alors que l'arbre Git publié les exclut. Les reçus de vérification restent liés au snapshot candidat, mais rien dans ce chemin de code observé n'établit systématiquement l'égalité de ce snapshot avec l'arbre publié. Les anciens SHA par chemin protègent contre certains écrasements, mais pas contre ce décalage. Ce n'est pas une preuve qu'une exploitation a eu lieu.

**Test P0 recommandé :** présence d'un changement local non sélectionné dont dépend la réussite des tests ; valider, publier, reconstruire en checkout propre au commit, exiger `verified_tree_oid == published_tree_oid` avant `update-ref` et vérifier le refus si cette égalité ne tient pas.

Sources :

- [https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/crates/ravum-workspace-coordinator/src/bin/ravum-collab.rs](https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/crates/ravum-workspace-coordinator/src/bin/ravum-collab.rs)
- [https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/crates/ravum-workspace-coordinator/src/git_writer.rs](https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/crates/ravum-workspace-coordinator/src/git_writer.rs)

### 4.2 Risque de fuite des secrets lors des tests — constat confirmé dans le code, exploitation non reproduite

`RustcMergeVerifier` démarre `rustc` et exécute le binaire de tests via `std::process::Command`, en supprimant quelques variables telles que `RUSTFLAGS` et `RUSTC_WRAPPER`, **sans `env_clear()` ni liste autorisée complète d'environnement**. Les hooks Git lancés par GitWriter héritent eux aussi de l'environnement par défaut. Les tests et hooks s'exécutent donc potentiellement avec les variables sensibles de l'utilisateur ; les tests ne sont pas sandboxés.

**Correctif à étudier :** environnement à liste autorisée et sans secrets, accès réseau retiré par défaut pour code candidat, contrôle des chemins montés, restrictions des processus enfants, logs redigés et gestion différente des hooks explicitement approuvés. La simple redaction des logs n'empêche pas l'exfiltration par un test malveillant.

Sources :

- [https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/crates/ravum-workspace-coordinator/src/bin/common/mod.rs](https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/crates/ravum-workspace-coordinator/src/bin/common/mod.rs)
- [https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/crates/ravum-workspace-coordinator/src/git_writer.rs](https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/crates/ravum-workspace-coordinator/src/git_writer.rs)

### 4.3 `ravum-store` n'est pas un CAS générique

Le module `crates/ravum-store/src/lib.rs` se décrit explicitement comme frontière SQLite et système de migrations de l'application M4 de tâches (`tasks` / `ravum-todo-core`). Il ne peut pas être compté comme moteur de stockage CAS prêt à extraire. Éviter de le réutiliser par son nom uniquement.

Source : [https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/crates/ravum-store/src/lib.rs](https://github.com/Rwanbt/Ravum/blob/integration/mvp0c-clean/crates/ravum-store/src/lib.rs)

### 4.4 Licence : corriger un faux « interdit » absolu

L'absence de licence open source sur Ravum signifie qu'**un tiers ne reçoit pas automatiquement l'autorisation** de copier, modifier et distribuer ce code. Elle n'interdit pas en soi au véritable titulaire des droits d'auteur de réutiliser son propre code dans un autre dépôt. Il faut clarifier les droits sur les contributions et le code généré, la licence de distribution prévue pour Flow et les dépendances tierces. Des commits sous l'identité « Ravum Agent » ne prouvent pas, à eux seuls, l'existence de droits de tiers ni leur absence.

Source officielle : [https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository)

### 4.5 Quelques erreurs factuelles dans les modèles

- **Scion** n'est pas ici un système propriétaire d'Anthropic : `GoogleCloudPlatform/scion` est un projet open source sous Apache-2.0, avec modes de travail partagés ou isolés. Source : [https://github.com/GoogleCloudPlatform/scion](https://github.com/GoogleCloudPlatform/scion)
- **GitButler** n'est pas « MIT / FLOSS » sans qualification : son dépôt indique une licence **FSL-1.1-MIT**, avec restrictions de concurrence commerciale et conversion MIT après deux ans pour chaque version. Source : [https://github.com/gitbutlerapp/gitbutler/blob/master/LICENSE.md](https://github.com/gitbutlerapp/gitbutler/blob/master/LICENSE.md)
- **jj workspaces** ne garantissent pas plusieurs vues isolées dans le même répertoire physique : des workspaces correspondent à des copies de travail distinctes, même quand le stockage Git est partagé.
- **Annexe Z :** la copie locale examinée (`RAVUM-FLOW-REVUE-MULTI-IA-UNIQUE.md`) contient bien 42 sections `Conséquences et compromis` avec une ligne commençant par `+` et une partie négative `-`. L'observation selon laquelle les 42 avantages auraient été transformés en désavantages n'est **pas confirmée par cette copie** ; il peut s'agir d'un export différent, non fourni.

### 4.6 État du dépôt Ravum

La PR [https://github.com/Rwanbt/Ravum/pull/85](https://github.com/Rwanbt/Ravum/pull/85) a été **fusionnée le 9 novembre 2026 à 11:26:56 UTC** dans `dev`. Les états historiques du dossier doivent être ré-épinglés avant toute extraction ou benchmark. Le dépôt GitHub `Rwanbt/Ravum-Flow` n'était pas accessible aux outils au moment de la revue (404 : absent ou non public) ; ne pas le déclarer créé.

### 4.7 SQLite WAL : ne pas confondre cohérence et durabilité

Dans un journal qui protège une publication, `synchronous=FULL` en mode WAL est préférable lorsque l'on exige la durabilité après arrêt du système ; avec `NORMAL`, SQLite préserve la cohérence mais une transaction récente peut être perdue après coupure de courant. Le réglage exact dépend du protocole de reprise et des coûts mesurés.

Source : [https://www.sqlite.org/pragma.html](https://www.sqlite.org/pragma.html)

## 5. Architecture recommandée pour la prochaine étape

```
Humain / Lead / agents
        │
        ▼
Flow CLI + protocole d'attribution de tâches et patches
        │
        ├── espaces éphémères privés (défaut MVP) ──┐
        ├── API de patches contrôlés (expérimental) ┤
        └── candidat branches virtuelles            ┘
        │
        ▼
Coordinateur d'intégration unique par dépôt
        │  snapshot candidate exact + contexte
        ▼
Adaptateurs vérification (Cargo, autres langages, Ravum)
        │  reçus liés à l'arbre exact, jamais faux PROVED
        ▼
GitWriter / publication Git contrôlée et reprise
        │
        ▼
GitHub / GitLab / Git local
```

**Principes :**

1. Pas de nouveau VCS ni VFS obligatoire au MVP ; conserver l'interopérabilité Git standard.
2. Git est canonique pour les états publiés ; les patches/snapshots sont des candidats non publiés ; journal durable pour les transactions en cours.
3. Prévenir l'écrasement silencieux en mode contrôlé ; pour les outils non intégrés, détection et restauration testées, jamais prétendre à une prévention absolue.
4. Le build doit être exécuté sur l'arbre qui sera publié, avec un contexte tracé ; les changements d'API et les dépendants inchangés sont revalidés.
5. Flow orchestre ; le compilateur Ravum reste propriétaire de la sémantique, des preuves et du hot swap.
6. Pas de réseau, service cloud, base distribuée ni daemon global obligatoires ; budgets CPU/RAM stricts.
7. CI par impact + E2E requis au SHA et merge queue si disponible ; ne jamais cacher un échec.

## 6. Gates de décision et roadmap avant construction

| Gate                                 | Livrable vérifiable                                                                                                  | Décision                                                                 |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| G0 — provenance et licence           | Titulaires de droits, statut des contributions, licence Flow choisie et compatibilité tiers                          | Pas d'extraction tant qu'il y a une incertitude juridique bloquante      |
| G1 — frontières                      | RFC précisant autorité Git / journal / snapshots ; propriété de l'orchestration et API Ravum–Flow                    | Pas de duplication de coordinateur ni de daemon fonctionnel              |
| G2 — sécurité                        | Tests de fuite de secrets, propagation d'environnement, hooks, crash et divergence arbre vérifié/publié              | Défauts P0 corrigés ou fail-closed avant publication contrôlée           |
| G3 — baseline                        | Comparer copies privées, vrais worktrees, mode partagé, jj/GitButler le cas échéant, sur fixtures Rust et polyglotte | Sélection du mode initial selon coût réel                                |
| G4 — prototype mince                 | Attribution + reçus exacts + GitWriter + recovery + E2E requis                                                       | Aucun changement perdu, récupération, publication SHA exact              |
| G5 — décision d'architecture avancée | Benchmark pré-enregistré du candidat « sans worktree obligatoire », avec budget et seuil fixés avant mesure          | Garder, améliorer ou abandonner CAS/VFS/branches virtuelles selon preuve |

**Ordre de travail recommandé :** traiter d'abord les risques P0 repérés dans Ravum (dans sa session dédiée) et documenter leurs contrats ; ne pas modifier Ravum depuis la session Flow. En parallèle, réviser l'architecture et préparer les benchmarks dans un dépôt Flow indépendant. Ensuite seulement développer le noyau minimal et choisir si un mode plus ambitieux se justifie.

**Décision finale :** `CONDITIONAL-GO` pour une trajectoire incrémentale et mesurée ; `NO-GO` pour un VFS/CAS complet construit immédiatement ; `NOT_REVIEWED` pour les implémentations non testées et les droits non établis. La revue documentaire ne remplace ni des reproductions de défauts, ni une décision du responsable sur la licence, les risques acceptables et le budget.




---

# ANNEXE F — RAPPORT FINAL CLAUDE SUR V5 (TEXTE FOURNI, CONSERVÉ INTÉGRALEMENT)

**Provenance :** fichier utilisateur `Markdown collé.md`, revue de V5 ; **ne s'applique pas comme validation indépendante de V6**. Aucun ordre opérationnel présent dans cette annexe ne peut contredire l'interdiction d'implémenter énoncée en tête.

## Rapport final de revue — Ravum Flow V5

J’ai vérifié deux fichiers Ravum au SHA `5408d9b3…` en lecture seule : `git_writer.rs` et bin/common/mod.rs. Je n’ai rien exécuté et rien modifié.

### 1. VERDICT

**DOCUMENT-CONDITIONAL.** La V5 est cohérente, traçable et ne prétend nulle part qu’un développement ou un test est déjà autorisé ou réalisé. Elle peut être soumise à ta validation. Il reste cinq corrections ponctuelles, listées au §9, et six décisions humaines déjà explicitées. Aucune contradiction ne rend le plan inexécutable en sécurité après un GO borné.

**Côté technique, rien ne change :** NO-GO pour l’implémentation aujourd’hui. Le seul GO envisageable ensuite est **GO_R0_FIXTURES**, et uniquement sur ta décision explicite.

### 2. BLOCKERS

**Aucun blocage documentaire.** Le blocage technique principal est **confirmé par lecture du code**, et il est plus précis que ce qu’en dit la V5 :

**B-1 — P0-HOOK-ORDER (statut CODE_READ, non reproduit).**

- **L’arbre est figé avant les hooks.** Dans `git_writer.rs:842`, `write-tree` calcule l’arbre. Les hooks ne s’exécutent qu’ensuite, via `run_commit_hooks` (l.\~855), puis `commit-tree` réutilise ce `tree` (l.868).
- **Conséquence :** si un `pre-commit` approuvé modifie l’index isolé (formateur, fichier généré), la modification est **perdue silencieusement**. Le commit publie l’arbre d’avant le hook.
- **L’ordre des hooks est aussi faux.** Les lignes 1223–1227 et la doc l.41–43 exécutent `prepare-commit-msg` avant `pre-commit`. Git fait l’inverse : `pre-commit` → `prepare-commit-msg` → `commit-msg` → `post-commit`.
- **Les hooks tournent dans la racine réelle du dépôt** (l.\~1347, `current_dir(&self.repo)`), avec l’index isolé. Un `pre-commit` qui réécrit des fichiers modifie donc le **working tree de l’utilisateur**, y compris des fichiers dirty. Cela viole INV-02.
- **Correction minimale :** exécuter les hooks dans l’ordre de Git, dans une matérialisation isolée de l’arbre candidat, puis faire `write-tree` **après** les hooks, puis valider cet arbre.
- **Test de non-régression :** HOOK-02, avec trois variantes : le hook modifie l’index, le hook modifie un fichier, le hook vérifie l’ordre.
- **Propriétaire :** session Ravum.

### 3. HIGH / MEDIUM / LOW

| Niveau | Section V5              | Constat                                                                                                                                                                                                       | Correction minimale                                                                                                                                                                                                                                         |
| ------ | ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| HIGH   | §4.3 / Q5               | `git update-ref <new> <old>` ne connaît pas l’epoch de fencing. La vérification « epoch courant » puis le CAS sur la ref font **deux opérations non atomiques**. Un ancien writer peut passer entre les deux. | Imposer que le writer prenne un verrou local exclusif, puis : relire l’epoch durable → vérifier → `update-ref` → enregistrer l’outcome, tout cela sous ce verrou. Admettre que n’importe quel `git` externe contourne le fencing (et l’écrire dans le doc). |
| HIGH   | §4.1 / Q4               | Rejouer les hooks « avant le gel » suppose un arbre matérialisé, puisque les hooks attendent un working tree. En pratique, c’est un **worktree éphémère**.                                                    | Le dire explicitement : ISOLATED_EXEC est nécessaire dès qu’un hook ou un build tourne. « Sans worktree persistant » reste vrai ; « sans matérialisation » est faux.                                                                                        |
| MEDIUM | §1.2                    | La reformulation d’INV-01 (« isolation éphémère autorisée ») **modifie** l’INV-01 original, qui dit « overlays, pas copies ». Or le §1.2 affirme n’être qu’un index.                                          | Présenter le changement comme un **amendement INV-01 proposé**, sans le glisser dans l’index.                                                                                                                                                               |
| MEDIUM | §2 P1-FILES             | Confirmé : `content.as_bytes()` lit du `String` UTF-8, et le mode par défaut est `100644` (l.\~818).                                                                                                          | Rien à corriger dans la V5. Ajouter FILE-09 au handoff P0.                                                                                                                                                                                                  |
| MEDIUM | §4.2 point 4            | « CAS de ref rejette les concurrents » laisse entendre que le fencing est appliqué par Git.                                                                                                                   | Renvoyer à la correction HIGH ci-dessus.                                                                                                                                                                                                                    |
| LOW    | §4.1, §4.5, archive B.2 | Trois nomenclatures d’états différentes : `APPROVED_HOOKS_REPLAYED` / `APPLIED`, `INTENT_DURABLE` / `PUBLICATION_INTENT_DURABLE`, `REF_CAS_PUBLISHED` / `GIT_REF_CAS`.                                        | Un glossaire unique, avec la V5 comme forme canonique.                                                                                                                                                                                                      |
| LOW    | §0.1                    | Scion d’Anthropic (« 13 octobre 2026 ») : pas vérifié par moi.                                                                                                                                                | Garder le statut UNVERIFIED tant que personne ne l’a revérifié.                                                                                                                                                                                             |

### 4. CLAIMS VERIFIED (lecture du code au SHA 5408d9b)

- **P0-HOOK-ORDER** : CODE_READ. C’est pire que ce que dit la V5, voir B-1.
- **P0-ENV** : CODE_READ. `common/mod.rs:371-372, 445-447` ne fait que des `env_remove` ciblés. Les hooks reçoivent seulement `env_remove(REDIRECTING_GIT_ENV)`. Je n’ai trouvé aucun `env_clear`.
- **P1-FILES** : CODE_READ, contenu `String` et mode `100644` par défaut.
- **reference-transaction** : CODE_READ. Il est bien laissé natif dans `update-ref`, conformément au §2.1 de la V5.
- **`post-commit`** : CODE_READ. Un échec n’est enregistré que comme avertissement. C’est le comportement de Git, et c’est correct.
- **Archive** : les ADR-000 à ADR-041 sont tous présents dans le texte fourni, tous `PROPOSED`. On y trouve aussi les 12 RF-RFC, INV-01..12, RF-0..9 et RF-001..026.
- **Doublon** : dans cette archive, Qwen et Gemini sont identiques. Dans le collage précédent, c’étaient Kimi et Gemini. Les deux constats sont vrais, chacun pour son propre corpus, et la V5 le traite correctement.

### 5. UNVERIFIED CLAIMS

Je n’ai pas relu ces points ; ils restent NOT_REVIEWED :

- **P0-TREE** : le chemin `ravum-collab.rs` → `coordinator.rs` à ce SHA.
- **P0-LEGACY** : le défaut V1.
- **P1-STORE.**
- **Rwanbt/Ravum-Flow** : son existence.
- Le **fichier LICENSE de GitButler**.
- **Scion d’Anthropic.**
- Les **pages SQLite**.

### 6. P0 HANDOFF TO RAVUM

Ce sont des demandes de tests sur fixtures jetables, jamais des correctifs imposés :

1. **HOOK-02a** : un `pre-commit` modifie l’index isolé. Attendu : le changement est absent du commit publié (reproduire le défaut).
2. **HOOK-02b** : un `pre-commit` réécrit un fichier du working tree utilisateur qui est dirty. Attendu : perte constatée ou non.
3. **HOOK-02c** : vérifier l’ordre réel d’exécution face à Git (un hook qui journalise son nom).
4. **TREE-01, ENV-04, LEGACY-06, FILE-09** : déjà spécifiés au §9 de la V5.

### 7. ADR/RFC COVERAGE

- **Les 42 ADR restent `PROPOSED`.** Aucun n’est ratifié par la V5. ADR-013, 019, 021, 029, 031 et 041 sont cités pour la frontière, sans changement de statut.
- **RF-RFC-001..012 :** toujours `PENDING`.
- **RF-RFC-013..024 :** des idées, honnêtement présentées comme telles.
- **Mapping RF-0..9 → R0..R7 :** cohérent.
- **Tags de mapping RF-001..026 :** la V5 reconnaît qu’ils ne sont pas faits. C’est acceptable, à condition que ce soit une condition d’entrée en R1.
- **P0-A..I :** préservés.

### 8. BENCHMARK DECISION

Le §6 de la V5 est correct, et **il corrige une erreur de ma revue précédente** : exiger que les intervalles de confiance marginaux ne se chevauchent pas était le mauvais test. Pour des campagnes appariées, il faut analyser la différence appariée.

Le design S9 (comparateurs A–G, NOT_AVAILABLE distinct d’un échec, conditions éliminatoires, règle d’arrêt O0) peut être pré-enregistré. Il ne manque que deux décisions de ta part : le seuil de gain et le budget.

### 9. CORRECTIONS BY SECTION

1. **§2, P0-HOOK-ORDER** : ajouter l’ordre inversé `prepare-commit-msg` / `pre-commit` et l’écriture dans le working tree réel (B-1).
2. **§4.3** : définir le fencing comme « vérification de l’epoch et `update-ref` sous un verrou exclusif du writer », avec la limite pour un `git` externe.
3. **§4.1 et §3.1, mode ISOLATED_EXEC** : écrire que les hooks et les builds exigent une matérialisation éphémère.
4. **§1.2** : présenter INV-01 comme un amendement proposé.
5. **Annexe** : glossaire unique des états (V5 canonique).

### 10. RÉPONSES AUX 16 QUESTIONS

- **Q1 :** oui, une divergence est possible. Elle est même confirmée côté hooks. Le mécanisme tree / context / policy la ferme **si** l’arbre est gelé après les hooks et que les dépendances externes sont dans `BuildContextId`. Sinon, le statut doit être UNKNOWN.
- **Q2 :** l’implémentation actuelle ne les traite pas selon leur rôle réel (B-1). Les effets externes des hooks (réseau, fichiers hors index) ne sont pas transactionnels.
- **Q3 :** traduire V1 `AUTO_SAFE` en `TEXTUALLY_APPLICABLE` (§5.2) et ne jamais le promouvoir. C’est suffisant.
- **Q4 :** MANAGED_PATCH, plus ISOLATED_EXEC éphémère. OBSERVED n’offre aucune prévention. VIRTUALIZED est DEFER. Promettre une isolation pour un outil qui écrit directement sur le disque serait impraticable, et la V5 ne le promet pas.
- **Q5 :** voir la correction HIGH au §3. Le fencing est nul pour les écritures UNMANAGED.
- **Q6 :** le stockage est minimal : objets Git, journal, pinning et GC. SQLite WAL n’est pas un CAS de contenu, il ne contient que des métadonnées transactionnelles.
- **Q7 :** `env_clear()` est nécessaire mais insuffisant. Linux : bubblewrap ou namespaces sont plausibles, à tester. Windows : à qualifier ; d’ici là, UNSUPPORTED ou MANUAL_REQUIRED est la position honnête.
- **Q8 :** pas couvert dans le code actuel. Couvert dans la spécification V5 (§4.4, FILE-09).
- **Q9 :** oui pour les consommateurs inchangés et les E2E. Si le graphe est partiel ou dynamique : validation élargie ou complète.
- **Q10 :** oui, avec PASSED et PROVED bien distingués.
- **Q11 :** oui, sur le papier. Il n’y a aucun cycle tant qu’on n’extrait pas de code avant la gate D0.
- **Q12 :** oui, sous réserve des tags RF-001..026.
- **Q13 :** oui, et c’est pré-enregistrable une fois le seuil et le budget fixés.
- **Q14 :** on peut réutiliser une validation si l’arbre et le contexte sont identiques (cas du squash). Il faut une requalification dès que l’arbre change : rebase avec conflit, merge queue, nouvelle base.
- **Q15 :** les droits sont suffisamment explicités. L’option from scratch est viable : GitWriter (environ 1 000 lignes) est réécrivable proprement, et la réécriture corrigerait B-1 au passage.
- **Q16 :** à **abandonner** : le CAS général, le VFS, le CRDT, le cache distant, le daemon obligatoire et Salsa dans le noyau. À **reporter** : les leases et le fencing (uniquement si S9 montre des collisions réelles), ainsi que le mode MANAGED_PATCH lui-même si O0/O1 l’égale. Ce qui **reste** dans tous les cas : un writer Git exact avec hooks corrects, des reçus liés à l’arbre, et un sandbox. C’est le seul noyau dont la valeur ne dépend pas du benchmark.

### 11. FINAL READINESS

| Objet          | Statut                                           |
| -------------- | ------------------------------------------------ |
| Document V5    | DOCUMENT-CONDITIONAL (5 corrections ponctuelles) |
| GO_R0_FIXTURES | Possible après ta décision                       |
| GO_R1..R7      | NO-GO                                            |
| Production     | NO-GO                                            |

### 12. HUMAN DECISIONS

1. Option provisoire : O0, O1, O2 ou O3.
2. Licence et droits d’extraction, ou bien from scratch.
3. Politique de sandbox, avec le niveau accepté sous Windows.
4. Seuil de gain S9 et budget, figés avant toute mesure.
5. GO_R0_FIXTURES : oui ou non, avec son périmètre et son budget.
6. Transmettre ou non le handoff du §6 (HOOK-02a/b/c) à la session Ravum.

**Je m’arrête ici, comme le demande le mandat. Je ne lance pas de nouveau cycle de contre-expertise.**

---

Une précision : la synchronisation du vault a échoué en début de session (le validateur v4 a dépassé son délai). Je n’ai donc mis à jour ni `memory.md` ni `LOG.md` du vault ; c’est à régler avant de clôturer la session.
