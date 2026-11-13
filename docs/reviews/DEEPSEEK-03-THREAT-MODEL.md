# DEEPSEEK-03 — THREAT MODEL, P0 ET GARANTIES À NE PAS PROMETTRE (D2)

**Statut :** `PROPOSED / NOT_REPRODUCED`. Aucun test exécuté, aucun sandbox lancé, aucun build, aucune écriture Ravum.
**Base :** [`DEEPSEEK-00`](DEEPSEEK-00-BASELINE.md) (vérification Lead), [B](raw/DEEPSEEK-RAW-B-SECURITY.md) (sécurité/OS/durabilité), [A](raw/DEEPSEEK-RAW-A-GIT-CONCURRENCY.md) (Git/états/fencing), [C](raw/DEEPSEEK-RAW-C-BUILD-CI.md) (CI/E2E), [H](raw/DEEPSEEK-RAW-H-ADVERSARIAL.md) (scénarios de perte).
**Code épinglé :** `Rwanbt/Ravum@dev = 5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96`. Blobs : `git_writer.rs` `0194caa…`, `bin/common/mod.rs` `783b87c…`, `path_policy.rs` `2486d3b…`, `publication_journal.rs` `ba08ab2…`, `ravum-store/src/lib.rs` `d6ab6e9…`.

---

## 1. Méthode et règle de preuve

Trois agents ont lu indépendamment le même code épinglé (B : 6 fichiers, A : chemin Git complet, H : métadonnées + artefact S8), plus une vérification Lead. Le tableau §3 ne compte **que** les constats recoupés.

**Règle :** `CODE_READ` prouve qu'un chemin de code existe, **jamais** qu'une perte a eu lieu. Les colonnes « reproduction » disent ce qu'un futur `GO_R0_FIXTURES` devrait démontrer, pas ce qui est démontré.

---

## 2. Actifs, adversaires, frontières de confiance

| Actif | Perte redoutée | Adversaire / cause |
|---|---|---|
| Fichiers dirty/untracked de l'utilisateur | écrasement silencieux | hook approuvé bogué, IDE, agent non managé, `fs::write` non atomique |
| Ref Git publiée | commit d'un arbre non vérifié ; ref déplacée hors CAS | writer concurrent, `git` externe, epoch périmé, worktree partagé |
| Secrets (SSH, tokens, `.env`, keychain) | exfiltration par code candidat | env hérité, FS accessible au même uid, réseau, logs persistés |
| Receipt / journal | faux vert, preuve non rejouable | `STALE` promu, contexte incomplet, objet GC'd, ligne déchirée |
| Objets Git candidats | suppression par `gc` avant décision | pas de ref de rétention |
| Machine 16 Go | famine, fork bomb, processus orphelin | absence de contrôle de descendants |
| Dépôt lui-même | corruption par un hook qui atteint le `gitdir` | worktree partageant le `.git` réel |

**Frontières :** `HOST_TOOLING` (git, compilateurs, IDE) · `TRUSTED_REPO` (hooks/scripts approuvés) · `UNTRUSTED_CANDIDATE` (code candidat, tests, outils non intégrés). **Une approbation humaine ne change pas la catégorie de risque d'un hook** : elle autorise à l'exécuter, elle ne le rend pas sûr.

---

## 3. P0 confirmés ou reformulés (état consolidé)

| ID | Constat vérifié | Preuve | Statut | Reproduction attendue |
|---|---|---|---|---|
| `TREE-01` | L'arbre écrit avant hooks puis `commit-tree` réutilise la variable `tree` | `git_writer.rs:841-844`, `:856`, `:866-873` | `CODE_READ` | un `pre-commit` qui `git add` un fichier ne doit **pas** apparaître dans `commit^{tree}` |
| `TREE-02` | La vérification part du `SourceSnapshot`, la publication d'un tree filtré depuis `HEAD` + chemins : l'égalité n'est établie nulle part | `common/mod.rs:156-176`, `git_writer.rs:787-789`, `ravum-collab.rs:347-364` | `CODE_READ` → **divergence structurelle**, impact non reproduit (A §4.1) | `TREE-01` : un dirty hors patch rend le build vert, le tree publié rouge |
| `HOOK-02a` | Ordre déclaré `prepare-commit-msg` → `pre-commit` → `commit-msg` ; l'en-tête revendique pourtant « in Git's order » | `:1221-1228`, en-tête `:37-43` **auto-contradictoire** (A §4.8) | `CODE_READ` | un hook qui journalise son nom doit révéler l'ordre réel |
| `HOOK-02b` | Hooks exécutés avec `current_dir(&self.repo)` = workspace réel | `:1343-1344` | `CODE_READ` | un `pre-commit` qui réécrit un fichier dirty doit le modifier (perte à constater) |
| `HOOK-02c` | Un hook non exécutable est **ignoré silencieusement** sur POSIX ; sur Windows tout fichier est « runnable » | `:1899-1916`, `:1231-1233` | `CODE_READ` | une gate déclarée peut être vide sans avertissement |
| `ENV-04` | Environnement hérité : seules 8 variables de redirection Git sont retirées | `:85-94`, `:1353-1355` | `CODE_READ` | `FLOW_FAKE_SECRET` visible dans le hook |
| `ENV-04b` | `rustc` : 3 variables retirées ; harnais de test : 2 ; **aucun `current_dir`** | `common/mod.rs:442-450`, `:367-372` | `CODE_READ` (B, nouveau) | un test qui écrit `./x` écrit dans le cwd du coordinateur |
| `ENV-04c` | Fuite par **logs** : les lignes `panicked at`/`FAILED` (≤5) et 2 KiB de stderr entrent dans le journal durable, sans rédaction | `common/mod.rs:403-425`, `:44-47` | `CODE_READ` (B, nouveau) | un test qui `assert_eq!` sur un faux token l'écrit dans le journal |
| `LEGACY-06` | Protocole par défaut `V1CollabCompat` ; `AUTO_SAFE` = applicabilité textuelle | `coordinator.rs:180`, `:164-168`, `:330` | `CODE_READ` (défaut documenté, `v2` existe) | `AUTO_SAFE` + `SKIPPED` ne doit jamais devenir reçu |
| `FENCE-07a` | Aucun verrou inter-processus ni epoch : seul un `Mutex` intra-processus | `git_writer.rs:430,442-445,1622-1623` ; grep `epoch/flock` = 0 | `CODE_READ` (B §DUR-06) | deux writers concurrents : un seul doit publier |
| `FENCE-07b` | `INDEX_OUT_OF_SYNC` est renvoyé en `Err` **après** journalisation `Published` | `:1050-1055`, `:1038-1049` | `CODE_READ` (B §D-07) | un appelant ne doit pas lire « Err = non publié » |
| `FILE-09` | Chemins et contenus en `String` ; mode `100644` pour un chemin **absent de l'index** ; mode d'index conservé sinon | `:122,124`, `:814-817`, `:1557-1577` | `CODE_READ` | un script neuf exécutable perd son bit `100755` ; un binaire lève `NotUtf8` (fail-closed) |
| `PATH-08` | Politique **purement lexicale** ; composants parents non inspectés ; noms réservés Windows et collisions de casse non filtrés | `path_policy.rs:15-16,56-99` ; `git_writer.rs:580-585,1073-1079` | `CODE_READ` (B, nouveau) | `vendor/` jonction vers `~/.ssh` : écriture hors workspace |
| `DUR-01` | Le journal n'est **pas** SQLite/WAL : JSONL append + `sync_all` par enregistrement ; pas de fsync de répertoire ; pas de checksum ; ligne déchirée = `open()` fatal | `publication_journal.rs:262-274,149-152,285-290` | `CODE_READ` (B, nouveau) | coupure après CAS : état ambigu au redémarrage |
| `RESOURCE-14` | Aucun contrôle de descendants : `child.kill()` sur un handle | `common/mod.rs:377-402` | `CODE_READ` (B, nouveau) | une boucle de `fork()` survit au timeout |

> **Correction de statut à porter au dossier :** la V6 qualifie `TREE-02` de « divergence plausible ». L'agent A démontre qu'elle est **structurelle** (deux constructions d'arbre différentes) ; seul l'**impact** reste non reproduit. La formulation « plausible » minimise un défaut d'identité.

---

## 4. Trouvailes qui ne sont PAS dans la V6

| ID | Trouvaille | Pourquoi c'est important |
|---|---|---|
| `X-HOOK-GITDIR` | Un `git worktree` **partage le `.git` réel**. Un hook exécuté dans le worktree peut réécrire `config`, `hooks/`, des objets, ou faire `git update-ref` sur la vraie ref — **hors CAS, hors epoch, hors verrou** (H §3 scénario 3). | C'est le trou le plus grave : la matérialisation « isolée » ne l'est pas vis-à-vis du dépôt. Aucune protection `GIT_INDEX_FILE` ne couvre le `gitdir`. |
| `X-CTX-COMPLETE` | `BuildContextId` **prétend** capturer les entrées de build ; aucun test ne peut prouver la **complétude** d'un contexte (fichier ignoré, variable, locale, temps, réseau, mtime). | Un `PASS` peut être périmé sans que rien ne le détecte. La garantie est non falsifiable en l'état. |
| `X-HOOK-EFFECTS` | Un hook peut avoir des effets **externes persistants** (push, signature, service). Si le candidat est ensuite rejeté, les effets restent et aucun état ne les représente (A §4.5). | Irréversible ; la machine d'états n'a pas d'état pour ça. |
| `X-HOOK-IDENTITY` | La V6 exige que la politique de hooks fasse partie du contexte, mais aucun digest du `hook_set` n'est calculé ; un hook remplacé entre admission et exécution n'est pas détecté (A §4.6). | Le reçu ne décrit pas ce qui a tourné. |
| `X-HOOK-TIMEOUT` | Aucune borne temporelle sur les hooks ; attente indéfinie (A §4.7). | Un hook qui pend bloque le writer. |
| `X-STATE-OBS` | `CANDIDATE_FROZEN` et `VALIDATING_EXACT_TREE` ne sont pas des états **observables** ; `REF_CAS_PUBLISHED` est indécidable après crash (A §4.2-4.3). | La machine d'états nomme des invariants que le code n'établit pas. Le modèle honnête ajoute `UNKNOWN_OUTCOME` comme état de premier rang. |
| `X-MTIME` | Les builds dépendants des mtimes (Make, incrémental) voient des dates « maintenant » dans une matérialisation neuve (H #13). | Rebuilds inutiles, ou pire, cache hit fondé sur mtime. |
| `X-DETACHED` | Un hook lance un processus détaché qui écrit **après** la vérification (H #15). | Le contrôle des descendants s'arrête au retour du process. |
| `X-GITATTR` | `hash-object` sans `--path` court-circuite `.gitattributes` : CRLF non normalisé, pointeur **LFS jamais généré** (B §FS-07). | Le contenu publié diffère de `git add` ; un gros fichier part en clair. |
| `X-CASE` | Collisions de casse/Unicode non détectées (B §FS-04, H #12). | Le tree publié ne correspond plus au disque sur NTFS/APFS. |
| `X-E2E-HIDDEN` | Un sous-ensemble « impacté vert » ne prouve pas l'absence de régression d'un consommateur inchangé ; la merge queue peut changer le tree (C §2.7). | Faux vert au dernier kilomètre. |

---

## 5. Matrice sandbox — ce qui isole réellement quoi

Lecture : ✅ isole · ◐ partiel · ❌ non · ? `NOT_REVIEWED`. Source : B §2.2, docs primaires Microsoft/man-pages citées.

| Mécanisme | FS | Réseau | Descendants | Secrets même uid | Prérequis |
|---|---|---|---|---|---|
| Job Object (Windows) | ❌ | ❌ | ✅ | ❌ | non — **pas une frontière de sécurité** |
| AppContainer (Windows) | ✅ | ✅ | ✅ | ◐ | compatibilité toolchain `rustc/node/python` **non prouvée** |
| Jeton restreint | ◐ (DACL) | ❌ | ❌ | ◐ | desktop séparé recommandé |
| Windows Sandbox | ✅ (VM) | ✅ (désactivable) | ✅ | ✅ | **Pro/Ent/Edu**, 1 instance, hyperviseur |
| namespaces Linux (userns+mount+pid+net) | ✅ | ✅ | ✅ | ◐ | userns autorisés |
| seccomp-bpf | ❌ | ◐ | ❌ | ❌ | allow-list par outil |
| Landlock | ✅ chemins | ◐ TCP (ABI≥4) | ❌ | ◐ (`stat`/`chmod` non restreignables) | noyau récent |
| bubblewrap | ✅ | ✅ | ✅ | ◐ | paquet externe, **Linux seulement** |
| cgroups v2 | ❌ | ❌ | ✅ | ❌ | `NOT_REVIEWED` |

**Conclusion honnête, et unique :** sur **Windows 11 famille**, il n'existe **aucune** primitive non-VM, non-administrateur, donnant une frontière FS+réseau pour du code non fiable. Sur **Linux desktop sans root**, une frontière crédible existe (userns + bwrap), conditionnée à la distribution et à l'autorisation des userns. **Aucune parité Windows n'est tenable aujourd'hui.**

**Politique H3 proposée (fail-closed) :**
- Linux : `UNTRUSTED_CANDIDATE` = sandbox obligatoire (bwrap ; Landlock en couche supplémentaire si ABI ≥ 4). Absence → `UNSUPPORTED`.
- Windows : `UNTRUSTED_CANDIDATE` → **`MANUAL_REQUIRED` par défaut**. AppContainer seulement après test de compatibilité de la chaîne d'outils ; Windows Sandbox = option assistée documentée, pas un mode automatique.
- `TRUSTED_REPO` : hooks autorisés mais **jamais** annoncés comme « sûrs » ; ils restent du code arbitraire avec les droits de l'utilisateur.

---

## 6. Durabilité : trois niveaux à ne jamais confondre

| Niveau | Ce que le code épinglé garantit | Ce qu'il ne garantit pas |
|---|---|---|
| Process kill | intent fsyncé avant CAS ; outcome après ; `ensure_not_published` empêche le double publish | rien sur les enfants ; rien sur un `fs::write` interrompu (fichier tronqué) |
| OS crash | même ordre ; journal relu ligne à ligne, `InvalidData` = fail-closed | pas de fsync du répertoire ; pas de checksum ; une ligne déchirée peut **bloquer le démarrage** |
| Power loss | `sync_all` pousse l'enregistrement | ni répertoire, ni fichiers de travail, ni objets Git non référencés (gc) ; **aucun ACID** Git+FS+index+journal+hooks+remote |

**Ce que le journal peut promettre :** « une intention enregistrée n'est pas oubliée et un identifiant n'est pas publié deux fois ».
**Ce qu'il ne peut pas promettre :** « le disque reflète la ref après coupure », « aucun fichier tronqué », « les objets candidats survivent à un gc », « l'effet du hook est atomique avec le commit ».

---

## 7. Garanties à NE PAS promettre (liste consolidée, B + G)

1. « `env_clear` / allowlist suffit à isoler » — faux sur les deux OS (secrets sur disque, agents, keychain, `/proc/<pid>/environ`).
2. « Zéro écrasement en mode non managé » — un outil externe n'est pas observable de façon fiable ; `OBSERVED_UNMANAGED` = détection au mieux.
3. « Le ref CAS est un fencing » — il compare un OID, pas une époque ; aucun verrou inter-processus dans le code observé.
4. « WAL = transaction globale » — le journal épinglé n'est pas SQLite ; aucun WAL ne couvre Git+FS+hooks.
5. « Sandbox universelle » — bwrap n'existe pas sur Windows ; Windows Sandbox n'existe pas sur Home ; AppContainer non qualifié.
6. « Les hooks approuvés sont sûrs » — un hook est du code arbitraire exécuté avec les droits de l'utilisateur.
7. « Les logs/artefacts sont sûrs vis-à-vis des secrets » — diagnostics et sortie de hook recopiés sans rédaction.
8. « Le tree publié correspond aux octets du disque » — pas de filtres `.gitattributes`, mode `100644` par défaut, collisions de casse.
9. « Les hooks voient leurs modifications dans le tree publié » — faux dans le prototype (tree figé avant).
10. « `UNKNOWN`/`UNSUPPORTED` peut être promu si le reste est vert » — c'est précisément le faux vert à interdire.
11. « Aucun conflit » / « merge automatique sûr » — Flow détecte, il ne fusionne pas le sens.
12. « Preuve de correction » — un reçu atteste l'exécution d'obligations déclarées, pas la justesse du programme.
13. « Compatible Git » — vrai pour lire/réparer ; faux pour l'équivalence avec `git commit` quand des hooks existent.
14. « Zéro perte » — aucun test ne couvre tous les entrelacements.
15. « Prêt pour l'entreprise » — R7 n'est pas une version, c'est une autre activité.

---

## 8. Matrice de tests négatifs — IDs canoniques proposés

**Problème résolu (`X-07`) :** deux numérotations coexistent (V6 §9 vs `SECURITY_TESTS.md`). Proposition : **IDs canoniques = ceux de la V6 §9**, et `SECURITY_TESTS.md` devient un **index d'alias**, pas une seconde source.

| Canonique (V6) | Alias `SECURITY_TESTS.md` | Comportement attendu (renforcé) |
|---|---|---|
| `TREE-01` | `TREE-01` | un dirty hors patch qui rend le build vert **doit** faire échouer la publication ; vérifier depuis le tree OID |
| `HOOK-02a` | `HOOK-02a` | staging du `pre-commit` inclus **ou** refus bruyant ; jamais succès silencieux |
| `HOOK-02b` | `HOOK-02b` | workspace utilisateur intact ; hook dans une matérialisation privée |
| `HOOK-02c` | `HOOK-02c` | ordre `pre-commit → prepare-commit-msg → commit-msg → post-commit` |
| `HOOK-02d` (= `02d` V6) | — | hook qui échoue / fichier non stagé / `post-commit` rouge après CAS |
| `CONTEXT-03` | `TREE-03` | entrée ignorée/LFS/submodule : contexte déclaré ou `UNKNOWN` |
| `ENV-04a` | `ENV-04` | variable canari absente du processus non fiable |
| **`ENV-04b` (nouveau)** | — | secrets **sur disque** (`~/.ssh`, `.env`, `%APPDATA%`) inaccessibles |
| **`ENV-04c` (nouveau)** | — | aucun canari dans le journal/les artefacts ; rédaction avant persistance |
| `SANDBOX-05` | `TRUST-15` | lecture de faux `~/.ssh` + réseau sortant refusés **si mode annoncé** ; sinon code non exécuté |
| `LEGACY-06` | `LEGACY-06` | `AUTO_SAFE` + `SKIPPED` → `TEXTUALLY_APPLICABLE` seulement |
| `FENCE-07a/b/c/d` | `FENCE-07` | lock + epoch + CAS + outcome ; git externe documenté comme hors promesse ; crash entre CAS et outcome = `UNKNOWN_OUTCOME` |
| `PATH-08` | `PATH-16` | `..`, **parents** symlink/jonction, noms réservés, casse, Unicode : refus ou `UNSUPPORTED` |
| `FILE-09` | `FILE-09` | binaire, `100755`, `120000`, `160000`, delete/rename : préservés ou `UNSUPPORTED` |
| `REF-10` | `GIT-10` | CAS échoue sur old OID divergent |
| `CRASH-11` | `CRASH-12` | replay idempotent à chaque frontière ; état publié identifiable |
| `E2E-12` | `E2E-13` | API cassée + consommateur inchangé → gate rouge |
| `CACHE-13` | `CACHE-14` | flags/lockfile/toolchain/OS/sandbox/policy changés → miss ou revalidation |
| `RESOURCE-14` | `RESOURCE-14` | 16 Go, fork bomb, annulation, descendants : aucun orphelin |
| `FORGE-15` | — | merge queue/squash → requalification au tree final |
| `RECOVERY-16` | `FS-11`/`RECOVERY-16` | journal tronqué, disque plein, objet GC : reprise honnête non destructive |
| `UNMANAGED-17` | — | écriture externe : détection, aucune promesse de prévention |
| `OS-18` | — | Windows casse/AV, Linux permissions/FUSE : modes annoncés, pas de fallback destructif |
| **`HOOK-GITDIR-19` (nouveau)** | — | un hook dans un worktree ne peut pas déplacer la vraie ref ni réécrire `.git/config` (sinon `UNSUPPORTED`) |

---

## 9. Correctifs exigés avant toute implémentation (ordonnés, bornés)

| Priorité | Correctif | Effet |
|---|---|---|
| 1 | `R-A1` : exécuter les hooks via `git commit` dans un **worktree détaché jetable**, puis CAS depuis le dépôt principal | supprime d'un coup tree-avant-hooks, ordre inversé, cwd réel |
| 2 | `R-A5` : vérifier depuis un checkout du **tree OID** (`worktree add --detach <tree>`), jamais depuis le snapshot ; assertion `receipt.tree_oid == commit^{tree}` avant CAS | rend le reçu vrai |
| 3 | `R-A3`/`B-R6` : lock OS exclusif `[relire old, CAS, append outcome]` + kill d'arbre de processus ; **pas** d'epoch au MVP | fencing honnête, pas de protocole distribué |
| 4 | `R-A4`/`A §4` : états observables seulement ; `UNKNOWN_OUTCOME` de premier rang ; `HOOKS_EXECUTED_ONCE` + `hook_set_digest` | machine d'états implémentable |
| 5 | `B-R2/R3` : env **par allowlist**, scrub explicite, rédaction avant persistance, 3 tests `ENV-04a/b/c` | ferme la fuite par logs |
| 6 | `B-R4` : vérifier les **parents** de chemin, noms réservés, casse, Unicode ; `UNSUPPORTED` sinon | ferme `PATH-08` |
| 7 | `B-R5` : `hash-object --path` ou refus explicite des dépôts à `.gitattributes` actifs ; préserver `100755` | fidélité Git |
| 8 | `B-R7` : fsync répertoire, longueur+checksum, tolérance de queue tronquée, `temp+rename`, refs de rétention `refs/flow/pending/*` | durabilité honnête |
| 9 | `B-R8` : type `Published` / `PublishedWithResidue` / `NotPublished` | contrat d'erreur non mensonger |
| 10 | `C §3.3` : Flow possède le **graphe d'obligations**, pas l'ordonnanceur de builds | évite un build system de plus |

**Aucun de ces correctifs n'est autorisé aujourd'hui** : ils exigent `GO_R0_FIXTURES` (fixtures) puis un `GO_R1` distinct, et le #1 appartient en priorité à la session Ravum, pas à Flow.

---

## 10. Risque résiduel et conclusion

Même après les dix correctifs, **quatre risques restent irréductibles** et doivent figurer dans toute promesse produit :

1. **Le `gitdir` partagé** (`X-HOOK-GITDIR`) : toute matérialisation par worktree laisse un hook atteindre le dépôt. Seule parade : sandbox OS réelle, ou refus des hooks, ou copie complète sans `.git` partagé (coût disque).
2. **La complétude du contexte** (`X-CTX-COMPLETE`) : non prouvable. La réponse honnête est `UNKNOWN` sur les entrées non déclarées, pas un `PASS`.
3. **Les effets externes des hooks** (`X-HOOK-EFFECTS`) : non transactionnels, non annulables. À déclarer comme tels.
4. **L'asymétrie OS** : aucune promesse Windows symétrique. `UNSUPPORTED`/`MANUAL_REQUIRED` est la seule position tenable.

**Verdict D2 : `PROPOSED` — non franchi.** Le threat model est documenté et sourcé, mais **aucune** des garanties n'est testée. Les trois risques P0 réels (tree/hooks, env/secrets, fencing/durabilité) sont **localisés** et **corrigeables** ; deux nouveaux trous structurels (`gitdir` partagé, complétude du contexte) ne le sont pas entièrement et doivent entrer dans le périmètre de décision H1/H3.

---

*Sources : [B](raw/DEEPSEEK-RAW-B-SECURITY.md) (30 constats, 9 recommandations), [A](raw/DEEPSEEK-RAW-A-GIT-CONCURRENCY.md) (17 constats Git, 9 recommandations), [C](raw/DEEPSEEK-RAW-C-BUILD-CI.md) (E2E/cache), [H](raw/DEEPSEEK-RAW-H-ADVERSARIAL.md) (15 scénarios), V6 lignes citées, docs primaires Microsoft Learn / man-pages 6.19 / bubblewrap / git-scm. Aucun test exécuté.*
