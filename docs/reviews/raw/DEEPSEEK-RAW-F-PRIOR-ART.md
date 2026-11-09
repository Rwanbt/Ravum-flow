# DEEPSEEK-RAW-F — Art antérieur, licences, O1 crédible

**Agent :** F (Existing tools / prior art) · **Date d'observation :** 2026-11-09, fenêtre ~15:55–16:25 Europe/Paris
**Dépôt Flow :** `D:\App\Ravum-flow`, branche `docs/deepseek-d0-d4-review`, HEAD `1d5fddf6efc5d0535a474bebfb8d33faf341d746`
**Statut :** rapport brut, sourcé, **non ratifié** ; le Lead consolide. Aucune installation, aucun build, aucun test, aucun clone, aucune modification de Ravum.
**Classement des affirmations :** `SOURCE_READ` (doc officielle lue), `API_READ` (métadonnée GitHub API), `NOT_REVIEWED` (inaccessible), `PROPOSED` (proposition F), `UNKNOWN`. Aucun `MEASURED` : je n'ai exécuté aucun outil.

---

## 1. Mission, méthode et baseline revérifiée

### 1.1 Périmètre

Répondre frontalement à trois questions :

1. Que garantissent réellement jj, GitButler, Scion, Delta, `git worktree`, Sapling, Graphite, la merge queue GitHub et les outils de build — support Windows, offline, non-écrasement, isolation par agent, CI, licences ?
2. Une solution existante, ou un assemblage, bat-elle Flow sur « temps jusqu'au livrable vérifié » à coût total inférieur ?
3. Quelle est la définition **crédible** de O1, quelles licences interdisent quoi, et quelles preuves manquent avant de trancher O0/O1/O2/O3 ?

### 1.2 Méthode et sources

- Lecture obligatoire effectuée : `AGENTS.md`, `docs/DEEPSEEK_GOAL.md` (Lot 1 + sources), V6 §3.1/§6/§10, `docs/ARCHITECTURE.md`, `docs/BENCHMARK_S9.md`, `docs/adr/FLOW-ADR-008`, `docs/adr/FLOW-ADR-011`, `docs/DECISION_REGISTER.md` (`SOURCE_READ`).
- Sources primaires : API GitHub (`repos`, `releases`, `license`), fichiers `LICENSE`/`LICENSE.md`/`COPYING`/`Cargo.toml` bruts, documentation officielle des outils, pages de téléchargement. Chaque URL est listée §9 avec sa date de consultation.
- Aucune vérification dynamique : tout ce qui suit décrit le **comportement documenté**, pas un comportement observé. Les affirmations marketing sont isolées et signalées.

### 1.3 Baseline revérifiée le 2026-11-09

| Élément | Observation | Statut |
|---|---|---|
| `Rwanbt/Ravum` `dev` | `5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96`, commit du 2026-11-09T12:46:36Z, signature PGP vérifiée, message « merge(mvp0d) … (#87) » | `API_READ` — **identique à la baseline** |
| Ravum public / licence | `private: false`, `license: null`, `GET /repos/Rwanbt/Ravum/license` → **404** | `API_READ` — **aucune licence** |
| Ravum taille / activité | `size: 2824` Ko, 91 issues ouvertes, 0 fork ; `language: Python` alors que le commit `5408d9b3…` contient des chemins Rust (`crates/ravum-batch/src/lib.rs`) — écart de métadonnée non expliqué, sans effet ici | `API_READ` + `SOURCE_READ` |
| `Rwanbt/Ravum-flow` | créé 2026-11-09T13:32:28Z, `size: 0`, `language: null`, `license: null` | `API_READ` — **sans code, sans licence** |
| Flow issues / PR | `open_issues_count: 48` observé (la baseline documentaire dit 47 ; écart de 1, non investigué) ; PR #33 présente, titre « docs: initialize Ravum Flow charter, V6, proposed ADRs/RFCs, issues and DeepSeek GOAL » | `API_READ` |
| `google/scion` | 404 — le projet Scion est `GoogleCloudPlatform/scion` | `API_READ` |
| Limite d'API | `api.github.com` a renvoyé `403 rate limit exceeded` vers 16:00 ; les vérifications restantes ont été faites sur `raw.githubusercontent.com` ou les pages HTML | `API_READ` |

**Conséquence immédiate :** Flow n'a pas plus de licence que Ravum. Le blocage H2 (droits) n'est donc pas seulement « peut-on copier Ravum ? » mais « peut-on distribuer Flow ? ». Aucun texte ne permet aujourd'hui de supposer MIT/Apache par défaut (cohérent avec `DECISION_REGISTER.md`).

---

## 2. Fiches outils

### 2.1 Tableau de synthèse

| Outil | Version observée | Licence (SPDX) | Windows | Offline | Isolation FS par agent | Non-écrasement garanti | CI intégrée | Maturité | Coût d'adoption |
|---|---|---|---|---|---|---|---|---|---|
| **Jujutsu / jj** | 0.47.1 (2026-11-05) | Apache-2.0 | Binaires MSVC x86_64 + aarch64 ; caveats EOL/symlinks | Oui (sauf ops distantes) | `jj workspace` : un working copy par workspace, objet store partagé | Non : concurrence lock-free, conflits exposés, corruption possible sur FS partagé | Non | Moyenne-haute (pré-1.0, 32 k★) | Élevé : nouveau modèle mental, hooks absents |
| **GitButler** (app + `but`) | 0.23.2 (2026-11-02) | **FSL-1.1-MIT** (non-OSI) | MSI x64 | Ops Git locales oui ; forge non | **Non** pour les branches parallèles (même dossier) ; **oui** via `but worktree` | Non (même dossier partagé) | Indirecte (forge) | Haute (22 k★, releases mensuelles) | Moyen : CLI + skill agents |
| **Scion** (Google Cloud) | pré-1.0, docs v0.3.0-preview | Apache-2.0 | **Client Hub uniquement** ; serveur/runtime « not supported on Windows » | Non : Hub + runtime conteneur + images | Oui (conteneurs ; worktrees/clones optionnels) | Non (pas un VCS) | Non | Faible-moyenne (pré-1.0, non supporté officiellement par Google) | Très élevé : Hub, Docker/Podman, images |
| **Delta / DeltaDB** (Zed) | bêta (2026) | DeltaDB non publié → `NOT_REVIEWED` ; plugin `delta-claude` AGPL-3.0-only | Oui (installateur Windows) | **Non** : remote Git obligatoire, synchronisation entre clients | Oui (checkouts matérialisés) | Non (pas un moteur de vérification) | Non | Bêta | Moyen-élevé : compte, sync, modèle |
| **`git worktree`** | Git 2.56.0 (2026-09-28) | GPL-2.0-only | Oui (doc non spécifique) | Oui | **Oui** (répertoire réel) | Non : aucune coordination, pas de CAS/lease | Non | Très haute | Faible : commande native |
| **Sapling** | 0.2.20261103-145137 (2026-11-03) | **GPL-2.0** | Zip Windows x64 | Oui (Git local) | Non (pas d'équivalent worktree documenté) | Non | Non | Moyenne | Élevé : autre SCM |
| **Graphite** | SaaS courant 2026 | CLI non vérifiable (repo 404) → `NOT_REVIEWED` | Client web/CLI | **Non** (GitHub requis) | Non | Non | Oui (service) | Haute (commercial) | Moyen : abonnement |
| **GitHub merge queue** | GitHub.com 2026 | propriétaire (service) | Navigateur/`gh` | **Non** | Non | Non | Oui (checks requis) | Haute | Faible si déjà sur GitHub |
| **Cargo** | 0.102.0 | MIT OR Apache-2.0 | Oui | Oui | Non (build) | Non | Non | Très haute | Faible |
| **Nx** | courant 2026 | MIT | Oui | Oui | Non | Non | Indirecte | Haute | Moyen |
| **Bazel** | courant 2026 | Apache-2.0 | Oui (caveats) | Oui | Non | Non | Indirecte | Haute | Élevé |
| **Buck2** | courant 2026 | Apache-2.0 | `NOT_REVIEWED` | Oui | Non | Non | Indirecte | Moyenne | Élevé |
| **Pants** | courant 2026 | Apache-2.0 | `NOT_REVIEWED` | Oui | Non | Non | Indirecte | Moyenne | Moyen |
| **sccache** | 0.19.0 (2026-10-24) | Apache-2.0 | Oui (MSVC x86_64/aarch64) | Oui (cache local) | Non | Non | Non | Haute | Faible |

### 2.2 Jujutsu / jj — le modèle de commits, pas un orchestrateur

**Faits documentés (`SOURCE_READ`).** jj 0.47.1, publié le 2026-11-05, Apache-2.0, binaires Windows MSVC x86_64 et aarch64 disponibles. Modèle : le working copy est un commit auto-snapshotté à presque chaque commande ; identités de changement (`change-id`) séparées des OID Git ; journal d'opérations avec `jj undo`/`jj op restore` ; pas d'index de staging ; bookmarks au lieu de branches suivies.

**`jj workspace`.** La doc d'architecture définit `Workspace` comme « the combination of a repo and a working copy (like Git's _worktree_ concept) ». `jj git init --git-repo=<path>` crée un working copy séparé partageant les commits ; `jj workspace add` ajoute des working copies. La doc de compatibilité dit explicitement : `git-worktree: No`, mais « there's native support for multiple working copies backed by a single repo. See the `jj workspace` family of commands. »

**Backend Git.** Les commits sont stockés dans le dépôt Git (gitoxide) ; `refs/jj/keep/*` empêche le GC ; les données hors modèle Git (change-id, prédécesseurs) vont dans `.jj/repo/store/extra/` ; un en-tête `change-id` non standard est écrit dans les commits Git et **n'est pas préservé par tous les outils Git** (exemple documenté : préservé par `--amend`, pas par rebase).

**Limites documentées, décisives pour Flow :** hooks Git **non supportés** (issue #405) ; `.gitattributes` **non supporté** (issue #53) ; submodules **non supportés** ; Git LFS **non supporté** (issue #80) ; partial clones non ; shallow « kind of » ; chemins supposés UTF-8 ; pas de support de `git worktree`.

**Windows.** « Jujutsu works the same on all platforms, but there are some caveats » : conversion EOL via `working-copy.eol-conversion` qui **n'honore ni `.gitattributes` ni `core.autocrlf`** ; symlinks seulement si Developer Mode + Windows 10 ≥ 14972, sinon matérialisés en fichiers ordinaires ; WSL met le bit exécutable sur tous les fichiers ; `@` doit être échappé en PowerShell ; le pager intégré `streampager` est utilisé par défaut.

**Concurrence.** Design lock-free : chaque opération voit un état cohérent et peut toujours committer ; les opérations divergentes sont fusionnées en 3-way et les conflits sont enregistrés. La doc reconnaît une **corruption possible avec le backend Git** parce que celui-ci n'est pas entièrement lock-free (issue #2193, récupération par `jj debug reindex`) et que l'usage via rsync/NFS/Dropbox n'est « not currently thoroughly tested ».

**Ce qu'il ne fait PAS.** Aucun hook, aucune politique de vérification, aucun reçu lié à un arbre, aucun build/test, aucun gate de publication, aucune isolation OS. Deux agents dans **le même workspace** partagent le même working copy : jj expose les conflits, il n'empêche pas l'écrasement de fichiers. L'isolation exige un workspace par agent, donc un working copy complet par agent (inférence : le store d'objets est partagé, les fichiers de travail ne le sont pas).

**Marketing vs fait.** « Git-compatible » est vrai pour les commits/refs, avec des lacunes explicitement documentées (hooks, attributs, submodules, LFS). Ce n'est pas un produit de vérification.

### 2.3 GitButler — branches virtuelles, et ce que FSL implique

**Faits (`SOURCE_READ`).** Version 0.23.2 du 2026-11-02 ; MSI Windows x64, `.deb`/`.rpm` Linux, DMG macOS ; CLI `but` ; `but agent setup` installe un skill pour Claude Code, Codex, Cursor, GitHub Copilot et d'autres.

**Branches parallèles ≠ isolation.** La doc est sans ambiguïté : « Work on several branches in one folder with one editor window, without switching branches or creating worktrees » ; « Your folder contains the changes from all your branches, combined » ; Git voit un commit de fusion unique sur `gitbutler/workspace`. Deux agents dans le même dossier voient donc **les mêmes fichiers**. GitButler le dit lui-même dans les notes 0.23.0 : « Parallel branches are light & fast while worktrees offer full code isolation so each tool can be used situationally. »

**Worktrees.** Depuis 0.23.0 (2026-10-27), GitButler gère les worktrees Git liés, avec `but worktree new/list/archive/restore/remove` ; `but status` les liste avec des IDs. C'est la seule voie d'isolation réelle dans GitButler.

**Offline / compte.** « GitButler needs no import step, config file, or account » et travaille sur un dépôt Git existant ; les fonctions forge (PR, CI, merge) exigent le réseau. Le mode local reste utilisable hors ligne.

**Ce qu'il ne fait PAS.** Aucun build/test, aucun reçu, aucune garantie que l'arbre testé est l'arbre publié, aucune orchestration de hooks, aucune isolation OS, aucune analyse d'impact. Il réduit la friction de branches et de commits pour agents ; il ne qualifie pas un livrable.

**Licence — voir §4.** FSL-1.1-MIT, non-OSI, restriction d'usage concurrent, conversion MIT après deux ans.

### 2.4 Scion (Google Cloud) — orchestration d'agents, dépendance Hub

**Faits (`SOURCE_READ`).** `GoogleCloudPlatform/scion`, Apache-2.0, Go, créé le 2026-03-10, ~2 106★, 73 issues ouvertes, pré-1.0, docs de release `v0.3.0`/`v0.3.0-preview.*` ; « This is not an officially supported Google product. »

**Architecture.** « The server is the hub that every `scion` command on your machine runs agents through » ; en mode Workstation, un « combo server » (Hub + Runtime Broker + dashboard web) tourne sur loopback via `scion server start` ; les agents tournent dans des conteneurs (Podman recommandé, Docker, Apple Container) ; Scion « uses `git worktree` to manage agent workspaces » et exige Git ≥ 2.48.0.

**Windows — fait décisif.** La doc d'installation : le build Windows est « a Hub client only » ; les commandes serveur, Runtime Broker et cycle de vie des conteneurs locaux « exit with a `"not supported on Windows"` error ». Un utilisateur Windows doit donc pointer vers un Hub distant. Ce n'est pas un outil local-first Windows.

**Offline.** Le Hub local est sur loopback, mais l'installation tire des images (`ghcr.io/homebrew-scion`), et les harness exigent des credentials de fournisseur. Aucune promesse d'usage entièrement hors ligne.

**Isolation.** Réelle du point de vue OS (conteneurs), avec choix « shared workspaces, Git worktrees, or separate clones ». Mais Scion n'est pas un VCS : aucun reçu, aucun CAS de ref, aucune politique de vérification, aucune garantie de non-écrasement du dossier utilisateur.

**Correction de la V6.** Le V6 §3.1 écrit « Scion Anthropic (toujours en ligne, serveur central) ». La source primaire dit Google Cloud / `GoogleCloudPlatform` / Apache-2.0, et le Hub local existe. L'argument local-first reste valable (Hub + conteneurs + images), mais l'attribution et le « toujours en ligne » sont à corriger.

### 2.5 Delta de Zed — tracer le travail, pas le vérifier

**Faits (`SOURCE_READ`).** Delta (Zed Industries), bêta, `delta.dev`. DeltaDB enregistre le travail comme un flux de deltas (éditions de fichiers, messages, commentaires, arborescence) au lieu d'instantanés par commit ; « Each connected Delta client holds a copy of the database, and Delta keeps the copies in sync ». Les commits restent dans Git ; Delta crée des checkouts matérialisés et synchronise les deux représentations. Contraintes documentées : le dossier ouvert doit être la racine d'un checkout Git ; **le dépôt doit avoir un remote configuré avant d'ouvrir un thread** ; deux remotes (`origin` + `local`) ; support jj colocalisé « early » et incomplet.

**Licence.** Aucun dépôt public du client/DeltaDB trouvé (`zed-industries/delta` et `deltadb` → 404 ; seuls `delta-claude` AGPL-3.0-only et `delta-nix` publics). Donc : `NOT_REVIEWED` pour DeltaDB ; le plugin Claude est AGPL-3.0-only. Tarification observée : Personal 0 $ (clés propres), Pro 10 $/mois, compte partagé avec Zed.

**Windows.** Oui (installateur, prompts Windows documentés dans le plugin).

**Ce qu'il ne fait PAS.** Pas de reçu lié à un arbre Git exact, pas de gate CI, pas de CAS de publication, pas de garantie de vérification. Et l'exigence d'un remote + d'une synchronisation le rend inadapté à un mode air-gapped strict.

### 2.6 `git worktree` natif — la brique d'isolation la plus crédible

**Faits (`SOURCE_READ`, doc Git 2.56.0).** Un worktree lié partage les refs mais possède `HEAD` et index propres ; `git worktree add` refuse une branche déjà extraite ailleurs sauf `--force` ; `remove` refuse un worktree sale sauf `--force` ; `lock` empêche l'élagage ; `repair` reconnecte après déplacement ; `--relative-paths`/`worktree.useRelativePaths` existe et exige un Git récent.

**Avertissement officiel (section BUGS) :** « Multiple checkout in general is still experimental, and the support for submodules is incomplete. It is NOT recommended to make multiple checkouts of a superproject. »

**Ce qu'il ne fait PAS.** Aucune coordination entre agents, aucun lease/epoch, aucun CAS, aucune capture de reçu, aucune politique de hooks, aucune analyse d'impact, aucun nettoyage automatique fiable, aucune protection contre un processus qui écrit hors de son worktree. Coût disque : un checkout complet par worktree (objets partagés).

### 2.7 Sapling — autre SCM, GPL

**Faits.** `meta/sapling` 0.2.20261103-145137+6282ac56 (publié 2026-11-04), **GPL-2.0** (API GitHub), zip Windows x64, 7,2 k★, Rust. Modèle à commits éditables, smartlog, sans staging. Pas d'équivalent worktree documenté dans ce que j'ai lu.

**Ce qu'il ne fait PAS.** Aucune vérification, aucune politique multi-agents, aucun reçu. Licence GPL-2.0 : toute dérivation distribuée déclenche des obligations de source ; l'invocation en processus séparé est la seule frontière raisonnable (voir §4).

### 2.8 Graphite — service de revue, dépendance GitHub

**Faits.** `graphite.com` se présente comme « AI code review platform » ; l'inscription est gratuite 30 jours puis payante ; les liens « Contact » pointent vers `cursor.com` (relation capitalistique `NOT_REVIEWED`, non affirmée). Le dépôt `withgraphite/graphite-cli` renvoie **404** ; l'organisation expose `graphite-desktop` en AGPL-3.0 (archivé) et une doc CLI (`gt`). Licence exacte du CLI : `NOT_REVIEWED`.

**Ce qu'il ne fait PAS.** Pas d'exécution locale, pas d'isolation, pas de vérification ; dépend de GitHub et d'un service distant.

### 2.9 GitHub merge queue — le gate final, pas l'isolation

**Faits (`SOURCE_READ`).** Disponible « in any public repository owned by an organization, or in private repositories owned by organizations using GitHub Enterprise Cloud » ; s'utilise via « Merge when ready » ou `gh pr merge` ; un PR peut être retiré si un check de groupe échoue ou après timeout ; nécessite une branche protégée et des checks requis.

**Ce qu'il ne fait PAS.** Pas de local, pas d'offline, pas de test du travail non commité, pas d'isolation d'agents. Il re-qualifie un SHA de groupe : c'est précisément la raison pour laquelle un reçu local doit être invalidé si le tree change (cf. V6 §4.2 assertion 10).

### 2.10 Outils de build — à consommer, pas à réinventer

| Outil | Licence | Rôle | Ce que Flow ne doit PAS refaire |
|---|---|---|---|
| Cargo 0.102.0 | MIT OR Apache-2.0 | graphe de crates, features, `cargo metadata` | résolution de dépendances/features, fingerprints |
| Nx | MIT | graphe de projets TS/JS, `affected` | analyse d'impact JS/TS, cache de tâches |
| Bazel | Apache-2.0 | build/test multi-langage, sandbox d'actions | hermétisme, graphe d'actions |
| Buck2 | Apache-2.0 | build distribué (Meta) | ordonnancement de build |
| Pants | Apache-2.0 | build Python/JVM/Go | résolution d'environnements |
| sccache 0.19.0 | Apache-2.0 | cache de compilateurs (local/remote) | cache d'objets de compilation |

**Constat.** Aucun de ces outils ne fournit d'isolation VCS, d'identité de commit par agent, de hooks ordonnés, de reçu ou de publication CAS. Flow doit les **appeler** (ou lire leurs graphes) et lier le cache à `tree + contexte + toolchain + flags + politique`, jamais réimplémenter un build system.

---

## 3. O1 crédible : quel assemblage exact, quelles limites, quel coût

### 3.1 Définition proposée de O1 (la plus crédible)

**O1-A — Git + worktrees + hooks natifs + CI impactée + merge queue optionnelle.**

1. Un dépôt Git normal, une branche d'intégration (`dev`).
2. Un **worktree éphémère par agent** : `git worktree add --detach <chemin-hors-dépôt> <base>` ; chemin sous un répertoire temporaire dédié ; `git worktree remove` + `prune` en fin de tâche.
3. Chaque agent commite sur une branche temporaire (`cand/<agent>/<id>`) ; les **hooks Git natifs** s'exécutent dans le worktree, donc dans l'ordre Git réel, sans toucher le dossier utilisateur.
4. **Intégration sérialisée** par un unique script : rebase ou merge sur `dev`, puis `git update-ref` (CAS sur l'ancien OID) ou merge protégé ; en cas d'échec, le candidat reste sur sa branche, rien n'est écrasé.
5. **Vérification** : CI impactée (Cargo/Nx/pytest/CMake) sur le SHA candidat, E2E requis bloquants ; option merge queue GitHub pour re-qualifier le SHA final.
6. **Preuve minimale** : log CI attaché au SHA, `git cat-file -p <commit>` pour vérifier le tree, artifacts conservés.

**Coût de maintenance.** Un script d'orchestration (de l'ordre de 200–500 lignes), une politique de nettoyage des worktrees, une CI impactée correcte, une gestion Windows (chemins longs, antivirus, `worktree.useRelativePaths` si Git récent). Aucun nouveau format, aucune migration, aucun daemon.

**Limites honnêtes.**
- Pas de garantie que l'arbre **testé** est l'arbre **publié** après rebase/merge : la CI doit re-tourner sur le SHA final, ou le reçu doit être invalidé.
- Pas de reçu structuré (`BuildContextId`, `PolicyVersion`, `ExecutionBoundaryId`) : la preuve est un log, pas un objet vérifiable.
- Pas de fencing contre les écritures `git` externes ni contre un agent qui écrit hors de son worktree.
- Pas de sandbox OS : les hooks/scripts voient les secrets de l'utilisateur.
- Les hooks peuvent être non idempotents ; rien ne garantit leur ordre hors d'un `git commit` normal.
- Windows : worktrees et sous-modules documentés comme fragiles ; antivirus peut verrouiller les fichiers.
- Coût disque : un checkout complet par worktree.

### 3.2 Variantes O1 et verdict

| Variante | Gain | Perte | Verdict |
|---|---|---|---|
| **O1-A** Git + worktrees + CI | Isolation réelle, hooks natifs, zéro nouveau runtime | Pas de reçu, pas de CAS structuré, pas de sandbox | **Meilleur candidat O1** ; à benchmarker contre O2 |
| **O1-B** jj workspaces | Modèle de conflits, undo, snapshot continu | **Aucun hook** (.gitattributes/submodules/LFS absents) ; interop Git partielle ; courbe d'apprentissage | Disqualifié si la politique exige des hooks ; sinon complément possible côté développeur, pas côté gate |
| **O1-C** GitButler + `but worktree` | UX agents, undo, branches parallèles | FSL interdit l'embarquement dans un produit concurrent ; l'isolation vient des worktrees | Acceptable comme **outil utilisateur optionnel**, jamais comme dépendance embarquée |
| **O1-D** Scion | Orchestration, conteneurs, rôles | Hub obligatoire, images, Windows serveur non supporté, pré-1.0 | Non pour le MVP local-first ; référence de conception seulement |

### 3.3 Coût total comparé (qualitatif, à falsifier par S9)

- **O1-A** : ~1 script + CI existante + temps humain de revue. Pas de nouvelle surface de panne.
- **O2 (Flow mince)** : schémas versionnés, journal durable, writer avec verrou + epoch + CAS, réconciliation, reprise après crash, tests négatifs, migrations. Surface de panne et coût de maintenance nettement supérieurs ; bénéfice seulement si la **garantie** (arbre exact + reçu) a une valeur mesurable.
- **O3** : O2 + matérialisation jetable par exécution ; ajoute disque, latence et cycle de vie.
- **O0** : Git + worktrees + CI, c'est-à-dire O1-A sans le script ; la seule différence est l'automatisation de l'intégration.

**Prédiction falsifiable (PROPOSED)** : sur des tâches à fichiers disjoints, O1-A et O2 devraient être dans le bruit ; O2 ne devrait gagner que sur les cas « même fichier / dirty / re-publication après rebase / preuve exigée ». Si O2 ne gagne pas là, il ne gagne nulle part.

---

## 4. Licences et compatibilité (documenté, pas un verdict juridique)

### 4.1 Tableau des obligations

| Licence | Composants observés | Obligations principales | Risque pour un Flow distribué |
|---|---|---|---|
| **Apache-2.0** | jj, Scion, Bazel, Buck2, Pants, sccache | Conserver licence + NOTICE, indiquer les modifications, clause brevets avec termination en cas de litige | Faible ; compatible avec un produit propriétaire ; **ne pas supprimer NOTICE** |
| **MIT** | Nx, Cargo (dual) | Attribution, notice | Très faible |
| **MIT OR Apache-2.0** | Cargo | Au choix | Très faible |
| **GPL-2.0-only** | Git (`COPYING` : « the only valid version … is _this_ particular version »), Sapling | Distribution d'un dérivé ⇒ source complète sous GPL-2.0 ; pas de liaison propriétaire | Élevé si du code est copié/lié ; invocation de `git`/`sl` en processus séparé = frontière usuelle, non un avis juridique |
| **AGPL-3.0-only** | Zed, `graphite-desktop`, `delta-claude` | Copyleft réseau : l'usage via réseau déclenche la mise à disposition du source | Élevé pour tout composant serveur/service copié |
| **FSL-1.1-MIT** | GitButler | Usage interne/éducation/recherche OK ; **Competing Use** interdit (produit/service commercial qui se substitue au logiciel ou offre une fonctionnalité substantiellement similaire) ; conversion MIT après 2 ans ; pas de droits de marque | **Élevé** : embarquer GitButler dans un Flow commercial « équivalent » est interdit ; l'appeler comme outil externe installé par l'utilisateur est la voie la moins risquée |
| **Aucune licence** | Ravum, Ravum-flow | Droit d'auteur par défaut : pas de droit de copie, de modification ni de distribution au-delà de ce que GitHub permet | **Bloquant** : aucune extraction de code/doc possible sans concession écrite ; aucune distribution de Flow possible |

### 4.2 Lecture FSL-1.1-MIT (texte intégral lu)

- « Permitted Purpose is any purpose other than a Competing Use. »
- Competing Use : produit/service commercial qui (1) se substitue au logiciel, (2) se substitue à un autre produit du concédant, ou (3) « offers the same or substantially similar functionality ».
- Redistribution : inclure la licence ou un lien, conserver les notices.
- Brévets : licence implicite limitée aux usages permis ; termination en cas d'attaque.
- Future license : MIT à la **deuxième date anniversaire** de la mise à disposition de chaque version.
- Copyright observé : « 2023-2024 GitButler Inc » (années non mises à jour dans le fichier).

**Conséquences documentées, non un verdict :** (a) utiliser `but` sur ses propres dépôts = permis ; (b) fournir un service qui « fait la même chose » que GitButler en l'embarquant = risque réel ; (c) copier du code GitButler dans Flow = interdit tant que la version n'est pas passée en MIT ; (d) un Flow qui se contente d'orchestrer un `but` installé séparément n'embarque pas le logiciel — l'analyse dépend de l'interface et du packaging, à faire valider.

### 4.3 Absence de licence côté Ravum et côté Flow

- `Rwanbt/Ravum` : endpoint `/license` → 404, `license: null`. Aucun `LICENSE` observé via l'API. Le droit d'auteur par défaut s'applique : **pas** de droit de copier, modifier, distribuer ; le fork GitHub n'est pas une licence.
- `Rwanbt/Ravum-flow` : `license: null` également. Un dépôt public sans licence ne peut pas recevoir de contributions externes sereinement (pas de clause d'entrée) et ne peut pas être distribué.
- Conséquence pour H2 : la question « peut-on extraire du code Ravum ? » est prématurée ; la première question est « qui possède quoi, et sous quelle licence Flow sera-t-il distribué ? ». Tant que ce n'est pas écrit, l'option **from scratch** reste la seule voie sûre.

### 4.4 Combinaisons à risque (à documenter, pas à trancher)

1. Code GPL-2.0 (Git/Sapling) copié dans Flow ⇒ Flow entier sous GPL-2.0 s'il est distribué.
2. Code AGPL-3.0 (Zed/Delta plugin) copié dans un service Flow ⇒ obligation de source pour les utilisateurs réseau.
3. Code FSL (GitButler) copié ou embarqué ⇒ violation de Competing Use tant que la version n'est pas MIT.
4. Code Ravum sans licence ⇒ aucune base légale.
5. Apache-2.0 dans un projet GPL-2.0-only ⇒ incompatibilité de clauses brevets débattue ; à éviter par séparation de processus.
6. Apache-2.0 sans NOTICE ⇒ violation de forme, risque de réclamation.

---

## 5. Table de décision O0/O1/O2/O3

### 5.1 Matrice

| Critère | O0 (ne rien construire) | O1 (assemblage existant) | O2 (Flow mince) | O3 (hybride) |
|---|---|---|---|---|
| **Fiabilité / perte de données** | Bonne si discipline ; aucune garantie formelle | Bonne : worktrees + Git ; hooks natifs ; pas de reçu | À prouver : journal, CAS, réconciliation | À prouver : idem + matérialisation |
| **Isolation FS réelle** | Oui si un worktree par agent (sinon non) | Oui (worktrees) | Non par défaut (`MANAGED_PATCH` logique) | Oui (vue jetable) |
| **Overhead** | Faible | Faible-moyen (disque par worktree) | Moyen (control plane, journal) | Moyen-élevé (disque, copie, latence) |
| **Multi-langage** | Oui (outils natifs) | Oui (CI impactée) | Oui via adaptateurs | Oui via adaptateurs |
| **Offline** | Oui | Oui (hors forge) | Oui (conçu local-first) | Oui |
| **Droits / licences** | Aucun nouveau risque | Outils Apache/MIT/GPL appelés, pas copiés | Dépend des composants choisis | Idem |
| **Maintenance** | ~0 | 1 script + CI + nettoyage | Élevée (schémas, recovery, tests) | Très élevée |
| **Sécurité (secrets/sandbox)** | Faible : hooks voient tout | Faible-moyenne : hooks dans worktree, pas de sandbox | Moyenne si sandbox implémentée | Moyenne-haute si sandbox qualifiée |
| **Preuve « arbre testé = arbre publié »** | Non | Non (log CI seulement) | Oui (reçu + CAS) | Oui |
| **Windows** | Oui | Oui (worktrees, caveats) | À qualifier | À qualifier |
| **Coût total 16 Go** | Minimal | Faible | Moyen | Moyen-élevé |

### 5.2 Ce qui manque comme preuve avant de trancher

1. **S9 apparié** O1-A vs O2 vs O0 sur charges identiques (fichiers disjoints, même fichier, rebase, dirty, binaire, E2E rouge) — sans exécution, aucun chiffre n'est revendicable.
2. **Comportement Windows réel** des worktrees + hooks + antivirus + chemins longs (doc Git elle-même dit « experimental »).
3. **Ordre des hooks dans un worktree** (HOOK-02c) : la doc Git décrit l'ordre pour un commit ordinaire ; à vérifier sur fixture.
4. **Comportement de jj sous deux agents** dans le même workspace (écrasement) et coût de `jj workspace` sur 16 Go.
5. **Interprétation FSL** par un conseil : l'usage de `but` comme outil externe vs embarqué.
6. **Coût CI** de la re-qualification (merge queue, minutes Actions) : non mesuré.
7. **Droits Ravum/Flow** : aucun document de propriété ou de licence observé.
8. **Version exacte de Scion** : la page `releases/latest` renvoie 404 ; seules les docs `v0.3.0-preview.*` sont visibles.

---

## 6. Réponse contradictoire : une solution existante bat-elle Flow ?

### 6.1 Réponse directe

**Oui pour l'orchestration et le confort ; non pour la garantie.** Aucun outil observé ne fournit la chaîne complète « proposition → arbre Git exact gelé → exécution isolée → reçu lié à l'arbre + contexte + politique → CAS de ref → réconciliation non destructive ». En revanche, l'essentiel de ce que Flow appelle « orchestration » est déjà disponible, gratuitement, hors ligne, sur Windows :

| Besoin Flow | Meilleur existant | Écart restant |
|---|---|---|
| Modèle de commits sans staging, undo, conflits | jj | hooks absents, interop partielle |
| Branches parallèles + UX agents + undo | GitButler | FSL ; pas d'isolation sans worktrees |
| Isolation réelle par agent | `git worktree` | pas de coordination, pas de reçu |
| Orchestration multi-agents + conteneurs | Scion | Hub obligatoire, Windows serveur non supporté, pré-1.0 |
| Trace de la conversation et des éditions | Delta/DeltaDB | bêta, remote obligatoire, pas de vérification |
| Revue/stacked PR/merge queue | Graphite, GitHub | SaaS, online |
| Build/test impactés | Cargo, Nx, Bazel, Pants, sccache | aucun reçu, aucune identité d'arbre |

### 6.2 Où l'existant perd réellement

1. **Identité de l'objet vérifié** : aucun outil ne lie un résultat de vérification à un `GitTreeOid` + `BuildContextId` + `PolicyVersion`. Un log CI est attaché à un SHA, pas à un arbre reproductible.
2. **Fencing de publication** : `git update-ref` CAS existe, mais aucun outil ne combine verrou writer + epoch durable + reçu + reprise idempotente. Les écritures `git` externes restent hors de toute promesse.
3. **Ordre des hooks en espace isolé** : jj n'a pas de hooks ; GitButler n'orchestre pas les hooks ; les worktrees les exécutent mais sans capture structurée des effets sur l'index.
4. **Statuts honnêtes** : aucun outil ne distingue `UNKNOWN/STALE/UNSUPPORTED` d'un `PASS` de politique.
5. **Réconciliation non destructive** : aucun outil ne garantit qu'aucun fichier dirty/untracked n'est écrasé après publication.

### 6.3 Verdict opérationnel

- Si la proposition de valeur de Flow est « plusieurs agents qui commitent plus vite », **O1-A la bat** : moins de code, moins de maintenance, offline, Windows, licences saines.
- Si la proposition est « je peux prouver que le livrable publié est exactement celui qui a été vérifié », **aucun existant ne le fait** ; c'est le seul espace défendable, et il est étroit.
- Le pari raisonnable n'est donc pas « Flow vs Git », mais « Flow = O1-A + un reçu minimal et un writer CAS », c'est-à-dire O2 réduit à sa plus petite surface vérifiable. Tout le reste (VFS, CRDT, daemon, cache distant) est à différer.

---

## 7. Désaccords réels

1. **« Scion Anthropic » (V6 §3.1) est une erreur de source.** Scion est `GoogleCloudPlatform/scion`, Apache-2.0, et dispose d'un Hub local (loopback). L'argument « dépendance serveur/online » reste valable pour le mode hébergé et pour les images, mais la formulation doit être corrigée — sinon le dossier perd de la crédibilité sur une source vérifiable.
2. **Le V6 sous-estime `git worktree` comme baseline.** La doc Git qualifie le multiple-checkout d'« experimental » et déconseille les superprojets à sous-modules, mais pour un dépôt mono-langage sans sous-modules, c'est une isolation réelle et gratuite. La baseline O0 honnête n'est pas « dossier partagé + garde », c'est « Git + worktrees + CI ».
3. **jj est un mauvais candidat pour le gate de vérification.** Il n'a **aucun hook** et toute commande snapshotte le working copy ; l'utiliser dans l'espace de vérification complique le gel « après hooks ». Il reste excellent comme VCS de développement, pas comme composant du contrat Flow.
4. **GitButler n'est pas un outil d'isolation.** La doc elle-même oppose branches parallèles (même dossier) et worktrees (isolation). Le classer en O1 « isolation » sans préciser `but worktree` est trompeur.
5. **Le risque de licence le plus immédiat n'est pas FSL, c'est l'absence de licence de Flow.** Aucun des deux dépôts n'a de licence ; la distribution et les contributions sont bloquées avant même la question de l'extraction Ravum.
6. **La V6 suppose que O1 échouera faute d'API/garanties.** C'est plausible, mais non démontré : O1-A avec un script de publication CAS + CI impactée couvre une partie du contrat à un coût très inférieur. Le fardeau de la preuve est sur O2/O3, pas sur O0/O1.

---

## 8. Limites et NOT_REVIEWED

| Élément | Statut | Impact sur la conclusion |
|---|---|---|
| Comportement réel des outils (aucun test, aucun build) | `NOT_REVIEWED` | Toutes les affirmations sont documentaires ; S9 seul peut falsifier |
| Licence du CLI Graphite | `NOT_REVIEWED` (repo 404) | Graphite reste hors périmètre O1 ; aucune recommandation d'embarquement |
| Licence de DeltaDB / Delta client | `NOT_REVIEWED` (aucun dépôt public) | Ne pas intégrer ; le plugin AGPL-3.0 est le seul élément vérifiable |
| Relation Graphite ↔ Cursor | `NOT_REVIEWED` | Non utilisée dans le raisonnement |
| Support Windows de Buck2, Pants, Bazel et Nx | `NOT_REVIEWED` (non testé, doc OS non relue) | Ces outils ne sont cités que comme « ne pas réinventer » ; leur disponibilité Windows n'est pas un critère de la décision O1 |
| Version exacte de Scion | `NOT_REVIEWED` (`releases/latest` 404 ; docs `v0.3.0-preview.*`) | Le caractère pré-1.0 est certain ; le numéro exact ne l'est pas |
| Comportement Windows réel de GitButler/jj | `NOT_REVIEWED` (installeurs observés seulement) | Le support est documenté, pas exécuté |
| Propriété/intentions de licence Ravum | `NOT_REVIEWED` (aucun document) | H2 reste bloquant ; from scratch par défaut |
| Compteur d'issues Flow 47 → 48 | `UNKNOWN` (dérive possible) | Sans effet sur les conclusions |
| Coût CI / minutes Actions | `NOT_REVIEWED` | Le coût total O1 n'est pas chiffré |
| Accès API GitHub (403 rate limit) | contourné par fichiers bruts | Métadonnées ponctuelles possiblement datées |

**Impact global.** La réponse « une solution existante bat Flow sur l'orchestration, pas sur la garantie » repose sur des capacités **documentées** et des licences **lues**, pas sur des mesures. Elle est donc suffisante pour disqualifier un O2 large et pour exiger un S9 honnête, mais pas pour trancher H1. Le seul point bloquant non technique est H2 : sans licence écrite, ni Ravum ni Flow ne peuvent être distribués.

---

## 9. Références (URL + date de consultation + version observée)

Consultation : **2026-11-09** (Europe/Paris), sauf mention contraire.

1. `https://api.github.com/repos/Rwanbt/Ravum` — 2026-11-09 — `license: null`, 91 issues, public.
2. `https://api.github.com/repos/Rwanbt/Ravum/commits/dev` — 2026-11-09 — `5408d9b3…`, 2026-11-09T12:46:36Z.
3. `https://api.github.com/repos/Rwanbt/Ravum/license` — 2026-11-09 — HTTP 404.
4. `https://api.github.com/repos/Rwanbt/Ravum-flow` — 2026-11-09 — `size: 0`, `license: null`, 48 issues ouvertes.
5. `https://github.com/Rwanbt/Ravum-flow/pull/33` — 2026-11-09 — PR « docs: initialize Ravum Flow charter… ».
6. `https://api.github.com/repos/jj-vcs/jj` — 2026-11-09 — Apache-2.0, 32 421★.
7. `https://api.github.com/repos/jj-vcs/jj/releases/latest` — 2026-11-09 — v0.47.1, publié 2026-11-05, assets Windows MSVC x86_64/aarch64.
8. `https://docs.jj-vcs.dev/latest/git-compatibility/` — 2026-11-09 — hooks No, `.gitattributes` No, submodules No, LFS No, `git-worktree` No.
9. `https://docs.jj-vcs.dev/latest/windows/` — 2026-11-09 — EOL, symlinks, WSL, PowerShell.
10. `https://docs.jj-vcs.dev/latest/technical/architecture/` — 2026-11-09 — `Workspace` ≈ worktree, `GitBackend`, `refs/jj/keep`.
11. `https://docs.jj-vcs.dev/latest/technical/concurrency/` — 2026-11-09 — lock-free, merge 3-way, corruption possible (#2193).
12. `https://api.github.com/repos/gitbutlerapp/gitbutler` — 2026-11-09 — licence GitHub `NOASSERTION`.
13. `https://raw.githubusercontent.com/gitbutlerapp/gitbutler/master/LICENSE.md` — 2026-11-09 — FSL-1.1-MIT, texte intégral.
14. `https://api.github.com/repos/gitbutlerapp/gitbutler/releases/latest` — 2026-11-09 — `release/0.23.2`, 2026-11-02.
15. `https://gitbutler.com/downloads` — 2026-11-09 — MSI Windows x64, `.deb`/`.rpm`, 0.23.2 ; notes 0.23.0 (worktrees).
16. `https://docs.gitbutler.com/getting-started` — 2026-11-09 — « no account », `but`, `but worktree new`, `but agent setup`.
17. `https://docs.gitbutler.com/parallel-branches` — 2026-11-09 — « all branches in the same folder », `gitbutler/workspace`.
18. `https://api.github.com/repos/GoogleCloudPlatform/scion` — 2026-11-09 — Apache-2.0, Go, créé 2026-03-10, 2 106★.
19. `https://raw.githubusercontent.com/GoogleCloudPlatform/scion/main/README.md` — 2026-11-09 — Hub obligatoire, conteneurs, worktrees/clones.
20. `https://googlecloudplatform.github.io/scion/getting-started/install/` — 2026-11-09 — Windows = Hub client only ; Git ≥ 2.48 ; Docker/Podman.
21. `https://api.github.com/repos/zed-industries/zed` — 2026-11-09 — 94 563★ ; `license: NOASSERTION` ; `LICENSE-APACHE` présent.
22. `https://raw.githubusercontent.com/zed-industries/delta-claude/main/README.md` — 2026-11-09 — plugin AGPL-3.0-only, Windows, Delta CLI.
23. `https://delta.dev/deltadb` et `https://delta.dev/docs/concepts/delta-and-git` — 2026-11-09 — DeltaDB, remote obligatoire, deux remotes, jj early.
24. `https://delta.dev/pricing` — 2026-11-09 — Personal 0 $, Pro 10 $/mois.
25. `https://git-scm.com/docs/git-worktree` — 2026-11-09 — doc Git 2.56.0 (2026-09-28) ; BUGS « multiple checkout experimental ».
26. `https://raw.githubusercontent.com/git/git/master/COPYING` — 2026-11-09 — GPL-2.0-only.
27. `https://api.github.com/repos/meta/sapling` — 2026-11-09 — GPL-2.0, 7 245★.
28. `https://api.github.com/repos/meta/sapling/releases/latest` — 2026-11-09 — `0.2.20261103-145137+6282ac56`, Windows x64.
29. `https://graphite.com/` — 2026-11-09 — SaaS de revue ; `graphite.com/docs/merge-queue` → 404.
30. `https://api.github.com/orgs/withgraphite/repos` — 2026-11-09 — `graphite-cli` absent ; `graphite-desktop` AGPL-3.0 archivé.
31. `https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/merging-a-pull-request-with-a-merge-queue` — 2026-11-09 — disponibilité, checks de groupe, retrait sur échec/timeout.
32. `https://raw.githubusercontent.com/rust-lang/cargo/master/Cargo.toml` — 2026-11-09 — `license = "MIT OR Apache-2.0"`, version 0.102.0.
33. `https://api.github.com/repos/nrwl/nx` — 2026-11-09 — MIT.
34. `https://api.github.com/repos/bazelbuild/bazel` — 2026-11-09 — Apache-2.0.
35. `https://api.github.com/repos/facebook/buck2` — 2026-11-09 — Apache-2.0.
36. `https://api.github.com/repos/pantsbuild/pants` — 2026-11-09 — Apache-2.0.
37. `https://api.github.com/repos/mozilla/sccache` — 2026-11-09 — Apache-2.0.
38. `https://api.github.com/repos/mozilla/sccache/releases/latest` — 2026-11-09 — v0.19.0, 2026-10-24, assets Windows MSVC.

---

**Signature agent F :** rapport brut produit sans exécution, sans installation, sans modification hors de ce fichier. Les points 7.1 (attribution Scion), 7.3 (jj sans hooks), 7.5 (Flow sans licence) et 3.1 (O1-A) sont mes contributions principales à la consolidation ; ils doivent être confrontés aux rapports A, B, C, E et H avant toute décision H1/H2.
