# R0-b — Sécurité et processus : résultats mesurés

**Statut :** `EXECUTED / LOCAL / NOT_A_SECURITY_PROOF`
**Date :** 2026-11-09 (généré à `2026-11-09T17:03:28Z`, durée 19,0 s)
**Hôte :** Windows (`win32`/`nt`), Python 3.13.7, git 2.51.1.windows.1, **utilisateur standard non élevé**
**Harnais :** `experiments/r0b/security/run_tests.py` → `RESULTS.json`, `logs/` (rédigés), `scratch/` (jetable)
**Périmètre :** `D:\App\Ravum-flow\experiments\r0b\security\` uniquement. Aucun accès Ravum, aucun Internet, aucun commit, aucune compilation lourde (git + python).

> **Deux règles de lecture.** (1) Un test vert n'est pas une preuve de sécurité : ces résultats prouvent des **mécanismes** sur un poste, pas l'absence de vulnérabilité d'un futur produit. (2) « Pas d'isolation » est écrit ici comme une **limite mesurée**, pas comme un bug à corriger.

---

## 1. Règles respectées (vérifiables)

| Contrainte | Mise en œuvre | Vérification |
|---|---|---|
| Écrire seulement sous `experiments/r0b/security/` | tout le harnais, `scratch/`, `logs/`, `RESULTS.json` | aucun fichier créé ailleurs (hors fixtures de mon périmètre) |
| `experiments/r0a/` non modifié | `redact.py` **copié** dans `r0b/security/`, jamais importé (aucun `__pycache__` créé dans r0a) ; aucune fixture R0-a exécutée | vérifié par construction (aucune commande n'écrit dans `r0a/`) ; l'arbre `experiments/` étant entièrement non suivi par git, `git status` ne peut pas le démontrer |
| `run_r0a.py clean|build|all` jamais lancé | aucune invocation | commandes tracées dans `RESULTS.json` |
| Aucun Internet | seule tentative réseau : `connect 127.0.0.1:9` (loopback, sans service) | `SANDBOX-05` ; aucune résolution DNS, aucun port distant |
| Aucun vrai secret | canari fictif `FLOW_FAKE_SECRET_CANARY_…` + faux `~/.ssh` sous `scratch/fake-home/` | `ENV-04b` |
| Canari jamais persisté dans un log/artefact | rédaction **avant** écriture (`write_redacted`, fail-closed) puis scan final | `summary.canary_scan` : **0 occurrence** dans 14 fichiers ; 1 occurrence uniquement dans la fixture `scratch/fake-home/.ssh/id_fixture` (nécessaire au test, `scratch/` ignoré par git via `scratch/.gitignore`) |
| Aucun commit | mes livrables restent non committés | `git status` (d'autres travaux non liés existent dans l'arbre : `.gitignore`, `Cargo.*`, `crates/` — hors de mon périmètre, non touchés) |
| Fixtures copiées avant mutation | `setup()` copie `fixtures/` → `scratch/fixtures/`, puis mute `scratch/` | `scratch/` reconstruit à chaque run |

**Écart documenté :** le brief indiquait `experiments/r0a/work/fake-home` comme faux `~/.ssh`. Ce répertoire **n'existe pas** dans l'arbre actuel (fixtures R0-a nettoyées) et le créer sortirait de mon périmètre d'écriture. Il a donc été recréé à l'identique sous `experiments/r0b/security/scratch/fake-home` (même contenu : `id_fixture` + `known_hosts`), et `FLOW_FAKE_HOME` pointe dessus.

---

## 2. Verdicts

**7 tests exécutés, 7 `REPRODUCED`, 0 `BLOCKED`, 0 `NOT_RUN`.**

| ID | Verdict | Preuve clé (résumé) | Catégorie |
|---|---|---|---|
| `ENV-04a` | `REPRODUCED` | enfant « brut » : canari visible, 72 variables héritées ; enfant « allowlist » : absent, 11 variables | environnement |
| `ENV-04b` | `REPRODUCED` | clé fictive lue intégralement (SHA-256 identique) ; contrôle négatif `FileNotFoundError` | secrets / FS |
| `ENV-04c` | `REPRODUCED` | log naïf = 2 occurrences persistées (transitoire supprimé) ; log rédigé = **0** occurrence | secrets / logs |
| `SANDBOX-05` | `REPRODUCED` | 6 tentatives loopback, codes `10061`/`10035`, **aucun** code de refus de politique | réseau |
| `PATH-08` | `REPRODUCED` | 5 sous-cas : traversée, jonction, noms réservés/points finaux, casse, divergence FS/Git | chemins |
| `RESOURCE-14` | `REPRODUCED` | petit-enfant vivant après kill direct ; mort après `taskkill /T /F` | processus |
| `UNMANAGED-17` | `REPRODUCED` | `git commit` externe rc=0 pendant verrou ; changement détecté par relecture | concurrence Git |

---

## 3. Détail par test

### ENV-04a — héritage d'environnement et mitigation par allowlist

- **Fixture :** `fixtures/env_child.py`, lancé deux fois ; le script n'imprime jamais de valeur secrète (booléens, noms de variables, empreinte SHA-256 tronquée).
- **Observation :** avec `env = os.environ + FLOW_TEST_SECRET=<canari>`, l'enfant voit la variable (`canary_var_present=true`, empreinte `a328968e12ca` = SHA-256 du canari sur 12 caractères) et hérite de **72** variables, dont `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `TEMP`. Avec une allowlist explicite (11 variables), la variable est **absente**.
- **Verdict :** `REPRODUCED` (héritage) + mitigation mesurée (`allowlist_mitigation_measured=true`).
- **Portée :** mécanisme OS de création de processus. Aucun code Flow n'existe : ce n'est pas une mesure du produit, c'est la primitive que le produit devra contraindre par allowlist.

### ENV-04b — lecture d'un secret sur disque (aucune isolation FS)

- **Fixture :** `probe.py` (copie de `f6-canary`) + `scratch/fake-home/.ssh/id_fixture` (canari fictif, `0600` non requis sur Windows).
- **Observation :** `read_secret=true`, `secret_sha256=4e1df2a3…eb51a7` **identique** au SHA-256 calculé sur la source. Contrôle négatif (`FLOW_FAKE_HOME` inexistant) : `read_secret=false`, `read_error=FileNotFoundError` — donc pas de faux positif du probe.
- **Verdict :** `REPRODUCED`.
- **Portée :** droits utilisateur normaux, aucun sandbox, aucune ACL. **Ce n'est pas une sandbox** : c'est la démonstration que « lancer un enfant » ne constitue aucune barrière FS. Non qualifié : autre compte, intégrité AppContainer, ACL restreinte, EFS, partages réseau.

### ENV-04c — canari sur stdout/stderr : persistance naïve vs rédaction

- **Fixture :** enfant Python imprimant le canari sur stdout **et** stderr.
- **Observation :** un log naïf (concaténation brute) contient **2** occurrences ; écrit dans un fichier **transitoire** de `scratch/`, relu (2 occurrences), puis **supprimé** (absence vérifiée). Le fichier conservé (`logs/ENV-04c-redacted.log`) contient **0** occurrence après rédaction (2 remplacements), avec contrôle fail-closed sur les marqueurs résiduels.
- **Verdict :** `REPRODUCED` (fuite naïve) + mitigation mesurée.
- **Portée :** la rédaction est une **denylist** de marqueurs (`redaction_is_denylist=true`) : un secret de forme inconnue passerait. La mitigation porte sur la persistance, pas sur la production de la sortie.

### SANDBOX-05 — tentative loopback (127.0.0.1:9)

- **Fixture :** `probe.py` (un `create_connection` 0,2 s) + matrice `connect_ex` (3 bloquants, 3 avec délai 0,5 s).
- **Observation mesurée :**

| mode | code | nom | latence |
|---|---|---|---|
| bloquant ×3 | `10061` | `WSAECONNREFUSED` | ~2 020 ms |
| délai 0,5 s ×3 | `10035` | `WSAEWOULDBLOCK` | ~505 ms |

- **Aucun code de refus de politique** (`WSAEACCES 10013`, `WSAENETUNREACH 10051`, `WSAEHOSTUNREACH 10065`) : la tentative n'est pas bloquée par une couche applicative. Le délai de ~2 s avant refus, puis un `EWOULDBLOCK` en mode non bloquant, est compatible avec un **filtrage local silencieux** (pare-feu d'hôte) plutôt qu'avec un refus immédiat — phénomène observé, **non instrumenté** ici.
- **Verdict :** `REPRODUCED` pour « la tentative loopback est permise ; aucune isolation réseau locale n'est observée ».
- **Conclusion honnête :** **loopback ≠ egress Internet**. Ce test **ne qualifie pas** le réseau externe (`NOT_QUALIFIED` : egress, DNS, proxy, pare-feu d'hôte, capacité réseau AppContainer/Windows Firewall, réseau de conteneur). Aucun serveur local n'a été démarré (interdit) : une connexion **réussie** n'a donc pas été mesurée, seulement l'absence de refus de politique.

### PATH-08 — chemins : naïve vs canonisation

Cinq sous-cas, tous dans un workspace **simulé** `scratch/path-08/ws` ; l'échappement reste à l'intérieur de `experiments/r0b/security/`.

| Sous-cas | Validation naïve (préfixe de chaîne) | Canonisation (`realpath`) | Effet mesuré |
|---|---|---|---|
| (a) `ws\..\evil.txt` | accepte | **rejette** | écriture effective **hors** `ws` (`path-08/evil.txt`) |
| (b) jonction `ws\vendor` → `outside` | accepte | **rejette** (realpath = `outside\pwn.txt`) | `pwn.txt` écrit **dans la cible** ; jonction retirée ensuite par `rmdir`, fichier cible conservé |
| (c) `CON`, `aux.txt`, `trail.`, `space ` | accepte | **accepte aussi** (aucune ne canonise ces formes) | `CON`/`aux.txt` **créés** comme entrées de répertoire ; `trail.`→`trail` et `space `→`space` : **renommage silencieux** (collision de deux chemins logiques sur un fichier) |
| (c2) divergence FS/Git | — | — | `git status` voit `?? CON` et `?? aux.txt`, mais `git add` **échoue** : `fatal: unable to stat 'CON'` (rc 128), `unable to index file 'aux.txt'` (rc 128) |
| (d) `Case.txt` / `case.txt` | 2 chemins distincts | — | **1 seule** entrée sur disque, `same_file=true`, la seconde écriture écrase la première (`lower`) |

- **Verdict :** `REPRODUCED` (`all_subcases_reproduced=true`, `fs_git_divergence_reproduced=true`).
- **Lecture :** la canonisation par `realpath` corrige (a) et (b) mais **pas** (c) ; les noms réservés et les suffixes `.`/espace exigent une **denylist explicite**. Le cas (c2) est le plus opérationnel : un nom que le FS accepte et que Git ne peut pas indexer crée une divergence entre l'arbre candidat et le disque.
- **Portée :** Windows, jonction créée **sans élévation** (`mklink /J` rc=0). Non testés : symlink de fichier (`mklink`), mount points, chemins non-UTF-8, volumes montés, partages réseau.

### RESOURCE-14 — descendants après timeout

- **Fixture :** `spawn_grandchild.py` (enfant direct → petit-enfant `python -c "time.sleep(60)"`, PIDs écrits dans un fichier).
- **Observation :** phase 1, `TerminateProcess` sur l'enfant direct (`returncode=1`) → **petit-enfant vivant** (`grandchild_alive=true`). Phase 2, `taskkill /T /F` sur le parent → **petit-enfant mort** (`taskkill_rc=0`, `grandchild_dead=true`). Nettoyage vérifié : aucun survivant (`cleanup_all_survivors_dead=true`).
- **Verdict :** `REPRODUCED`.
- **Portée :** `taskkill /T` est une **mitigation mesurée, pas une garantie** (course possible sur la création de descendants, pas de Job Object, pas de limites mémoire/CPU). Aucun Job Object n'a été implémenté. Linux (groupe de processus, `pidfd`, cgroup) : `NOT_REVIEWED`.

### UNMANAGED-17 — écriture Git externe pendant verrou

- **Fixture :** dépôt git synthétique `scratch/unmanaged-17/repo` + « writer » détenant un verrou fichier exclusif (`msvcrt.locking`), puis relecture de `HEAD`.
- **Observation :** `git commit` externe exécuté **pendant** que le verrou est détenu → `rc=0` ; `HEAD` change (`e6fd74e2…` → `b74224f9…`) ; le writer détecte le changement par relecture (`detected=true`).
- **Verdict :** `REPRODUCED`.
- **Portée :** verrou fichier + relecture, **pas** le protocole Flow. Détection possible, **prévention non** (`prevention_by_lock=false`). Non qualifiés : push distant, écritures d'index concurrentes, répertoires de travail non suivis.

---

## 4. Verdicts par catégorie

| Catégorie | Verdict | Ce que cela établit |
|---|---|---|
| **Environnement** | `REPRODUCED` | l'héritage est intégral par défaut ; une allowlist explicite l'élimine (mécanisme, pas produit) |
| **Secrets / FS** | `REPRODUCED` | un enfant au même utilisateur lit une clé privée fictive sans barrière |
| **Secrets / logs** | `REPRODUCED` | un log naïf persiste un canari ; la rédaction avant persistance l'élimine (denylist) |
| **Réseau** | `REPRODUCED` (loopback) / `NOT_QUALIFIED` (egress) | aucune isolation locale observée ; réseau externe non testable sans sandbox réelle |
| **Chemins** | `REPRODUCED` | traversée et jonction contournent une validation naïve ; canonisation partielle ; noms réservés et casse non couverts par `realpath` |
| **Processus** | `REPRODUCED` | un timeout qui ne tue que l'enfant direct laisse des orphelins |
| **Concurrence Git** | `REPRODUCED` | un verrou coopératif n'empêche pas une écriture Git externe ; détection seule |

---

## 5. Cinq constats majeurs

1. **L'héritage d'environnement est total par défaut et la seule mitigation démontrée est une allowlist explicite** — 72 variables héritées, dont le profil utilisateur ; une allowlist de 11 variables suffit à faire disparaître le canari (`ENV-04a`).
2. **Aucune isolation de fichiers n'existe « par défaut »** : un enfant lancé sous le même compte lit une clé privée fictive octet pour octet (`ENV-04b`). Ce n'est pas un défaut de Flow mais la frontière réelle à annoncer.
3. **La fuite de secrets par les logs est indépendante de l'environnement** : la sortie de l'enfant contient le canari, et seule une rédaction **avant** persistance le retire (`ENV-04c`). La rédaction étant une denylist, elle ne protège pas d'un secret de forme inconnue.
4. **Les validations de chemins naïves sont contournables de trois façons mesurées** : `..` (écriture hors workspace), **jonction Windows** créée sans élévation (écriture dans la cible), et **casse** (deux chemins → un fichier). `realpath` corrige les deux premiers, pas les noms réservés ni les suffixes `.`/espace ; en plus, `CON`/`aux.txt` sont créés sur disque mais **inindexables par Git**.
5. **Un timeout qui ne tue que l'enfant direct laisse des descendants vivants** ; `taskkill /T /F` les tue sur ce poste, mais sans Job Object il n'existe aucune garantie de confinement de l'arbre (`RESOURCE-14`). Corollaire concurrence : un verrou coopératif ne prévient pas une écriture Git externe, seule la relecture la détecte (`UNMANAGED-17`).

---

## 6. `BLOCKED` / `NOT_RUN`

**Aucun test `BLOCKED`, aucun `NOT_RUN`** : les 7 IDs canoniques ont produit un verdict (`summary.blocked_or_not_run = []`).

Ce qui **n'a pas été exécuté**, et pourquoi :

| Sujet | Statut | Raison |
|---|---|---|
| Egress Internet réel | `NOT_QUALIFIED` | interdit par le protocole (aucun accès Internet) |
| AppContainer / Windows Sandbox / WDAC | `NOT_RUN` | exigent une configuration privilégiée ou une VM ; hors périmètre |
| Job Object via `ctypes` | `NOT_RUN` | `taskkill /T` a suffi à mesurer la mitigation ; un Job Object reste la voie robuste à qualifier |
| Linux (namespaces, bwrap, cgroups) | `NOT_RUN` | poste Windows uniquement cette session |
| ACL / autre compte utilisateur / intégrité | `NOT_RUN` | nécessitent des comptes et des droits hors périmètre |

---

## 7. Limites explicites

1. **Un seul poste, un seul compte, un seul OS.** Windows `win32`/`nt`, Python 3.13.7, git 2.51.1 ; édition Windows non qualifiée (Home/Pro non distinguées), antivirus/EDR non instrumenté. Rien ne garantit la reproductibilité sur une autre build.
2. **Droits utilisateur standard non élevé.** Les résultats ne disent rien d'un contexte administrateur, service, ou d'un autre utilisateur ; l'absence de barrière observée ici est celle d'un compte non privilégié qui possède déjà ses propres fichiers.
3. **« Pas d'isolation » est une limite, pas un bug.** `ENV-04b` mesure que lancer un enfant ne confine rien ; c'est une propriété du système observé, à écrire dans la documentation produit, pas à « corriger » par un `env_clear`.
4. **Loopback ≠ réseau.** `SANDBOX-05` ne teste que `127.0.0.1:9`, sans serveur ; une connexion réussie n'a pas été observée. Le comportement dépend du mode socket (refus après ~2 s vs `EWOULDBLOCK`), signature d'un filtrage local non instrumenté. L'egress, le DNS, les proxys et le pare-feu restent `NOT_QUALIFIED`.
5. **Rédaction = denylist.** `ENV-04c` prouve l'élimination d'un marqueur **connu** ; un secret d'une autre forme serait persisté. Le scan final ne prouve que l'absence du canari défini, dans les 14 fichiers scannés.
6. **Jonctions seulement.** `mklink /J` (jonction de répertoire) a été testée, sans élévation. Les symlinks de fichier/répertoire (`mklink`/`mklink /D`), mount points et autres reparse points n'ont pas été testés ; `realpath` les résout probablement de la même façon, ce n'est pas mesuré.
7. **Noms réservés : mesure partielle.** La création de `CON`/`aux.txt` est un comportement observé via Python (`open(..., "x")`) ; le devenir de ces entrées vis-à-vis du namespace de périphériques Win32 (au-delà de `git add`) n'est pas qualifié.
8. **`taskkill /T` n'est pas un Job Object.** Il n'y a ni limite mémoire/CPU, ni garantie de capture des descendants créés pendant le kill, ni équivalent Linux testé.
9. **`UNMANAGED-17` est un modèle**, pas le protocole Flow : verrou fichier + relecture, commit local seulement, pas de push, pas d'index concurrent.
10. **Aucune preuve de sécurité.** Sept mécanismes reproduits sur un poste ne constituent ni une preuve formelle, ni une mesure de production ; les fixtures sont synthétiques et les secrets fictifs.

---

## 8. Reproduire

```powershell
cd D:\App\Ravum-flow
$env:PYTHONDONTWRITEBYTECODE='1'
python experiments\r0b\security\run_tests.py
```

Sorties : `experiments/r0b/security/RESULTS.json` (7 enregistrements, schéma `r0b-security-results/1`), `logs/*.log` (rédigés, 7 fichiers), `scratch/` (jetable, ignoré par git via `scratch/.gitignore`). Les PIDs, OIDs et latences varient d'un run à l'autre ; les verdicts et les invariants mesurés sont identiques sur les deux exécutions consécutives du harnais final (les runs antérieurs utilisaient une version différente de `SANDBOX-05`/`PATH-08`, avant ajout de la matrice d'errno et du cas (c2)).
