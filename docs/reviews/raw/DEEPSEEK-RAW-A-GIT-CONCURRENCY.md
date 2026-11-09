# DEEPSEEK-RAW-A — Git concurrency, tree/hooks/refs, O0 suffit-il ?

**Agent :** A — Git concurrency · **Date d'observation :** 2026-11-09 (Europe/Paris)
**Statut :** `RAW / EVIDENCE-ONLY` — rapport brut, non consolidé, aucune décision, aucun GO.
**Périmètre :** mission A du mandat. Aucune exécution, aucun build, aucun test, aucun hook lancé, aucun git dans `D:\App\Ravum`. Ravum lu uniquement par `raw.githubusercontent.com` au SHA épinglé.

---

## 1. Mission et méthode

### 1.1 Question posée

1. Vérifier, par lecture au SHA épinglé avec numéros de ligne : `write-tree` avant hooks puis `commit-tree` réutilisant l'arbre ; ordre `prepare-commit-msg` avant `pre-commit` ; hooks avec `current_dir(repo)` ; index alternatif `GIT_INDEX_FILE` et variables retirées ; conséquence exacte des quatre comportements de hook (index, fichier non stagé, message, échec).
2. Arbitrer : O0 (Git natif + copies privées/worktrees éphémères + intégration sérialisée + CI existante) suffit-il pour « temps jusqu'au livrable vérifié » ? Où est l'écart irréductible ?
3. Critiquer la machine d'états V6 §4.1 et le protocole `FENCE-07`.
4. Comparer brièvement GitButler et jj (documentaire seulement ; F traite l'antériorité en profondeur).

### 1.2 Méthode et intégrité des preuves

- Téléchargement des quatre fichiers de la baseline + `publication_journal.rs` depuis `raw.githubusercontent.com/Rwanbt/Ravum/5408d9b3…`, dans `%TEMP%\ravum-a\`.
- **Blob SHA-1 recalculés localement** (`blob <len>\0` + contenu, SHA-1) et **identiques** à la baseline pour les quatre fichiers :
  `git_writer.rs=0194caadac4c695bdc758f2f0f17c27d1d961c3c` (75 434 o, 2 012 l.), `coordinator.rs=a56f0ed8…` (87 825 o), `bin/common/mod.rs=783b87cf…` (29 490 o), `bin/ravum-collab.rs=968f03bc…` (18 271 o).
  Fichier supplémentaire non couvert par la baseline : `publication_journal.rs`, blob auto-calculé `ba08ab25ce4700b24e7ee22e7a7002a8a1198f8c` (14 092 o) — **revérifié par URL seule**, pas par API (voir §6).
- Les numéros de ligne sont ceux du fichier upstream (mêmes octets que le blob).
- Étiquettes : `CODE_READ` (lu dans le code épinglé), `SOURCE_READ` (doc primaire ou fichier de dépôt tiers lu), `PROPOSED` (inférence de conception, non exécutée), `NOT_REPRODUCED`, `NOT_REVIEWED`, `UNKNOWN`. **Aucun `REPRODUCED` n'apparaît dans ce rapport.** Une lecture de code n'est pas une preuve de comportement.
- Les tests négatifs cités (`HOOK-02a/b/c`, `TREE-01`, `REF-10`, `FENCE-07…`) restent des **spécifications** ; ils n'ont pas été exécutés (`NOT_REPRODUCED`).

---

## 2. Constats

Légende d'impact : **P0** = peut produire une publication non conforme ou une perte ; **P1** = garantie annoncée non tenue ; **P2** = défaut documentaire ou de coût.

| ID | Constat | Preuve + étiquette | Impact |
|---|---|---|---|
| `A-01` | `write-tree` est exécuté **avant** les hooks ; `commit-tree` réutilise l'OID calculé avant | `git_writer.rs:841-844` (`write-tree`), `:856` (`run_commit_hooks`), `:866-873` (`commit-tree … &tree`) — `CODE_READ` | **P0** : tout contenu stagé par un hook est absent du commit ; l'arbre publié n'est pas celui que les hooks ont vu. |
| `A-02` | Les hooks sont lancés dans l'ordre `prepare-commit-msg` → `pre-commit` → `commit-msg` | `git_writer.rs:1221-1228` (tableau `steps`) — `CODE_READ` | **P1** : divergence avec `git commit` ; parité hooks impossible sans correction, même avec un index isolé correct. |
| `A-02b` | L'en-tête du module **affirme** « in Git's order » puis liste l'ordre non-Git | `git_writer.rs:37-38` vs `:41-43` — `CODE_READ` | **P2** : commentaire auto-contradictoire ; un relecteur peut croire la parité acquise (voir §4, désaccord avec la baseline). |
| `A-03` | Hooks exécutés avec `current_dir(&self.repo)` — le répertoire de travail réel de l'utilisateur | `git_writer.rs:1343-1344` — `CODE_READ` | **P0** : un hook peut écrire/effacer n'importe quel fichier non suivi ou sale du workspace ; aucune isolation, aucun rollback. |
| `A-04` | Index alternatif réellement isolé : répertoire temporaire privé, `GIT_INDEX_FILE` retiré de l'environnement de tous les `git`, puis réinjecté ciblé | `git_writer.rs:85-94` (`REDIRECTING_GIT_ENV`), `:1756-1793` (`TempArea`), `:1405-1410`, `:1791-1793` — `CODE_READ` | **P1** : l'isolation d'index existe, mais arrive **après** `write-tree` → elle protège l'index réel, pas la cohérence du tree publié. |
| `A-05` | Un hook qui modifie l'index isolé voit ses changements **détruits silencieusement** (index jeté avec `TempArea::drop`, tree déjà figé) | `git_writer.rs:856`, `:866-873`, `:1796-1800` — `CODE_READ` ; conséquence `NOT_REPRODUCED` | **P0** : un `pre-commit` qui fait `git add` (cas courant des formateurs/linters) est un **no-op silencieux**, pas une erreur. La V6 §2.1 ne mentionne que « l'index est protégé » ; elle omet cette diversion. |
| `A-06` | Un hook qui modifie un **fichier** agit sur le workspace réel ; si le fichier est dans le patch, la publication peut ensuite être refusée (`StaleWorkingTree`) ou écrite | `git_writer.rs:1018-1021` (`classify_disk` avant CAS), `:1136-1163` (`apply_disk`) — `CODE_READ` | **P0** : effets de bord persistants même si le candidat est ensuite refusé ; la réconciliation ne couvre que les chemins publiés. |
| `A-07` | Un hook qui modifie le **message** est honoré : le fichier est relu après tous les hooks, un message vide refuse la publication | `git_writer.rs:850-863` — `CODE_READ` | **P1 (correct)** : c'est le seul effet de hook correctement intégré. À noter : `prepare-commit-msg` **et** `commit-msg` reçoivent le même fichier, donc l'ordre inversé ne change pas le résultat message. |
| `A-08` | Un hook qui **échoue** interrompt la préparation ; rien n'est publié, un `REFUSED` est journalisé | `git_writer.rs:1234-1241` (`HookFailed`), `:506-517` (`prepare` → `record_refusal`), `:1652-1670` — `CODE_READ` | **P1 (correct)** : pas de commit partiel. En revanche l'échec n'annule pas les effets de bord déjà produits par les hooks précédents. |
| `A-09` | **Aucun verrou inter-processus, aucun epoch, aucun lease** dans le writer : seul un `Mutex` en mémoire pour l'ensemble `live_intents` | recherche exhaustive `lock\|epoch\|fenc\|lease\|flock` dans `git_writer.rs` → `:1620-1627` uniquement ; `compare_and_swap_ref` `:614-650` = simple `git update-ref` — `CODE_READ` | **P1** : `FENCE-07` (verrou + epoch) est **une exigence documentaire sans implémentation** ; la seule protection réelle est le CAS de ref. |
| `A-10` | `recover()` résout tout `INTENT` sans `OUTCOME` **non vivant dans ce processus** ; le journal est documenté « one writer per file » | `git_writer.rs:654-750` + `:1629-1637` ; `publication_journal.rs:134-135` (« intended for one writer per file »), `:239-243` (dernier outcome) — `CODE_READ` | **P0 (si multi-processus)** : un second processus peut étiqueter « unpublished » un intent encore en vol, puis le premier publie → journal contradictoire (une ligne fausse reste). Le mode multi-processus est non supporté **et non interdit**. |
| `A-11` | Aucun délai maximal sur les hooks ni sur `git` : attente bloquante | `git_writer.rs:1359` (`command.output()`), `:1437-1439` (`wait_with_output`) — `CODE_READ` | **P1** : un hook qui pend (réseau, prompt, daemon) bloque Flow indéfiniment ; aucun contrôle des descendants. |
| `A-12` | Après le point de commit (CAS), des erreurs sont retournées comme `Err` alors que la ref a bougé : `Journal`, `IndexOutOfSync` | `git_writer.rs:1024` (CAS) puis `:1038-1055` ; `:529-539` (exceptions à la journalisation de refus) — `CODE_READ` | **P0** : un appelant qui lit seulement `Err` conclut « non publié » à tort. Le type d'erreur, pas le statut, porte l'information. |
| `A-13` | La vérification matérialise le **`SourceSnapshot`**, jamais le tree Git : deux objets distincts, aucun témoin d'égalité | `common/mod.rs:156-176` (`scratch`/`materialize` par `snapshot.id()`), `:191` ; `ravum-collab.rs:124-132` (base = snapshot du répertoire) vs `git_writer.rs:789` (`read-tree HEAD`) et `:841` — `CODE_READ` | **P0** : `P0-TREE` n'est pas « plausible » mais **structurel** dès que le répertoire est sale ou que les ensembles de chemins diffèrent (voir §4.1). |
| `A-14` | Un contenu dirty hors patch est inclus dans le snapshot vérifié, absent du tree publié (index semé depuis `HEAD`) | `git_writer.rs:787-789` ; `ravum-collab.rs:333-364` (chemins = union des `integrated_paths`, contenu = overlay) — `CODE_READ` | **P0** : le résultat de vérification n'est pas transportable au tree publié ; `TREE-01` doit échouer. |
| `A-15` | Le CAS de ref est correct et natif : `git update-ref <ref> <new> <old>`, refus sans écriture si la ref a bougé | `git_writer.rs:626-630`, `:635-645` ; doc `git-update-ref` (« after verifying that the current value … matches `<old-oid>` », « Otherwise, no modifications are performed ») — `CODE_READ` + `SOURCE_READ` | **P1 (acquis)** : la primitive de publication existe déjà dans Git ; Flow n'ajoute rien sur ce point. |
| `A-16` | **ABA** : le CAS compare l'OID courant, pas l'histoire ; un acteur externe qui déplace la ref puis la remet à `old` passe le CAS | doc `git-update-ref` (vérification de la valeur courante) — `SOURCE_READ` ; conséquence `PROPOSED` | **P1** : ni le CAS ni l'epoch ne détectent l'ABA, car un acteur externe ne bump pas l'epoch. Le « fencing » est structurellement partiel. |
| `A-17` | L'**identité des hooks** n'est pas figée : le répertoire est résolu à l'exécution (`git rev-parse --git-path hooks`), aucun digest des scripts, aucun lien au candidat | `git_writer.rs:1269-1286`, `:1219-1234` — `CODE_READ` | **P1** : entre `ADMITTED` et l'exécution, un hook peut être remplacé ; le reçu ne prouve pas quels hooks ont tourné, alors que V6 §4.2(9) en fait une entrée de contexte. |
| `A-18` | `reference-transaction` est un **gate natif** réel, avec sémantique propre (`preparing`/`prepared`/`committed`/`aborted`, exit ignoré sauf `preparing`/`prepared`) | `git_writer.rs:626-630` (update-ref natif) ; doc `githooks` §reference-transaction — `CODE_READ` + `SOURCE_READ` | **P1** : un hook `reference-transaction` peut annuler la publication ; « Flow seule autorité de publication » est faux par construction, et c'est heureux. |
| `A-19` | `git worktree` : refs sous `refs/` **partagées** (sauf `refs/bisect|worktree|rewritten`), `HEAD`/index **par worktree**, objet store partagé ; `worktree lock` ne sert qu'à empêcher l'élagage ; « multiple checkout … still experimental … submodules incomplete » | doc `git-worktree` (REFS, DETAILS, lock, BUGS) — `SOURCE_READ` | **P1** : excellente primitive d'isolation éphémère pour O0, **mais** pas un verrou d'exclusion et pas garanti pour superprojets/submodules. |
| `A-20` | jj **n'exécute pas les hooks Git** : « **Hooks: No.** » (issue #405) | `jj-vcs/jj` `docs/git-compatibility.md` (branche `main`, non épinglée) — `SOURCE_READ` | **P1** : O1-jj ne peut pas fournir le gate hooks ; il résout le problème en le supprimant, ce qui est un choix, pas une parité. |
| `A-21` | GitButler : licence `FSL-1.1-MIT` (non OSI), modèle « stacked/parallel branches … drop-in Git UI replacement » | `gitbutlerapp/gitbutler` `LICENSE.md` + `README.md` (branche `master`, non épinglée) — `SOURCE_READ` | **P2** : pas de vendoring possible sans contrainte ; c'est une couche d'automatisation/UI, pas une transaction. |
| `A-22` | Ravum n'a **aucun** `LICENSE`, `LICENSE.md` ni `COPYING` au SHA épinglé | trois requêtes `raw` → HTTP 404 (API `api.github.com` en 403 rate-limit, voir §6) — `SOURCE_READ` | **P1** : aucun grant explicite ; extraction de code à écarter par défaut (`from scratch`). |
| `A-23` | `ARCHITECTURE.md` utilise le vocabulaire que V6 §4.1 déclare **archivé** | `ARCHITECTURE.md:40` (`APPROVED_HOOKS_APPLIED … CHECKS_PENDING … PUBLICATION_INTENT_DURABLE … REF_PUBLISHED`) vs V6 `:184-208` (table d'alias) — `SOURCE_READ` | **P2** : deux vocabulaires « canoniques ». Constat **indépendant** de `DOC-01` de la baseline, même conclusion ; je ne le duplique pas au §4. |
| `A-24` | `APPROVED_HOOKS_REPLAYED` contredit sa propre définition : « replayed » alors que la règle interdit le rejeu | V6 `:188` (état) vs `:204` (« “replayed” ne donne pas le droit de rejouer des effets non idempotents ; exécution contrôlée **une fois** ») — `SOURCE_READ` | **P2** : un implémentateur qui lit le nom rejoue ; un auditeur qui lit la règle ne peut pas prouver « une fois ». |
| `A-25` | §4.1 est une **chaîne**, pas une machine : pas de gardes, pas d'arêtes d'invalidation, pas d'état pour « hooks exécutés puis candidat abandonné » | V6 `:184-198` (états), `:198` (états d'écart sans transitions) — `SOURCE_READ` | **P1** : les effets externes des hooks survivent à `ABORTED`/`REJECTED` sans être modélisés. |
| `A-26` | L'architecture V6 impose **deux matérialisations** par candidat (hooks isolés + validation isolée) là où O0/worktree en impose une | V6 `:112`, `:158`, `:190`, `:210-212` — `PROPOSED` (raisonnement, non mesuré) | **P1** : coût de matérialisation doublé sur la métrique même du produit (« temps jusqu'au livrable ») ; à mesurer avant O3. |

---

## 3. Réponse à la question contradictoire — O0 suffit-il ?

### 3.1 Ce que Git natif donne déjà, sans Flow

Quatre des cinq mécanismes que Flow présente comme son cœur sont **natifs** :

1. **Publication atomique conditionnelle** : `git update-ref <ref> <new> <old>` vérifie la valeur courante et n'écrit rien si elle a bougé (`git-update-ref`). C'est exactement `REF_CAS_PUBLISHED`.
2. **Ordre et contexte des hooks** : `git commit` exécute `pre-commit → prepare-commit-msg → commit-msg → post-commit`, avec le répertoire de travail comme `cwd` et les variables `GIT_*` exportées (`githooks`, DESCRIPTION + §pre-commit). Ravum ne perd cet ordre que **parce qu'il reconstruit le commit avec du plumbing** (`commit-tree`) au lieu d'utiliser `git commit`.
3. **Arbre post-hooks** : `write-tree` construit l'arbre à partir de l'index **courant** (`git-write-tree`) ; en laissant `git commit` écrire l'arbre, les modifications stagées par `pre-commit` sont incluses. Ravum casse cette propriété en figeant l'arbre avant (`A-01`).
4. **Isolation par worktree** : `git worktree add` fournit `HEAD` et index privés, objet store et refs partagés (`git-worktree`, REFS/DETAILS). Un worktree **détaché** jetable donne l'isolation d'exécution sans branche supplémentaire.

### 3.2 O0 « prepare/publish » en commandes Git natives

Séquence native, sans moteur (proposition, `PROPOSED`, à falsifier en R0) :

```text
git worktree add --detach <tmp> <expected_old>        # vue isolée, HEAD = base attendue
  <apply patch bytes dans <tmp>>                      # contenu du candidat
  git -C <tmp> commit -m <msg>                        # hooks Git réels, ordre Git, index du worktree
new=$(git -C <tmp> rev-parse HEAD)                    # OID du commit candidat (objet store partagé)
tree=$(git -C <tmp> rev-parse 'HEAD^{tree}')          # identité du tree
git -C <main> update-ref <ref> "$new" <expected_old>  # CAS natif, refus sinon
git worktree remove --force <tmp>                     # nettoyage
```

Coût : **un** checkout. Ce que ça donne gratuitement : ordre des hooks, index par worktree, tree post-hooks, CAS, conservation des objets, reflog. Ce qui reste à écrire : la politique de vérification, le reçu, l'attribution — soit quelques centaines de lignes, pas un VCS.

**Point à falsifier :** que les modifications stagées par `pre-commit` soient bien dans le commit produit par `git commit` (comportement attendu de Git, non reproduit ici). C'est le test `HOOK-02a` appliqué à O0 ; s'il échoue, mon argument principal tombe.

### 3.3 Où O0 est réellement faible (honnêtement)

- **Sérialisation** : rien n'empêche deux scripts O0 de préparer en parallèle ; le CAS rejette le second, mais le travail est perdu. C'est un problème de **scheduling**, pas de Git : un simple verrou de fichier ou une file suffit.
- **Reçu et politique** : O0 ne fournit ni `VerificationReceipt` ni lien tree+contexte. C'est scriptable, mais c'est du code à écrire — c'est précisément la partie que Flow peut légitimement revendiquer.
- **Hooks sales** : un worktree jetable ne contient **pas** l'état ignoré (`node_modules`, `target/`, `.venv`, `.env`). Un hook ou un build qui en dépend échoue ou se comporte différemment. C'est le vrai coût d'isolation, et il frappe **toute** architecture, y compris O3.
- **Hooks qui écrivent** : dans O0, ils écrivent dans le worktree jetable (bien) mais certains hooks écrivent des chemins absolus ou attendent l'état local (mal). Aucune architecture ne peut satisfaire les deux.

### 3.4 Où est l'écart irréductible ?

**Il n'est pas dans la concurrence Git.** Ce que Flow promet autour de Git est soit déjà natif (§3.1), soit impossible à garantir de façon universelle (fencing d'un `git` externe, atomicité Git+FS+DB+hooks+forge, hooks idempotents). Un acteur externe — IDE, `git` en ligne de commande, script — ne connaît ni le verrou ni l'epoch ; l'ABA (`A-16`) le prouve : même le CAS ne raconte pas l'histoire de la ref.

L'écart irréductible est ailleurs, et il est **double** :

1. **La matérialisation de l'environnement de vérification.** Vérifier « le tree exact » exige des dépendances installées, une toolchain, des fichiers générés. Ce coût (disque, I/O, installations) domine largement le coût Git, et il est payé **par candidat**. Flow, tel que spécifié, le paie deux fois (`A-26`) : espace hooks + espace de validation. C'est mesurable et c'est le vrai risque du produit.
2. **La preuve que les octets validés sont les octets publiés** quand la validation consomme des entrées non-Git (toolchain, env, fichiers ignorés). Aucun état, aucun journal, aucun verrou ne rend cette égalité vraie ; seule une **identité** (tree OID + contexte) la rend *vérifiable*. C'est un schéma de reçu, pas une machine transactionnelle.

### 3.5 Verdict argumenté

**O0 suffit pour la tranche Git/concurrence telle que définie par le mandat**, à une condition : exécuter les hooks via `git commit` dans une vue jetable, puis publier par CAS depuis le dépôt principal. Dans ce montage, les quatre propriétés revendiquées par Flow (ordre des hooks, tree post-hooks, isolation, CAS) sont obtenues **sans code Flow**, avec un seul checkout.

**O0 ne suffit pas** pour ce que Flow appelle « vérifié » : politique, reçu lié au tree+contexte, attribution multi-agents. Mais ce n'est pas un moteur VCS ; c'est une couche de politique et de provenance. La conclusion pratique est donc : **O2 « mince » au sens strict** (reçu + politique + CLI) reste défendable ; **le writer transactionnel de Flow (O2 complet/O3) n'a pas de justification propre** tant que S9 n'a pas montré que le coût dominant est humain/politique et non mécanique/Git.

**Condition d'abandon de Flow (la mienne, falsifiable) :**

- `ABANDON-1` : si un script O0 ≤ ~300 lignes + CI reproduit les résultats attendus de `HOOK-02a/b/c`, `TREE-01`, `REF-10`, `CRASH-12`, `FENCE-07c` sur fixtures, avec un temps total par livrable ≤ celui de Flow (p95, même matériel), alors **abandonner** le writer/transaction de Flow et ne conserver qu'un schéma de reçu.
- `ABANDON-2` : si une exigence de Flow ne peut être satisfaite qu'en fencant un acteur Git/IDE externe, **abandonner l'exigence** immédiatement (elle est insatisfaisable) et la déclarer `OBSERVED_UNMANAGED`. La V6 le dit déjà (§4.3) ; il faut en faire une règle, pas une note.
- `ABANDON-3` : si S9 montre que la matérialisation des dépendances domine le temps par livrable, et que Flow exige deux matérialisations par candidat, **abandonner O3** au profit d'O0/copies tant que le cache de matérialisation n'est pas démontré.
- `ABANDON-4` (partiel) : si les projets cibles dépendent de hooks (lint/format/signature), **jj est disqualifié comme O1** pour ces projets (`A-20`) ; ce n'est pas un abandon de Flow, c'est une élimination d'alternative.

### 3.6 Comparaison brève : GitButler et jj (documentaire seulement)

F traite l'antériorité en profondeur ; je ne retiens ici que ce qui change la réponse à « O0 suffit-il ? ».

- **jj** (`Apache-2.0`, vérifié sur `LICENSE`) : le working copy **est** un commit, recréé automatiquement à chaque commande (`docs/working-copy.md`) ; le modèle décrit n'a **pas d'étape de staging séparée**, et les conflits sont de premier ordre dans les commits (`docs/conflicts.md`, non audité ici). Conséquence : le « tree figé » est naturel chez jj, mais **les hooks Git n'existent pas** (« **Hooks: No.** », `docs/git-compatibility.md`, issue #405). jj ne satisfait donc pas l'exigence « hooks dans l'ordre Git » : il la **dissout** en checks natifs. Pour un projet dont la gate est un `pre-commit` (lint, format, signature), O1-jj = perte de la gate, pas équivalence.
- **GitButler** (`FSL-1.1-MIT`, vérifié sur `LICENSE.md` ; licence non OSI, « MIT future license ») : l'outil applique des branches « stacked/parallel » dans **un seul répertoire de travail** et se présente comme un « drop-in Git user interface replacement » avec un CLI `but`. Conséquence : il résout l'ergonomie **humaine** du travail parallèle, pas la question « quel tree a été vérifié ». Il opère sur le répertoire réel : ni isolation de hooks, ni fencing d'un acteur externe. La licence interdit de le vendorer dans un Flow permissif sans travail juridique.
- **Commun** : les deux délèguent objets et refs à Git ; aucun n'introduit de reçu lié `tree + contexte` ; aucun ne rejoue les hooks Git dans l'ordre (jj : non ; GitButler : `NOT_REVIEWED`). Ils ne tranchent donc pas O0 vs Flow — ils montrent au contraire que « travailler en parallèle dans un répertoire » est une couche d'UX/automatisation au-dessus du Git natif, ce qui **renforce** la thèse O0.

---

## 4. Désaccords réels avec V6 et les docs

Je ne liste que des désaccords argumentés ; `DOC-01` (alias de `ARCHITECTURE.md`) est confirmé indépendamment mais déjà traité par la baseline et n'est pas répété.

### 4.1 « P0-TREE est une divergence plausible » — non, elle est structurelle

V6 `:89` : « Divergence **plausible non reproduite** ». La lecture des deux côtés du chemin montre une divergence **par construction** : la vérification matérialise `SourceSnapshot` (`common/mod.rs:156-176`, nom de répertoire dérivé de `snapshot.id()`), la publication construit un tree depuis `HEAD` + chemins (`git_writer.rs:787-789`) et le contenu de l'overlay (`ravum-collab.rs:347-364`). Dès que le répertoire est sale hors patch, ou que les ensembles de chemins diffèrent, **les octets vérifiés ne sont pas les octets publiés** — ce n'est pas un aléa, c'est une identité manquante. Ce qui n'est pas reproduit, c'est l'**impact** (un test rouge devenu vert, ou l'inverse). Reformulation honnête : `P0-TREE` = *divergence structurelle, impact non reproduit*. La différence n'est pas rhétorique : elle interdit de classer le risque en « à confirmer » et impose un test `TREE-01` **qui doit échouer**.

### 4.2 `CANDIDATE_FROZEN` et `VALIDATING_EXACT_TREE` ne sont pas observables

V6 `:189` : `CANDIDATE_FROZEN [write-tree FINAL après les hooks ; tree OID immuable]`. Un OID est immuable **par construction** ; « frozen » est une propriété de politique (personne ne rejoue les hooks), pas un fait observable. La seule preuve dans le code est la réutilisation de la variable `tree` (`:866-873`) — c'est-à-dire une variable locale, pas un état du système.

Même problème pour `VALIDATING_EXACT_TREE` (`:190`) : rien, dans le code épinglé, ne relie la matérialisation de validation au tree OID ; elle part du snapshot (`A-13`). L'état **nomme une invariante que le chemin d'exécution n'établit pas**. Un état de machine doit être franchi par un événement observable ; ici c'est une affirmation de reçu. Recommandation : transformer ces deux états en **champs** (`tree_oid`, `hooks_run_once_at`, `hook_set_digest`) et ne garder comme états que ceux qui ont un témoin externe — `INTENT_DURABLE`, puis `UNKNOWN_OUTCOME`/`RECONCILED` redéfinis comme en §4.3.

### 4.3 `REF_CAS_PUBLISHED` vs réalité : la chaîne linéaire masque la fenêtre d'ambiguïté

V6 `:194` : `REF_CAS_PUBLISHED [epoch recontrôlé + update-ref CAS sous verrou writer exclusif]`, et `:235` reconnaît : « Un crash après la ref CAS et avant journal outcome est un état récupérable, pas une preuve qu'il n'y a pas eu publication ». Les deux ne peuvent pas être vrais ensemble : si le crash est possible dans la fenêtre, alors `REF_CAS_PUBLISHED` **n'est pas un état observable** — au redémarrage on ne sait pas si on y est. La chaîne `… → INTENT_DURABLE → REF_CAS_PUBLISHED → RECONCILED` suggère une progression connaissable ; la réalité est un **2PC avec une fenêtre indécidable** dont le seul oracle est la relecture de la ref. Le modèle honnête est : `INTENT_DURABLE → (CAS tenté) → {PUBLISHED | NOT_PUBLISHED | UNKNOWN_OUTCOME} → RECONCILED`, avec `UNKNOWN_OUTCOME` comme état de premier rang, pas comme « écart » rejeté en §4.5.

Preuve que le code ne distingue pas ces cas à l'API : `publish()` retourne `Err` pour `IndexOutOfSync` et `Journal` **après** le CAS (`:1050-1055`, `:1038-1049`, exceptions `:529-539`). Le type d'erreur ne dit pas « publié » ; le statut de la ref le dit. C'est une fragilité d'interface qui se répercutera dans le schéma de reçu.

### 4.4 `APPROVED_HOOKS_REPLAYED` : le nom contredit la règle

V6 `:188` nomme l'état `APPROVED_HOOKS_REPLAYED` ; V6 `:204` précise : « “replayed” ne donne pas le droit de rejouer des effets non idempotents ; **exécution contrôlée une fois** pour le candidat gelé ». Un état qui s'appelle « rejoué » et une règle qui dit « une fois » ne peuvent pas être implémentés sans arbitrage. Ajoutez l'ambiguïté de « APPROVED » (les hooks sont-ils approuvés, ou l'exécution est-elle approuvée ?) et l'état devient inutilisable comme contrat. `HOOKS_EXECUTED_ONCE` + `hook_set_digest` serait à la fois fidèle et vérifiable.

### 4.5 Il manque l'état des effets de bord des hooks

La V6 exige des hooks « exécutés uniquement selon politique, dans une matérialisation isolée » (`:210`) et interdit de les rejouer. Mais un hook peut avoir déjà : écrit dans un service externe, poussé une image, signé un artefact, modifié le workspace réel (dans le prototype, `A-03`/`A-06`). Si le candidat est ensuite `REJECTED`/`ABORTED`, **les effets persistent** et aucun état ne les représente. La matrice `HOOK-02d` (`:411`) couvre « hook échoue / fichier non stagé », pas « hooks exécutés, candidat abandonné, effets externes persistés ». C'est le trou le plus dangereux de la machine d'états, parce qu'il est irréversible.

### 4.6 L'identité des hooks n'est pas dans le contexte, malgré §4.2(9)

V6 `:228` : « Un hook facultatif/obligatoire et sa policy font partie du contexte. » Le writer résout le répertoire de hooks **à l'exécution** (`:1269-1286`) et n'en calcule aucun digest (`A-17`). Un hook remplacé entre l'admission et l'exécution produit donc un reçu qui ne décrit pas ce qui a tourné. La phrase de la V6 est une exigence sans mécanisme ; il faut soit hasher le `hook_set` dans `BuildContextId`, soit renoncer à la revendication.

### 4.7 Le contrat de hooks n'a pas de borne temporelle

V6 §5.4 parle de budgets et de préemption au niveau de l'ordonnanceur, mais le contrat de hooks (§2.1, §4.1) ne fixe **aucun délai** ni règle de mort des descendants ; le code attend indéfiniment (`A-11`). Un hook qui pend rend le candidat non qualifiable et bloque le writer. « Exécution isolée » sans deadline n'est pas une garantie opérationnelle.

### 4.8 Désaccord avec la baseline, pas avec V6 : l'en-tête de `git_writer.rs` n'est pas un « écart assumé »

La baseline (`:88`) écrit : « l'en-tête du module **documente** `prepare-commit-msg` avant `pre-commit` … C'est un écart **assumé** au comportement Git, pas une omission cachée. » Je lis autrement : `git_writer.rs:37-38` affirme « the writer runs the commit hooks itself, **in Git's order** », puis `:41-43` liste un ordre qui n'est pas celui de Git. L'en-tête est **auto-contradictoire** : il revendique la parité et documente la divergence dans le même paragraphe. Ce n'est pas un écart assumé, c'est une affirmation fausse — et c'est plus grave pour un auditeur, parce qu'elle décourage la vérification.

---

## 5. Recommandations bornées et falsification

Chaque recommandation est bornée (un fichier, pas de nouveau protocole) et accompagnée du test qui pourrait la réfuter. Aucune n'est autorisée sans `GO` distinct.

| ID | Recommandation | Borne | Falsification |
|---|---|---|---|
| `R-A1` | Remplacer `write-tree`+`commit-tree` par un `git commit` dans un **worktree détaché jetable**, puis CAS depuis le dépôt principal. Supprime `A-01`, `A-02`, `A-03`, `A-05` d'un coup. | 1 module, ~100 lignes ; aucun nouveau format | `HOOK-02a` : si le staging du `pre-commit` n'apparaît pas dans `commit^{tree}`, la recommandation est fausse. |
| `R-A2` | Si le plumbing est conservé : déplacer `write-tree` **après** les hooks, relire l'index isolé, refuser explicitement tout changement d'index non déclaré (pas de no-op silencieux). | 1 fonction (`prepare_inner`) | `HOOK-02a` sur fixture : un `git add` de hook doit produire soit inclusion, soit refus bruyant — jamais succès silencieux. |
| `R-A3` | Ne pas implémenter epoch/leases au MVP. Garder : CAS natif + lock OS exclusif couvrant `[relire old, CAS, append outcome]` + idempotency key existante. | 1 fichier de lock, ~50 lignes | `FENCE-07b` : deux writers concurrents ; un seul doit publier. Si le lock OS ne suffit pas, réintroduire l'epoch avec preuve. |
| `R-A4` | Renommer et réduire les états : `HOOKS_EXECUTED_ONCE`, `TREE_RECEIPT` (champ), `PUBLICATION_UNKNOWN_OUTCOME` (état explicite), `HOOK_SIDE_EFFECTS_PERSISTED`. Ajouter les arêtes d'invalidation (`STALE` depuis chaque état). | 1 section de doc + schéma de reçu | Revue : tout état doit citer l'événement externe qui le franchit ; un état sans témoin est supprimé. |
| `R-A5` | **Un seul arbre** : vérifier depuis un checkout du tree OID (`git worktree add --detach <tree-oid>` ou `git archive`), jamais depuis le snapshot ; assertion `receipt.tree_oid == commit^{tree}` avant CAS. | 1 chemin de vérification | `TREE-01` modifié : un fichier dirty hors patch doit faire échouer la publication. Aujourd'hui il peut passer. |
| `R-A6` | Délai maximal + mort de l'arbre de processus pour hooks **et** `git` ; `UNSUPPORTED` documenté sur FS réseau (verrous POSIX non fiables). | 1 helper de spawn | Test : hook `sleep 600` → `TIMEOUT` en temps borné, aucun processus résiduel. |
| `R-A7` | Figer l'identité des hooks : digest du `hook_set` + policy dans le contexte ; refuser si le digest change entre admission et exécution. | 1 champ + 1 vérification | Test : remplacer un hook entre `ADMITTED` et l'exécution → refus contrôlé. |
| `R-A8` | Mesurer **le nombre de matérialisations par candidat** comme variable explicite de S9 (1 vs 2), avec cache de dépendances activé/désactivé. | protocole S9 (agent E) | Si 2 matérialisations ne coûtent pas plus que 1 grâce au cache, `A-26` tombe ; sinon O3 doit être rejeté. |
| `R-A9` | Documentaire : corriger `git_writer.rs:37-43` (commentaire faux) et aligner `ARCHITECTURE.md:40` sur V6 §4.1. | 2 fichiers, aucun code | — (correction factuelle, non falsifiable). |

**Ordre de priorité :** `R-A1` (supprime 4 constats P0), puis `R-A5` (le seul qui rend le reçu vrai), puis `R-A3` (simplification), puis le reste. `R-A4` avant toute implémentation d'états.

---

## 6. Limites et `NOT_REVIEWED`

| Sujet | Statut | Raison |
|---|---|---|
| Reproduction dynamique de tout constat | `NOT_REPRODUCED` | Interdiction d'exécuter ; aucun hook, test, build ou fixture lancé. |
| `api.github.com` (issues, PR, protection, licences via API) | `UNKNOWN` | **HTTP 403 rate limit** au moment de l'audit ; les faits de la baseline sur les issues/PR/protection ne sont pas revérifiés ici. |
| `publication_journal.rs` | `SOURCE_READ` non croisé | Blob auto-calculé, pas de référence indépendante (API indisponible). Lu à la même URL épinglée. |
| Comportement de `git commit` vis-à-vis d'un `pre-commit` qui stage | `PROPOSED` | Inféré des docs (`githooks`, `write-tree`), **non reproduit** ; c'est l'hypothèse centrale de `R-A1`. |
| `git worktree add --detach <tree-oid>` / coût réel d'un checkout | `NOT_REVIEWED` | Non mesuré ; dépend du dépôt et du FS. |
| Sémantique interne de jj (conflits de premier ordre, workspaces) | `NOT_REVIEWED` | Hors périmètre (agent F) ; seule la ligne « Hooks: No » et la licence sont vérifiées. |
| Comportement hooks de GitButler | `NOT_REVIEWED` | Non documenté dans les fichiers lus. |
| Version de Git installée sur la machine de référence | `UNKNOWN` | Non interrogée ; les docs citées sont celles servies par git-scm.com le 2026-11-09. |
| Comportement FS réseau (SMB/NFS), verrous, `update-ref` | `NOT_REVIEWED` | Non mesuré ; l'affirmation « non supporté » est une prudence, pas un résultat. |
| Les 42 ADR Ravum, le protocole S9, l'antériorité complète | `NOT_REVIEWED` ici | Délégués (agents D, E, F). |
| Codes/scripts de `coordinator.rs` hors surface Git | `CODE_READ` partiel | Lu ciblé (constructeurs, `V1CollabCompat`, pas de Git) ; sémantique métier non auditée. |

**Ce que je n'ai pas pu vérifier et qui compte :** que `git commit` inclue bien le staging d'un `pre-commit` (hypothèse de `R-A1`) ; le coût réel d'une matérialisation ; la fiabilité des verrous sur le FS de la machine cible.

---

## 7. Références (URL + SHA + date)

**Date d'observation : 2026-11-09, Europe/Paris.** Aucune URL n'a été consultée en écriture.

### 7.1 Code Ravum — lecture seule, SHA épinglé

Base : `https://github.com/Rwanbt/Ravum` @ `5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96` (`dev`, tête observée par la baseline). Accès : `raw.githubusercontent.com` ; blobs recalculés et conformes.

| Fichier | URL | Blob SHA-1 |
|---|---|---|
| `git_writer.rs` | `…/5408d9b3…/crates/ravum-workspace-coordinator/src/git_writer.rs` | `0194caadac4c695bdc758f2f0f17c27d1d961c3c` |
| `coordinator.rs` | `…/src/coordinator.rs` | `a56f0ed88dae7e08872360cb2932e557d2bb65f0` |
| `bin/common/mod.rs` | `…/src/bin/common/mod.rs` | `783b87cfbacee3d95a502ee6006197c870d270d8` |
| `bin/ravum-collab.rs` | `…/src/bin/ravum-collab.rs` | `968f03bc2107a6542f42dc98d56ba6e919a4f736` |
| `publication_journal.rs` | `…/src/publication_journal.rs` | `ba08ab25ce4700b24e7ee22e7a7002a8a1198f8c` (auto) |

### 7.2 Documentation Git primaire

| Source | URL | Version servie |
|---|---|---|
| `githooks` (ordre, cwd, `GIT_DIR` exporté, `reference-transaction`) | `https://git-scm.com/docs/githooks` | màj 2.54.0 |
| `git-update-ref` (CAS 3 arguments, tout-ou-rien, reflog) | `https://git-scm.com/docs/git-update-ref` | màj 2.53.0 |
| `git-worktree` (refs partagées, HEAD/index privés, `lock`, BUGS) | `https://git-scm.com/docs/git-worktree` | màj 2.56.0 |
| `git-update-index` (`--cacheinfo`, `--force-remove`, `--add`) | `https://git-scm.com/docs/git-update-index` | màj 2.52.0 |
| `git-write-tree` (« Creates a tree object using the current index ») | `https://git-scm.com/docs/git-write-tree` | page du 2026-11-09 |
| `git` §Environment Variables (`GIT_INDEX_FILE`, `GIT_COMMON_DIR`) | `https://git-scm.com/docs/git` | page `git(1)` du 2026-11-09 |

### 7.3 Outils existants — documentaire seulement

| Source | URL | Étiquette |
|---|---|---|
| jj — licence | `https://raw.githubusercontent.com/jj-vcs/jj/main/LICENSE` → Apache License 2.0 | `SOURCE_READ` (branche `main`, non épinglée) |
| jj — « Hooks: No » | `…/jj/main/docs/git-compatibility.md` (issue #405) | `SOURCE_READ` |
| jj — working copy = commit | `…/jj/main/docs/working-copy.md` | `SOURCE_READ` |
| jj — conflits de premier ordre (survol) | `…/jj/main/docs/conflicts.md` | `NOT_REVIEWED` (profondeur) / `SOURCE_READ` (existence) |
| GitButler — licence | `…/gitbutlerapp/gitbutler/master/LICENSE.md` → `FSL-1.1-MIT` | `SOURCE_READ` |
| GitButler — README (« stacked branches, parallel branches … drop-in Git UI ») | `…/gitbutlerapp/gitbutler/master/README.md` | `SOURCE_READ` |

> Les deux dépôts tiers ont été lus sur leur branche par défaut **sans SHA épinglé** (API GitHub indisponible, 403). C'est une limite explicite : ces deux lignes ne valent pas ancre de provenance.

### 7.4 Documents Flow (dépôt local, HEAD `1d5fddf`, branche `docs/deepseek-d0-d4-review`)

- `docs/reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md` — §2, §2.1, §4.1-4.3, §9, §10.2 (lignes citées dans le texte).
- `docs/ARCHITECTURE.md:40` ; `docs/SECURITY_TESTS.md:13` ; `docs/ROADMAP.md` ; `docs/BACKLOG.md` ; `docs/adr/FLOW-ADR-001/003/005/010` ; `docs/rfc/RF-RFC-003/005/006` ; `AGENTS.md`.
- `docs/reviews/DEEPSEEK-00-BASELINE.md` (§2.1 blobs, §3 constats, `DOC-01`).

---

*Fin du rapport brut A. Aucun test, aucun build, aucune écriture dans Ravum, aucun changement de branche, aucun commit. Les constats `A-*` sont des lectures ; aucun n'est un bug reproduit.*
