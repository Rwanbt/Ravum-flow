# DEEPSEEK-RAW-B — Audit sécurité indépendant (sandbox, secrets, hooks, chemins, crash)

**Agent :** B — Security (sous-agent indépendant, avis propre, non consolidé)
**Date d'observation :** 2026-11-09, Europe/Paris
**Dépôt de travail :** `D:\App\Ravum-flow`, branche `docs/deepseek-d0-d4-review`, HEAD `1d5fddf` (issue de `dev`)
**Cible auditée :** `Rwanbt/Ravum@dev` = `5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96`, **lecture seule via raw.githubusercontent.com**
**Statut du document :** `RAW-OPINION / NOT_CONSOLIDATED` — aucun test exécuté, aucun build, aucun sandbox lancé, aucun fichier Ravum touché.

---

## 1. Mission, périmètre et méthode

### 1.1 Question posée

Modèle de menace et **garanties réellement atteignables** pour Ravum Flow sur deux OS de référence (Windows 11 famille/pro, Linux desktop sans root), sans exécuter de code : héritage d'environnement, hooks Git malveillants ou bogués, builds/tests non fiables, TOCTOU, traversée de chemins, symlinks/jonctions, Unicode/casse, chemins réservés Windows, LFS/submodules, secrets dans logs/artefacts, durabilité après kill/crash/coupure. Question contradictoire imposée : **quelles garanties ne faut-il PAS promettre ?**

### 1.2 Ce que j'ai réellement fait

1. Lecture des documents internes obligatoires : `AGENTS.md`, `docs/DEEPSEEK_GOAL.md`, V6 §0–§10 (l. 1–460), `docs/SECURITY_TESTS.md`, `docs/adr/FLOW-ADR-004/010`, `docs/rfc/RF-RFC-010/011`, `docs/RAVUM_BOUNDARY.md`, `docs/BACKLOG.md`.
2. Téléchargement de **6 fichiers source** Ravum au SHA épinglé dans `%TEMP%\ravum-sec\`, puis **vérification d'identité par blob SHA** (`git hash-object`, opération locale sur fichier temporaire, aucun dépôt Ravum ouvert) :
   - `git_writer.rs` → `0194caadac4c695bdc758f2f0f17c27d1d961c3c` ✅ (2012 lignes)
   - `bin/common/mod.rs` → `783b87cfbacee3d95a502ee6006197c870d270d8` ✅ (743 lignes)
   - `bin/ravum-collab.rs` → `968f03bc2107a6542f42dc98d56ba6e919a4f736` ✅
   - `path_policy.rs` → `2486d3bc5c81946af1550695f5a12ff00d627cff` (**non fourni par le Lead**, ajouté par moi)
   - `publication_journal.rs` → `ba08ab25ce4700b24e7ee22e7a7002a8a1198f8c` (**ajouté par moi**)
   - `coordinator.rs` → `a56f0ed88dae7e08872360cb2932e557d2bb65f0` (**ajouté par moi**)
3. Lecture de sources primaires constructeurs (Microsoft Learn, man-pages 6.19, README bubblewrap) pour la matrice de capacités OS ; URLs en §7.
4. Vérifications locales dans Flow : aucun fichier source (`*.rs/ts/py/go/c/cpp/toml/json`) dans le dépôt, aucun `.github/workflows/` — cohérent avec « Flow sans code ».
5. **Je n'ai pas ouvert `D:\App\Ravum`** (clone d'une autre session), n'ai lancé aucun `git`, build, test ou sandbox sur Ravum.

### 1.3 Bornes de méthode

- Les constats `CODE_READ` prouvent la **présence d'un chemin de code**, jamais une perte ou une fuite reproduite. Il n'y a **aucun `REPRODUCED`, aucun `MEASURED`, aucun `PASS`** dans ce rapport.
- L'API GitHub (`/repos`, `/license`, `/contents`) a renvoyé **403 rate-limit** pendant ma session : les chiffres hérités (47 issues, PR #33, absence de protection de branche, 42 ADR) ne sont **pas re-vérifiés par moi** et restent `NOT_REVIEWED (baseline Lead)`. En revanche `raw.githubusercontent.com` fonctionnait : j'ai re-vérifié moi-même l'**absence de LICENSE** (4 noms testés en 404) et la présence de `docs/adr/README.md` (200).
- Une source primaire non récupérée dans ma session est marquée `NOT_REVIEWED`, jamais citée de mémoire comme si je l'avais lue.

### 1.4 Étiquettes utilisées

`CODE_READ` (lu dans le code au SHA épinglé) · `SOURCE_READ` (doc primaire éditeur/OS lue) · `PROPOSED` (recommandation, non implémentée) · `NOT_REPRODUCED` (chemin observé, effet non reproduit) · `UNKNOWN` · `NOT_REVIEWED`.

---

## 2. Constats

### 2.1 Tableau des constats

| ID | Constat | Preuve + étiquette | Impact |
|---|---|---|---|
| `ENV-01` | **P0-ENV : CONFIRMÉ, non réfuté.** Les hooks Git reçoivent l'environnement complet du parent ; seules **8** variables de *redirection Git* sont retirées. Aucun `env_clear()`. | `git_writer.rs` l.85-94 `REDIRECTING_GIT_ENV = [GIT_DIR, GIT_WORK_TREE, GIT_INDEX_FILE, GIT_OBJECT_DIRECTORY, GIT_ALTERNATE_OBJECT_DIRECTORIES, GIT_NAMESPACE, GIT_PREFIX, GIT_COMMON_DIR]` ; l.1353-1355 `for key in REDIRECTING_GIT_ENV { command.env_remove(key) }` ; l.1336-1359 `run_hook_process`. — `CODE_READ` | `HOME`/`USERPROFILE`, `PATH`, `SSH_AUTH_SOCK`, `GH_TOKEN`, `AWS_*`, `NPM_TOKEN`, `GIT_ASKPASS`, `GIT_CONFIG_GLOBAL`, `GIT_SSH_COMMAND` **persistent**. Un hook (même « approuvé ») peut lire `~/.ssh`, signer via l'agent, ou pousser ailleurs. Le retrait est un garde **anti-redirection Git**, pas un garde de secrets. |
| `ENV-02` | **P0-ENV côté builds/tests : CONFIRMÉ, non réfuté.** `rustc` est lancé avec **3** variables retirées, le binaire de test avec **2**. Aucun `env_clear()`. | `bin/common/mod.rs` l.442-450 (`env_remove("RUSTFLAGS")`, `"RUSTC_WRAPPER"`, `"RUSTC_WORKSPACE_WRAPPER"`) ; l.367-372 (`RUSTFLAGS`, `RUSTC_WRAPPER` seulement) ; grep `env_clear` = 0 occurrence dans les 6 fichiers. — `CODE_READ` | Même surface que `ENV-01` pour tout code candidat compilé/exécuté. Le code **l'admet lui-même** : « this is not a sandbox (same user, same machine) » (l.18-19) et « no environment variable of the caller is trusted » (l.21-23) — cette dernière phrase est **fausse au sens littéral** : seuls 3 variables de *toolchain* sont neutralisées, pas les secrets. |
| `ENV-03` | **Le binaire de test candidat hérite du répertoire courant du coordinateur** : aucun `current_dir()` n'est fixé, ni pour `rustc`, ni pour le harnais de test. | `bin/common/mod.rs` : grep `current_dir` = **0 occurrence** ; l.367-372 `Command::new(&exe).stdout(..).stderr(..).env_remove(..)` sans `current_dir`. — `CODE_READ` | Le code non fiable s'exécute avec le cwd de l'agent Flow : un `open("Cargo.toml","w")` ou `./pre-commit` relatif touche le **projet de l'utilisateur**, pas le bac à sable. C'est une fuite d'écriture *sans* symlink ni traversée : il suffit d'un chemin relatif. |
| `ENV-04` | **Fuite de secrets par les logs de test, indépendante de l'environnement.** Les lignes de sortie contenant `panicked at`/`FAILED`/`assertion` (≤5) et un extrait stderr de `rustc` (≤2 KiB) sont injectées dans `MergeDiagnostic` → `verification reason` → **journal de décision durable**. | `bin/common/mod.rs` l.403-425 (filtre + `.take(5)` + `message: line.trim()`), l.44-47 (doc : « reach the durable decision journal »). — `CODE_READ` | Un test qui imprime un jeton (`assert_eq!(token, "ghp_…")`) écrit ce jeton dans un artefact persistant. **`ENV-04` (test V6/SECURITY_TESTS) est mal scopé** : il exige « no leak to logs » en ne testant que l'environnement ; aujourd'hui la fuite passe par les diagnostics, même avec un env parfait. |
| `ENV-05` | Sortie de hook **capturée** (stdout+stderr concaténés, tronqués à 8 KiB) et renvoyée telle quelle dans le texte d'erreur. Aucune rédaction. | `git_writer.rs` l.1871-1889 `combined_output` + `truncate(MAX_ERROR_BYTES=8*1024)` (l.83) ; usage l.1239 et l.1263. — `CODE_READ` | Un hook bavard (ou piégé) fait remonter des secrets dans l'erreur remontée à l'appelant. Persistance éventuelle = `NOT_REVIEWED` (dépend du CLI, non lu). |
| `HOOK-01` | **P0-HOOK-ORDER : CONFIRMÉ.** `write-tree` **avant** les hooks ; ordre `prepare-commit-msg` → `pre-commit` → `commit-msg` (inversé vs Git) ; hooks exécutés dans le **workspace réel**. | `git_writer.rs` l.841-844 (`write-tree`), l.856 (`run_commit_hooks`), l.868 (`commit-tree` réutilise `tree`) ; ordre l.1221-1228 ; `current_dir(&self.repo)` l.1344. — `CODE_READ` | Les modifications d'index d'un `pre-commit` sont **perdues** pour le tree publié ; un hook peut écrire dans les fichiers non commités de l'utilisateur. Effet non reproduit (`NOT_REPRODUCED`). Le handoff V6 est exact aux lignes près. |
| `HOOK-02` | Sur POSIX, un hook est ignoré silencieusement s'il n'a pas le bit exécutable ; sur Windows, **tout fichier** est considéré exécutable. | `git_writer.rs` l.1899-1916 `hook_is_runnable` (`#[cfg(unix)] mode & 0o111`, `#[cfg(not(unix))] true`) ; `run_commit_hooks` l.1231-1233 `continue`. — `CODE_READ` | Divergence inter-OS de la politique : un hook exigé peut ne pas s'exécuter sans aucun avertissement (Linux), donc un « gate » déclaré peut être vide. À traiter comme `UNSUPPORTED` explicite, pas comme `continue`. |
| `HOOK-03` | Le hook `post-commit` échoue **après** la publication et est converti en simple `warning` ; c'est correct, mais le warning n'est pas journalisé pour lui-même. | `git_writer.rs` l.1056-1058 (`if let Err(warning) = self.run_post_commit_hook() { warnings.push(warning) }`), l.1038-1048 (outcome ne journalise que `warnings.first()`). — `CODE_READ` | Perte d'information d'audit : plusieurs avertissements post-publication sont réduits au premier. |
| `FS-01` | **La politique de chemins est purement lexicale et le dit explicitement** : « no filesystem access ». Elle refuse `..`, absolu, lecteur `C:`, `:`, `.git`, caractères de contrôle, composant vide. | `path_policy.rs` l.15-16 (« Both answers are pure functions of the path: no filesystem access »), l.56-99 ; tests l.145-179. — `CODE_READ` | Bonne base anti-traversée **lexicale** ; ne peut pas, par construction, voir un lien. |
| `FS-02` | **Les composants parents du chemin ne sont pas contrôlés** : seule la feuille est testée (`symlink_metadata` sur le chemin complet = métadonnées de la cible finale). Une jonction/symlink de répertoire **parent** (`vendor/ → C:\Users\x\.ssh`) n'est pas vue. | `git_writer.rs` l.580-585 et l.1073-1079 : `symlink_metadata(&absolute)` + `is_symlink()` sur la feuille ; `absolute_path` = simple jointure (l.1072) ; `path_policy.rs` ne fait aucun accès FS. — `CODE_READ` | Écriture **hors workspace** possible via un parent reparse point, alors que `FS-01` laisse croire à un confinement. `PATH-08`/`PATH-16` ne peuvent pas passer en l'état. La sémantique exacte de `FileType::is_symlink()` pour une **jonction** Windows (Rust std) est `NOT_REVIEWED` — je ne l'affirme pas. |
| `FS-03` | **Noms réservés Windows et points/espaces finaux non filtrés.** `:` est refusé (donc pas d'ADS), mais `CON`, `PRN`, `AUX`, `NUL`, `COM1..9`, `LPT1..9`, `foo.`, `foo ` sont acceptés. | `path_policy.rs` l.56-99 : aucune liste de noms réservés ni test de suffixe `.`/espace. — `CODE_READ` (absence de contrôle) ; comportement NTFS correspondant = `NOT_REVIEWED` (doc Microsoft non récupérée). | Sur Windows, un chemin accepté peut être refusé/inversé par le FS (création implicite de flux, nom tronqué) → divergence entre le tree Git et le disque, donc reçu `STALE` silencieux ou écriture dans un fichier inattendu. |
| `FS-04` | **Aucune détection de collision de casse ni de normalisation Unicode.** Le seul test insensible à la casse porte sur `.git`. | `path_policy.rs` l.90-92 (`part.eq_ignore_ascii_case(".git")`), aucune autre normalisation ; `canonical_relative_path` l.43-49 ne fait que `\`→`/`. — `CODE_READ` | Deux entrées `README.md` / `readme.md` (ou NFC/NFD) sont valides pour l'index Git mais **une seule** existe sur NTFS/APFS ; le tree publié ne correspond plus au disque. Cas explicitement listé par V6 §4.4 mais **non couvert** par la politique. |
| `FS-05` | **Non-UTF-8 et binaire : refus explicite, pas corruption.** Les chemins et contenus sont des `String` ; un binaire lève `NotUtf8`. | `git_writer.rs` l.589-590 `String::from_utf8(bytes).map_err(|_| GitError::NotUtf8{..})` ; signature `paths: &[String]` l.562-564. — `CODE_READ` | Confirme `P1-FILES` **avec une nuance importante** : le comportement est *fail-closed* (refus), donc pas de perte silencieuse ; mais aucune publication binaire n'est possible → `UNSUPPORTED` à déclarer honnêtement. |
| `FS-06` | **Mode : `100644` pour tout nouveau chemin, mode d'index conservé pour un chemin existant.** | `git_writer.rs` l.814-817 `mode_in_index(&path)?.unwrap_or_else(|| "100644")` ; `mode_in_index` l.1557-1577 (lecture `ls-files -s` sur l'index isolé). — `CODE_READ` | Nuance vs V6 §2 (« 100644 par défaut dans le parcours examiné ») : c'est vrai **uniquement pour un chemin absent de l'index**. Un script neuf exécutable devient non-exécutable au commit : régression silencieuse de bit `100755`. |
| `FS-07` | **Aucun filtre `.gitattributes` n'est appliqué** : le blob est créé par `git hash-object -w --stdin` **sans `--path`**, donc sans conversion CRLF/LFS/`clean`. | `git_writer.rs` l.800-807 ; l'index est écrit ensuite par `update-index --cacheinfo` (l.1170-1190), ce qui **court-circuite** `git add`. — `CODE_READ` (absence de `--path`) + sémantique des filtres = doc Git `NOT_REVIEWED`. | Committed bytes = bytes disque tels quels : CRLF non normalisé, **pointeur LFS jamais généré** (un gros fichier est commité en clair), pas de `smudge` à la réconciliation. `CONTEXT-03` (LFS externe) ne peut pas être satisfait par ce chemin. |
| `FS-08` | **Submodules/gitlinks non représentables** : le contenu est du texte ; le mode n'est hérité que si le chemin existe déjà dans l'index. | `git_writer.rs` l.797-823 (`Some(content)` → texte+blob), l.814-817. — `CODE_READ` | Un gitlink existant peut être *préservé* par accident (mode d'index), mais un submodule ne peut être ni créé ni modifié ; un `160000` ne doit pas être annoncé supporté. |
| `FS-09` | **TOCTOU résiduel entre la re-validation et l'écriture, et écriture non atomique.** | `git_writer.rs` l.1137-1162 : `classify_disk` puis `std::fs::write(&absolute, …)` — pas de `temp + rename`, pas de verrou ; fenêtre entre l.1138 et l.1149. — `CODE_READ` | (a) une édition humaine dans cette fenêtre est écrasée ; (b) une coupure pendant `fs::write` (truncate+write) laisse un fichier **tronqué** alors que la ref est déjà publiée. Le commentaire l.1133-1135 (« left untouched ») est vrai *hors* cette fenêtre seulement. |
| `DUR-01` | **Le journal réel n'est pas SQLite** : c'est un JSONL append-only, un `write_all` + `File::sync_all()` par enregistrement. | `publication_journal.rs` l.262-274 (`append` → `file.sync_all()`), l.147-161 (`open`), l.277-294 (`read_records`). — `CODE_READ` | V6 §4.5 discute `WAL synchronous=FULL/NORMAL` : vocabulaire **non conforme au code épinglé**. À ce SHA, la durabilité repose sur fsync de fichier, pas sur WAL. |
| `DUR-02` | **Pas de fsync du répertoire parent** après création du journal. | `publication_journal.rs` l.149-152 `create_dir_all` + `OpenOptions::create(true).append(true)` ; aucun `File::open(dir)`+`sync_all`. — `CODE_READ` | Sur POSIX, l'**entrée de répertoire** du premier enregistrement n'est pas garantie après coupure : `INTENT_DURABLE` peut être perdu alors que `sync_all` a réussi. Sur Windows la sémantique diffère (`NOT_REVIEWED`). |
| `DUR-03` | **Aucune somme de contrôle ni longueur par enregistrement** ; une ligne finale déchirée rend `open()` **fatal** (fail-closed), pas tolérant. | `publication_journal.rs` l.285-290 (`InvalidData` si parse échoue), l.143-146 (doc « fails closed »). — `CODE_READ` | Sécurité = pas de fausse histoire ; disponibilité = un crash au mauvais octet peut **empêcher le démarrage** du coordinateur sans réparation manuelle. `RECOVERY-16` n'est pas satisfait par le code actuel. |
| `DUR-04` | **Ordre crash : intent fsyncé → CAS ref → écriture disque → outcome fsyncé.** Un crash entre CAS et réconciliation laisse « ref publiée, disque non réconcilié » — état récupérable, pas échec. | `git_writer.rs` l.881-897 (intent), l.1010-1024 (relecture HEAD, `classify_disk`, puis `compare_and_swap_ref`), l.1029-1048 (apply_disk, index, outcome), l.1050-1054 (`IndexOutOfSync` **après** journalisation de `Published`). — `CODE_READ` | Bon ordre logique ; le code retourne une **erreur** (`IndexOutOfSync`) pour un commit **publié** — piège d'interprétation pour tout appelant qui mappe `Err` → « non publié ». À documenter dans le contrat, pas seulement dans le code. |
| `DUR-05` | **Objets Git non référencés produits avant décision** : `hash-object -w` puis `commit-tree` écrivent dans l'object DB avant le CAS. | `git_writer.rs` l.800-807 (blob), l.866-873 (commit). — `CODE_READ` | Un refus/timeout laisse des blobs+commits **inaccessibles** : `git gc`/`git prune` peut les supprimer → un reçu ou un journal pointant dessus devient non rejouable. Exige des refs de rétention épinglées. |
| `DUR-06` | **Pas de verrou inter-processus ni d'epoch dans le code épinglé** : la seule sérialisation est un `Mutex` **intra-processus** sur l'ensemble des `publication_id` vivants. | `git_writer.rs` l.430, l.442-445, l.1622-1623 (`live_intents: Arc<Mutex<BTreeSet<String>>>`) ; grep `epoch|flock|exclusive|try_lock` dans `git_writer.rs`/`coordinator.rs` = 0 occurrence pertinente. — `CODE_READ` | Le protocole V6 §4.2/4.3 (« verrou exclusif inter-processus couvrant epoch + CAS + outcome ») est **une proposition, pas un état du code**. Cohérent avec `AGENTS.md` (« ref CAS seul ne fence pas ») : ici, même pas le verrou. |
| `DUR-07` | **Aucun contrôle de descendants ni de ressources** : `child.kill()` sur un seul handle, pas de Job Object, pas de `rlimit`, pas de groupe de processus. | `bin/common/mod.rs` l.377-402 (boucle `try_wait` + `TEST_TIMEOUT` + `let _ = child.kill()`). — `CODE_READ` | Une fork bomb ou un test qui détache un démon **survit** au timeout et au retour de Flow ; budget 16 Go non protégé (`RESOURCE-14` impossible en l'état). |
| `SBX-01` | **Windows — Job Objects** : gestion de groupe (limites working set/priorité/temps, `KILL_ON_JOB_CLOSE`, énumération) mais **aucune isolation FS, réseau ou secrets**. Les limites sont « silently ignored » si dépassées, sauf notification. | Microsoft Learn, *Job Objects* (l. « Job Limits and Notifications ») — `SOURCE_READ` | Utile pour `DUR-07`/quotas, **inutilisable comme frontière de sécurité**. À retirer de la liste « sandbox » de V6 §5.1. |
| `SBX-02` | **Windows — Windows Sandbox** : isolation par hyperviseur (noyau séparé), jetable, mais **pas sur Windows Home**, licence Pro/Enterprise/Education requise, **une seule instance**, réseau **activé par défaut** (désactivable par `.wsb`). | Microsoft Learn, *Windows Sandbox* (tableau éditions + notes) — `SOURCE_READ` | Frontière réelle mais **VM interactive**, pas une primitive par test : démarrage lourd, non scriptable en banc de tests parallèle, absente de la cible « Windows 11 famille ». Ne peut pas fonder une promesse « sandbox par exécution ». |
| `SBX-03` | **Windows — AppContainer** : isole credentials, devices, fichiers+registre, réseau (Internet/Intranet/serveur par capacité), processus et fenêtres ; identité composite utilisateur/application. | Microsoft Learn, *AppContainer isolation* — `SOURCE_READ` | **Seul candidat non-VM crédible** pour une frontière FS+réseau sur Windows, mais : compatibilité d'une chaîne `rustc/cargo/git/node` non prouvée, coût de création de profil, exigences admin éventuelles = `NOT_REVIEWED`; ne pas l'annoncer sans test. |
| `SBX-04` | **Windows — jetons restreints** : `CreateRestrictedToken` (privileges retirés, SID deny-only, restricting SIDs) ; `CreateProcessAsUser` avec version restreinte du jeton de l'appelant **ne exige pas** `SE_ASSIGNPRIMARYTOKEN_NAME`. Limite documentée : isoler le processus restreint sur un **desktop non par défaut** (sinon `SendMessage`/`PostMessage`). | Microsoft Learn, *Restricted Tokens* — `SOURCE_READ` | Utilisable en **défense en profondeur**, pas comme frontière unique (pas de contrôle réseau, pas de vue FS propre). Le détail desktop rend l'automatisation non triviale. |
| `SBX-05` | **Windows — WDAC / minifilter** : `NOT_REVIEWED` (pages non récupérées). Par construction ces mécanismes sont **machine-wide**, exigent administrateur/signature de pilote et ne fournissent pas de confinement par projet. | Aucune source primaire lue cette session. | À ne pas citer comme option MVP ; un filtre FS kernel n'est pas distribuable par une CLI utilisateur. |
| `SBX-06` | **Linux — user/mount/pid/net namespaces** : userns non privilégiés depuis 3.8 ; `CAP_SYS_ADMIN` dans l'userns suffit pour créer les autres namespaces ; montage **block-based** réservé au userns initial ; overlayfs en userns depuis 5.11. | man-pages 6.19, `user_namespaces(7)` (§ « Interaction … other namespaces », « Effect of capabilities ») — `SOURCE_READ` | Base d'un sandbox non-root réel : FS (mount+bind), réseau (netns loopback seul), process (pidns), uid (userns). Ne protège **pas** des secrets accessibles par le même uid **hors** des montages refaits. |
| `SBX-07` | **Linux — Landlock** : LSM non privilégié, restrictif et héréditaire ; FS depuis ABI 1 (5.13), TCP bind/connect depuis ABI 4 (6.7), scopes signal/socket abstrait depuis ABI 6 (6.12). Caveats explicites : `chdir`, `stat`, `chmod`, `chown`, `setxattr`, `utime`, `access`, `flock` **non restreignables** ; un FD déjà ouvert avant le sandbox n'est pas soumis. | man-pages 6.19, `landlock(7)` (§ VERSIONS, CAVEATS) — `SOURCE_READ` | Meilleur candidat Linux pour **filet de sécurité en profondeur** sans root, mais dépend de l'ABI noyau (distro récente) et laisse fuiter des métadonnées (`stat`) et des droits (`chmod`) : ce n'est pas une frontière complète à soi seul. |
| `SBX-08` | **Linux — seccomp-bpf** : filtre sur **numéro d'appel système + 6 arguments**, pas sur les chemins ; exige `no_new_privs` ou `CAP_SYS_ADMIN` dans l'userns ; pièges ABI x32 documentés. | man-pages 6.19, `seccomp(2)` (§ DESCRIPTION, Caveats) — `SOURCE_READ` | Ne protège pas un fichier, seulement des primitives. Une allow-list seccomp sur `rustc`/`node` est un projet en soi (risque de casser l'outil) : à traiter comme option, pas comme défaut. |
| `SBX-09` | **Linux — bubblewrap** : construit un mount namespace + tmpfs racine, `--unshare-pid/net`, `PR_SET_NO_NEW_PRIVS` ; **le mode setuid a été supprimé** → dépend des userns non privilégiés ; « not a complete, ready-made sandbox » : la politique est celle de l'appelant ; `--new-session` nécessaire si `TIOCSTI` n'est pas filtré. | README bubblewrap (main) — `SOURCE_READ` | Dépendance externe Linux-only, non garantie sur toutes les distros (restrictions userns possibles) : `UNSUPPORTED` si absente/refusée. **N'existe pas sur Windows** → aucune parité. |
| `SBX-10` | **cgroups v2** (`memory.max`, `pids.max`, `cpu.max`) : `NOT_REVIEWED` cette session (doc noyau non récupérée). | — | Utile pour quotas (`DUR-07`) ; ne pas l'inclure dans une promesse avant lecture et test. |
| `DIST-01` | **Ravum n'a aucune licence** au SHA épinglé : `LICENSE`, `LICENSE.md`, `LICENSE.txt`, `COPYING` → **404** (HTTP vérifié). Le dépôt reste lisible (`docs/adr/README.md` → 200). | `raw.githubusercontent.com/Rwanbt/Ravum/5408d9b…/<nom>` — `CODE_READ` (HTTP), re-vérifié par moi | Aucune réutilisation de code Ravum dans un bundle distribué n'est autorisée par défaut ; « from scratch » doit rester l'option par défaut (H2). |
| `DIST-02` | **Flow n'a ni code ni CI** (vérifié localement : aucun `*.rs/ts/py/go/c/cpp/toml/json`, aucun `.github/workflows/`). | `glob` sur `D:\App\Ravum-flow` — `MEASURED` (local, trivial) | Un « bundle sandbox » n'existe pas et ne peut pas être promis ; toute distribution future est un choix séparé (licence Flow + licences tierces). |

### 2.2 Matrice de capacités — ce qui isole quoi (synthèse)

Lecture : ✅ isole réellement · ◐ partiel / conditionnel · ❌ n'isole pas · ? `NOT_REVIEWED`.

| Mécanisme | FS | Réseau | Process/descendants | Secrets du même uid | Admin / prérequis |
|---|---|---|---|---|---|
| Job Object (Windows) | ❌ | ❌ | ✅ (kill-on-close, limites) | ❌ | non ; mais aucune frontière de sécurité |
| AppContainer (Windows) | ✅ (par capacité) | ✅ (Internet/Intranet/serveur) | ✅ | ◐ (credentials isolés, fichiers hors capacité non lisibles) | ? création de profil / compat toolchain |
| Jeton restreint (Windows) | ◐ (DACL seulement) | ❌ | ❌ | ◐ (deny-only SIDs) | non ; desktop séparé recommandé |
| Windows Sandbox | ✅ (VM) | ✅ (désactivable) | ✅ | ✅ (machine séparée) | **Pro/Ent/Edu**, 1 instance, hyperviseur |
| WDAC / minifilter | ◐ / ✅ | ❌ | ❌ | ❌ | **admin + pilote signé** ; machine-wide |
| namespaces (Linux) | ✅ (mount) | ✅ (netns) | ✅ (pidns) | ◐ (uid map ≠ secrets lisibles via chemins non montés) | non si userns autorisés |
| seccomp-bpf | ❌ | ◐ (par syscall) | ❌ | ❌ | non (`no_new_privs`) |
| Landlock | ✅ (chemins, ABI≥1) | ◐ (TCP, ABI≥4) | ❌ | ◐ (pas de `stat`/`chmod`) | non ; noyau récent |
| bubblewrap | ✅ | ✅ | ✅ | ◐ | non ; paquet externe, userns requis |
| cgroups v2 | ❌ | ❌ | ✅ | ❌ | ? délégation systemd |

**Conclusion de la matrice.** Sur **Windows 11 famille**, il n'existe **aucune** primitive non-VM, non-administrateur, fournissant une frontière FS+réseau pour du code non fiable : la seule frontière réelle est une VM (Windows Sandbox, absente de Home) ou AppContainer (à qualifier, compatibilité toolchain inconnue). Sur **Linux desktop sans root**, une frontière crédible existe (userns+mountns+netns+pidns via bubblewrap, Landlock en profondeur), mais elle dépend de la distribution, du noyau et de l'autorisation des userns non privilégiés.

**Promesse tenable, et une seule :** « sur Linux avec userns autorisés et bubblewrap présent, l'exécution non fiable est confinée FS/réseau/process, avec les limites documentées ; partout ailleurs, Flow refuse (`UNSUPPORTED`) ou exige une action humaine (`MANUAL_REQUIRED`) ». Aucune promesse symétrique Windows n'est tenable aujourd'hui.

### 2.3 Durabilité — trois niveaux à ne pas confondre

| Niveau | Ce que le code épinglé garantit | Ce qu'il ne garantit pas |
|---|---|---|
| **Process kill** (SIGKILL/tueur de test) | intent fsyncé avant CAS ; outcome fsyncé après ; `ensure_not_published` empêche le double publish ; disque/index peuvent rester en arrière (état `NEEDS_RECONCILIATION` implicite). | Rien sur les enfants (`DUR-07`) ; rien sur un fichier en cours de `fs::write` (`FS-09`). |
| **OS crash** (panic noyau, BSOD) | Même ordre que ci-dessus ; le journal relu ligne par ligne, `InvalidData` = fail-closed. | Pas de fsync de répertoire (`DUR-02`) ; pas de checksum → une ligne déchirée bloque `open()` (`DUR-03`). |
| **Power loss** | `File::sync_all()` sur le journal : les octets de l'enregistrement sont poussés vers le support. | Ni le répertoire parent, ni les fichiers de travail (écriture non atomique), ni les objets Git (non référencés → gc). **Aucun ACID multi-systèmes** : ref Git, disque, index, journal et sortie de hook ne partagent pas de transaction. |

Conclusion : le journal Flow peut garantir **« une intention enregistrée ne sera pas oubliée et un identifiant ne sera pas publié deux fois »**. Il **ne peut pas** garantir « le disque reflète la ref après coupure », « aucun fichier tronqué », « les objets candidats survivent à un gc », ni « l'effet du hook est atomique avec le commit ».

---

## 3. Réponse à la question contradictoire : garanties à NE PAS promettre

1. **« `env_clear()` (ou une allowlist) suffit à isoler. »** Faux sur les deux OS : un processus au même uid lit `~/.ssh`, `%APPDATA%`, les sockets d'agent, les DPAPI blobs, les variables de processus parents, `/proc/<pid>/environ` (Linux), et peut écrire où l'ACL l'autorise. Preuve : `ENV-01/02` + `SBX-*`. *Falsification : un canari `HOME`+clé SSH lisible par un test « sandboxé » invalide la promesse.*
2. **« Zéro écrasement en mode non managé. »** Non : un outil externe qui écrit dans le dossier partagé n'est pas observable de façon fiable ; Flow détecte au mieux (`OBSERVED_UNMANAGED`), et `FS-09` montre une fenêtre résiduelle même en mode managé. *Falsification : éditeur externe qui écrit entre `classify_disk` et `fs::write`.*
3. **« Le ref CAS est un fencing. »** Non : `update-ref <new> <old>` compare un OID, pas une époque ; aucun verrou inter-processus ni epoch dans le code (`DUR-06`). Un writer stale peut gagner si l'oid attendu est encore la tête.
4. **« WAL = transaction globale. »** Non : le journal épinglé n'est même pas SQLite (`DUR-01`), et aucun WAL ne couvre Git + disque + index + hooks. `DUR-04` montre l'ordre, pas une atomicité.
5. **« Sandbox universelle / un bundle qui marche partout. »** Non : bubblewrap n'existe pas sur Windows, Windows Sandbox n'existe pas sur Home, AppContainer n'est pas qualifié pour une chaîne Rust. Toute promesse « sandbox » doit être **par OS et par mode**, avec `UNSUPPORTED` explicite (`SBX-01..09`, `DIST-02`).
6. **« Les hooks approuvés sont sûrs. »** Non : un hook est du code arbitraire, exécuté dans le workspace réel (`HOOK-01`), avec l'environnement complet (`ENV-01`) ; « approuvé » ne veut dire que « l'utilisateur l'a autorisé ».
7. **« Les logs/artefacts sont sûrs vis-à-vis des secrets. »** Non : diagnostics de test et sortie de hook sont recopiés (`ENV-04/05`) sans rédaction.
8. **« Le tree publié correspond aux octets du disque. »** Non en général : pas de filtres `.gitattributes` (`FS-07`), mode `100644` pour les nouveaux chemins (`FS-06`), collisions de casse (`FS-04`).
9. **« Les hooks voient leurs modifications dans le tree publié. »** Non dans le prototype : tree figé avant hooks (`HOOK-01`). Flow doit le corriger ou refuser, jamais publier en silence.
10. **« Un `UNKNOWN`/`UNSUPPORTED` peut être promu si tout le reste est vert. »** Non : c'est précisément le faux-vert que la politique doit interdire ; la moindre garantie non mesurée doit rester `NOT_VERIFIED`.

---

## 4. Désaccords réels avec V6 et les documents internes

**D-01 — Job Objects ne sont pas une option de sandbox (désaccord avec V6 §5.1).**
V6 liste « Job Objects / Windows Sandbox / AppContainer » comme un continuum « à évaluer ». Job Objects ne fournissent **aucune** isolation FS/réseau/secrets (`SBX-01`) : ce sont des quotas et un groupe de mort. Les garder dans la liste entretient l'illusion d'une solution Windows légère. *Recommandation : les déplacer dans la colonne « ressources/descendants » et retirer le mot sandbox.*

**D-02 — L'allowlist d'environnement est nécessaire mais insuffisante, et le code le surestime (désaccord avec FLOW-ADR-004 et V6 §5.1).**
FLOW-ADR-004 propose « allowlist env, pas de secrets » comme décision candidate. Une allowlist ne retire pas les secrets **sur disque** ni les canaux hors environnement (agent SSH, keychain, `.env` du projet, cache de credentials). `ENV-02` montre en plus que le commentaire du code (« no environment variable of the caller is trusted ») décrit une intention plus forte que le code (3 variables retirées). *Désaccord : l'allowlist doit être présentée comme hygiène, jamais comme frontière.*

**D-03 — Le test `ENV-04` est mal spécifié (désaccord avec `docs/SECURITY_TESTS.md` et V6 §9).**
`ENV-04` exige « candidate worker cannot read/access, no leak to logs/network » en injectant une variable. Or (a) la lecture de secrets ne passe pas par l'environnement seul, (b) la fuite par **diagnostics** existe indépendamment (`ENV-04` constat). Le test tel qu'écrit peut passer au vert alors que la fuite persiste par le journal, ou échouer pour la mauvaise raison. *Désaccord : il faut au minimum trois tests distincts — `ENV-04a` (env), `ENV-04b` (FS/secrets disque), `ENV-04c` (rédaction des diagnostics/artefacts).*

**D-04 — `PATH-08`/`PATH-16` ne sont pas atteignables avec une politique lexicale (désaccord avec V6 §9).**
V6 attend « aucune écriture hors cible » pour `../`, symlink/jonction, Unicode/casse, chemins réservés. Le code fait une validation lexicale correcte (`FS-01`) mais n'inspecte aucun parent (`FS-02`), ignore les noms réservés (`FS-03`) et les collisions de casse (`FS-04`). Le test échouera sur ses propres fixtures. *Désaccord : le comportement attendu doit devenir `UNSUPPORTED` explicite pour les cas non couverts, pas « écriture bornée ».*

**D-05 — Le vocabulaire de durabilité de V6 ne décrit pas le code (désaccord avec V6 §4.5).**
V6 §4.5 discute `synchronous=FULL` vs `NORMAL` et « WAL impose un stockage local ». Le journal épinglé est un JSONL fsyncé (`DUR-01`), sans WAL, sans checksum, sans fsync de répertoire. Ces deux sujets (SQLite éventuel / journal réel) doivent être **séparés** : sinon un lecteur croit que `FULL` protège un état que le code n'a pas.

**D-06 — « Contrôle des descendants » est un objectif sans mécanisme (désaccord avec V6 §5.4 et `RESOURCE-14`).**
V6 §5.4 demande « contrôle des descendants » et `RESOURCE-14` « pas de deadlock/fuite de processus ». Le seul mécanisme est `child.kill()` sur un handle (`DUR-07`). Sur Windows, sans Job Object + `AssignProcessToJobObject`, un descendant survit ; sur Linux, sans groupe de processus/`pidfd`, idem. *Désaccord : la ligne doit être retirée des objectifs « atteints par conception » et devenir un prérequis bloquant.*

**D-07 — Le contrat « erreur = échec » est faux après publication (nuance que V6 ne formule pas).**
`publish_inner` retourne `GitError::IndexOutOfSync { commit, … }` **après** avoir journalisé `Published` (`DUR-04`). Tout appelant qui traite `Err` comme « rien n'a été publié » produira un rapport mensonger. V6 §4.1 décrit les états mais pas cette asymétrie au niveau du type de retour. *Désaccord : le contrat doit rendre la publication **observable dans l'erreur** (type dédié `PublishedWithResidue`), pas seulement dans le journal.*

---

## 5. Recommandations bornées et conditions de falsification

Toutes `PROPOSED`. Chacune est bornée : ce qu'elle fait, ce qu'elle ne fait pas, et le test qui la réfuterait.

**R1 — Politique H3 par OS, fail-closed (bloquant avant toute promesse).**
- Linux : mode `UNTRUSTED_CANDIDATE` = sandbox **obligatoire** (userns+mountns+netns+pidns, bubblewrap de préférence, Landlock en couche supplémentaire si ABI ≥ 4). Absence de bwrap / userns désactivés / noyau sans Landlock → `UNSUPPORTED`, jamais de repli silencieux.
- Windows : `UNTRUSTED_CANDIDATE` → **`MANUAL_REQUIRED` par défaut**. Seule exception : AppContainer, après un test de compatibilité de la chaîne `rustc/cargo/git/node` et validation des prérequis de profil. Windows Sandbox = option « exécution manuelle assistée » documentée, pas un mode automatique.
- Ne fait pas : aucune promesse de parité, aucun téléchargement/installation automatique de binaire tiers dans le dépôt utilisateur.
- *Falsifiée si* un test montre qu'une exécution non fiable sur Windows 11 Home obtient une frontière FS+réseau sans admin et sans VM — auquel cas la politique peut être élargie.

**R2 — Environnement : allowlist + scrub explicite + preuve d'absence.**
Construire l'environnement enfant **par liste blanche** (`PATH`, `TEMP`, `SYSTEMROOT`, `LANG`, toolchain), jamais par soustraction ; n'ajouter que ce que l'outil déclare. Interdire nommément `HOME`/`USERPROFILE`, `SSH_AUTH_SOCK`, `*_TOKEN`, `*_KEY`, `AWS_*`, `GIT_ASKPASS`, `GIT_CONFIG_GLOBAL`, `GIT_SSH_COMMAND`. Le refus de `GIT_TERMINAL_PROMPT=0` doit rester.
- Ne fait pas : ne protège pas des secrets sur disque (cf. R3/R1).
- *Falsifiée si* un canari absent de l'allowlist est malgré tout visible dans l'enfant (lecture `/proc/self/environ` ou Wolfram/`GetEnvironmentStrings`) sur l'un des deux OS.

**R3 — Secrets : trois tests séparés + rédaction avant persistance.**
Séparer `ENV-04a` (environnement), `ENV-04b` (accès disque : `~/.ssh`, `.env`, keychain/agent, `%APPDATA%`), `ENV-04c` (rédaction). Rediriger les diagnostics avant écriture : troncature **et** masquage des motifs de credentials, ou ne jamais journaliser le texte de sortie (garder hash + longueur + code de sortie).
- Ne fait pas : ne rend pas un journal existant propre (les entrées déjà écrites restent).
- *Falsifiée si* un canari connu est retrouvé dans le journal/les artefacts après un run volontairement bavard.

**R4 — Chemins : refuser ce qui n'est pas prouvable, pas « nettoyer ».**
(a) vérifier la chaîne de parents (reparse points/jonctions) avant toute écriture, avec refus si un composant n'est pas un répertoire ordinaire ; (b) refuser les noms réservés Windows, les suffixes `.`/espace, les collisions de casse et les formes Unicode non NFC **lorsqu'un chemin existe déjà** ; (c) conserver le refus des symlinks en feuille et des non-UTF-8 ; (d) `UNSUPPORTED` pour `160000`/LFS au lieu d'un comportement implicite.
- Ne fait pas : ne rend pas les filtres `.gitattributes` corrects (sujet distinct, cf. R5).
- *Falsifiée si* un fixture `vendor → hors-workspace` (jonction Windows, symlink Linux) aboutit à une écriture hors workspace, ou si deux chemins ne différant que par la casse produisent deux entrées d'index publiables.

**R5 — Fidélité Git : dire ce qui est supporté, refuser le reste.**
Soit `hash-object --path` (donc filtres et LFS corrects), soit **déclarer explicitement** que les filtres ne s'appliquent pas et refuser les dépôts avec `.gitattributes` actifs sur les chemins publiés. Préserver le bit exécutable d'un fichier neuf (`100755` si le FS le porte). Documenter que les gitlinks ne sont pas supportés.
- Ne fait pas : ne garantit pas la parité d'écriture avec `git add`.
- *Falsifiée si* un fichier CRLF/LFS publié par Flow produit un blob différent de `git add` sur le même contenu, alors que la documentation annonce l'équivalence.

**R6 — Descendants et ressources (bloquant `RESOURCE-14`).**
Windows : Job Object par exécution, `KILL_ON_JOB_CLOSE`, `ActiveProcessLimit`, `ProcessMemoryLimit`, assignation avant reprise. Linux : groupe de processus ou `pidfd` + `cgroup` si disponible, `RLIMIT_NPROC/AS`, `SIGKILL` au groupe entier. Timeout actuel conservé.
- Ne fait pas : ne protège pas d'un processus qui s'échappe du job (breakaway) — à interdire explicitement (`JOB_OBJECT_LIMIT_BREAKAWAY_OK` non défini).
- *Falsifiée si* un test qui `fork()` en boucle ou détache un enfant laisse un processus vivant après retour de Flow.

**R7 — Durabilité honnête.**
Ajouter : fsync du répertoire parent à la création du journal ; enregistrement avec longueur + checksum, et tolérance **explicite** d'une queue tronquée (avec état `JOURNAL_TRUNCATED` visible) ; écriture de fichier via `temp + fsync + rename` ; épinglage des objets candidats par une ref (`refs/flow/pending/<id>`) avant `commit-tree`, relâchée après outcome. Distinguer dans le contrat les trois niveaux (`process kill` / `OS crash` / `power loss`) — comme en §2.3, sans « ACID ».
- Ne fait pas : ne rend pas atomiques ref+disque+index+hooks+remote.
- *Falsifiée si* une coupure injectée après CAS mais avant réconciliation produit un état non détectable au redémarrage, ou si `git gc --prune=now` supprime un objet référencé par un reçu.

**R8 — Contrat de retour de publication.**
Créer un type de retour explicite distinguant `Published` / `PublishedWithResidue{commit, residue}` / `NotPublished`. Interdire qu'un `Err` signifie « non publié » sans preuve.
- Ne fait pas : ne change pas le comportement de Git externe.
- *Falsifiée si* un test d'injection montre un `Err` ambigu (publication réelle) encore possible.

**R9 — Distribution/licence.**
Aucun bundle « sandbox » multi-OS : Linux = dépendance système déclarée (bwrap/landlock détectés, sinon `UNSUPPORTED`) ; Windows = pas d'artefact sandbox. Toute réutilisation de code Ravum est exclue tant qu'aucune licence n'existe (`DIST-01`) ; « from scratch » reste l'option par défaut (H2). Toute dépendance tierce éventuelle exige un audit de licence avant emballage.
- Ne fait pas : ne tranche pas la licence de Flow (H2, humain).
- *Falsifiée si* une licence Ravum apparaît au SHA alors épinglé (à re-vérifier avant toute décision).

---

## 6. Limites et `NOT_REVIEWED`

1. **Aucune exécution.** Tous les constats sont statiques. `NOT_REPRODUCED` partout où un effet est décrit ; `P0-HOOK-ORDER` et `P0-ENV` restent `CODE_READ ≠ bug reproduit`.
2. **API GitHub en 403** durant ma session : 47 issues, PR #33, absence de protection de branche, 42 ADR = `NOT_REVIEWED (baseline Lead, non re-vérifiée par moi)`. Seuls faits re-vérifiés par moi : LICENSE absent (4×404), `docs/adr/README.md` présent (200), blobs des 6 fichiers.
3. **Pages non récupérées** : WDAC, minifilters, cgroups v2, `.gitattributes`/filtres Git, sémantique `FileType::is_symlink()` pour les jonctions Windows, exigences admin d'AppContainer pour une app legacy. Toutes marquées `NOT_REVIEWED` et **non utilisées comme preuve**.
4. **Non lu** : `coordinator.rs` en entier (87 Ko, seulement grep ciblé epoch/lock), CLI `ravum-collab.rs` (persistance des erreurs), `overlay.rs`, `evaluation.rs`, `semantics.rs`. Toute conclusion sur la persistance des erreurs de hook (`ENV-05`) est donc bornée.
5. **Pas de mesure** de coût mémoire/disque des options sandbox sur la machine 16 Go ; pas de benchmark. Les recommandations R1/R6 sont conditionnées à un futur `GO_R0_FIXTURES`.
6. **Pas d'avis** sur la concurrence Git pure (writer lock, leases) au-delà de `DUR-06` : sujet de l'agent A ; mes constats s'y articulent mais ne le remplacent pas.
7. Le caractère public du dépôt Ravum est cohérent avec mes accès anonymes (raw 200), mais je n'ai pas appelé l'API repo pour le confirmer (`403`).

---

## 7. Références (URL + SHA/révision + date)

**Code Ravum (`Rwanbt/Ravum@dev = 5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96`, lu le 2026-11-09) :**
- `crates/ravum-workspace-coordinator/src/git_writer.rs` — blob `0194caadac4c695bdc758f2f0f17c27d1d961c3c` — <https://raw.githubusercontent.com/Rwanbt/Ravum/5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96/crates/ravum-workspace-coordinator/src/git_writer.rs>
- `.../src/bin/common/mod.rs` — blob `783b87cfbacee3d95a502ee6006197c870d270d8` — <https://raw.githubusercontent.com/Rwanbt/Ravum/5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96/crates/ravum-workspace-coordinator/src/bin/common/mod.rs>
- `.../src/bin/ravum-collab.rs` — blob `968f03bc2107a6542f42dc98d56ba6e919a4f736` — <https://raw.githubusercontent.com/Rwanbt/Ravum/5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96/crates/ravum-workspace-coordinator/src/bin/ravum-collab.rs>
- `.../src/path_policy.rs` — blob `2486d3bc5c81946af1550695f5a12ff00d627cff` — <https://raw.githubusercontent.com/Rwanbt/Ravum/5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96/crates/ravum-workspace-coordinator/src/path_policy.rs>
- `.../src/publication_journal.rs` — blob `ba08ab25ce4700b24e7ee22e7a7002a8a1198f8c` — <https://raw.githubusercontent.com/Rwanbt/Ravum/5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96/crates/ravum-workspace-coordinator/src/publication_journal.rs>
- `.../src/coordinator.rs` — blob `a56f0ed88dae7e08872360cb2932e557d2bb65f0` — <https://raw.githubusercontent.com/Rwanbt/Ravum/5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96/crates/ravum-workspace-coordinator/src/coordinator.rs>
- Licence : `LICENSE`, `LICENSE.md`, `LICENSE.txt`, `COPYING` → HTTP 404 au même SHA (testés le 2026-11-09) — <https://raw.githubusercontent.com/Rwanbt/Ravum/5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96/LICENSE>

**Documents Flow lus (`D:\App\Ravum-flow@1d5fddf`) :** `AGENTS.md`, `docs/DEEPSEEK_GOAL.md`, `docs/reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md` (§0–§10, l. 1–460), `docs/SECURITY_TESTS.md`, `docs/adr/FLOW-ADR-004.md`, `docs/adr/FLOW-ADR-010.md`, `docs/rfc/RF-RFC-010.md`, `docs/rfc/RF-RFC-011.md`, `docs/RAVUM_BOUNDARY.md`, `docs/BACKLOG.md`.

**Sources primaires OS (lues le 2026-11-09) :**
- Microsoft Learn, *Job Objects* (mis à jour 2025-07-14) — <https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects>
- Microsoft Learn, *Windows Sandbox* (mis à jour 2026-03-29) — <https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/>
- Microsoft Learn, *AppContainer isolation* (mis à jour 2025-07-08) — <https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation>
- Microsoft Learn, *Restricted Tokens* (mis à jour 2021-01-07) — <https://learn.microsoft.com/en-us/windows/win32/secauthz/restricted-tokens>
- man-pages 6.19 (tarball 2026-09-09), `user_namespaces(7)` — <https://man7.org/linux/man-pages/man7/user_namespaces.7.html>
- man-pages 6.19, `landlock(7)` — <https://man7.org/linux/man-pages/man7/landlock.7.html>
- man-pages 6.19, `seccomp(2)` — <https://man7.org/linux/man-pages/man2/seccomp.2.html>
- bubblewrap, `README.md` (branche `main`) — <https://raw.githubusercontent.com/containers/bubblewrap/main/README.md>

**Non utilisées comme preuve (non récupérées) :** Microsoft Learn WDAC / file-system minifilters ; `Documentation/admin-guide/cgroup-v2.rst` ; `gitattributes(5)` / `git-hash-object(1)`.

---

*Fin du rapport brut B. Aucun fichier autre que celui-ci n'a été créé ou modifié ; aucun test, build, sandbox, branche ou commit exécuté.*
