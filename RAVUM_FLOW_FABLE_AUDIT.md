# RAVUM FLOW — Contre-audit indépendant et adversarial (Fable)

## 1. Métadonnées

| Champ | Valeur |
|---|---|
| Date | 2026-11-15 |
| Dépôt | `Rwanbt/Ravum-flow` (checkout local `D:\App\Ravum-flow`) |
| SHA audité | `1952325c973c3b002d6436a83656a5593003ad48` (`main` = `origin/main`) |
| `origin/dev` | `2ab79c754e0a48746fc50689af28d8124b536563` (contenu équivalent, merge de #69 → #70) |
| Version | `0.2.0-rc.2` |
| Périmètre lu | `proposal.rs` (en entier), `commands.rs` (politique, `cmd_run`, `cmd_publish`, `cmd_gc` en-tête), `state.rs` (config, écriture atomique, journal, reçus), `lock.rs` (acquisition/vol), `gitcli.rs` (env, `update_ref`, obligations), `scratch.rs`, `json.rs` (ciblé), liste des tests de `tests/concurrency.rs` |
| Non lu | `main.rs` (hors dispatch), `paths.rs` (seulement via appels), `cmd_reconcile` en détail, le reste des tests, CI, docs V6 |
| Nature des preuves | Lecture de code au SHA audité, puis deux tests jetables (G1, G3) exécutés le 2026-11-15 dans un clone local (`git clone` vers le scratchpad, Windows 11, `cargo test --release`). Le dépôt n'a pas été modifié. FBL-001 est `CONFIRMED_BY_TEST` ; les autres constats restent `CONFIRMED_BY_CODE` ou `HYPOTHESIS`. Le code des tests est en annexe §12. |
| Restrictions respectées | Aucun code produit modifié, aucune branche touchée, aucune PR. Seul ce fichier est créé (non commité). |

## 2. Résumé exécutif

**Verdict : `NOT_READY` pour un usage multi-agents concurrent ; `PRODUCTION_PARTIAL` reste défendable pour un opérateur unique séquentiel.**

Le chemin de publication (`cmd_publish`) est solide : verrou, recalcul de la politique, CAS `update-ref <new> <old>`, journal intent/outcome. Les défauts sont **autour** de ce chemin, dans tout ce qui n'est pas sous verrou : `propose`, `assemble`, `run`, `gc`. La promesse produit (« collaboration continue multi-agents ») est précisément le cas où ces commandes tournent en parallèle.

Constats bloquants :

1. **FBL-001 (P0)** — `admit()` écrit le `.patch` puis le `.json` sans verrou ni vérification : deux admissions concurrentes de même identifiant peuvent laisser un patch **d'un agent** sous les métadonnées (`content_digest`, `tree_oid`, `changes`) **d'un autre**. `assemble` lit ensuite ce patch sans le re-hacher → candidat construit à partir d'octets non attribués. (AUD-01 + AUD-05 confirmés et aggravés.)
2. **FBL-002 (P1)** — `cmd_run` n'a ni verrou ni isolation par exécution : deux runs du même arbre écrasent les logs l'un de l'autre et le dernier reçu écrit gagne (AUD-06 confirmé).
3. **FBL-003 (P1)** — le vol d'un verrou mort est un TOCTOU lire-vérifier-supprimer : deux voleurs concurrents peuvent supprimer le verrou fraîchement créé par l'autre → deux `publish`/`reconcile` simultanés → perte d'un enregistrement du journal (réécriture complète par `append_journal`).
4. **FBL-004 (P1)** — `context_id` ne couvre ni les valeurs d'environnement (`PATH`, `CARGO_HOME`, `RUSTUP_HOME`…), ni le binaire résolu, ni la version des toolchains (AUD-02 confirmé).

Le reste (AUD-03, AUD-04, FBL-005..012) est P2/P3 : décodage permissif, attribution ambiguë, `gc` non sérialisé, verrou sans identité d'hôte.

## 3. Architecture réellement implémentée

CLI Rust mono-crate, zéro dépendance, Git piloté en sous-processus (`gitcli.rs`). État dans `.flow/` :

```
.flow/config.json          politique (obligations)         écrit par init/approve
.flow/proposals/<id>.patch octets du patch                 propose   (SANS verrou)
.flow/proposals/<id>.json  métadonnées + content_digest    propose   (SANS verrou)
.flow/candidates/<id>.json arbre + liste ordonnée          assemble  (SANS verrou)
.flow/logs/<tree>/<ob>.*   sorties obligations             run       (SANS verrou)
.flow/receipts/<tree>.json reçu complet (1 slot / arbre)   run       (SANS verrou)
.flow/journal.jsonl        intent/outcome                  publish/reconcile (verrou)
.flow/lock                 flow-lock/2 (pid, nonce)        publish/reconcile
```

Flux : `propose` (index temporaire, `apply --cached`, `write-tree`, validation chemins/arbre) → `assemble` (même base, détection de conflits par chemin, application ordonnée) → `run` (`commit-tree`, `git worktree add` dans `%TEMP%`, détachement `.git`, vérification de fidélité avant/après, obligations sous env allowlisté) → `publish` (verrou, recontrôle reçu/politique/contexte, CAS, journal).

Points forts vérifiés par lecture : écriture atomique fsync + rename (`state.rs:357-384`), refus des clés JSON dupliquées (`json.rs:232`), garde hard-link (`state.rs:340`), contrôle de fidélité du checkout avant et après exécution (`commands.rs:1413,1479`), CAS ref + recheck post-CAS (`commands.rs:2002-2058`), refus des symrefs (`commands.rs:1685`).

## 4. Constats ChatGPT (AUD-01..06)

| ID | Verdict | Preuve | Commentaire |
|---|---|---|---|
| AUD-01 | **CONFIRMED_BY_CODE**, P0 (→ FBL-001) | `proposal.rs:638-654` (check), `:732-738` (écritures) ; aucun `acquire_lock` hors publish/reconcile (`grep`) | Pire que décrit : ce n'est pas seulement « double création », c'est un **mélange patch/métadonnées** entre deux écrivains. |
| AUD-02 | **CONFIRMED_BY_CODE**, P1 (→ FBL-004) | `commands.rs:724-734`, `effective_policy` `:697-718` | Seuls *noms* d'env allowlistés, version Git, OS, arch. Pas de valeurs, pas de `rustc/cargo --version`, pas de chemin résolu du programme (`gitcli.rs:578`). |
| AUD-03 | **CONFIRMED_BY_CODE**, P2 (→ FBL-006) | `state.rs:117-176` | Champs inconnus ignorés, `args` non-string silencieusement **supprimés**, `timeout_s` négatif/NaN → `0`, `1e30` → `u64::MAX`, `required` non-booléen → `true`. Les doublons de clés, eux, sont refusés (réfutation partielle). La direction est majoritairement « fail-safe », sauf `args` et `timeout_s=0`. |
| AUD-04 | **PARTIELLEMENT CONFIRMÉ**, P2 (→ FBL-007) | `state.rs:641-645` (slot par arbre), `commands.rs:1717` (load par arbre, pas de contrôle `candidate_id`) | Sûreté préservée (le même arbre exact a été vérifié), mais le reçu peut nommer un autre `candidate_id`/`parent_oid` que celui publié → attribution/audit faux. Pas de publication d'un arbre non qualifié. |
| AUD-05 | **CONFIRMED_BY_CODE**, P1 (fusionné dans FBL-001) | `proposal.rs:95-118` (load sans re-hash), `:884-893` (assemble lit le patch brut) | Ni `content_digest` ni `tree_oid` ne sont revérifiés à la lecture. Un patch altéré/torn est appliqué tel quel ; le candidat reste cohérent *avec lui-même* (arbre recalculé), mais l'attribution à l'agent est fausse. |
| AUD-06 | **CONFIRMED_BY_CODE**, P1 (→ FBL-002) | `commands.rs:1420-1441` | `logs/<tree>/<ob>.stdout.log` : même chemin pour deux runs du même arbre, sans identifiant d'exécution. |

## 5. Nouveaux constats indépendants

Format : sévérité · statut de preuve · emplacement · cause · scénario · conséquence · correctif · critères d'acceptation.

### FBL-001 — Admission non transactionnelle : patch et métadonnées peuvent provenir de deux écrivains · **P0** · CONFIRMED_BY_TEST
- **Reproduction (G1)** : 15 tours × 8 processus `flow propose` avec la même `--key` et des patches différents. Résultat : `multi_admit_rounds=15` (à **chaque** tour, plusieurs écrivains reçoivent `ADMITTED` sans `idempotent`, jamais `IDEMPOTENCY_CONFLICT`) et `integrity_breaks=[12]` (au tour 12, le `.patch` sur disque ne correspond pas au `content_digest` du `.json`). Le mélange patch/métadonnées est donc observé, pas seulement déduit.
- **Où** : `proposal.rs::admit` L638-654, L732-738 ; `proposal.rs::assemble` L884-893 ; `Proposal::load` L95-118.
- **Cause** : séquence check-then-act sans verrou ; deux fichiers écrits par deux `rename` indépendants ; aucune revérification du hash à la relecture.
- **Scénario** : agents A et B, même `agent/task/base/--key` (cas réel : retry d'un orchestrateur après timeout, ou deux workers qui reprennent la même tâche), patches différents. A et B voient `load()==None`. Entrelacement : A rename `.patch` → B rename `.patch` → B rename `.json` → A rename `.json`. Résultat : `.json` de A (digest A, `tree_oid` A, `changes` A) + `.patch` de B. Les deux reçoivent `ADMITTED` ; aucun `IDEMPOTENCY_CONFLICT`.
- **Conséquences** : `path_conflicts` raisonne sur les `changes` de A alors que `assemble` applique les octets de B → la détection de conflits de chemins (renommage/suppression/symlink) est contournée ; `validate_path` (symlink escape, `.git/`, `.flow/`) a été exécuté sur le patch de B *dans le process B* seulement — OK ici, mais un patch altéré sur disque après admission n'est jamais revalidé. Le candidat est attribué à A pour des octets de B.
- **Correctif** : (1) sérialiser `admit` par un verrou par identifiant (`create_new` sur `<id>.lock` ou réutiliser `lock.rs` avec un purpose) ; (2) écrire un unique fichier contenant métadonnées + patch, ou écrire le `.patch` sous un nom dérivé du `content_digest` (`<digest>.patch`, contenu-adressé) que le `.json` référence ; (3) publier le `.json` par `hard_link`/`create_new` (échec si existant) au lieu de `rename` écrasant ; (4) dans `Proposal::load`/`assemble`, recalculer `sha256(patch) == content_digest` et refuser sinon (`PROPOSAL_INTEGRITY`) ; (5) dans `assemble`, revérifier `validate_path` sur les `changes` recalculées de chaque patch appliqué.
- **Acceptation** : test `concurrency.rs` : N=8 `flow propose` simultanés même clé, patches distincts → exactement 1 `ADMITTED` (+ éventuellement des `idempotent:true` pour le même contenu), N-1 `IDEMPOTENCY_CONFLICT`, et `sha256(.patch) == content_digest` sur disque. Test : modifier un `.patch` après admission → `assemble` refuse `PROPOSAL_INTEGRITY`.

### FBL-002 — `run` non sérialisé et logs non isolés par exécution · **P1** · CONFIRMED_BY_CODE
- **Où** : `commands.rs::cmd_run` L1420-1441, L1601 ; `state.rs::save_receipt` L641-661.
- **Cause** : chemin de log = `logs/<tree>/<ob_id>.{stdout,stderr}.log`, slot de reçu = `receipts/<tree>.json` ; aucune identité d'exécution, aucun verrou.
- **Scénario** : deux agents lancent `flow run` sur le même index/candidat (même arbre) ; run 1 FAILED, run 2 PASSED, terminaisons entrelacées. Le reçu final peut être PASSED alors que les logs sur disque sont ceux du run FAILED (ou l'inverse). Variante : `--timeout` override sur run 2 → reçu PASSED à politique différente écrase le reçu publiable de run 1 → `STALE_POLICY` au publish (déni de service, pas faux vert).
- **Conséquence** : la preuve référencée par le reçu (logs) peut ne pas être la preuve de ce reçu. Pas de publication d'arbre non qualifié (le statut vient des outcomes du run qui écrit le reçu), mais l'audit d'évidence est faux.
- **Correctif** : `run_id` unique (`unique_suffix` + pid) ; logs dans `logs/<tree>/<run_id>/` ; reçu inclut `run_id` et le **sha256 de chaque log** ; écrire le reçu complet seulement si aucun reçu PASSED plus récent pour la même politique n'existe (ou slot `receipts/<tree>.<policy12>.json`).
- **Acceptation** : test 2 runs concurrents même arbre → deux dossiers de logs distincts ; chaque log référencé par le reçu a le digest enregistré ; `explain` détecte un log altéré.

### FBL-003 — Vol de verrou mort : TOCTOU lire-vérifier-supprimer · **P1** · CONFIRMED_BY_CODE, NOT_REPRODUCED
- **Tentative (G3)** : test unitaire, 60 tours × 8 threads qui volent un verrou `flow-lock/2` d'un pid mort prouvé ; on mesure le nombre de détenteurs simultanés. Résultat : `acquired_total=109`, `overlap_rounds=0`. **Non reproduit.** Limite du harnais : les threads partagent le même pid, donc dès qu'un thread détient le verrou, les autres sortent tout de suite en `Held` (`lock.rs:261`), ce qui réduit la fenêtre. Un test fidèle demande plusieurs **processus** avec un point d'arrêt injecté entre la relecture et `remove_file`. Le défaut reste établi par la lecture du code ; sa probabilité réelle n'est pas mesurée.
- **Où** : `lock.rs::acquire_lock` L289-306 ; idem `LockGuard::drop` L151-154.
- **Cause** : `read_to_string` → compare nonce → `remove_file` n'est pas atomique.
- **Scénario** : verrou orphelin de nonce N. P et Q lisent N, constatent le pid mort. P relit (=N), supprime, crée son verrou NP et entre en section critique. Q relit… si la relecture de Q a eu lieu avant la suppression par P, Q supprime **le verrou de P** puis crée NQ. P et Q sont tous deux dans `publish`/`reconcile`.
- **Conséquence** : `update-ref` CAS protège la ref (au plus une publication par `old`), mais `append_journal` fait read-modify-rename du fichier entier (`state.rs:418-428`) → un intent ou un outcome peut être **perdu** ; `reconcile` concurrent peut dupliquer/omettre des outcomes. Le journal est l'enregistrement de récupération.
- **Correctif** : ne jamais supprimer par nom ; renommer atomiquement le verrou mort vers `lock.stale.<nonce>` (`rename` échoue/écrase de façon détectable) puis vérifier le contenu du fichier renommé ; ou utiliser un verrou OS (`flock`/`LockFileEx`) qui disparaît avec le process. Même correction dans `drop`.
- **Acceptation** : test stress : verrou orphelin + 8 publishers concurrents → jamais deux détenteurs simultanés (instrumenter via un fichier sentinelle `create_new` dans la section critique) ; journal sans enregistrement manquant (chaque intent a ≤1 outcome, aucun outcome sans intent).

### FBL-004 — `BuildContextId` n'identifie pas l'environnement effectif · **P1** · CONFIRMED_BY_CODE
- **Où** : `commands.rs::build_context_id` L724-734 ; `effective_policy` L697-718 ; `gitcli.rs::run_obligation` L578-589.
- **Cause** : seuls la liste des *noms* allowlistés, `git --version`, OS et arch sont hachés.
- **Scénario** : `run` PASSED avec `rustc 1.82` ; `rustup default 1.75` (ou changement de `PATH`, de `CARGO_HOME`, d'un `Cargo.lock` hors arbre, d'un registre) ; `publish` accepte (`CONTEXT_MISMATCH` non déclenché). Le README/le message d'erreur L1809 prétend pourtant « the toolchain or platform changed ».
- **Conséquence** : un `PASSED` est réutilisé pour un environnement que la vérification n'a pas couvert. Le message d'erreur sur-promet.
- **Correctif** : inclure dans le contexte : valeurs des variables allowlistées (ou leur hash), chemin canonique + sha256 (ou `--version`) de chaque programme résolu, sortie d'une sonde de toolchain déclarée dans la politique (`context_probes: [["rustc","-Vv"], ["cargo","-V"]]`). Corriger le message d'erreur sinon.
- **Acceptation** : test : modifier `PATH` (ou un env allowlisté) entre `run` et `publish` → `CONTEXT_MISMATCH`. Test : remplacer le binaire d'obligation → `CONTEXT_MISMATCH`.

### FBL-005 — `gc` non sérialisé avec `run`/`propose`/`assemble` · **P2** · CONFIRMED_BY_CODE
- **Où** : `commands.rs::cmd_gc` L2413-2600 (aucun `acquire_lock`).
- **Scénario** : `gc --max-age-days 0` pendant un `run` long → supprime `logs/<tree>/` en cours d'écriture ou un candidat qu'un `run`/`publish` vient de charger. `pending_intents` est vérifié avant, sans verrou : un `publish` peut écrire son intent juste après.
- **Conséquence** : reçu pointant vers des logs absents ; `UNKNOWN_CANDIDATE` intermittent. Pas de faux vert.
- **Correctif** : `gc` prend le verrou `publish` et refuse si un run est actif (marqueur `runs/<run_id>.active` créé par `run`).
- **Acceptation** : test gc concurrent d'un run → logs du run intacts, ou gc refuse `RUN_ACTIVE`.

### FBL-006 — Décodage de politique permissif · **P2** · CONFIRMED_BY_CODE (= AUD-03)
- **Où** : `state.rs::Config::from_json` L123-175.
- **Cas** : `args: ["test", 5]` → `["test"]` (commande différente de celle écrite) ; `timeout_s: -1` ou `0.4` → `0` ; `timeout_s: "60"` → 300 ; clé mal orthographiée (`"requierd": false`) ignorée ; `execution_boundary` : chaîne libre.
- **Correctif** : décodeur strict : ensemble fermé de clés, types exacts, `timeout_s` entier dans `[1, 86400]`, `execution_boundary` ∈ énumération, refus `INVALID_POLICY` sinon. Migration : `policy_version` incrémentée.
- **Acceptation** : tests unitaires, un par cas ci-dessus → `Err`.

### FBL-007 — Reçu non lié au candidat publié · **P2** · CONFIRMED_BY_CODE (= AUD-04)
- **Où** : `commands.rs::cmd_publish` L1717-1868 ; reçu `candidate_id`/`parent_oid` L1559, L1570.
- **Scénario** : candidats C1 et C2 (bases ou listes différentes) produisant le même arbre T, ou index == T. `run --candidate C1` puis `publish --candidate C2` (ou sans candidat) → accepté ; sortie publish annonce C2, reçu dit C1.
- **Correctif** : si `--candidate` est donné, exiger `receipt.candidate_id == candidate` **ou** documenter explicitement que la preuve porte sur l'arbre seul et exposer `receipt_candidate_id` dans la sortie et le journal.
- **Acceptation** : test : publish C2 avec reçu de C1 → `RECEIPT_CANDIDATE_MISMATCH` (ou champ explicite présent).

### FBL-008 — Verrou sans identité d'hôte / d'espace de PID · **P2** · HYPOTHESIS
- **Où** : `lock.rs::create_lock_file` L187-191, `pid_alive` L322+.
- **Scénario** : `.flow/` partagé (volume réseau, bind-mount entre conteneurs, WSL ↔ Windows sur `D:`). Le pid d'un détenteur vivant sur l'autre hôte est « mort » localement → vol. Inverse : réutilisation de PID → verrou orphelin jamais volé (DoS, résolu à la main).
- **Correctif** : ajouter `hostname` + boot-id/heure de démarrage du process au record ; ne voler que si même hôte et même boot ; sinon `LOCK_UNKNOWN`.
- **Acceptation** : test unitaire : record d'un autre hôte → `Unknown`, jamais volé.

### FBL-009 — Candidat : écriture non atomique vis-à-vis des concurrents · **P3** · CONFIRMED_BY_CODE
- **Où** : `proposal.rs::assemble` L968-983 (`exists()` puis `write_atomic`).
- **Conséquence** : bénigne (contenu identique par construction de l'id, sauf `created_at`). À corriger avec le même primitive `create_new`/`hard_link` que FBL-001.

### FBL-010 — Pas de revalidation des enregistrements chargés · **P2** · CONFIRMED_BY_CODE
- **Où** : `Candidate::load` L214-295, `cmd_run` L1379-1407.
- **Cause** : `run --candidate` fait confiance à `tree_oid`/`base_commit` du JSON ; aucun recalcul (re-assemblage) ni vérification `base_commit^{tree} == base_tree`. Un candidat altéré sur disque fait vérifier et publier un arbre arbitraire sous l'étiquette « candidat X ». La publication reste « arbre vérifié », mais l'attribution aux propositions est fausse. Le modèle de menace du projet exclut un écrivain local hostile (`commands.rs:498`), d'où P2.
- **Correctif** : sceller candidats et propositions comme les reçus (`seal_receipt`) et vérifier à la lecture ; optionnellement re-dériver `candidate_id` depuis `(base, tree, proposals)` et comparer.
- **Acceptation** : modifier `tree_oid` d'un candidat → `run`/`publish` refusent `CANDIDATE_INTEGRITY`.

### FBL-011 — Fuite de chemins personnels dans les enregistrements · **P3** · CONFIRMED_BY_CODE
- **Où** : `proposal.rs:723-726` (`patch_path` absolu), `commands.rs:1606` (`receipt_path`), `:1958`, `:2140`.
- **Conséquence** : les JSON (souvent recopiés dans des logs d'agents/LLM) contiennent `C:\Users\<nom>\…` — contraire à la règle « Resource & privacy constraints » d'AGENTS.md.
- **Correctif** : enregistrer des chemins relatifs à la racine.

### FBL-012 — Journal O(n) par append · **P3** · CONFIRMED_BY_CODE (documenté)
- **Où** : `state.rs::append_journal` L418-428 ; `read_journal` parse tout.
- **Conséquence** : 4–5 appends par publish, chacun relit/réécrit/fsync tout le fichier ; sous charge multi-agents (milliers de publications) devient quadratique. Aucune rotation hors `gc`.
- **Correctif** : segments `journal/NNNNNN.jsonl` immuables + segment courant ; compaction des intents résolus.

### Hypothèses écartées ou non démontrées
- **Symlink vers `.git`/`.flow`** (cible syntaxiquement sûre, `proposal.rs:481-487`) : le scratch est jetable et sans `.git` → `NOT_VERIFIED`, risque faible.
- **Conflits dossier/fichier** (A ajoute `a`, B ajoute `a/b`) : non vus par `path_conflicts` (chemins exacts) mais `apply --cached` doit échouer → `CONFLICT` générique. `NOT_VERIFIED`.
- **Fichier temp partagé `with_extension("tmp-flow-<pid>")`** : `X.json` et `X.patch` partagent le même nom temporaire, mais écrits séquentiellement par un même process → pas de bug. `REFUTED`.

## 6. Dette technique et compromis

- **Couplage** : `commands.rs` = 3 588 LOC (seuil « refactor obligatoire » 1 500 du standard du dépôt) ; `cmd_publish` ≈ 500 LOC, `cmd_run` ≈ 320 LOC. Extraire `publish::preflight`, `publish::commit`, `run::execute`, `evidence::{logs,receipt}`.
- **Duplication** : décodage de `Change` copié deux fois (`proposal.rs:149-184` et `:251-286`) ; `refusal`/`refused_code` dans deux modules.
- **Modèle de concurrence implicite** : un seul verrou global (`publish`) ; pas de couche « store » transactionnelle. Toute future commande (ordonnanceur, SDK) devra réinventer la sérialisation. Recommandation : un module `store` unique avec primitives `create_once(path, bytes)` (hard-link), `put_content_addressed(bytes) -> digest`, `with_lock(scope, f)` ; toutes les écritures `.flow/` passent par lui.
- **Évolution vers ordonnanceur/SDK/vérif incrémentale** : faisable sans réécrire le noyau si (1) les enregistrements deviennent contenu-adressés et scellés, (2) chaque run a un `run_id`, (3) le contexte est une structure versionnée (`CONTEXT_CODEC`) extensible avec des sondes. La vérification incrémentale demandera un graphe obligation→chemins, absent aujourd'hui.

## 7. Matrice des fonctionnalités

| Fonction | État | Note |
|---|---|---|
| Admission de patch en index temporaire | Implémenté | Non transactionnel (FBL-001) |
| Idempotence par clé | Partiel | Correcte en séquentiel seulement |
| Validation chemins/symlink/gitlink/casse | Implémenté | — |
| Assemblage ordonné + conflits de chemins | Implémenté | Exact-path seulement |
| Vérification sur arbre exact matérialisé | Implémenté | Fidélité pré/post vérifiée |
| Identité de contexte de build | Partiel | FBL-004 |
| Reçu scellé (anti-corruption) | Implémenté | Pas une signature (assumé) |
| Publication CAS + journal | Implémenté | Sûr sauf FBL-003 |
| Récupération (`reconcile`) | Implémenté | Non relu en détail |
| Exécutions concurrentes multi-agents | **Absent** | Aucune sérialisation hors publish |
| Isolation forte (sandbox) | Absent | `local-trusted`, `HOME`/`CARGO_HOME` partagés |
| Rebase automatique d'une base périmée | Absent | `BASE_MISMATCH` (refus, pas de perte) |
| Ordonnanceur / SDK | Absent | — |

## 8. Plan de correction (ordre par dépendances)

1. **Primitive store** (`state.rs`) : `create_once` (hard-link, échec si existe) + `put_blob_by_digest`. Tests unitaires. *Débloque 2, 3, 6.*
2. **FBL-001** : admission via `create_once`, patch contenu-adressé, re-hash à `load`/`assemble`. Test concurrence N=8.
3. **FBL-003** : vol de verrou par `rename` vers nom unique, ou verrou OS. Test stress orphelin.
4. **FBL-002 + FBL-005** : `run_id`, logs par run, digests des logs dans le reçu, marqueur de run actif, `gc` sous verrou.
5. **FBL-004** : contexte étendu (valeurs env + identité des programmes + sondes déclarées) ; bump `CONTEXT_CODEC`.
6. **FBL-010 + FBL-007** : sceller propositions/candidats ; liaison reçu↔candidat.
7. **FBL-006** : décodeur de politique strict ; bump `policy_version` + migration.
8. **FBL-008, 011, 012, 009** et refactor de `commands.rs`.

Chaque étape : PR ≤ 400 LOC, `cargo clippy --all-targets -- -D warnings && cargo test`.

## 9. Portes pour `PRODUCTION_READY`

- G1 : test concurrent `propose` (même clé) → un seul admis, intégrité disque vérifiée.
- G2 : test concurrent `run` (même arbre) → évidence non mélangée, digests des logs vérifiés par `explain`.
- G3 : test stress verrou orphelin → jamais deux détenteurs ; journal cohérent (intent ↔ ≤1 outcome).
- G4 : changement d'env/toolchain entre `run` et `publish` → `CONTEXT_MISMATCH`.
- G5 : tout enregistrement `.flow/` altéré → refus nommé (propositions, candidats, reçus, logs).
- G6 : décodeur strict de politique avec matrice de tests négatifs.
- G7 : CI Linux + Windows exécutant G1–G6 (non vérifié dans cet audit : CI non relue).
- G8 : documentation des garanties multi-hôtes (ou refus explicite d'un `.flow` partagé).

## 10. Limites de preuve et questions humaines

- G1 a reproduit FBL-001. G3 n'a pas reproduit FBL-003 (harnais à threads, voir FBL-003). FBL-002 n'a pas été testé. Prochain test utile : G3 multi-processus avec un point d'arrêt de type `FLOW_CRASH_POINT` entre la relecture et la suppression.
- Non relus : `cmd_reconcile`, `paths.rs`, `main.rs`, CI, `MEASUREMENTS.md`, `CAPABILITIES.md`, `DECISION_REGISTER.md` (le verdict existant `PRODUCTION_PARTIAL` a été seulement repéré, `FLOW-PRODUCTION-VERDICT-2026-11-13.md:101`).
- **Décisions humaines** :
  1. Le modèle de menace reste-t-il « écrivain local de confiance » ? Si non, FBL-010 passe P1 et il faut une clé (signature) — hors portée actuelle.
  2. `.flow/` partagé entre hôtes/conteneurs est-il un cas supporté ? (FBL-008)
  3. Un reçu prouve-t-il « l'arbre » ou « le candidat » ? (FBL-007)
  4. Quel ensemble de sondes de toolchain est obligatoire dans le contexte ? (FBL-004)
- Rappel AGENTS.md : NO-GO implémentation tant qu'un `GO(scope, …)` humain n'est pas enregistré.

## 11. HANDOFF_DEEPSEEK

```
REPO      Rwanbt/Ravum-flow  SHA 1952325c973c3b002d6436a83656a5593003ad48 (main)
STATUS    audit only, nothing changed; needs human GO before code (AGENTS.md)
VERDICT   NOT_READY (multi-agent) / PRODUCTION_PARTIAL (single operator)
ORDER     store-primitive -> FBL-001 -> FBL-003 -> FBL-002+005 -> FBL-004 -> FBL-010+007 -> FBL-006 -> rest
FIRST     write failing tests G1 (tests/concurrency.rs: 8x `flow propose` same --key,
          different patches; assert 1 ADMITTED and sha256(.patch)==content_digest)
          and G3 (orphan lock + 8 publishers; sentinel create_new in critical section)
KEY LOCS  proposal.rs:638-654,732-738 (admit race) | proposal.rs:884-893 (no re-hash)
          commands.rs:1420-1441 (log paths) | commands.rs:724-734 (context)
          lock.rs:289-306,151-154 (steal TOCTOU) | state.rs:418-428 (journal RMW)
          state.rs:123-175 (lenient policy decode) | commands.rs:2413 (gc, no lock)
RULES     PR <=400 LOC; clippy -D warnings + cargo test; never weaken tests;
          no force-push; zero-dependency crate (CI --locked --offline)
OPEN Q    threat model, shared .flow across hosts, receipt=tree or candidate, toolchain probes
TESTS     G1 REPRODUCED FBL-001 (15/15 rounds multi-admit, 1 digest mismatch);
          G3 thread harness NOT_REPRODUCED (0/60) - needs multi-process version
          code of both tests: section 12 (paste into tests/ and lock.rs tests)
NOT DONE  FBL-002 untested; reconcile/paths/main/CI not reviewed
```

## 12. Annexe — tests jetables exécutés (clone du SHA audité)

Exécution : `cargo test --release --test fable_g1 -- --nocapture` puis `cargo test --release --bin flow -- --nocapture fable_g3`.

**G1** — `crates/flow/tests/fable_g1.rs` (nouveau fichier, inclut `src/json.rs` et `src/sha256.rs` via `#[path]`). Procédure :
1. créer un dépôt temporaire contenant `a.txt` = `alpha`, puis `flow init` ;
2. écrire 8 patches `-alpha` → `+beta{w}` ;
3. pour chacun des 15 tours, avec la clé `k{round}`, lancer 8 `flow propose --agent a --task t --base HEAD --key k{round} --patch p{w}.patch` en parallèle ;
4. compter les `ADMITTED` sans `idempotent` (attendu : ≤ 1) ;
5. vérifier `sha256(<id>.patch) == content_digest` dans `<id>.json`.

Sortie : `FABLE_G1 rounds=15 writers=8 multi_admit_rounds=15 integrity_breaks=[12]`, donc **FAILED**, ce qui est le résultat attendu tant que le bug existe.

**G3** — test ajouté au module `tests` de `src/lock.rs` :
1. lancer puis récupérer un enfant (`cmd /C exit 0`), dont le pid est mort (prouvé par `pid_alive == Some(false)`) ;
2. pour chacun des 60 tours, écrire `.flow/lock` = `flow-lock/2 <pid mort> 0 dead-nonce publish` ;
3. 8 threads synchronisés par une `Barrier` appellent `acquire_lock`, gardent le verrou 30 ms et comptent le maximum de détenteurs simultanés.

Sortie : `FABLE_G3 rounds=60 stealers=8 acquired_total=109 overlap_rounds=0`, donc **passed** (non reproduit).
```
