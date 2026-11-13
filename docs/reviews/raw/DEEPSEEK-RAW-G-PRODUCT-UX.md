# DEEPSEEK-RAW-G — Produit & UX (rapport brut, non consolidé)

**Agent :** G — Product/UX · **Date :** 2026-11-09 (Europe/Paris) · **Statut :** brut, à consolider par le Lead
**Dépôt :** `Rwanbt/Ravum-flow` @ `docs/deepseek-d0-d4-review` = `1d5fddf` (issue de `dev`) · **Ravum :** `dev` `5408d9b3…`, lecture seule, non cloné
**Portée :** documents uniquement. Aucun code, build, test, maquette exécutable, ADR accepté, modification de Ravum.
**Thèse G (à contredire) :** la seule valeur aujourd'hui *démontrable par lecture* est un **publieur Git sûr pour changements d'agents** (ordre des hooks, arbre exact, hygiène d'environnement, refus explicites) ; la **coordination multi-agents** est une hypothèse non mesurée, et le seul chiffre disponible (S8) pénalise sa prémisse. Le MVP doit être le publieur, pas l'orchestrateur.

---

## 1. Mission et méthode

1. Cartographier `US-001..015 × RF-001..026 × R0..R7` (couverture, doublons, stories orphelines, RF sans story, stories qui présupposent une architecture non décidée).
2. Auditer la pertinence produit : preuve du problème, hypothèses de demande non prouvées, qui paie quoi, cas majoritaire « solo + 1 agent ».
3. Spécifier un parcours CLI minimal et une **ergonomie du refus** (pourquoi un agent abandonne au premier refus), avec un test d'acceptation observable sans explication humaine.
4. Trimer le MVP (`must/should/defer/kill`), nommer les éléments superflus et ceux **indispensables mais sous-spécifiés**.
5. Lister les promesses à ne jamais faire + reformulations honnêtes.
6. Coût d'adoption et de maintenance annuelle vs « ne rien installer », puis plus petite version vérifiable et critères de mise à mort.

**Sources lues intégralement ou par section :** `AGENTS.md`, `README.md`, `CONTRIBUTING.md`, `docs/DEEPSEEK_GOAL.md` (Lots 4–5), V6 §1/§3/§6.2/§7/§10 (+ §2, §2.1, §8.1, §9), `PROJECT_CHARTER.md`, `USER_STORIES.md`, `BACKLOG.md`, `ISSUE_MAP.md`, `ROADMAP.md`, `DECISION_REGISTER.md`, `TRACEABILITY.md`, `ARCHITECTURE.md`, `BENCHMARK_S9.md`, `SECURITY_TESTS.md`, `RAVUM_BOUNDARY.md`, `adr/README.md` + FLOW-ADR-000/001/002/003/005/006/007/008/010/011, `rfc/README.md` + RF-RFC-004/006/009/012, `.github/*`.

**Étiquettes de preuve :** `CODE_READ`, `SOURCE_READ`, `MEASURED_HISTORICAL`, `PLAUSIBLE_NOT_REPRODUCED`, `PROPOSED`, `UNKNOWN`. J'ajoute **`ESTIMATE`** = ordre de grandeur raisonné, non mesuré, révisable. Les jugements produit sont `PROPOSED` et signés comme tels : je ne les déguise pas en faits.

**Limite de méthode majeure :** aucun accès au marché (pas d'utilisateurs interrogés, pas de télémétrie, pas de dépôt Unifia analysé). Toutes mes affirmations de demande sont des **hypothèses falsifiables**, pas des mesures. C'est précisément le point §3.

---

## 2. Cartographie US / RF / phases

### 2.1 Vue par RF (26 lignes, une ligne = un ticket historique, formulation inchangée)

| RF | Phase (mapping BACKLOG) | US rattachée | Verdict G | Commentaire |
|---|---|---|---|---|
| RF-001 nom/visibilité/licence | D0/D4 | — | **must** (governance) | Bloque tout le reste via H2. À fusionner avec RF-002. |
| RF-002 inventaire Ravum transférable | D0/R0 | — | **kill** (portée produit) | N'existe que si l'on copie du code Ravum ; « from scratch » est disponible. Tâche D0, pas produit. |
| RF-003 benchmark S8/S9 | D3/R0 | US-011 | **must**, redéfini | Doit mesurer la **demande** avant la performance (§9). |
| RF-004 SourceSnapshot + proposition | R1 | — | **must**, élagué | Abandonner `SourceSnapshotId` au MVP ; identité = OID Git. |
| RF-005 CAS local + journal + GC | R1 | — | **should** | Pas de CAS maison : objets Git + refs épinglées + journal. GC/pinning = must. |
| RF-006 CLI snapshot/status/doctor | R1 | US-001 | **must** | Fusionner avec RF-016 (une seule surface CLI/JSON). |
| RF-007 proposition sur base immuable | R2 | US-002 | **must** | Cœur du produit. |
| RF-008 conflits rename/hunk/symbole, rebase, undo | R2 | — | **should** | MVP = détecter et refuser ; merge/rebase/undo = defer. |
| RF-009 recovery crash + chaos | R4 | US-003 | **must**, borné | Journal idempotent, pas de protocole de reprise universel. |
| RF-010 writer Git transactionnel + hooks | R3/R4 | US-003, US-008 | **must** (writer) / **defer** (hooks) | Deux capacités, deux risques : à scinder. |
| RF-011 protocole d'évaluation v2 | R1/R3 | US-002 | **must** (politique) | N'est pas une tâche : c'est la règle de statut. À fusionner dans RF-004/007. |
| RF-012 adaptateur Rust | R3 | US-006 | **must** | Seul langage du MVP. |
| RF-013 scheduler/annulation/budgets | R3/R5 | — | **should** | File séquentielle + annulation = must ; leases/coalescence = defer. |
| RF-014 cache + identity context | R3/R5 | — | **should** | Cache par entrées exactes ; graphe d'impact = defer. |
| RF-015 CI impact + E2E | R3/R5 | US-006 | **should** | « E2E jamais masqué » = must (politique) ; sélection par impact = defer. |
| RF-016 JSONL/SDK agents | R5 | US-004 | **defer** | `--json` sur chaque commande = must ; serveur JSONL = defer. |
| RF-017 UI revue + politique publication | R5 | — | **kill** (MVP) | `git diff` + éditeur suffisent ; l'UI est un client, pas un moteur. |
| RF-018 adaptateur TS/JS | R5 | US-010 | **should** | Après Rust, sur demande réelle. |
| RF-019 adaptateur Python | R5 | — | **defer** | Idem, encore plus tard. |
| RF-020 adaptateur C/C++ | R5 | US-010 | **kill** (MVP) | Variance toolchains/ABI maximale pour valeur minimale. |
| RF-021 forge GitHub/GitLab | R6 | — | **defer** | Hors chemin critique local. |
| RF-022 benchmark D vs alternatives | D3/R0 puis R5 | US-004, US-011 | **must**, redéfini | Deux phases = deux gates = ticket ingérable ; à scinder. |
| RF-023 VFS/IDE | R7 | — | **kill** | Superflu, et non « différé » : rien ne le rend nécessaire. |
| RF-024 sync multi-machines/RBAC/leases | R7 | — | **kill** | Autre produit, autre entreprise. |
| RF-025 CAS distant/remote executors | R7 | — | **kill** | Idem. |
| RF-026 adaptateur Ravum/PatchHost | R6 | US-012 | **defer** | Appartient au backlog de Ravum, pas à Flow. |

**Couverture :** 26 RF, dont **14 sans aucune US** (001, 002, 003, 004, 005, 008, 011, 013, 014, 017, 021, 023, 024, 025) = **54 %**. Inversement **5 US sans RF** (005, 009, 013, 015, 007) tracent vers des *gates*, des *INV* ou des *constats* (`R3/R4`, `D0/D1`, `P0-ENV`) et non vers un ticket. **9 RF sur 26 portent deux phases ou deux gates** (001, 002, 003, 010, 011, 013, 014, 015, 022) : un ticket, deux gates, aucun critère de sortie unique. Conclusion : le backlog est **piloté par l'ingénierie**, pas par l'usage ; la traçabilité US↔RF est décorative.

### 2.2 Vue par US (15 lignes) — verdict et problème

| US | Persona réel ? | RF/phase | Verdict G | Problème principal |
|---|---|---|---|---|
| US-001 | oui (solo) | RF-006 / R1 | **must** | À fusionner avec `doctor` : une seule surface de diagnostic. |
| US-002 | oui (agent) | RF-007/011 / R2 | **must** | C'est le produit. |
| US-003 | oui | RF-009/010 / R3-R4 | **must** | Deux capacités dans une story : « ne pas toucher mes fichiers » (trivial) + « reprendre après crash » (coûteux). |
| US-004 | partiel (équipe d'agents) | RF-016/022 / R5 | **should** | « mesurer le gain réel » n'est pas une capacité utilisateur : mélange produit et benchmark. |
| US-005 | oui (mainteneur) | R3/R4 (aucun RF) | **must** | « empêcher qu'un commit non testé soit publié » = promesse à borner (§6). |
| US-006 | non (rôle d'organisation) | RF-012/015 / R3-R5 | **should** | « tests impactés + dépendants » présuppose un graphe d'impact non spécifié. |
| US-007 | non (rôle d'organisation) | RF-RFC-010, P0-ENV / R3 | **should** | Le refus est must ; la sandbox est defer. |
| US-008 | oui | P0-HOOK-ORDER, RF-010 / R3 | **defer** | **Présuppose ADR-003 + INV-01-A (non ratifiés).** MVP : détecter les hooks et refuser/déléguer. |
| US-009 | oui | INV, R4 (aucun RF) | **must** (contrainte) | N'est pas une fonctionnalité : c'est un test de non-régression Git. |
| US-010 | non (équipe) | RF-018/020 / R5 | **must** (contrainte) | Idem : « ne pas imposer un compilateur » est un non-objectif, pas une story. |
| US-011 | non (le responsable) | RF-003/022 / D3 | **kill** (comme story) | N'a pas d'utilisateur : c'est une action de gouvernance. |
| US-012 | non (autre dépôt) | RF-026 / R6 | **defer** | Persona d'un autre produit ; H2 non tranché. |
| US-013 | non (processus de revue) | D0/D1 (aucun RF) | **kill** (comme story) | Décrit la revue documentaire, pas le logiciel : ne sera jamais « implémentée ». |
| US-014 | oui (le cas majoritaire !) | RF-RFC-004 / R2 | **must** | La page « ce que Flow ne protège pas » : le meilleur rapport valeur/coût du lot. |
| US-015 | non (administrateur) | D4, H1-H6 (aucun RF) | **should** (exigence) | Le noyau produit = plan + confirmation avant mutation, pas un protocole de GO. |

**Personas :** 8/15 sont des rôles d'organisation ou d'autres projets (005, 006, 007, 009, 011, 012, 013, 015), et 2/15 visent des « équipes ». Seules 5/15 (001, 002, 003, 008, 014) concernent l'audience déclarée « développeur solo d'abord ». **Le jeu de stories contredit la cible.** Un développeur solo n'a ni « Responsable CI », ni « Auditeur sécurité », ni « Administrateur » : il est les quatre à la fois, en 30 secondes, sans réunion.

**Stories qui présupposent une architecture non décidée (6/15) :** US-004 (INV-01-A, matérialisation), US-007 (H3, sandbox OS), US-008 (ADR-003 + INV-01-A, hooks en espace isolé), US-005 (identité de reçu D2), US-012 (H2, droits), US-014 (RFC-004, mode isolé vs observé). Ces stories ne sont pas « prêtes » : elles décrivent des conséquences d'options encore ouvertes.

### 2.3 Redondances à fusionner (IDs conservés, aucune suppression)

| Fusion | Contenu | Pourquoi |
|---|---|---|
| F1 | RF-004 + RF-005 + RF-011 → « identité, journal, reçus » | Trois tickets pour un seul contrat ; RF-011 est une règle de statut, pas une tâche. |
| F2 | RF-012 + RF-014 | Le cache dépend de l'identité de contexte : un seul objet de travail. |
| F3 | RF-013 + RF-015 | Même boucle d'orchestration (exécuter, annuler, décider quoi exécuter). |
| F4 | RF-018 + RF-019 + RF-020 | Une interface d'adaptateur, trois instances ; garder 3 IDs comme instanciations. |
| F5 | RF-024 + RF-025 | Famille « distribution » ; les deux sont hors produit. |
| F6 | RF-006 + RF-016 + RF-017 | Une surface CLI/JSON unique ; l'UI et le SDK sont des clients. |
| F7 | RF-001 + RF-002 | Même blocage H2 (droits/provenance). |
| F8 | US-004 + US-011 + US-013 + US-015 | Un seul artefact « gouvernance et mesure », hors du set de stories produit. |
| F9 | US-009 + US-010 + US-001 | Un seul « contrat de non-régression Git/outils natifs » (contraintes, pas fonctionnalités). |
| F10 | US-003 + US-008 | Chevauchement « ne pas toucher mon workspace » ; fusionner l'acceptation, garder les IDs. |

---

## 3. Pertinence produit et hypothèses non prouvées

### 3.1 Quelle preuve que le problème existe ?

| Affirmation du corpus | Source | Statut réel |
|---|---|---|
| « Plusieurs agents génèrent des éditions qui se chevauchent » (trad. de *overlapping edits*) | `PROJECT_CHARTER.md:5` | `PROPOSED` — assertion non sourcée, reprise de V6 §1.1 (narration du même auteur). Preuve circulaire. |
| « PR, builds et E2E s'accumulent » | V6 §1.1 | `PROPOSED` — aucune mesure (profondeur de file, délai médian, taux de conflit). |
| « Copies/worktrees coûteux » | CHARTER | **Contredit** par le seul chiffre disponible : A (copies privées) = 111,1 s, B (dossier partagé) = 237,7 s. |
| « Recompilation répétitive coûteuse » | CHARTER | `UNKNOWN` — pas de part du temps de compilation dans le temps d'un agent ; Cargo/tsc/pytest sont déjà incrémentaux. |
| Conflits d'agents = perte de travail | implicite | **Contredit** par S8 : 0 perte dans 9/9 campagnes (`MEASURED_HISTORICAL`). |
| Les hooks/tree/environnement de Ravum sont défectueux | V6 §2, `CODE_READ` | Le seul problème **réellement observé par lecture**, mais non reproduit dynamiquement. |

**Seul chiffre existant** (`MEASURED_HISTORICAL`, fixture Rust réduite, 27 sous-agents, 9 campagnes, 3 répétitions) : A copies privées + intégration **111,110 s** ; B dossier partagé + garde optimiste **237,674 s** ; C séquentiel **528,481 s** ; 0 perte. Trois lectures possibles, et elles ne sont pas équivalentes :
(i) la sérialisation est chère → un orchestreur a un intérêt ;
(ii) **la matérialisation par agent est le mode mesuré le plus rapide** → la prémisse d'INV-01 (« un seul répertoire canonique ») est le mode perdant ;
(iii) l'écart A/B peut n'être qu'un artefact de contention disque sur une petite fixture.

Flow est architecturalement plus proche de B (assemblage dans un dossier canonique, copies seulement pour l'exécution) que de A. Le corpus interdit de présupposer que B est plus rapide — mais il construit un produit dont la thèse ergonomique est B. **C'est la contradiction produit centrale du dossier**, et elle n'est traitée nulle part.

### 3.2 Hypothèses de demande non prouvées (falsifiables)

- **HD-1** — un développeur solo a *fréquemment* ≥3 agents concurrents sur le même dépôt. Non mesuré ; le cas majoritaire est 1 agent, parfois 2.
- **HD-2** — les éditions concurrentes causent une perte/reprise *perceptible*. S8 : 0/9. Aucun incident documenté.
- **HD-3** — la recompilation/CI est une part dominante du temps. Non mesuré ; probablement dominé par le LLM et la revue humaine.
- **HD-4** — l'utilisateur préfère un dossier canonique unique à des clones par agent. C'est un invariant du *concepteur*, pas une exigence utilisateur ; `git worktree add` est connu, documenté, et gagne dans la seule mesure disponible.
- **HD-5** — « temps jusqu'au livrable vérifié » est la métrique que l'utilisateur optimise. Un solo optimise peut-être l'*interruption*, la *confiance*, ou le *nombre de contextes à recharger*.
- **HD-6** — les agents adopteront un protocole managé. Faux par défaut : aucun agent n'appelle `flow propose` sans intégration dédiée ; il faut N intégrations (Claude Code, Codex, Gemini CLI, OpenCode, Cursor…) qui cassent à chaque release. **Coût d'intégration non budgété dans les 26 RF.**
- **HD-7** — les équipes viendront ensuite (R7). L'entreprise achète SSO, RBAC, audit, SLA, support : ce n'est pas une version ultérieure de ce produit, c'est une autre société.
- **HD-8** — l'offline est un critère d'achat. C'est une contrainte de l'auteur (et un vrai argument de confidentialité), pas une demande mesurée.

### 3.3 Qui paie quoi

| Acteur | Paie | Reçoit |
|---|---|---|
| Développeur solo | Installation, config capacités, nouvel outil dans le chemin de publication, ~1 full copy par vérification (disque : `target/` + `node_modules` = plusieurs Go ; NTFS Windows n'a pas de reflink) | Moins de collisions **s'il** a ≥3 agents ; sinon presque rien |
| Auteur/mainteneur | 500–900 h/an (`ESTIMATE`, cf. §6) + astreinte bugs + dérive toolchains + bus factor 1 | Un outil interne, éventuellement un produit |
| Ravum | **heures non consacrées au langage** — c'est le vrai coût d'opportunité | Rien de direct |
| Agents | Un appel CLI supplémentaire, un schéma JSON à respecter | Publier sans se faire refuser |

### 3.4 Le cas majoritaire : solo + 1 agent

Réponse sans complaisance : **Flow n'apporte presque rien, et probablement une valeur négative.** Avec un seul agent : pas de conflit (une seule plume), pas de contention de compilation, pas de coordination. Ce qui reste est un reçu lié à l'arbre exact et un refus de publication non vérifiée — que `git status` + CI sur PR + la présence de l'humain couvrent déjà, à coût nul. Ajouter un état intermédiaire (propositions, reçus, journal) crée une **seconde source de vérité à comprendre**, pour un cas qui n'existe pas.

Deux conséquences, et je les assume :
1. **La segmentation par « solo vs équipe » est fausse.** Le bon axe est le **nombre d'agents concurrents** (1 / 2 / ≥3) et la **présence d'une vérification chère**. Un solo avec 5 agents est la cible ; une équipe de 6 humains qui commit à la main ne l'est pas.
2. **Il faut donc soit repositionner, soit arrêter.** Le repositionnement que je défends (§9) rend le produit utile même à 1 agent : un **portail de publication** que l'agent doit franchir (1 commande, ~1 s), sans changer le workflow, et qui refuse un arbre non vérifié. C'est la seule version où « 1 agent » justifie l'installation.

---

## 4. Parcours CLI minimal et ergonomie du refus

### 4.1 Les commandes indispensables

| Commande | Doit faire | Ne doit jamais faire | Verdict |
|---|---|---|---|
| `flow init` | Détecter dépôt/langage/toolchain, écrire `.flow/`, afficher la **matrice de capacités** (sandbox : oui/non/dégradé ; adaptateurs ; hooks présents) | modifier le working tree, installer un hook, demander un compte | **must** |
| `flow status` | Vérité unique : base SHA, propositions et âge, état du verrou, dernier reçu, prochaine action recommandée | afficher un état « vert » sans reçu | **must** |
| `flow doctor` | Prérequis : version Git, `core.autocrlf`, `core.hooksPath`, casse/longueurs Windows, FS (reflink ?), espace disque, toolchains, sandbox réellement disponible | promettre un mode non testé ; « tout est OK » quand une capacité est absente | **must** |
| `flow propose` | Ingérer un patch (diff ou JSON) avec `base` + préconditions ; rendre un `proposal_id` et un statut d'admission ; utilisable sans TTY | exiger une explication interactive | **must** |
| `flow verify` | Matérialiser la vue jetable, exécuter les obligations, produire un reçu `(tree, contexte, obligations, statut)` | rendre `PASSED` par omission, réutiliser un reçu d'un autre contexte | **must** |
| `flow publish` | Unique mutation de ref : CAS + intention durable + réconciliation ; jamais le working tree | publier sur `STALE`/`UNKNOWN`/`UNSUPPORTED` ; `--force` silencieux | **must** |
| `flow recover` | Lire le journal et dire **exactement** où l'on en est ; n'offrir que des actions non destructives | proposer `reset --hard`, « réparer » en aveugle | **must** (peut être `status --recover`) |
| `flow explain <id>` | Expliquer un refus : code, cause, ce qui n'a pas changé, action suivante | noyer dans du JSON brut | **must** (le plus rentable) |
| `flow disable` | Retirer hooks, refs et configuration Flow ; rendre le dépôt utilisable par Git seul | laisser un daemon, un verrou, ou des refs que l'utilisateur doit nettoyer | **must** (INV-11) |

**Fusions que je recommande :** `propose` + `verify` → `flow run` (un agent ne devrait pas pouvoir vérifier ce qu'il n'a pas proposé) ; `snapshot` disparaît (c'est `status`) ; `recover` devient `status --recover`. Surface finale : **7 verbes** — `init`, `status`, `doctor`, `run`, `publish`, `explain`, `disable`. Toute commande doit accepter `--json` (sortie stable, versionnée : `{"schema":"flow.result/v0", …}`) et **ne jamais bloquer sur un TTY**.

### 4.2 Ce que l'utilisateur doit voir pour comprendre un refus

Un refus doit répondre, en moins de 10 lignes, à quatre questions : **qu'est-ce qui a échoué / qu'est-ce qui n'a pas changé / que faire maintenant / combien ça coûte**. Exemple normatif (texte) :

```text
STALE — refs/heads/dev a bougé : 1d5fddf -> 9ab3c21
Rien n'a été publié ; votre working tree est intact.
p-42 avait été proposé sur 1d5fddf ; le reçu rcpt-0187 ne couvre plus cet arbre.
→ flow verify --proposal p-42 --rebase      (~90 s, 1,4 Go disque)
   autre option : flow status
```

Et la forme machine (même contenu, contractuelle) :

```json
{"schema":"flow.result/v0","outcome":"STALE","changed_nothing":true,
 "ref":"refs/heads/dev","expected_old":"1d5fddf","actual_old":"9ab3c21",
 "receipt":"rcpt-0187","reasons":[{"code":"BASE_MOVED"}],
 "next_actions":[{"cmd":"flow verify --proposal p-42 --rebase","safe":true,"cost_s":90,"disk_mb":1400}],
 "needs_human":false,"exit":11}
```

**Règles non négociables du refus :** (a) un refus **précoce** — toute précondition est vérifiée *avant* le premier build (un `STALE` doit coûter < 2 s, pas 3 min) ; (b) **`changed_nothing: true|false` explicite** — l'agent doit savoir s'il peut rejouer sans dupliquer ; (c) **`next_actions` non vide, avec coût annoncé**, et une seule action « recommandée » ; (d) **codes de sortie stables et distincts** : `0` OK, `10` échec de vérification, `11` périmé, `12` conflit, `13` non supporté, `14` inconnu, `15` réconciliation requise, `16` humain requis, `2` usage ; (e) **un identifiant stable** (`proposal_id`, `receipt id`, `code`) pour que l'agent puisse chercher et rapporter ; (f) **jamais de cul-de-sac** : si Flow refuse, il indique toujours la sortie légale (`git` manuel, `flow publish --manual` réservé au TTY humain). Un agent ne doit jamais être obligé de contourner Flow pour finir sa tâche : sinon il apprendra à le contourner **toujours**.

### 4.3 Pourquoi un agent abandonne au premier refus

1. **Refus sans action suivante** : l'agent doit inventer → il invente `git reset`/`git checkout -- .` (destructif) ou escalade.
2. **Refus tardif** : après 3 minutes de compilation, le coût d'essai suivant est prohibitif → l'agent change de stratégie et sort du protocole.
3. **Refus ambigu sur l'état** (« publié ? partiellement ? ») → rejeu et doublons.
4. **Refus non déterministe** (même entrée, deux verdicts) → boucle : 2 essais, puis abandon.
5. **Refus qui exige un humain sans dire lequel ni pourquoi** → la tâche meurt silencieusement.
6. **Refus qui contredit `git`** (git dit OK, Flow dit `UNSUPPORTED`/`UNKNOWN`) sans explication de portée → perte de confiance immédiate, désinstallation.
7. **Refus sans identifiant** : impossible de rapporter, de rechercher, de reproduire.

Corollaire de conception : **le refus est le produit**. Un refus bien fait (précoce, codé, unique, reproductible, avec coût et échappatoire) vaut mieux que dix fonctionnalités ; un refus mal fait transforme un outil optionnel en obstacle et déclenche la désinstallation.

### 4.4 Test d'acceptation utilisateur observable (sans explication humaine)

Protocole : un participant (ou un agent) reçoit **uniquement le binaire** et `--help`. Aucune lecture de documentation, aucune réponse humaine.

| Test | Scénario | Pass | Fail (éliminatoire) |
|---|---|---|---|
| **T1 — premier livrable** | Clone d'un dépôt Rust avec 1 test rouge ; objectif : commit vérifié publié | ≤ 15 min, ≤ 3 commandes, 0 question posée | > 30 min, ou besoin de lire la doc/source |
| **T2 — refus compréhensible** | 3 refus injectés (base périmée, chevauchement, sandbox absente) | `next_actions[0]` exécutée → succès en ≤ 2 appels, 3/3 | 1 boucle, ou escalade humaine, ou `changed_nothing` absent |
| **T3 — refus peu coûteux** | `publish` sur base périmée avec build chaud | retour < 2 s, aucun processus de build lancé | un build démarre avant le refus |
| **T4 — désinstallation propre** | `flow disable` puis `git status`, `git log`, `git fsck --strict` | identiques, aucun processus, aucun verrou | objet/ref orphelin, hook résiduel |
| **T5 — non-régression Git** | `git commit`, `git rebase -i`, hooks natifs, avec Flow installé | comportement Git inchangé | toute différence observable |

**T1 est le vrai test produit** : « temps jusqu'au premier livrable vérifié, sans explication humaine » est la seule métrique qui mesure l'adoption, et elle est observable en une session. Si T1 échoue, aucun raffinement de S9 ne sauvera le produit.

---

## 5. Trim du MVP : must / should / defer / kill

### 5.1 Composants

| Composant | Réf. | Verdict | Raison |
|---|---|---|---|
| CLI + `--json` + codes de sortie | RF-006/016 | **must** | Seule interface ; contrat machine obligatoire pour les agents. |
| Admission de propositions (base + préconditions) | RF-007 | **must** | Le cœur : c'est ce qui distingue Flow de `git`. |
| Arbre candidat exact (OID Git) | RF-004 | **must** | Sans identité exacte, le reçu ne veut rien dire. |
| Reçu `(tree, contexte, obligations, statut)` | RF-011 | **must** | La valeur vérifiable ; statuts honnêtes. |
| Writer Git local : CAS + intention durable + recovery | RF-010/009 | **must** | Empêche la publication d'un arbre non vérifié. |
| Adaptateur Rust | RF-012 | **must** | Langage de l'auteur ; un seul au MVP. |
| Journal durable (SQLite ou append-only, cf. ADR-006) + `status`/`recover`/`explain` | RF-005/006 | **must** | Débogabilité = confiance = rétention. |
| Matrice de capacités + fail-closed | ADR-004 | **must** | Honnêteté ; sans ça, fausse promesse de sûreté. |
| `disable` / réversibilité | INV-11 | **must** | Condition d'installation (§6). |
| Politique E2E « jamais masqué » | RF-015 | **must** | Une seule règle, pas un moteur. |
| Détection de hooks → refus/délégation | RF-010 | **must** | Cheap et honnête ; évite le P0-HOOK-ORDER. |
| Hooks en espace isolé, ordre Git, capture d'index | RF-010, ADR-003 | **should** | La vraie douleur quotidienne (fmt/clippy), mais présuppose INV-01-A. |
| Cache par contexte exact (entrées complètes) | RF-014 | **should** | Sûr et simple ; remplace le graphe d'impact. |
| Isolation des outils non intégrés (`ISOLATED_EXEC`) | RF-RFC-004 | **should** | Utile, mais dégradable explicitement. |
| Adaptateurs TS, Python | RF-018/019 | **should** | Sur demande réelle seulement. |
| File d'exécution + annulation + budgets | RF-013 | **should** | Séquentiel suffit d'abord. |
| Graphe d'impact + tests sélectifs | RF-015 | **defer** | Faux vert garanti sans preuve ; R5+. |
| Serveur JSONL / SDK / MCP | RF-016 | **defer** | `--json` couvre 90 % du besoin agent. |
| Forge, merge queue, PR | RF-021 | **defer** | Hors chemin local. |
| Leases/fencing multi-writer | RF-013 | **defer** | Un seul writer local : verrou exclusif suffit. |
| C/C++ (et Go, hors backlog) | RF-020 | **kill** (MVP) | Variance maximale, preuve de valeur nulle. |
| Adaptateur Ravum | RF-026 | **defer** | Backlog de Ravum. |
| UI riche / revue de patches | RF-017 | **kill** | `git diff` + éditeur. |
| VFS / overlay IDE | RF-023 | **kill** | Rien ne le rend nécessaire ; risque énorme. |
| CRDT / merge sémantique | O4 | **kill** | Ne garantit aucune validité sémantique. |
| Daemon obligatoire | — | **kill** | Explicitement hors MVP ; coût de vie > bénéfice. |
| Multi-tenant / RBAC / remote CAS | RF-024/025 | **kill** | Autre produit. |
| CAS de contenu maison | RF-005 | **kill** (au MVP) | Objets Git + refs épinglées ; ne pas écrire un second magasin. |
| `SourceSnapshotId` (double identité) | RF-004 | **kill** (au MVP) | Une seule identité : OID Git. Utile seulement à l'adaptateur Ravum. |

### 5.2 Indispensables mais sous-spécifiés (le vrai risque)

1. **Contrat de refus/statuts** : 11 états nommés, zéro spécification de sortie machine, de code d'erreur, de `next_actions`. Rien dans les 26 RF. **C'est le manque le plus coûteux.**
2. **Sémantique de `obligations`** : qu'est-ce qui doit être exécuté pour une classe de changement, qui le définit, quel défaut ? Un reçu sans obligations déclarées est un tampon décoratif.
3. **Choix de stockage durable** : ADR-006 dit « comparer objets Git vs SQLite WAL ». Indispensable et non tranché ; c'est aussi le lieu du bug le plus probable (GC vs propositions).
4. **Coût de la matérialisation jetable sur Windows/NTFS** : pas de reflink, copies de plusieurs Go, Defender qui verrouille. Décide de la faisabilité sur la machine cible 16 Go — et n'est spécifié nulle part.
5. **Contrat de capacités d'adaptateur** : INV-06 exige la véracité, aucun schéma ni test de conformité. Sans lui, `PASSED` devient menteur.
6. **Budget de latence ajoutée au commit humain** : aucun objectif. Si `flow publish` ajoute > 2 s à `git commit`, l'outil est mort-né.
7. **Modèle de plan/confirmation avant mutation** (US-015) : ce que l'humain voit avant que les refs bougent — non spécifié.
8. **Versionnement du format sur disque + migration** : mentionné une fois (R1), aucune politique. Un outil solo qui casse son format perd la confiance définitivement.
9. **Test d'acceptation de désinstallation** (INV-11) : invariant sans test.
10. **Détection des écritures non managées** : présentée comme « best effort », mais ce qui n'est *pas* protégé n'est écrit nulle part en langage utilisateur.

---

## 6. Promesses à ne jamais faire

| Promesse interdite | Pourquoi elle est fausse ou toxique | Reformulation honnête |
|---|---|---|
| « zéro worktree » | Dès qu'un hook, un build ou un outil non intégré a besoin de chemins, il faut une vue disque ; V6 §3.2 l'admet. | « aucun worktree persistant obligatoire ; des vues temporaires jetables peuvent être créées et sont supprimées ; le coût disque est réel et annoncé. » |
| « aucun conflit » | Aucun outil ne supprime les conflits sémantiques ; Flow ne peut que les *détecter*. | « les chevauchements détectables sont refusés avant publication ; la résolution reste manuelle. » |
| « isolation totale » | Dépend de l'OS ; Windows n'offre pas de garantie uniforme ; un outil qui écrit directement dans le dossier n'est jamais isolé. | « isolation au niveau que l'OS permet, annoncée par capacité ; sinon `UNSUPPORTED` et refus d'exécuter. » |
| « preuve de correction » | Un reçu atteste l'exécution d'obligations déclarées, pas la justesse du programme. | « un reçu qualifie un arbre et un contexte exacts pour un périmètre déclaré ; `PASSED` ≠ `PROVED`. » |
| « merge automatique sûr » | Il n'y a pas de mergeur sémantique ; un merge textuel réussi peut casser la compilation. | « Flow applique des propositions sur une base et refuse en cas de chevauchement ; il ne fusionne pas le sens. » |
| « fonctionne avec tout outil non intégré » | Vrai au sens « ne casse pas », faux au sens « protège ». | « les outils non intégrés continuent de fonctionner, sans garantie anti-écrasement ; Flow le dit avant l'installation. » |
| + « plus rapide » | Le seul chiffre disponible (S8) ne mesure pas Flow ; le seuil de 15 % (`ESTIMATE` historique) ne paierait pas la maintenance. | « plus rapide sur *votre* dépôt, mesuré en paires, ou nous ne le recommandons pas. » |
| + « compatible Git » | Vrai pour lire ; faux pour les hooks (un commit `commit-tree` ne les exécute pas comme `git commit`). | « Git peut toujours lire, réparer et continuer ; la publication Flow n'est pas équivalente à `git commit` si des hooks existent. » |
| + « zéro perte » | Aucun test ne couvre tous les entrelacements ; S8 = 0/9 sur une fixture. | « pas de perte *silencieuse* dans les cas testés ; les cas non couverts sont listés. » |
| + « prêt pour l'entreprise » | R7 n'est pas une version : SSO/RBAC/audit/SLA/support sont une autre activité. | « usage local solo ; aucune revendication entreprise. » |

**Règle générale :** toute promesse doit porter son **périmètre**, son **mode** et son **statut** ; une phrase qui ne peut pas être falsifiée par un test nommé ne doit pas figurer dans le README.

---

## 7. Réponse contradictoire : « quels éléments du MVP sont superflus ? »

Classement du plus au moins superflu, avec la conséquence de la coupe :

1. **UI riche / IDE (RF-017, RF-023)** — superflus ET dangereux : ils attirent l'effort vers la présentation avant la preuve. Coupe : rien.
2. **CRDT / merge sémantique (O4)** — superflu *par nature* : un CRDT ne valide pas un programme. Coupe : rien ; gain : suppression d'une classe entière de bugs.
3. **Multi-tenant / RBAC / remote CAS / remote executors (RF-024/025)** — superflus pour 10 ans si le produit reste solo-local. Coupe : rien.
4. **CAS de contenu maison (RF-005)** — superflu : Git *est* un CAS adressé par contenu, avec GC et fsck. Coupe : un magasin, un format, un GC à maintenir. **Attention** : le pinning/GC des propositions reste indispensable (refs `refs/flow/*`).
5. **`SourceSnapshotId` au MVP (RF-004)** — superflu hors adaptateur Ravum : deux identités = deux sources de bugs de comparaison (V6 l'interdit déjà). Coupe : une notion.
6. **Serveur JSONL / SDK / MCP (RF-016)** — superflu au MVP : `--json` sur le CLI donne le même service avec un processus de moins.
7. **Graphe d'impact + tests sélectifs (RF-015)** — superflu *et* contre-productif sans corpus : la sélection par impact est la fabrique à faux verts. Coupe : on garde « exécuter les obligations déclarées, jamais masquer un E2E ».
8. **Leases/fencing multi-writer (RF-013)** — superflu mono-machine : verrou exclusif + epoch. Coupe : un protocole distribué.
9. **Le « dossier canonique unique » (INV-01) comme invariant produit** — c'est mon assertion la plus discutée : je le sors des *invariants* pour le mettre dans les *options*, car S8 pénalise ce mode. Si on le garde comme dogme, on construit le mode perdant.

**Puis je me contredis, parce que trois coupes sont fragiles :**
- **Couper les hooks (US-008 → defer)** est le trim le plus risqué : c'est *le seul* problème démontré par lecture (`CODE_READ`) et la douleur quotidienne réelle (fmt/clippy en `pre-commit`). Le publieur sans hooks est plus sûr mais moins utile ; si les hooks sont refusés, Flow devient « encore un `git commit` ».
- **Couper le merge (RF-008)** laisse l'utilisateur avec des refus de chevauchement — c'est-à-dire du travail manuel, exactement ce que `git rebase` fait déjà mieux. La valeur perçue peut s'effondrer.
- **Couper la mesure (US-011/US-013)** au nom de la « pureté produit » serait une faute : ces stories sont mal formées mais leur *intention* (ne pas construire sans preuve) est la seule protection contre 900 h/an perdues. Je les sors du set de stories, pas du projet.

---

## 8. Désaccords réels

1. **Avec le problème déclaré** (`PROJECT_CHARTER.md:5`) : c'est une pétition de principe reprise de V6 §1.1. Aucune mesure de file de PR, de taux de conflit, ni de part de compilation. Le seul chiffre existant contredit la prémisse du produit.
2. **Avec la segmentation « solo d'abord »** : l'axe pertinent est le nombre d'agents concurrents. « Solo + 1 agent » est un cas O0, pas un cas Flow ; le produit tel que spécifié est inutile pour son audience déclarée.
3. **Avec la phase R1 avant toute mesure de demande** : on spécifie des contrats (RF-004/005/011) avant de savoir si le problème dépasse le seuil qui justifie 100 jours/an. La mesure de demande coûte **une semaine** et peut tuer le projet ; elle devrait précéder R1, pas la suivre (voir §9, F1).
4. **Avec le seuil de 15 %** (V6 l. 714, archive) : même atteint, il ne paie pas la maintenance (§6). Le seuil doit être exprimé en **heures humaines épargnées par an**, pas en pourcentage de wall-clock d'une fixture.
5. **Avec le nom** « Ravum Flow » : il signale « outillage pour le langage Ravum » alors que la cible est polyglotte — un développeur TypeScript n'installe pas un outil nommé d'après le langage d'un autre. Pire, le binaire `flow` **entre en collision avec Flow (Meta), le vérificateur de types JavaScript**, présent dans `node_modules/.bin` de nombreux dépôts TS/JS : confusion garantie, ombrage possible dans le PATH. `ravum-flow` (RFC-001) évite la collision mais aggrave le problème de marque. **Rien de cela n'est discuté dans les 26 RF.**
6. **Avec deux familles d'identifiants `RF-`** (`RF-0..RF-9` jalons *et* `RF-001..026` tickets) : un message d'erreur qui dit « voir RF-4 » est ambigu pour l'utilisateur comme pour le triage. V6 §7.1 le reconnaît sans le corriger.
7. **Avec la présence de RF-017 (UI) en P1** et de 4 adaptateurs en R5 : l'effort est placé là où la valeur n'est pas prouvée, pas là où le risque est démontré (hooks, refus, réversibilité).
8. **Avec R7 « entreprise »** : ce n'est pas une phase, c'est une autre entreprise (support, conformité, SLA, bus factor). La garder comme engagement de roadmap est un mensonge de planification.
9. **Avec moi-même, sur INV-01** : je propose de le rétrograder d'invariant à option, ce qui contredit l'INV-01 historique et une partie de V6 §1.2. Je l'assume : un invariant qui contredit la seule mesure disponible doit être réexaminé, pas protégé.

---

## 9. Recommandations et falsification

### 9.1 Décider d'abord *quel produit* c'est

Deux produits différents sont mélangés dans les mêmes documents :
- **(P1) un orchestrateur de workspace multi-agents** (O2/O3, dossier canonique, VFS futur) : gros, non prouvé, coûteux, en conflit avec S8.
- **(P2) un portail de publication vérifiée pour agents** : les agents travaillent où ils veulent (clones privés = mode A mesuré le plus rapide) ; avant `push`, ils passent une porte qui vérifie l'arbre exact et refuse la publication non qualifiée.

**Je recommande P2**, pour quatre raisons : (a) il est aligné sur la seule mesure disponible ; (b) il est utile même avec **1 agent** (ce qui résout le cas majoritaire) ; (c) il est 3 à 5 fois plus petit ; (d) il repose sur ce qui est réellement démontré par lecture (ordre des hooks, arbre exact, hygiène d'environnement, refus explicites). P1 ne devient justifiable que si un test de demande montre ≥3 agents concurrents fréquents **et** un gain ≥20 % sur P2.

### 9.2 La plus petite version vérifiable

**Flow-mini (P2), 1 binaire, 1 langage, 0 daemon, 0 UI, 0 merge, 0 nouveau format de contenu :**
`init` · `status` · `doctor` · `run` (= propose+verify) · `publish` · `explain` · `disable`, tous `--json`.
Contenu : propositions en objets Git sur `refs/flow/*` + journal durable (SQLite ou append-only, ADR-006) ; vérification dans une vue jetable (copie ou reflink, supprimée) ; reçu `(tree, contexte, obligations, statut)` ; publication locale CAS + intention durable + recovery ; refus précoces, codés, avec `next_actions`. Détection de hooks → refus ou délégation explicite à `git`. Aucun accès réseau.

**Alternative encore plus petite, que je recommande d'évaluer en premier (`ESTIMATE` 3–5 jours) :** un *wrapper* de ~500 lignes au-dessus de `git worktree` + `git commit` + un reçu JSON + un `pre-publish` hook. Si ce wrapper suffit à faire gagner du temps, alors le moteur n'a pas lieu d'être — c'est l'option O0 instrumentée, et elle est la vraie baseline à battre.

### 9.3 Test de demande (une semaine, avant tout code de contrat)

Sur les dépôts réels de l'auteur, pendant 4 semaines, compter : (i) occurrences d'écrasement/conflit entre agents ; (ii) temps passé à réparer ; (iii) part du temps d'agent en compilation/CI ; (iv) délai de file PR/CI ; (v) nombre d'agents concurrents réels. **Aucun de ces chiffres n'existe aujourd'hui.** Ils coûtent quelques heures et décident de tout.

### 9.4 Critères de mise à mort (falsification)

| # | Condition observée | Décision |
|---|---|---|
| F1 | < 5 incidents/écrasements en 4 semaines **et** < 10 % du temps en vérification/CI, avec ≥3 agents | **Ne pas construire** ; documenter O0 instrumenté |
| F2 | Test apparié à 3 agents : Flow-mini < 20 % plus rapide que clone privé + CI | **Ne pas généraliser** ; garder P2 en usage interne ou arrêter |
| F3 | À 1 agent, Flow-mini plus lent que `git commit` + CI (dans le bruit) | **Retirer du chemin humain** ; portail réservé aux agents |
| F4 | Après 4 semaines de dogfooding, `flow disable` pendant 1 semaine sans manque | **Kill** |
| F5 | > 1 jour/semaine de maintenance pendant 8 semaines consécutives | **Kill ou réduction au wrapper** |
| F6 | Un refus erroné bloque une publication légitime (dogfooding) | Correctif < 2 jours, sinon **retrait du chemin critique** |
| F7 | Un cas où `git` seul ne peut plus lire/réparer le dépôt | **Kill immédiat** (viole INV-11) |
| F8 | Coût de maintenance > valeur annuelle épargnée (cf. §6) | **Internal tooling** ou arrêt |

### 9.5 Coût de maintenance vs « ne rien installer »

| Option | Coût annuel mainteneur (`ESTIMATE`) | Coût utilisateur | Valeur annuelle attendue |
|---|---|---|---|
| Ne rien installer (O0 instrumenté) | 0 h | 0 | 0 (par définition) ; le problème reste |
| Wrapper `git worktree` + reçu (~500 lignes) | 80–200 h | ~30 min | Peut-être 20–50 h si ≥3 agents |
| **Flow-mini (P2)** | **300–500 h** | ~1–2 h | 100–200 h (hypothèse HD-1/HD-2) |
| Flow O2/O3 complet (26 RF) | 1 100–1 900 h | jours | Non démontrée ; risque de valeur négative |

Lecture brutale : avec **un seul utilisateur**, la maintenance de P2 (300–500 h/an) dépasse la valeur plausible (100–200 h/an). Flow n'est donc rentable que si (a) il sert d'abord l'auteur sur Ravum et que les heures sont gagnées sur le langage, ou (b) il acquiert des utilisateurs. **C'est la décision qui manque aux six décisions H1–H6 : H7 — « Flow est-il un outil interne pour Ravum, ou un produit public ? »** La réponse change tout le reste : périmètre, licence, support, et même la nécessité des 15 user stories.

### 9.6 Ce qui doit exister pour qu'un solo l'installe et le garde

Installation en une ligne, sans compte, sans daemon, sans config (auto-détection Rust uniquement au départ) ; premier succès en ≤ 15 min (T1) ; `doctor` qui dit « non supporté » au lieu de mentir ; refus actionnables (T2) ; désinstallation prouvée (T4) ; Git intact (T5) ; journal lisible à la main ; 0 télémétrie ; un binaire, `--version --json`, format versionné. Et surtout : **ne jamais être sur le chemin critique de l'humain**. Un humain bloqué par un bug de Flow désinstalle en 30 secondes ; un agent bloqué est un incident tolérable.

---

## 10. Limites

1. Aucune mesure propre : pas de test, pas de build, pas de benchmark, pas de protocole S9 exécuté. Tous mes chiffres de coût sont des `ESTIMATE` à réviser, pas des faits.
2. Aucun accès au marché : pas d'entretien utilisateur, pas de télémétrie, pas d'analyse du dépôt Unifia ni de la file de PR réelle. Mes hypothèses de demande (HD-1..HD-8) sont falsifiables, pas vérifiées.
3. Je n'ai pas lu l'intégralité des 5 838 lignes de V6 : §1, §2, §2.1, §3, §6.1–6.2, §7, §8.1, §9, §10 ; l'archive V4 n'est lue qu'incidemment (citations reprises d'`AGENTS.md`/`DECISION_REGISTER`).
4. Je n'ai pas lu les rapports des autres agents (`docs/reviews/raw/` était vide) : mes « désaccords » portent donc sur le corpus documentaire, pas sur A/B/C/D/E/F/H. Certains seront convergents, d'autres non — le Lead doit traiter mon §8 comme une entrée, pas comme un arbitrage.
5. Le jugement sur la collision de nom `flow` avec Flow/Meta est du raisonnement (`PROPOSED`), non une vérification de disponibilité de nom/marque : à confirmer par une recherche de noms avant toute décision.
6. L'ergonomie du refus est spécifiée à partir de la machine d'états de V6 §4.1 et de `ARCHITECTURE.md` ; je n'ai pas pu confronter le vocabulaire à un prototype (il n'y en a pas).
7. Je n'ai pas évalué le coût réel de la matérialisation sur la machine cible (Windows/NTFS, 16 Go) : c'est identifié comme risque majeur §5.2, non mesuré.

---

## 11. Références

**Interne (lecture directe, HEAD `1d5fddf`) :**
- `AGENTS.md` · `README.md` · `CONTRIBUTING.md` · `.github/PULL_REQUEST_TEMPLATE.md`
- `docs/DEEPSEEK_GOAL.md` (Lots 4–5) · `docs/PROJECT_CHARTER.md` (§Problem, Audience, non-goals)
- `docs/USER_STORIES.md` (US-001..015) · `docs/BACKLOG.md` (RF-001..026) · `docs/ISSUE_MAP.md` (47 issues)
- `docs/ROADMAP.md` (R0–R7) · `docs/DECISION_REGISTER.md` (H1–H6, D0–D4) · `docs/TRACEABILITY.md`
- `docs/ARCHITECTURE.md` (état → `RECEIVED`…`RECONCILED`, statuts secondaires)
- `docs/BENCHMARK_S9.md` · `docs/SECURITY_TESTS.md` (TREE-01, HOOK-02a/b/c, ENV-04, LEGACY-06, FENCE-07, GIT-10, FS-11, CRASH-12, E2E-13, CACHE-14, TRUST-15, PATH-16) · `docs/RAVUM_BOUNDARY.md`
- `docs/adr/FLOW-ADR-000..011` (surtout 001, 002, 003, 004, 005, 006, 007, 008, 010, 011) · `docs/rfc/README.md` + RF-RFC-004/006/009/012
- `docs/reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md` : §1.1–1.3, §2/2.1 (P0-TREE, P0-HOOK-ORDER, P0-ENV, P0-LEGACY, P1-FILES, P1-STORE), §3.1–3.3 (O0–O4, INV-01-A), §6.1 (S8 : 111,110 s / 237,674 s / 528,481 s), §6.2 (S9 pré-enregistré), §7.2–7.4 (D0–D4, R0–R7), §8.1 (COR-01..21), §9 (matrice négative), §10.1–10.3 (H1–H6, handoff)

**Externe (cité, non re-vérifié ici) :** Git hooks officiels <https://git-scm.com/docs/githooks> (via V6 §2.1) ; `Rwanbt/Ravum` @ `5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96` (lecture seule, non cloné).

**Preuves historiques internes citées par le corpus :** `evidence/mvp0c-s8-live-summary-2026-11-09.json` (Ravum `6b0ac7b6…`) ; `docs/mvp/MVP-0c-s8.md` — `MEASURED_HISTORICAL`, non re-mesurés ici.

---

*Fin du rapport G. À consolider par le Lead ; aucune ratification implicite ; aucune décision humaine présumée.*
