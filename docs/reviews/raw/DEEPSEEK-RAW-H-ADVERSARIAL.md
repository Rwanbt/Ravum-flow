# DEEPSEEK RAW H — RED-TEAM ADVERSARIAL : POURQUOI NE PAS CONSTRUIRE FLOW

**Agent :** H (red-team adversarial, indépendant) · **Date :** 2026-11-09, Europe/Paris · **Branche :** `docs/deepseek-d0-d4-review` @ `1d5fddf`
**Statut du document :** rapport brut, non consensuel, non ratifié. Il n'existe aucune autorisation d'implémentation et ce rapport n'en crée aucune.
**Règle de lecture :** chaque affirmation porte une étiquette de preuve. `MEASURED_HISTORICAL` peut être faux ; `ESTIMATE` est une estimation assumée, réfutable ; `VERIFIED_LOCAL` et `SOURCE_READ` ont été vérifiés par moi le 2026-11-09 ; `PLAUSIBLE_NOT_VERIFIED` est une hypothèse technique que je n'ai pas exécutée.

---

## 1. Mission et méthode

### 1.1 Mandat reçu

Construire le meilleur argument **contre** l'existence de Ravum Flow et **contre** le mandat lui-même : attaques structurelles, scénarios de perte non couverts, attaques sur les preuves, attaques sur le GOAL, kill criteria hiérarchisés, arbre buy-vs-build minimal, puis le meilleur cas **pour** une version minimale (charité obligatoire) et ce qui y reste non prouvé. Aucun code, aucun test, aucun build, aucun commit, aucune modification de `Rwanbt/Ravum`.

### 1.2 Sources lues (intégralement sauf mention)

| Source | Portée | Étiquette |
|---|---|---|
| `AGENTS.md` (31 l.) | politique d'accès, protocole multi-agents | `VERIFIED_LOCAL` |
| `docs/DEEPSEEK_GOAL.md` (85 l.), en entier | mandat, lots 0–5, livrables, stop conditions | `VERIFIED_LOCAL` |
| V6 `docs/reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md` | §0–§10 (l. 1–460) + synthèse critique et annexe Claude (l. 5534–5838) ; archive V4 (l. 461–5533) non lue ligne à ligne | `VERIFIED_LOCAL` |
| `PROJECT_CHARTER.md`, `ARCHITECTURE.md`, `DECISION_REGISTER.md`, `BENCHMARK_S9.md`, `ROADMAP.md`, `RAVUM_BOUNDARY.md`, `USER_STORIES.md`, `ISSUE_MAP.md`, `BACKLOG.md`, `SECURITY_TESTS.md`, `TRACEABILITY.md`, `BRANCH_WORKFLOW.md`, `README.md` | intégral | `VERIFIED_LOCAL` |
| `docs/adr/FLOW-ADR-000..011` (12) et `docs/rfc/RF-RFC-001..012` (12) | intégral | `VERIFIED_LOCAL` |
| Métadonnées GitHub Flow | repo, branches, PR #33, compteurs | `SOURCE_READ` (API, 2026-11-09) |
| Métadonnées GitHub Ravum | branche `dev`, PR #85/#87, workflows, LICENSE 404 | `SOURCE_READ` (API, 2026-11-09) |
| `evidence/mvp0c-s8-live-summary-2026-11-09.json` @ `Ravum@6b0ac7b` | artefact S8 intégral, analysé chiffre par chiffre | `SOURCE_READ` |

### 1.3 Ce que je n'ai pas fait — donc ce que je ne prétends pas

- Aucune exécution : ni hook, ni build, ni benchmark, ni test. Je ne prétends donc **rien** sur le comportement dynamique de Ravum ou de Flow.
- Aucun clone de Ravum, conformément à la consigne ; je n'ai lu de Ravum que des métadonnées GitHub et **un** artefact `evidence/` (JSON), jamais du code source.
- Je n'ai pas lu les fichiers `docs/reviews/raw/DEEPSEEK-RAW-*.md` des autres agents. Transparence : un `grep` sur `docs/` a fait apparaître une ligne de `docs/reviews/DEEPSEEK-00-BASELINE.md` ; je n'ai pas ouvert ce fichier.
- Je n'ai pas vérifié les 42 ADR Ravum ligne à ligne ; j'ai vérifié mécaniquement leur **présence** (42 en-têtes `ADR-000..ADR-041`, `VERIFIED_LOCAL`).
- Aucun conseil juridique : les points licence sont des risques signalés, pas des conclusions de droit.

### 1.4 Comment j'ai cherché à me tromper moi-même

Pour chaque attaque structurelle, j'indique **ce qui la réfuterait** (« Réfutation »). Une attaque que rien ne peut réfuter est une opinion ; je l'ai marquée comme telle ou écartée. J'ai aussi cherché le meilleur cas pour Flow (§8) avant de conclure, et je signale en §9 ce qui, dans mes propres attaques, reste faible.

---

## 2. Attaques structurelles (S1–S10)

### S1 — Le trilemme d'ownership avec Ravum : Flow n'a pas de cœur qui lui soit propre

**Thèse.** Le seul cœur que la V6 déclare non négociable en cas d'abandon du reste (l. 5814 : « ce qui reste dans tous les cas : un writer Git exact avec hooks corrects, des reçus liés à l'arbre, et un sandbox ») est **déjà implémenté dans Ravum**, par le même propriétaire, le même jour. La frontière `RAVUM_BOUNDARY.md` attribue à « Flow candidate » le « generic multi-agent patch admission, attribution, coordination » et la « publication Git » ; or `crates/ravum-workspace-coordinator` de Ravum **est** ce coordinateur, et la PR #85 (fusionnée 2026-11-09T11:26:56Z) revendique « publication Git transactionnelle robuste (15 tests de crash) », « protocole sémantique v2 », « CI par impact conservateur et fail-closed » ; la PR #87 (fusionnée 12:46:36Z, tête `dev` = `5408d9b3`) revendique « overlays transactionnels », « incrémentalité avancée », « groupement adaptatif » (`SOURCE_READ`, corps des PR ; non vérifié dans le code).

**Conséquence.** Flow est coincé dans un trilemme :
1. **dupliquer** le coordinateur Ravum → exactement la « double orchestration » que la V6 interdit (COR-13) ;
2. **dépendre** de crates Ravum non licenciés (H2 ouvert, aucun LICENSE) et instables (une autre session les réécrit) → cycle de propriété et risque juridique ;
3. **réécrire from scratch** → le coût complet, sans réutilisation, pour un composant qui existe déjà à côté.

**Pourquoi O0 gagne ici.** Si le besoin est réel, le chemin le moins coûteux est de **finir et durcir le coordinateur Ravum** (déjà en cours, déjà testé, déjà dans la CI) et de l'exposer comme CLI, pas de créer un second dépôt qui doit le réimplémenter ou en dépendre. Le seul argument inverse — « Flow doit être indépendant du langage Ravum » — est un argument de **produit**, pas de besoin : aucun second utilisateur n'existe (§E5).

**Réfutation.** Serait réfuté si (a) une mesure montrait que le coordinateur Ravum ne peut pas servir hors Ravum sans réécriture majeure, **et** (b) un besoin non couvert par O0 était documenté chez un utilisateur autre que le propriétaire. Aucune des deux n'existe dans le corpus.
**Statut.** `SOURCE_READ` pour les PR ; `PLAUSIBLE_NOT_VERIFIED` pour la conclusion ; l'inférence est falsifiable.

### S2 — L'arithmétique du remboursement : le projet ne peut pas se payer avant plusieurs années

**Thèse.** Le cœur minimal (R1–R4) est estimé par la V6 elle-même à un `GitWriter` « environ 1 000 lignes réécrivable proprement » (l. 5813) — mais le writer n'est qu'un des huit sous-systèmes de S3. Mon estimation, à réfuter : **500–700 h** de développement pour R0–R4 sur un seul développeur (`ESTIMATE`, fourchette large), plus **1–3 h/semaine** de maintenance perpétuelle (Git change, Windows change, les toolchains changent), plus la charge de relire la spec.

**Calcul de seuil (assumptions explicites, à remplacer par des mesures).** Gain espéré : 10 min par tâche × 10 tâches/semaine = **1,7 h/semaine**. Coût de maintenance : **2 h/semaine**. Le projet est **net négatif dès la maintenance**. Pour amortir 600 h sur 18 mois (78 semaines) il faut 7,7 h/semaine d'amortissement + 2 h de maintenance ≈ **9,7 h/semaine**, soit ~2 h/jour ouvré, soit **~58 tâches/semaine (~12/jour) à 10 min de gain**. Pour un développeur solo, ce débit de tâches agentiques est improbable ; le seuil est donc hors d'atteinte sauf si le gain unitaire est de l'ordre de 30–60 min.

**Réfutation.** Donner un log daté de 30 jours de tâches réelles, mesurant le temps perdu en intégration/vérification, et un devis horaire du noyau. Si le temps perdu médian est > 45 min/tâche à ≥ 15 tâches/semaine, mon attaque tombe. Aujourd'hui, **aucun de ces deux chiffres n'existe**.
**Statut.** `ESTIMATE` + arithmétique vérifiable.

### S3 — La complexité est irréductible et couplée, pas « documentable »

**Thèse.** Le noyau minimal n'est pas un module, c'est l'intersection de huit problèmes durs, chacun avec un mode d'échec silencieux : (1) tree Git exact ; (2) ordre et effets des hooks, y compris non idempotents ; (3) sandbox OS réelle (Windows **et** Linux) ; (4) fencing epoch/verrou/CAS non atomiques par nature ; (5) recovery après crash à chaque frontière ; (6) sémantique de fichiers (bytes, modes, symlinks, casse, LFS, submodules) ; (7) graphe d'impact polyglotte conservateur ; (8) cache à contexte exact sans faux vert. La V6 le reconnaît elle-même : 13 états + 11 états d'écart, 18 tests négatifs, 2 OS, N types de fichiers. Chaque paire (état, OS, type) est une cellule de test ; le nombre de chemins à prouver est **combinatoire**, pas linéaire.

**Conséquence.** Un noyau « mince » qui ne traite pas les huit problèmes n'a **pas de valeur** (il ne peut pas promettre ce qui fait sa raison d'être) ; un noyau qui les traite tous n'est pas mince. La V6 a résolu cette tension par la prose (fail-closed, `UNSUPPORTED`, `MANUAL_REQUIRED`) — c'est honnête, mais cela signifie que le produit passe son temps en mode dégradé (§E8, §3 scénarios 8, 12).

**Réfutation.** Montrer un prototype de ≤ 2 000 lignes qui passe TREE-01, HOOK-02a/b/c, ENV-04, FENCE-07, CRASH-11, PATH-08, FILE-09 sur Windows **et** Linux. Personne ne l'a fait ; le corpus ne contient aucun prototype.
**Statut.** `SOURCE_READ` (V6) + raisonnement.

### S4 — Un outil « VCS-like » pour un solo : le composant le plus critique, maintenu par une personne, sans licence

**Thèse.** Flow se place **entre** les agents et Git : c'est le point par lequel passent tous les commits. Un bug y perd du travail, un bug de sécurité y expose des secrets, un bug de hook y corrompt un index. C'est le profil de risque de `git`, `jj` ou GitButler — des projets à équipe financée. Flow serait maintenu par un seul humain (`VERIFIED_LOCAL` : les 12 commits de Flow sont tous signés « Erwan Barat » ; les PR Ravum #85/#87 et la PR Flow #33 sont toutes ouvertes/fusionnées par le compte `Rwanbt`, auteur du même e-mail dans les commits — l'identité unique est une inférence raisonnable, pas une pièce d'état civil), sans LICENSE (`VERIFIED_LOCAL` : `license: null` côté API Flow, README l. 39 le confirme), sans CI (`VERIFIED_LOCAL` : aucun workflow dans `.github/workflows`, seuls des templates d'issues), sans utilisateur tiers. La réversibilité (`INV-11`) protège le dépôt, **pas le temps investi** ni la confiance perdue après une perte de données.

**Réfutation.** Un second mainteneur nommé, une licence publiée, une CI réelle et un utilisateur externe avant R1. Aucun n'existe.
**Statut.** `VERIFIED_LOCAL` + `SOURCE_READ`.

### S5 — Spécification infinie : le corpus est déjà le projet

**Thèse.** La preuve n'est pas théorique, elle est datée : le corpus Flow a produit **V4 → V5 → V6 en une journée**, avec 21 corrections (`COR-01..COR-21`), puis la V6 écrit « on ne lance pas une sixième contre-expertise par défaut » (l. 457), et **le même jour** le GOAL lance une 7e passe à 8 sous-agents avec 10 livrables (`VERIFIED_LOCAL`). Le dépôt contient 46 fichiers Markdown, dont un de 445 500 octets (5 838 lignes) pour **0 ligne exécutable** (`VERIFIED_LOCAL`). Aucun budget, aucune date, aucun critère de sortie de la phase documentaire n'existe ; `ROADMAP.md` ajoute 8 autorisations R0–R7, chacune conditionnée par la précédente.

**Conséquence.** La structure du mandat **récompense la production de documents** (10 livrables, critères de complétude 42/42, 12/12, 26/26) et **ne récompense jamais l'arrêt**. Un ratchet de coût irréversible s'installe : chaque cycle rend l'abandon plus coûteux à justifier, sans jamais produire de fait nouveau. C'est le mécanisme classique de l'escalade d'engagement, ici instrumenté en Markdown.

**Réfutation.** Montrer un critère écrit, daté et chiffré de fin de la phase documentaire, et un fait empirique produit par cette phase. Il n'y en a aucun.
**Statut.** `VERIFIED_LOCAL`.

### S6 — Coût d'opportunité : ce que 600 h achètent ailleurs

**Thèse.** Les mêmes 600 h (`ESTIMATE`, cf. S2) peuvent acheter, au choix : (a) 42 ADR Ravum passés de `PROPOSED` à testés, ou les P0 Ravum **reproduits puis corrigés** dans la session qui possède le code ; (b) une CI d'impact + E2E sur le projet polyglotte réel (Unifia) — c'est-à-dire la réduction mesurable du « temps jusqu'au livrable vérifié » **sans** écrire de VCS ; (c) l'intégration de `jj`/GitButler (O1) avec une évaluation de licence ; (d) un script de 50–300 lignes `git worktree` + `gh` + hooks, qui capture l'essentiel de O2 sans nenhum des huit problèmes de S3. La charte elle-même impose de recommander l'arrêt si O0/O1 égalent Flow à coût total inférieur (`PROJECT_CHARTER.md` l. 32) : le corpus viole **l'ordre** de sa propre règle en construisant la spécification avant la mesure.

**Réfutation.** Montrer que (a)–(d) ont été chiffrés et que Flow gagne. Aucun chiffrage n'existe dans le corpus.
**Statut.** `SOURCE_READ` + raisonnement économique.

### S7 — La demande est prouvée par un seul comportement : celui du propriétaire

**Thèse.** Le seul signal de demande réel est le *churn* du propriétaire lui-même (PR #82 remplacée par #85 « 39 commits rejoués sur la chaîne propre de #80 », 81 744 lignes ajoutées, 329 fichiers — `SOURCE_READ`). C'est un signal authentique, mais il prouve un besoin **de re-chaînage et de vérification**, pas un besoin d'**orchestrateur universel**. Le public déclaré (« solo local developer first ») est un public d'un seul ; il n'y a ni issue d'un tiers, ni dépôt adoptant, ni utilisateur, ni licence permettant l'adoption (`VERIFIED_LOCAL`). Un outil dont l'audience est une personne et dont le périmètre est « universel, multi-langages, multi-agents » est un décalage de périmètre : le besoin réel justifie un script, pas 26 tickets, 12 RFC et 7 phases.

**Réfutation.** Un deuxième utilisateur réel (même un collègue), ou une issue externe, ou un fork actif. Aucun.
**Statut.** `SOURCE_READ` + `VERIFIED_LOCAL`.

### S8 — La prémisse mesurée n'est pas la prémisse du produit

**Thèse.** La motivation affirme le coût des « worktrees/copies » (`PROJECT_CHARTER.md` l. 5). La seule mesure disponible compare A = **copies privées**, B = dossier partagé + garde, C = séquentiel — **aucun worktree Git** (`BENCHMARK_S9.md` l. 2 ; V6 l. 286). Donc le coût qui motive le produit n'a **jamais** été mesuré, et la « solution sans worktree persistant » répond à un problème non établi. Pire : `BENCHMARK_S9.md` l. 7–8 définit B comme « réal Git worktrees » — l'expérience décisive est prévue mais non faite.

**Réfutation.** Mesurer A' = vrais worktrees contre B' = copies contre Flow. Tant que ce n'est pas fait, l'axe central du produit est une hypothèse.
**Statut.** `SOURCE_READ`.

### S9 — Sandbox Windows : le mode phare est probablement indisponible sur l'OS de référence

**Thèse.** O3 (le mode qui donne sa valeur au produit : exécuter hooks/builds sur le contenu candidat) exige une sandbox OS réelle. La V6 écrit que Windows est « à évaluer, pas à annoncer comme solution prête » (l. 262) et que sans sandbox démontrée la réponse honnête est `UNSUPPORTED/MANUAL_REQUIRED` (l. 446). La machine de référence est Windows/Linux. Donc, si Windows échoue, O3 est indisponible sur l'OS principal → le produit retombe à O2 (pas de hooks/builds isolés) ou ment. `PLAUSIBLE_NOT_VERIFIED` : je n'ai pas testé AppContainer/Job Objects sur MSVC/Node ; mais le corpus non plus, et il en fait pourtant la condition d'existence de O3.

**Réfutation.** Un test de 5 jours démontrant AppContainer + Job Objects + FS virtuel fonctionnels pour `cargo`, `node`, `python` sur la machine de référence. Aucun test n'existe.
**Statut.** `PLAUSIBLE_NOT_VERIFIED`.

### S10 — Les invariants sont déclarés avant le mécanisme qui les tiendrait

**Thèse.** `INV-02` (« aucune écriture perdue silencieusement »), `INV-03` (recovery idempotente), `INV-09` (budgets CPU/RAM bornés) sont des invariants **produit** ; or le mécanisme qui les tiendrait (scheduler, quotas, fencing, recovery) est planifié en R2–R5 et le corpus reconnaît que les outils non intégrés contournent tout (`ARCHITECTURE.md` l. 13, V6 l. 158, 236). Un invariant vrai seulement pour les clients « managés » n'est pas un invariant du produit : c'est une garantie conditionnelle. La V6 le sait et le dit ; la charte, elle, ne le dit pas. Le risque est de vendre (à soi-même) un invariant absolu qui deviendra un jour une « limitation documentée » — exactement le glissement que la V6 combat par ailleurs.

**Réfutation.** Réécrire les invariants en garanties conditionnelles explicites, ou démontrer le mécanisme. Ni l'un ni l'autre n'est fait.
**Statut.** `SOURCE_READ`.

---

## 3. Scénarios de perte et de dommage non couverts ou mal couverts

Colonnes : `Prob.` qualitative · `Impact` · `Détect.` (détectabilité par l'utilisateur) · `Couv. V6` = couverture réelle dans la V6 (pas la couverture déclarée). Tous ces scénarios sont **spécifiés** quelque part dans la V6 ; le problème est qu'ils sont spécifiés **sans test exécuté** et, pour plusieurs, sans comportement défini.

| # | Scénario | Prob. | Impact | Détect. | Couv. V6 | Verdict |
|---|---|---|---|---|---|---|
| 1 | Dirty humain sur un chemin réécrit par un hook approuvé (formateur, codegen) pendant la fenêtre de préparation | Moyenne | Élevé | Faible | `HOOK-02b` (refus/réconciliation) ; spec seule | **Partiel** : le comportement de réconciliation sur un fichier dirty *également présent dans le patch* n'est pas défini ; `INV-02` devient invérifiable |
| 2 | Kill **pendant** l'exécution d'un hook non idempotent (codegen à timestamp, append, incrément) → tree gelé à partir d'un effet partiel | Moyenne | Moyen-élevé | Faible | `CRASH-11` couvre les frontières journal/ref, **pas** l'intérieur d'un hook ; §2.1 dit « ne jamais rejouer » sans définir l'état « hook interrompu » | **Non couvert** |
| 3 | Hook qui s'échappe de la matérialisation : un `git worktree` **partage le `.git` réel** ; un hook peut donc réécrire `config`, `hooks/`, des objets, ou faire `git update-ref` sur la vraie ref, contournant CAS, epoch et verrou | Faible-moyenne (hook malveillant ou buggé) | **Très élevé** (corruption, publication hors Flow) | Moyenne (reflog) | Rien d'explicite : la V6 protège `GIT_INDEX_FILE` (l. 114) mais pas le `gitdir` partagé ; `TRUST-15`/`SANDBOX-05` testent secrets/réseau, pas le gitdir | **Non couvert — trou le plus grave** |
| 4 | Antivirus/EDR Windows verrouille ou met en quarantaine un blob juste écrit (`objects/`, matérialisation) → `update-ref`/CAS intermittents ou objet manquant | Élevée (Windows) | Moyen | Moyenne | `OS-18` nomme « antivirus » mais ne définit ni retry, ni re-vérification d'intégrité du tree après gel, ni comportement en cas d'objet altéré | **Nommé, non spécifié** |
| 5 | FS réseau/cloud (OneDrive, SMB) : placeholders, verrous non honorés, réordonnancement de synchro, WAL non supporté | Moyenne | Élevé | Faible | §4.5 dit « WAL local à la machine » ; §4.3 dit « documenter si FS non supporté » ; **pas de fail-closed sur FS non supporté** (contrairement aux types de fichiers) | **Partiel** : la documentation n'est pas une barrière |
| 6 | Deux instances Flow sur deux machines partageant un stockage synchronisé → deux epochs, deux « autorités d'écriture » | Faible | Élevé | Faible | §4.3 suppose les epochs « attribuées par la même autorité » ; l'hypothèse casse précisément sur stockage synchronisé | **Partiel (hypothèse non testée)** |
| 7 | Merge queue / squash / rebase : le tree final diffère du tree qualifié localement → reçu inutilisable | Élevée | Moyen (perte de valeur, pas de données) | Élevée | `FORGE-15` : requalifier ou bloquer | **Couvert — mais cela annule le gain au dernier kilomètre** |
| 8 | Submodule/LFS : identité du contenu externe ; LFS indisponible offline → `UNKNOWN`/`UNSUPPORTED` en masse | Élevée sur un front polyglotte | Moyen | Élevée | `CONTEXT-03`, `FILE-09` ; politique présente, **conséquence produit non tirée** | **Partiel** : honnête mais rédhibitoire pour l'usage visé |
| 9 | Entrée de build non capturée (fichier ignoré, généré, variable d'env, locale, réseau, temps) → `PASS` périmé mais accepté | Moyenne-élevée | Élevé | **Très faible** | `BuildContextId` **prétend** capturer les entrées ; `CACHE-13` ne teste que flags/lockfile/toolchain ; aucun test d'**entrée inconnue** | **Non couvert** : on ne peut pas prouver la complétude d'un contexte |
| 10 | Hook « vert » qui n'a rien fait (linter absent, `exit 0` avec warning) → faux vert silencieux | Moyenne | Moyen | Faible | Aucun test d'**efficacité** d'un hook ; Git ne distingue pas « passed » de « did nothing » | **Non couvert** |
| 11 | IDE : sauvegarde d'un buffer ouvert après publication (write-back de contenu périmé) → écrase le contenu publié | Moyenne | Moyen | Moyenne | `UNMANAGED-17` : best-effort, « aucune promesse anti-écrasement » ; la charte maintient pourtant `INV-02` | **Couvert comme limitation, contredit la charte** |
| 12 | Symlink/junction : `core.symlinks=false` sur Windows matérialise un mode `120000` en **fichier texte** ; la re-publication change la sémantique | Élevée (Windows) | Moyen | Faible | `FILE-09` exige « préservé ou `UNSUPPORTED` » ; la conversion est faite **par git**, invisible pour Flow | **Piège non couvert** |
| 13 | Builds dépendants des mtimes (Make, incrémental) : une matérialisation neuve donne des mtimes « maintenant » → rebuilds inutiles ou, pire, cache hit fondé sur mtime | Élevée | Moyen (perf ou correction) | Moyenne | Le cache V6 est content-addressed, mais les outils natifs gardent leurs sémantiques ; non traité | **Non couvert** |
| 14 | Disque plein / arrêt brutal au milieu de la matérialisation ou du journal → état partiel, recovery manuelle | Faible-moyenne | Moyen | Élevée | `RECOVERY-16` : diagnostic honnête, reprise non destructive | **Partiel** : honnête, mais manuel |
| 15 | Processus détaché lancé par un hook qui écrit **après** la vérification (service Windows, tâche planifiée, daemon enfant) | Faible | Élevé | Faible | `RESOURCE-14` compte les descendants pendant l'exécution ; rien sur la persistance post-run | **Non couvert** |

**Lecture adverse de ce tableau.** Les lignes 3, 9, 13 et 10 sont les plus dangereuses parce qu'elles produisent un **faux sentiment de sûreté** : le système se déclare vert alors qu'une garantie centrale est absente. Les lignes 8, 12 et 5 produiront au contraire un **taux de refus élevé** sur des dépôts réels (front-end avec LFS, Windows avec symlinks, dépôts synchronisés) : le produit sera soit dangereux, soit inutilisable — rarement les deux à la fois, ce qui rend le diagnostic difficile.

---

## 4. Attaques sur les preuves

### E1 — S8 : l'unique artefact quantitatif ne soutient pas la conclusion qu'on en tire

J'ai récupéré l'artefact cité par la V6 : `evidence/mvp0c-s8-live-summary-2026-11-09.json` @ `Ravum@6b0ac7b` (`SOURCE_READ`, `generated_at: 2026-11-09T02:25:31.248Z`). Contenu utile :

| Bras | n | échantillons (ms) | médiane | min–max |
|---|---|---|---|---|
| A copies privées | 3 | 100 664 / 111 110 / 208 694 | 111 110 | 100,7–208,7 s |
| B partagé + garde | 3 | 126 224 / 237 674 / 286 275 | 237 674 | 126,2–286,3 s |
| C séquentiel | 3 | 269 594 / 528 481 / 611 003 | 528 481 | 269,6–611,0 s |

Faits vérifiables : **n = 3 par bras** ; les intervalles **A et B se chevauchent** (126,2–208,7 s) et **B et C se chevauchent** (269,6–286,3 s) ; l'artefact ne contient **ni seed, ni identité de tâche, ni SHA de fixture, ni appariement** (les chemins de campagne sont expurgés : `<temp>\s8-mvp0c-live\A1`) ; les drapeaux `all_tests_green`, `zero_lost`, `all_protected_ok` sont des booléens auto-déclarés ; les tokens sont quasi identiques entre bras (≈ 0,86–2,36 M par agent-campagne, cache-read dominant), ce qui est cohérent avec un coût LLM constant.

**Conséquences.** (a) Avec n = 3, aucun test apparié ne peut atteindre une signification conventionnelle ; l'ordre A < B < C n'est **pas** démontré, seul A vs C est séparé. (b) L'artefact ne satisfait pas les exigences de contrôle de `BENCHMARK_S9.md` (l. 14 : « Freeze fixture SHA, OS/toolchain/adapter version, task definition, model/provider and parameters, warm/cold cache regime, order/seeds ») : il ne gèle **rien** de tout cela. (c) Les tâches sont des micro-tâches (7 à 17 messages assistant par agent, 30–160 s de wall time) : le résultat ne se transpose pas à une tâche de développement réelle de 30 min à plusieurs heures. (d) La V6 l'admet en partie (l. 286), mais `BACKLOG.md` l. 3 écrit « Le benchmark S8 **est passé** », formulation qui laisse croire à une étape de preuve franchie ; et la V6 l. 3289 écrit « A < B < C », c'est-à-dire un ordre que les données ne portent pas.

**Réfutation.** Rejouer S9 avec n ≥ 8 paires réelles, seeds et SHA figés, tâches ≥ 30 min, et publier les différences appariées. C'est précisément ce que le corpus n'a pas fait.
**Statut.** `SOURCE_READ` + arithmétique vérifiable.

### E2 — Aucune preuve du problème, aucune preuve du gain, dans aucun document

Le corpus contient **zéro** mesure de « temps jusqu'au livrable vérifié » sur une tâche réelle, **zéro** mesure du temps perdu en O0, **zéro** devis de développement, **zéro** utilisateur. Les seuls chiffres sont S8 (micro-fixture) et des tokens. Or le critère de succès du projet est exactement ce qui n'est pas mesuré (`PROJECT_CHARTER.md` l. 31). Un dossier de 445 Ko peut donc être **entièrement conforme** à ses propres critères de complétude et **entièrement vide** de la seule information qui déciderait.

### E3 — `CODE_READ` honnête, mais `CODE_READ` ne justifie pas du code — et la cible bouge

La V6 est exemplaire sur l'étiquetage : P0-TREE/HOOK/ENV sont `CODE_READ`, `PLAUSIBLE_NOT_REPRODUCED`, et la V6 refuse de les déclarer corrigés. Mais la décision de construire Flow s'appuie sur ces constats **non reproduits**, contre un composant qui est en cours de réécriture par une autre session : la PR #85 revendique une « publication Git transactionnelle robuste (15 tests de crash) » et la PR #87 des « overlays transactionnels » (`SOURCE_READ`, corps de PR, non vérifié dans le code). Si le défaut est déjà corrigé (ou corrigé autrement) dans Ravum, Flow construit un garde-fou pour un fantôme ; si le correctif est différent, Flow et Ravum divergent sur la même garantie. Dans les deux cas, la justification technique de Flow expire avant R1.

### E4 — « Revues IA indépendantes » : le compte ne tient pas

La V6 et l'annexe Claude établissent que, selon les corpus, **Qwen et Gemini sont identiques**, ou **Kimi et Gemini sont identiques** (l. 370, l. 5751). Le gate D1 exige « duplicate reports not counted twice » (`ROADMAP.md` l. 6). Le décompte réel de réviseurs **distincts** est donc d'environ **un** (Claude, sur la V5), et la V6 n'a **pas** été re-signée par un réviseur indépendant (l. 436). La présente session est une 7e passe du **même** modèle, sur le **même** corpus, le **même** jour, pilotée par le **même** propriétaire : ce n'est pas une indépendance, c'est un échantillonnage corrélé. Le GOAL l'anticipe (l. 42) mais ne fournit aucun mécanisme (pas de veto, pas de double aveugle, pas de budget adversarial symétrique) : 7 agents orientés construction contre 1 sceptique produisent un **consensus fabriqué par la composition de l'équipe**, pas par les faits.

### E5 — Absence de LICENSE : le produit n'a pas de chemin d'adoption, et le corpus non plus

Faits : `Rwanbt/Ravum-flow` → `license: null` (`SOURCE_READ`) ; aucun fichier `LICENSE` dans le dépôt (`VERIFIED_LOCAL`) ; `README.md` l. 39 : « no project LICENSE has been selected or published » ; `Rwanbt/Ravum` → `license: null`, `contents/LICENSE` = 404 (`SOURCE_READ`). Conséquences : (a) personne hors le titulaire ne peut légalement réutiliser, modifier ou distribuer Flow — donc l'« orchestrateur universel » ne peut pas être adopté ; (b) le corpus de revue lui-même est « tous droits réservés » par défaut, ce qui limite sa valeur d'audit externe ; (c) le statut des contenus produits par des modèles est juridiquement incertain dans plusieurs juridictions — un point que H2 ne couvre pas ; (d) toute réutilisation de crates Ravum reste bloquée (H2 « BLOCKER for copying »), ce qui **force** la réécriture from scratch et renchérit S2. Le corpus traite la licence comme une décision parmi six ; c'est en réalité une **condition d'existence** du produit.

### E6 — Les métriques peuvent être améliorées en trichant, et le protocole ne l'empêche pas

Le critère primaire est « temps de l'assignation à un livrable vérifié », avec un « policy » de checks **défini par l'arme elle-même**. Un système qui déclare moins de checks « requis » gagne du temps et se déclare vert. La V6 interdit de masquer un E2E connu, mais **ne fige pas la politique de vérification à celle de O0** : c'est la faille centrale du protocole. Autres leviers : le temps de supervision humaine est auto-chronométré ; les tokens dépendent des prompts ; le pic de RAM dépend de l'intervalle d'échantillonnage ; le « succès » est jugé par l'équipe qui construit. Aucun score à l'aveugle, aucun pré-enregistrement horodaté par un tiers, aucun corpus adversarial imposé **dans chaque campagne** (la V6 le prévoit « à part », l. 298).

**Correctif minimal, falsifiable.** (i) Figer la politique = configuration CI de O0, hash-commitée avant le premier run ; (ii) exiger le corpus adversarial dans **chaque** campagne, un échec annulant le run ; (iii) protocole + seuil commités avant mesure ; (iv) succès défini au **SHA final distant** vert, pas au tree local auto-qualifié.

### E7 — Les « scores O0/O1/O2/O3 » demandés par le GOAL sont une quantification sans données

`DEEPSEEK_GOAL.md` l. 56 demande des « scores O0/O1/O2/O3 ». Aucune des quatre options n'a été exécutée, mesurée ou chiffrée. Un score attribué dans ces conditions est un **artefact de décision** fabriqué à partir d'opinions, qui donnera au lecteur l'impression d'une comparaison. Il faut soit supprimer les scores, soit exiger qu'un chiffre Juste chacun cite un artefact (log, devis, benchmark).

### E8 — Claims de fait non étayés (liste courte et datée)

1. `DEEPSEEK_GOAL.md` l. 7 : « Le projet Ravum Flow **est** un orchestrateur universel, local-first, Git-compatible, **multi-langages** et multi-agents » — présent de l'indicatif pour un projet sans code ; « universel » est contredit par les non-goals de la charte (pas de merge sémantique universel, pas de VCS).
2. V6 l. 85 : « Une PR d'intégration #85 a été observée fusionnée le 9 novembre 2026 » — exact, mais l'ancre `dev = 5408d9b3` est en réalité le commit de merge de la **PR #87** (`2026-11-09T12:46:36Z`), soit 80 minutes plus tard (`SOURCE_READ`). Le document est déjà en retard sur sa propre ancre.
3. `BACKLOG.md` l. 3 : « Le benchmark S8 est passé » — voir E1.
4. V6 l. 5805 : « Windows : à qualifier » — c'est une **absence** de preuve présentée comme un plan ; le mode O3 en dépend.
5. Couverture 42/42 ADR : vraie au niveau des en-têtes (j'ai compté 42, `VERIFIED_LOCAL`), mais les 12 ADR Flow et 12 RFC du dépôt sont des **stubs** de 650–1 100 octets, essentiellement du boilerplate, qui renvoient au monolithe de 445 Ko : la complétude est celle des identifiants, pas de l'analyse.
6. L'ancre d'intégrité du corpus (V4, SHA-256 `700c604d…`, V6 l. 463) est **invérifiable** : le fichier original n'est pas dans le dépôt. Un hash sans artefact n'est pas une provenance.

### E9 — Le corpus confond exhaustivité et vérité

Les critères de validité de la revue (`DEEPSEEK_GOAL.md` l. 58–66) sont, pour l'essentiel, des **compteurs de couverture** et des exigences de format. Un modèle de langage excelle à produire de la couverture. Le seul critère qui pourrait être faux — une mesure — est interdit. La revue est donc structurellement **non falsifiable** : elle peut réussir parfaitement sans qu'aucune de ses hypothèses ne soit testée.

---

## 5. Attaques sur le mandat

### M1 — Le GOAL présuppose l'existence de Flow, puis demande si Flow doit exister

Le mandat maintient O0 comme option (l. 48, `PROJECT_CHARTER.md` l. 32), ce qui est à porter à son crédit. Mais la **forme** du mandat suppose Flow : 8 sous-agents nommés, 10 livrables, 26 tickets, 12 RFC, 12 ADR, 7 phases R0–R7, un mapping D→R. O0 n'a **aucun** workstream, aucun livrable, aucune date, aucun budget : c'est une colonne de tableau, pas une option de travail. Le test est simple et réfutable : chercher dans le corpus un plan d'instrumentation de O0 avec dates et propriétaire. Il n'existe pas.

### M2 — Le découpage D0–D4 est un mécanisme de report, pas une procédure de décision

Les cinq gates sont franchissables **par déclaration** : D0 admet `NOT_REVIEWED` (`ROADMAP.md` l. 5), D1 admet des rapports dupliqués (constat E4), D2 admet `PROPOSED`, D3 admet `DRAFT`, D4 attend un humain. Le registre affiche d'ailleurs D0 `PARTIAL`, D1 `DOCUMENT-CONDITIONAL`, D2 `PROPOSED`, D3 `DRAFT`, D4 `NOT GIVEN` — et le projet continue. Une gate dont le critère peut être satisfait par « je n'ai pas regardé » ne gate rien. Il faut compter **19 points de décision humaine** avant/pendant l'exécution (D0–D4, H1–H6, autorisations R0–R7), aucun avec échéance, aucun avec règle de décision par défaut. Le régime stationnaire du projet est donc la documentation.

### M3 — Les livrables demandés sont vérifiables en forme, pas en vérité

Sur les 10 livrables (`DEEPSEEK_GOAL.md` l. 68–79), **aucun** ne produit un fait empirique nouveau : ce sont des documents sur des documents, dont l'acceptation se mesure en compteurs (42/42, 12/12, 26/26). Le seul livrable qui pourrait être faux — le protocole S9 — est explicitement privé d'exécution (l. 52) et son seuil est une décision humaine non prise (H4). Le mandat demande par ailleurs « un benchmark S9 **équitable** » (l. 9) tout en interdisant de le lancer : la phrase décrit un résultat qui ne peut pas exister dans ce GOAL. C'est une promesse non tenable, pas une consigne.

### M4 — La lecture seule de Ravum est réaliste techniquement, pas épistémiquement

Techniquement, oui : je l'ai fait (API GitHub, `raw.githubusercontent`, aucune écriture). Mais elle rend les P0 **indéchargeables** : le mandat exige `CODE_READ ≠ bug reproduit`, interdit d'exécuter sur Ravum et n'a pas donné H5 (fixtures). Or la seule façon de reproduire un défaut du writer Ravum est d'exécuter du code Ravum sur une fixture — ce qui suppose de copier du code non licencié (H2 bloqué) ou d'obtenir une autorisation qui n'existe pas. Résultat : chaque P0 reste « plausible non reproduit » **pour toujours**, tout en justifiant la construction de garde-fous. La lecture seule organise l'ignorance tout en l'utilisant comme argument. S'ajoute la dérive d'ancre (E8.2) : tout constat est périmé avant d'être publié.

### M5 — Huit sous-agents d'un même modèle produisent un faux consensus

Le GOAL prévoit l'avertissement (l. 42) mais pas de mécanisme. Or : même modèle de base, même corpus, même jour, même propriétaire, prompts orientés (certaines questions présupposent la réponse : « Quels modules ne doivent **PAS** être extraits ? »), pas de double aveugle, pas de budget symétrique, pas de veto. La red-team est 1 voix sur 8 ; le Lead consolide et peut diluer l'objection. Le résultat est un **consensus corrélé présenté comme une délibération**. Le correctif minimal : le décideur humain lit le rapport H **en premier**, et le Lead doit lister explicitement les affirmations qui **invalideraient** sa recommandation, avec la mention de celles qui ont été testées.

### M6 — Le mandat élargit le périmètre au-delà des non-goals

« Orchestrateur **universel**, multi-langages » (l. 7) vs non-goals de la charte (pas de merge sémantique universel, pas de VCS, pas de compilateur universel). « Universel » n'est défini nulle part et ne peut pas être testé ; c'est un mot qui autorise tous les périmètres. Un mandat qui ne peut pas être violé par excès de périmètre n'est pas un mandat, c'est une direction.

---

## 6. Kill criteria hiérarchisés

Chaque critère est **observable** et **datable**. Les dates proposées sont des propositions de ma part ; le propriétaire peut les changer, mais il doit alors écrire la nouvelle date.

### Niveau 1 — Arrêt immédiat (le projet ne continue pas)

- **K1 — Douleur non démontrée. Date limite : 2026-11-16.** Le propriétaire produit une liste datée de **≥ 3 tâches des 30 derniers jours** où l'intégration/vérification a coûté **> 30 min** chacune, avec lien d'issue/PR. **Sinon : arrêt.** Observable : liste + horodatages. Falsifiable : la liste existe ou non.
- **K2 — O0 suffit déjà. Date limite : 2026-11-30.** Un journal de 10 jours ouvrés mesure, en O0 (worktrees + CI existante), le temps médian jusqu'au livrable vérifié sur les tâches réelles. Si l'écart avec la cible Flow n'est pas > 15 %, **arrêt**. Observable : journal horodaté, même définition du « vérifié » que S9.
- **K3 — Perte silencieuse.** Toute perte d'une écriture vérifiée ou dirty dans **n'importe quel** essai (y compris fixture) → **arrêt du design concerné et post-mortem obligatoire**, sans exception « limitation documentée ». Date : à chaque essai.
- **K4 — Preuve non figée.** Si le protocole S9, la politique de vérification et le seuil ne sont pas commités (hash) **avant** la première mesure, ou si le seuil bouge après observation → **la mesure est nulle et l'allégation est retirée**, quel que soit le résultat. Date : avant le premier run.
- **K5 — Bus factor et droits. Date limite : 2026-12-09.** Aucun second mainteneur nommé **et** aucune LICENSE publiée → le projet reste un script privé : **pas de produit, pas de R1–R7**. Observable : fichier LICENSE + second mainteneur au commit.

### Niveau 2 — Réduction à une classe de tâches (le projet continue, amputé)

- **K6 — Si seules les tâches à fichiers disjoints montrent un gain** → restreindre à O2 `MANAGED_PATCH` disjoint, **supprimer hooks et builds isolés** (80 % de la complexité de S3). Date : après S9.
- **K7 — Si la sandbox Windows n'est pas démontrée en 5 jours ouvrés** (AppContainer/Job Objects pour `cargo`/`node`/`python`) → O3 est **Linux-only**, ou Flow refuse l'exécution non fiable et l'annonce. Date limite : 2026-12-09.
- **K8 — Si plus d'un langage n'est pas couvert en 3 mois** → Flow est **Rust-only**, et le mot « polyglotte » disparaît du produit. Date limite : 2027-02-09.
- **K9 — Si le dernier kilomètre (merge queue/forge) exige systématiquement une requalification** → le gain de Flow est local, pas « jusqu'au livrable publié » : reformuler la promesse ou arrêter. Date : après FORGE-15.

### Niveau 3 — Nouvelle preuve requise avant de continuer à affirmer

- **K10 — « Tree exact »** : toute revendication de reçu réutilisable exige TREE-01 + HOOK-02a/b/c + CRASH-11 exécutés et verts, avec le test du **gitdir partagé** (scénario 3). Date : avant R4.
- **K11 — « Contexte complet »** : toute revendication de cache exact exige un test d'**entrée inconnue** (fichier ignoré ajouté après coup, variable d'env, locale) produisant `UNKNOWN` — pas un faux hit. Date : avant R3.
- **K12 — « Plus rapide »** : toute revendication de vitesse exige S9 pré-enregistré, n ≥ 8 paires réelles, politique figée à celle de O0, corpus adversarial dans la campagne. Date : avant toute communication externe.

**Le critère décisif, s'il n'en fallait qu'un : K2.** Parce qu'il est peu coûteux (10 jours, aucun code), qu'il teste la prémisse du produit, et qu'il rend toutes les autres discussions inutiles si O0 suffit. Les autres critères protègent contre des dégâts ; K2 protège contre l'inutilité.

---

## 7. Arbre buy-vs-build minimal (≤ 12 nœuds, applicable sans nouvelle étude)

Chaque nœud est une question à laquelle l'humain peut répondre avec ses propres données, sans audit supplémentaire. « Non » sur un nœud d'arrêt = stop.

1. **N1.** As-tu eu **≥ 3 tâches dans les 30 derniers jours** où intégration + vérification t'ont coûté > 30 min ? *Non →* **STOP** (O0, écrire un mémo d'une page, ne pas créer de projet).
2. **N2.** Peux-tu tenir un **journal de 10 jours** (début/fin de tâche, fin de vérification) en O0 ? *Non →* **STOP** : sans mesure, la décision est une croyance.
3. **N3.** Sur ces 10 jours, O0 (worktrees + ta CI actuelle) est-il à **≤ 15 %** de ta cible de temps ? *Oui →* **STOP** (O0 documenté, aucune construction).
4. **N4.** Un outil existant (jj, GitButler, ton propre coordinateur Ravum) couvre-t-il le besoin mesuré, licence et offline compatibles ? *Oui →* **O1** (adopter/intégrer, pas construire).
5. **N5.** Le temps perdu est-il concentré sur des **agents en fichiers disjoints** ? *Oui →* **O2 minimal** (patch managé + reçu exact + writer CAS + journal). Pas de hooks, pas de sandbox, pas de build isolé.
6. **N6.** Sinon, les hooks et builds doivent-ils tourner sur le contenu candidat ? *Non →* **O2** (même périmètre). *Oui →* N7.
7. **N7.** Une sandbox OS est-elle **démontrée en 5 jours** sur ton OS principal ? *Non →* **O2 seulement** ou **refus explicite** de l'exécution non fiable (jamais de faux `PASS`).
8. **N8.** As-tu **plus d'un langage** avec une CI réelle ? *Oui →* achète/intègre l'impact CI existant (Nx/Bazel/turborepo/cargo) ; ne construis **que** l'adaptateur Rust.
9. **N9.** Un prototype de 2 semaines produit-il un reçu lié à un **Git tree OID** vérifié de bout en bout (TREE-01, HOOK-02, FENCE-07, CRASH-11) ? *Non →* reste un wrapper CLI autour de `git` ; **ne promets aucune garantie**.
10. **N10.** Sur ≥ 8 tâches réelles appariées, le prototype gagne-t-il **≥ 20 %** de temps médian, politique CI identique, corpus adversarial vert ? *Non →* **archiver Flow**, publier le résultat négatif.
11. **N11.** Existe-t-il un **second mainteneur** et une **LICENSE** ? *Non →* garder comme script privé, pas de produit, pas de R5–R7.
12. **N12.** **Re-évaluer dans 6 mois** : si maintenance > gain mesuré, soleil couchant (archiver, laisser Git utilisable).

Chemin le plus court vers une décision : **N1 → N2 → N3** (une semaine, zéro code). Si N3 = oui, le projet s'arrête avant d'avoir coûté autre chose que du temps d'écriture.

---

## 8. Meilleur cas pour Flow (charité) et ce qui y reste non prouvé

### 8.1 Le meilleur cas, honnêtement

1. **La douleur est réelle et documentée par le comportement du propriétaire, pas par des opinions.** PR #82 remplacée par #85 avec « 39 commits rejoués sur une chaîne propre », 329 fichiers, 81 744 lignes, puis #87 avec « overlays transactionnels ». Un développeur qui re-chaîne 39 commits à la main a un problème d'**intégration vérifiée** ; c'est exactement la classe de problème que O2 adresse. Ce signal est plus convaincant que n'importe quel benchmark (`SOURCE_READ`).
2. **Le défaut P0 est réel dans le code lu** : tree calculé avant les hooks, ordre `prepare-commit-msg`/`pre-commit` inversé, hooks dans le workspace réel, environnement hérité. Même non reproduit, c'est un faisceau cohérent, et le coût du garde-fou (ordre Git + gel post-hooks + matérialisation) est faible comparé au coût d'un commit publié non testé.
3. **Le noyau O2 est petit et testable** : reçu lié à `GitTreeOid + BuildContextId + PolicyVersion`, writer CAS, journal idempotent, réconciliation non destructive — quelques milliers de lignes, sans VFS, sans daemon, sans CRDT, sans réseau. C'est faisable par un solo, et c'est **vérifiable** par des tests négatifs déjà spécifiés.
4. **Aucun concurrent ne couvre exactement ce créneau** : jj et GitButler ne produisent pas de reçu de vérification lié au tree ; Scion est un orchestrateur cloud ; Nx/Bazel ignorent la publication Git ; les CI ne savent pas ce que le développeur avait dans son workspace. Le créneau « local-first, offline, reçu exact, Git natif » est réel.
5. **La culture d'honnêteté est rare et précieuse** : statuts `UNKNOWN/STALE/UNSUPPORTED`, refus de promouvoir un E2E manquant, `PASSED ≠ PROVED`. Si ce noyau existe un jour, il empêche la classe de faux verts que les CI masquent tous les jours. C'est le meilleur argument **pour** Flow, et il ne dépend pas du benchmark.
6. **Le coût marginal d'un essai honnête est bas** : 10 jours de journal O0 + un prototype de 2 semaines suffisent à trancher (arbre §7). Ce n'est pas un pari à 600 h ; c'est un pari à 4 semaines, si le propriétaire accepte de mesurer d'abord.

### 8.2 Ce qui, dans ce meilleur cas, reste non prouvé

- (a) Que la douleur du propriétaire **dépasse le coût** de l'outil (aucune mesure, cf. S2).
- (b) Qu'un solo peut maintenir hooks + sémantique de fichiers + recovery sur deux OS (aucun devis validé).
- (c) Que le moteur de politique ne peut pas **s'auto-satisfaire** (le système qui définit « vérifié » est celui qui est vérifié).
- (d) Que O0 + un script de 50–300 lignes ne capture pas **80 %** du bénéfice (personne ne l'a essayé).
- (e) Que la demande Ravum se transfère aux projets polyglottes (aucune donnée sur Unifia, aucun dépôt examiné).
- (f) Que la sandbox Windows est atteignable (aucun test, cf. S9).
- (g) Que quiconque d'autre que le propriétaire utilisera l'outil (aucune licence, aucun utilisateur, aucune distribution).

**Conclusion charitable.** Il existe un produit minimal défendable — O2, sans hooks ni builds isolés, avec refus explicite du non-fiable — et il vaut un prototype de 2 semaines. Il n'existe **aucune** preuve, dans le corpus, que ce produit doive être « universel, multi-langages, multi-agents », ni qu'il doive être construit **avant** d'avoir mesuré O0. La version défendable est un script de quelques milliers de lignes, pas un programme à 7 phases.

---

## 9. Limites de ce rapport

1. **Aucune exécution.** Toutes mes affirmations sur le comportement dynamique (sandbox Windows, hooks, mtimes, LFS) sont `PLAUSIBLE_NOT_VERIFIED` ; elles sont des attaques à tester, pas des résultats.
2. **Pas de clone Ravum.** Je n'ai pas lu le code de `git_writer.rs` ; je m'appuie sur la V6, l'annexe Claude et les corps de PR. Les corps de PR sont des déclarations de l'auteur, pas des preuves (`SOURCE_READ`).
3. **Artefact S8 unique.** Je n'ai vérifié qu'un JSON d'agrégat ; les logs bruts n'existent pas dans le dépôt. Mon analyse (chevauchement, absence de seeds/SHA) est donc une analyse de l'artefact, pas des runs.
4. **Chiffrages.** Les 500–700 h et 1–3 h/semaine sont mes estimations ; elles doivent être remplacées par un devis. Mon attaque de S2 survit à toute fourchette plausible, mais elle est réfutable par des mesures — c'est voulu.
5. **Droit.** Je ne suis pas juriste ; les points licence sont des risques à faire qualifier (H2), pas des conclusions.
6. **Indépendance partielle.** Je suis un modèle de la même famille que les autres agents, lancé par le même propriétaire le même jour : mon « indépendance » est procédurale (prompt, sources, absence de lecture des autres rapports), pas épistémique. Le lecteur doit traiter ce rapport comme un avis adversarial supplémentaire, pas comme une preuve.
7. **Dérive temporelle.** Toutes les données Ravum sont datées du 2026-11-09 ; `dev` peut avancer à tout moment. La V6 elle-même est déjà en retard sur sa propre ancre (E8.2).
8. **Une vérification bloquée.** L'API GitHub m'a renvoyé un 403 (rate limit) sur une requête de contenu ; j'ai contourné par `raw.githubusercontent.com` pour l'artefact S8. Les autres vérifications (branches, licences, PR) ont abouti.
9. **Je n'ai pas cherché à être exhaustif sur les scénarios de perte** : j'ai retenu les 15 qui me paraissent les plus probables ou les plus silencieux. D'autres agents peuvent en voir d'autres ; l'absence d'un scénario ici n'est pas une preuve de son absence.
10. **Mon biais propre.** Un red-team a un biais de sévérité et un intérêt structurel à paraître rigoureux. Les kill criteria de §6 sont conçus pour me réfuter : si K1/K2 sont satisfaits, mon attaque principale tombe.

---

## 10. Références

### Dépôt Flow (local, `docs/deepseek-d0-d4-review` @ `1d5fddf`)

- `AGENTS.md` l. 5–8 (politique), 19–24 (protocole de preuve).
- `docs/DEEPSEEK_GOAL.md` l. 3, 7, 9, 29–42 (rôles), 44–56 (lots), 58–66 (critères), 68–79 (livrables), 81–85 (stop).
- V6 : l. 1–460 (§0–§10), l. 85 (ancre/#85), l. 89–94 (P0), l. 110–114 (hooks), l. 158, 236 (limites), l. 286–300 (S8/S9), l. 435–457 (verdict, décisions, « pas de 6e revue »), l. 461–463 (SHA-256 V4), l. 5534–5838 (synthèse + annexe Claude, dont l. 5751 sur les doublons, l. 5813–5814 sur le noyau).
- `PROJECT_CHARTER.md` l. 5, 26–32 ; `ARCHITECTURE.md` l. 13, 40–45 ; `DECISION_REGISTER.md` l. 4–17 ; `BENCHMARK_S9.md` l. 1–23 ; `ROADMAP.md` l. 4–21 ; `RAVUM_BOUNDARY.md` l. 3–18 ; `BACKLOG.md` l. 3–9 ; `SECURITY_TESTS.md` l. 4–21 ; `TRACEABILITY.md` l. 18–19 ; `BRANCH_WORKFLOW.md` l. 14–21 ; `README.md` l. 39.
- `docs/adr/FLOW-ADR-003.md` (hooks/tree), `-005.md` (fencing), `-008.md` (S9), `-010.md` (fichiers).

### GitHub (lecture seule, 2026-11-09)

- `https://api.github.com/repos/Rwanbt/Ravum-flow` → `created_at` 2026-11-09T13:32:28Z, `pushed_at` 14:37:18Z, `license: null`, `open_issues: 48` (= 47 issues + PR #33), `size: 0`.
- `https://api.github.com/repos/Rwanbt/Ravum-flow/branches` → `dev` et `main` : `protected: false`.
- `https://api.github.com/repos/Rwanbt/Ravum-flow/pulls/33` → ouvert, `draft: true`, `dev` (`1d5fddf`) → `main` (`5672087`), créé 14:32:48Z.
- `https://api.github.com/repos/Rwanbt/Ravum/branches/dev` → `5408d9b3…`, `protected: true`, check requis `mvp`, commit du 2026-11-09T12:46:36Z = merge PR #87.
- `https://api.github.com/repos/Rwanbt/Ravum/pulls/85` → fusionnée 11:26:56Z, merge `6b0ac7b`, 40 commits, +81 744/−1 545, 329 fichiers, corps : 495 tests / 78 suites, −69,1 %, 15 tests de crash.
- `https://api.github.com/repos/Rwanbt/Ravum/contents/.github/workflows?ref=dev` → `docs-check.yml`, `mvp-impact.yml` (18 172 B), `mvp.yml` (13 528 B).
- `https://api.github.com/repos/Rwanbt/Ravum/contents/LICENSE` → **404** ; `license: null`.
- `https://raw.githubusercontent.com/Rwanbt/Ravum/6b0ac7b…/evidence/mvp0c-s8-live-summary-2026-11-09.json` → artefact S8 analysé en E1.

### Contrôles locaux (`VERIFIED_LOCAL`)

- 46 fichiers `.md` + 4 `.yml` (templates d'issues) + `.gitignore` ; **aucun** code (`Cargo.toml`, `src`, `crates`, `tests` absents) ; **aucun** `LICENSE` ; **aucun** workflow CI (`.github/workflows` absent) ; `.github/` ne contient que des templates.
- 12 commits du 2026-11-09 15:13:45 → 15:37:17 (+0100), tous du même auteur, tous documentaires.
- 42 en-têtes `ADR-000..ADR-041` présents dans la V6 (comptage mécanique).

---

*Fin du rapport H. Ce document n'autorise rien, ne ratifie rien, ne prétend aucune mesure exécutée. Il est conçu pour être réfuté par K1 et K2.*
