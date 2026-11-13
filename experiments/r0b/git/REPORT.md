# R0-b — Git mechanics (agent A)

**Date :** 2026-11-09 (Europe/Paris) · **Hôte :** Windows 11, `git version 2.51.1.windows.1`, Python 3.13.7, NTFS.
**Périmètre :** `experiments/r0b/git/` uniquement. Aucun réseau, aucun `cargo`/`npm`, aucun code Ravum exécuté, aucun commit dans le dépôt Flow.
**Artefacts :** `RESULTS.json` (13 objets), `logs/<ID>.log` (commandes exactes, rc, stdout/stderr, faits), `fixtures/` (reconstruites à chaque run), `scratch/` (jetable), `run_tests.py` (harnais).

## Méthode et provenance des fixtures

Les fixtures R0-a étaient **absentes** au moment de l'exécution : `experiments/r0a/work/fixtures/` était vide (0 fichier) et `experiments/r0a/out/manifest.json` (généré à 16:48) décrivait des chemins disparus ; `work/created-paths.json` valait `[]` (un `clean` a eu lieu à 17:49:22). La consigne interdisant `run_r0a.py build|clean|all`, les scénarios documentés dans `experiments/r0a/harness/fixtures.py` ont été **rejoués localement** sous `experiments/r0b/git/fixtures/` (mêmes hooks, mêmes contenus de fichiers, même `publisher.py` copié verbatim). Les résultats portent donc sur le **comportement Git**, pas sur l'identité bit-à-bit des fixtures R0-a ; les seeds/commits R0-a ne sont pas reproduits.

Chaque test copie sa fixture dans `scratch/` avant toute mutation (`shutil.copytree`) ; les fixtures partagées ne sont jamais mutées. Chaque commande est journalisée avec son `cwd`, son code retour et ses sorties ; les chemins absolus sont remplacés par `<REPO>` et aucun canari n'est utilisé.

## Résultats

| ID | Verdict | Preuve clé |
|---|---|---|
| TREE-01 | `REPRODUCED` | test rc=0 avec dépendances non suivie/ignorée ; rc=1 sur export propre du tree |
| TREE-02 | `REPRODUCED` | tree OID inchangé, octets mutés, test vert sur le worktree / rouge sur le tree |
| HOOK-02a | `REPRODUCED` | `extra.txt` ajouté par `pre-commit` présent dans `HEAD^{tree}` |
| HOOK-02b | `REPRODUCED` | fichier sale utilisateur réécrit par le hook, hors commit |
| HOOK-02c | `REPRODUCED` | ordre `pre-commit → prepare-commit-msg → commit-msg → post-commit` |
| HOOK-02d | `REPRODUCED` | refus rc=1, HEAD/index intacts, aucun objet orphelin |
| HOOK-GITDIR-19 | `REPRODUCED` | `.git/config` commun modifiable et ref du principal déplaçable depuis un worktree lié |
| REF-10 | `REPRODUCED` | CAS périmé rc=128 sans déplacement ; CAS correct rc=0 |
| FENCE-07a | `REPRODUCED` | écrivain périmé refusé par le CAS |
| FENCE-07b | `REPRODUCED` | `git commit` externe réussit, notre CAS refuse ensuite |
| FENCE-07c | `REPRODUCED` | crash post-CAS : intent seul, ref publiée, récupération sans republier |
| CRASH-11 | `REPRODUCED` | matrice 4 points de crash + rejeu naïf non idempotent |
| FILE-09 | `REPRODUCED` | modes/octets/rename exacts ; lien et collision de casse cassent sur Windows |

**Bilan : 13 `REPRODUCED`, 0 `DISPROVED`, 0 `NOT_REPRODUCED`, 0 `BLOCKED`, 0 `NOT_RUN`.**

## Détail par test

**TREE-01 — un test passe grâce à un fichier absent du tree.** Fixture `tree-imports` : `test_app.py` (committé) importe `local_config.py` (non suivi) et `gen/values.py` (ignoré). Dans le worktree : `python -m unittest -q test_app` → rc=0. Dans un export propre (`git worktree add --detach HEAD`) : rc=1 (ImportError). Le tree ne contient que `.gitignore`, `README.md`, `test_app.py`. Preuve : `logs/TREE-01.log`, faits `dirty_rc=0`, `clean_rc=1`, `tree_entries`.

**TREE-02 — un reçu sur le tree OID serait faux.** Fixture `tree-mutation` : `calc.py` committé renvoie 2, `test_calc.py` attend 3. On mute `calc.py` dans le worktree (sha256 `d3fe3723…` → `86508686…`) : le tree OID reste `9f4f0974…` et le blob du tree renvoie toujours `return 2`. Le test passe sur les octets mutés (rc=0) et échoue sur l'export du tree (rc=1) : un reçu lié à ce tree OID décrirait un autre contenu que celui testé. Preuve : `logs/TREE-02.log`.

**HOOK-02a — Git inclut ce que `pre-commit` stage.** Fixture `f2-hooks/mutate` : `pre-commit` fait `printf >> extra.txt` puis `git add extra.txt`. Après `git commit` : rc=0, `extra.txt` présent dans `HEAD^{tree}` avec le blob `hook-mutated\n` (HEAD `8186b4e…` → `f4fb9e8…`). L'arbre est donc écrit **après** les hooks. La comparaison avec Ravum (`write-tree` avant hooks, `commit-tree` réutilisant l'arbre) est `CODE_READ` de la phase documentaire, non exécutée ici. Preuve : `logs/HOOK-02a.log`.

**HOOK-02b — aucun isolat pour un hook.** Fixture `hook-isolate` : `dirty.txt` est sale (non stagé), `work.txt` est stagé ; un `pre-commit` fait `printf 'hooked' >> dirty.txt`. Après commit (rc=0) : `dirty.txt` = `b'dirty user edit\nhooked\n'`, `git status` = `M dirty.txt`, et le commit ne contient que `work.txt`. Le hook s'exécute avec `cwd` = racine du worktree utilisateur. Preuve : `logs/HOOK-02b.log`.

**HOOK-02c — ordre réel des hooks.** Fixture `f2-hooks/order` (quatre hooks journalisants). Le log donne exactement `pre-commit`, `prepare-commit-msg`, `commit-msg`, `post-commit` ; le message contient `prepared-by-hook` (le hook de message édite bien le fichier). Preuve : `logs/HOOK-02c.log`.

**HOOK-02d — refus propre.** Fixture `f2-hooks/refuse` : `pre-commit` sort 1. `git commit` rc=1, stderr `pre-commit deliberately refused`, HEAD inchangé (`9ce9f47…`), 1 commit avant/après, `work.txt` toujours stagé, et `git fsck --unreachable` ne montre **ni commit ni tree** orphelin. Preuve : `logs/HOOK-02d.log`.

**HOOK-GITDIR-19 — les fichiers de contrôle d'un worktree lié sont partagés.** Depuis un worktree lié (`git-dir` = `.git/worktrees/gitdir-linked`, `common-dir` = `.git` principal), `git config r0b.shared …` écrit dans le `.git/config` commun (lu depuis le principal) et `git update-ref refs/heads/moved <commit>` déplace une ref du dépôt principal (`5311cd80…`, visible depuis le principal). Sur une copie privée complète, les mêmes commandes n'affectent pas l'original. Preuve : `logs/HOOK-GITDIR-19.log`.

**REF-10 — CAS de ref.** `update-ref refs/heads/x B A` → rc=0 ; puis `update-ref refs/heads/x C A` (A périmé) → rc=128, stderr `cannot lock ref … is at 41b27b9… but expected bb140b8…`, ref inchangée ; enfin `update-ref refs/heads/x C B` → rc=0, ref = `89fb556…`. Preuve : `logs/REF-10.log`.

**FENCE-07a — écrivain périmé refusé.** Deux candidats `C1`/`C2` construits sur `old=6722bc4…`. L'écrivain 2 publie `C2` (rc=0) ; l'écrivain 1 tente `C1` avec le même `old` → rc=128, ref = `b06a88c…` (C2). Le CAS suffit entre écrivains coopérants ; aucun epoch n'est nécessaire pour ce cas. Preuve : `logs/FENCE-07a.log`.

**FENCE-07b — un `git` externe contourne.** Entre la lecture de `old` et notre CAS, un `git commit --allow-empty` externe réussit (rc=0, ref → `a23c443…`) ; notre CAS (`candidate=77736f8…`, `old`) est refusé (rc=128) et notre candidat reste inatteignable. Le contournement n'est pas empêché, seulement détecté trop tard. Preuve : `logs/FENCE-07b.log`.

**FENCE-07c — crash après le CAS, avant l'outcome.** `publisher.py` (copié de R0-a) avec `after-ref` : rc=70 ; le journal ne contient qu'un `INTENT` (`new=717715d…`, `old=2a2c6c1…`) et la ref vaut déjà `717715d…`. La récupération par relecture de la ref classe `PUBLISHED`, écrit un seul `RECOVERED_PUBLISHED` et **ne republie pas** (ref avant = après). L'ambiguïté est résolue par la ref, pas par le journal. Preuve : `logs/FENCE-07c.log`.

**CRASH-11 — matrice de crash et rejeu naïf.** Quatre points : `before-intent` → journal vide (`NOT_STARTED`) ; `after-intent` → intent seul, ref=old (`NOT_PUBLISHED`) ; `after-ref` → ref bougée sans outcome (`PUBLISHED` après relecture) ; `before-outcome` → l'outcome `PUBLISHED` est **déjà écrit** avant le `os._exit(70)` (point mal nommé, récupération triviale). Rejeu naïf sans clé d'idempotence : `after-intent` produit deux `INTENT` et deux commits distincts pour le même tree ; `after-ref` publie un second commit par-dessus le commit déjà publié (ref avancée deux fois). Preuve : `logs/CRASH-11.log`.

**FILE-09 — types de fichiers.** `f3-filetypes/base` : `script.sh` = `100755`, `b.bin` = `100644` et blob identique aux octets du worktree (`282ec23…`), rename détecté (`rename keep.txt => renamed.txt (100%)`), `delete.txt` absent. Sur cet hôte, le blob de `crlf.txt` ne contient pas de CRLF (attribut `text` → normalisation LF, donc octets du tree ≠ octets du disque). `link.txt` = `120000` : checkout rc=0, un **vrai symlink** est créé (Developer Mode actif), mais la cible du blob inclut un saut de ligne → lien pendant, lecture `b''`. Collision de casse : `['Case.txt', 'case.txt', 'seed.txt']` dans le tree, `['case.txt', 'seed.txt']` sur disque, `git status` = `M Case.txt` : une entrée est écrasée et le worktree sort immédiatement modifié. Preuve : `logs/FILE-09.log`.

## Ce que ces résultats établissent (et n'établissent pas)

- Git natif fournit déjà, sur cet hôte : l'ordre des hooks, l'inclusion du staging de `pre-commit` dans l'arbre, un CAS de ref propre, et un refus de hook sans objet partiel.
- Les écritures externes ne sont pas fencées : un `git` concurrent gagne, notre CAS le constate ensuite.
- Le seul point de crash ambigu est `after-ref` (ref bougée, outcome absent) ; il est résoluble par relecture de la ref, à condition d'une garde idempotente explicite — un rejeu naïf republie.
- Un worktree lié **n'est pas** une frontière de contrôle : config et refs du principal sont accessibles en écriture depuis le worktree lié.

Aucun de ces constats n'est une preuve de sécurité, ni une validation du writer de Flow : ce sont des comportements Git observés sur NTFS/Windows.

## Limites

- **Fixtures R0-a absentes** : répliques locales construites depuis `harness/fixtures.py` ; les seeds/commits R0-a ne sont pas reproduits, l'identité bit-à-bit n'est pas vérifiée. `run_r0a.py build|clean|all` n'a pas été lancé (interdit).
- **Un seul OS** : Windows 11 + Git for Windows 2.51.1. Rien sur Linux/macOS, rien sur FS réseau (SMB/NFS), rien sur `packed-refs` ou reftable.
- **Pas de vraie concurrence** : FENCE-07a/b sont des entrelacements **séquentiels** (deux invocations successives), pas deux processus concurrents ; les fenêtres de course réelles ne sont pas exercées.
- **Crash simulé** par `os._exit(70)`, pas SIGKILL/power-loss ; pas de perte de cache OS, pas de test fsync du FS.
- **Pas de sandbox** : les hooks tournent avec les privilèges de l'utilisateur ; le test démontre l'absence d'isolation, pas une évasion de confinement.
- **`before-outcome` est mal nommé** : le crash survient après l'écriture de l'outcome, donc ce point n'exerce pas la fenêtre « ref bougée, outcome absent » (couverte par `after-ref`).
- **Chemin de vérification** : l'export propre utilise `git worktree add --detach` (même objet store), pas `git archive` ; le contenu est celui du tree, sans fichiers non suivis/ignorés.
- **Comportement symlink dépendant de l'hôte** : contrairement à l'observation R0-a (`worktree_is_real_symlink=false`), cet hôte crée un vrai symlink ; le résultat n'est pas portable.
- **Ravum** : aucun code Ravum exécuté ; les comparaisons avec `git_writer.rs` sont `CODE_READ` (SHA épinglé de la phase documentaire) et hors de ce run.

## Reproduire

```powershell
cd D:\App\Ravum-flow
$env:PYTHONIOENCODING='utf-8'
python experiments\r0b\git\run_tests.py
```

Le harnais reconstruit `fixtures/` (rmtree robuste aux fichiers read-only de `.git`), rejoue les 13 tests et réécrit `RESULTS.json`, `logs/` et `logs/summary.txt`.
