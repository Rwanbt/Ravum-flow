# Flow — décisions humaines : dossier de réponse en une passe

Date : 2026-11-12 · Projet : `f49a3c31` (Ravum Flow) · Branche : `feat/flow-production-candidate`

Ce dossier existe parce que les cartes du tableau sont des titres de travail, pas des questions
auxquelles on peut répondre. Chaque fiche donne : **la question**, **les options**, **la recommandation**,
**ce que ça coûte**, **ce que ça débloque** et **ce qui se passe si on ne décide pas**. Les affirmations
sont classées `SOURCE_READ` (lu dans le dépôt), `MESURÉ` (preuve reproductible) ou `PROPOSÉ` (mon avis).

**Pour répondre**, il suffit d'écrire par exemple : `A1 oui, A2 oui, A6 non, B1 option 2, C1 GO, D1 adopter`.
Chaque fiche a un identifiant (`A1`…`D1`).

---

## 1. Tableau de bord

| # | Carte | Question en une phrase | Recommandé | Effort si GO | Si refus / différé |
|---|---|---|---|---|---|
| A1 | H1 `t-mw9yd7ig-a17dqt` | Acceptez-vous le dossier d'options architecturales O0–O3 (sans choix ratifié) ? | **Oui** | 0 | H1 reste ouverte, S9 et RF-012+ restent bloquées |
| A2 | H3 `t-mw9ydoip-5suz1v` | Acceptez-vous la comparaison sandbox (3 sources primaires, le reste `INCONNU`) ? | **Oui** | 0 | la question sandbox reste sans base documentaire |
| A3 | H5 `t-mw9ye8hw-0lt61l` | Acceptez-vous le constat « GO du 09/11 consommé » + le trou d'habilitation des campagnes du 11/11 ? | **Oui, avec réserve** | 1 h de registre | les campagnes restent non rattachées à un GO traçable |
| A4 | RF-001 `t-mw9yemp2-xdwxaz` | Acceptez-vous l'état des droits (nom, licence MPL-2.0, provenance) ? | **Oui** | 0 | RF-004→011 restent bloquées |
| A5 | RF-003 `t-mw9yf2b4-1w0uz3` | Acceptez-vous le pré-enregistrement S9 (sans exécution) ? | **Oui** | 0 | S9 n'a pas de protocole gelé |
| A6 | G10 `t-mw9ym6ae-a5hrrj` | Acceptez-vous la revue G10 **sachant qu'elle n'est pas indépendante** ? | **Non — elle ne clôt pas la porte** | relecteur externe | G10 reste ouverte, c'est correct |
| A7 | Métadonnées `t-mw9yx8mc-r1uqe3` | Acceptez-vous les métadonnées d'import (25 flags humains, lien H2 corrigé) ? | **Oui** | 0 | l'import garde ses métadonnées fausses |
| A8 | P0 `t-mwaiks1h-7wzjzb` | Acceptez-vous la fermeture de la fidélité Git (attributs locaux + hooks) ? | **Oui** | 0 | le correctif P0 reste « en revue » indéfiniment |
| A9 | W4 `t-mwaw7ucl-sgah6i` | Acceptez-vous les écarts de preuve W4 (couverture branches, mutation) ? | **Oui** | 0 | rien — c'est de la mesure |
| A10 | W5 `t-mwb35m2t-s1xc92` | Acceptez-vous les seuils 95/90 et la mutation élargie ? | **Oui** | 0 | la gate de couverture reste non ratifiée |
| A11 | W7 `t-mwbzcdym-tgx65p` | Acceptez-vous les trous d'assertion W7 et les sites `cfg(unix)` ? | **Oui** | 0 | idem |
| A12 | W8 `t-mwc3lw50-trmy2x` | Acceptez-vous les taux Linux par module et la fermeture des survivants ? | **Oui** | 0 | idem |
| B1 | H2 `t-mw9ydgdy-ys8vh5` | Quel régime de droits pour le code Ravum ? | **Option 2 : clean-room** | 2–4 h d'audit | RF-026 et le handoff restent bloqués |
| B2 | H4 `t-mw9ydwm1-jvv03v` | Quels seuils et budget pour la campagne S9 ? | **Option 3 : différer après H1** | 1 h quand H1 est tranchée | S9 et RF-022 restent impossibles |
| B3 | H6 `t-mw9yefcd-tr5397` | Approuvez-vous le handoff P0 vers la session Ravum ? | **Oui** | 15 min | la session Ravum ne reçoit rien |
| B4 | H7 `t-mw9yknb0-qpm9mq` | Usage interne ou produit indépendant ? | **Option 1 : usage interne d'abord** | 0 | la feuille de route reste ambiguë |
| B5 | H8 `t-mw9ykuff-tr13u4` | Quelle règle d'arrêt des revues documentaires ? | **Option 2 : arrêt sur « aucun constat P0/P1 »** | 0 | on continue à empiler des revues |
| C1 | RF-008 (GO R2) | Autoriser conflits/undo ? | **DIFFÉRER** | — | RF-008 reste en backlog |
| C2 | RF-012→020 (GO R3/R5) | Autoriser les adaptateurs ? | **GO après H4**, sinon différer | 2–5 j | adaptateurs non livrés |
| C3 | RF-021 (GO R6) | Autoriser l'intégration forge ? | **DIFFÉRER** | — | rien de perdu |
| C4 | RF-022 (S9) | Autoriser la campagne S9 ? | **DIFFÉRER** (dépend de H4) | — | pas de preuve de productivité |
| C5 | RF-023→025 (GO R7) | Autoriser VFS / sync distribuée / CAS distant ? | **DIFFÉRER** | — | périmètre recherche seulement |
| C6 | RF-026 | Adaptateur Ravum ? | **DIFFÉRER** (dépend de B1+B3) | — | — |
| C7 | G6 (×2) | Ratifier les gates candidates ? | **GO pour ratification**, pas pour seuils | 30 min | gates non ratifiées |
| C8 | G7 (×2) | GO CI : épingler les actions, builds jetables ? | **GO** (déjà partiellement fait) | 1 h | reproductibilité non prouvée |
| D1 | 15 × `[US-0xx][STORY]` | Que faire de ces 15 cartes ? | **Adopter comme checklist d'acceptation** | 0 | elles restent des cartes mortes |

---

## 2. Famille A — les 12 cartes en revue

Accepter une carte en revue veut dire : **le livrable est lu et jugé conforme à ce qui était demandé**.
Ce n'est pas un feu vert de fusion, ni une validation produit. Presque toutes ont un effet mécanique
immédiat : elles sont les prérequis des 8 cartes RF en `todo`.

### A1 — H1, options architecturales (`t-mw9yd7ig-a17dqt`)
- **Livré** : `docs/planning/2026-11-11-flow-h1-options.md` — quatre options O0 (ne rien construire),
  O1, O2, O3, définies depuis V6 §3.1, avec sources datées (SHA-256 des trois documents), matrice de
  coûts sur une machine 16 Go, objections par option, O4 différée. **H1 reste `OPEN` : rien n'est ratifié.**
- **Accepter =** reconnaître que la question est correctement posée, pas choisir une option.
- **Reste ouvert** : le choix lui-même, et un écart constaté (registre `D4 = PARTIAL` vs V6 `NOT GIVEN`).
- **Débloque** : B2 (H4) et une partie des GO de la famille C.
- **Recommandation : OUI.** Le document ne prétend pas décider.

### A2 — H3, comparaison sandbox (`t-mw9ydoip-5suz1v`)
- **Livré** : `docs/planning/2026-11-11-flow-h3-sandbox-comparison.md` — état de Flow (allowlist
  d'environnement, rédaction, exécution bornée), matrice Windows/Linux, **trois sources primaires
  réellement récupérées** (Landlock 7.4.0-rc2, README bubblewrap, MS Job Objects 2025-07-14).
  AppContainer, Windows Sandbox, seccomp, cgroups, firejail : **`INCONNU`** — non récupérés, et c'est
  écrit comme tel.
- **Accepter =** valider la base documentaire, pas le choix d'un bac à sable.
- **Recommandation : OUI.** La partie honnête est justement celle qui dit ce qui n'a pas été lu.

### A3 — H5, GO consommé (`t-mw9ye8hw-0lt61l`) — **le point le plus délicat**
- **Livré** : `docs/planning/2026-11-11-flow-h5-consumed-go.md` — le GO du 2026-11-09 est retracé et
  **consommé** ; matrice permission → preuve → consommation ; et un **écart signalé** : les campagnes du
  2026-11-11 (couverture, mutation, adverse, polyglotte) sont des « nouvelles campagnes » au sens de ce
  commentaire, or leur habilitation **n'est pas traçable au registre** comme un `GO(scope, …)`.
- **Votre choix** : (a) ratifier *a posteriori* ces campagnes, (b) constater l'écart et l'assumer, (c)
  rouvrir un GO borné pour la suite.
- **Recommandation : OUI à la carte, option (b) constater l'écart + (c) un GO borné pour la suite.**
  Ratifier *a posteriori* est possible mais affaiblit la valeur du registre : un GO qui se ratifie après
  coup n'est plus une autorisation préalable.
- **Ce que ça coûte** : ~1 h pour écrire le constat d'écart au registre.

### A4 — RF-001, droits (`t-mw9yemp2-xdwxaz`)
- **Livré** : nom `Rwanbt/Ravum-flow`, dépôt **PUBLIC**, branche par défaut `main`, **MPL-2.0** (API +
  LICENSE + Cargo.toml), provenance mesurée (`git shortlog` : une identité humaine + des identités
  d'agents `@example.invalid`, `Cargo.lock` = 1 entrée, **aucune référence Ravum**), droits restants
  renvoyés à H2.
- **Accepter =** valider l'état des droits **constaté**, pas la décision sur le code Ravum (c'est B1).
- **Débloque** : RF-002, RF-004, RF-005 (chaîne des qualifications).
- **Recommandation : OUI.**

### A5 — RF-003, pré-enregistrement S9 (`t-mw9yf2b4-1w0uz3`)
- **Livré** : `docs/planning/2026-11-11-flow-rf003-s9-preregistration.md` — S8 préservé (médianes,
  chevauchement, p = 0,125) ; bras A–G conservés ; ajouts : matrice de disponibilité par OS,
  appariement/randomisation, méthodes de coût (humain, CPU, RAM, disque, plafond 16 Go), veto de perte
  avec 5 conditions, gabarit de métriques. **H4 reste HUMAN-DECISION ; aucun run.**
- **Accepter =** geler le protocole **documentairement**, pas exécuter S9.
- **Recommandation : OUI.**

### A6 — G10, seconde revue indépendante (`t-mw9ym6ae-a5hrrj`)
- **Livré** : `docs/reviews/FLOW-PRODUCTION-G10-MAINTAINABILITY-REVIEW.md`. **L'indépendance n'est pas
  atteinte et le document le dit en tête** : il a été produit par un agent de la même équipe. La prémisse
  de la carte était périmée (une seconde revue existait déjà depuis `0263c35`). Constats neufs :
  **N1** `dist/PROVENANCE.json` épingle `f5bb271` (pas la tête candidate) ; **N2** `dist/` est gitignoré
  donc provenance et SBOM ne sont pas versionnés ; N3 `SHA256SUMS` partiel ; N4 dérive `D4` ; N5 cible
  mouvante ; N6 `commands.rs` 2 917 lignes.
- **Recommandation : NON pour la clôture.** Acceptez le document comme *revue interne*, gardez la porte
  G10 **ouverte** jusqu'à un relecteur hors équipe. C'est la seule réponse honnête : un agent de
  l'équipe ne peut pas signer une revue indépendante de lui-même.
- **Actionnable tout de suite sans vous** : N1/N2 (régénérer `dist/` et versionner le manifeste) — mais
  c'est une action de build, donc il faut un GO (famille C8).

### A7 — Métadonnées d'import (`t-mw9yx8mc-r1uqe3`)
- **Livré** : 25 flags `waitingForHuman` posés avec une raison spécifique (H2, H4, H6, RF-008, RF-012→026,
  H7, H8, G6, G7), statuts et dépendances inchangés, et le lien H2 corrigé dans la description **et** le
  prompt. Les 15 cartes `[US-0xx]` n'ont pas été touchées (voir D1).
- **Recommandation : OUI.** C'est de l'hygiène de tableau.

### A8 — P0 fidélité Git (`t-mwaiks1h-7wzjzb`) · **6/6**
- **Livré** : la fermeture du défaut P0 de fidélité du checkout de vérification (attributs locaux +
  fichiers de hooks), avec la campagne adverse rejouée.
- **Accepter =** considérer que le P0 est corrigé **au vu des preuves** ; cela ne dit rien de la
  qualification produit globale.
- **Recommandation : OUI.** C'est le seul P0 connu et il a une preuve reproductible.

### A9–A12 — W4, W5, W7, W8 (mesure)
- **Livré** : couverture branches et mutation élargie (W4) ; seuils 95/90 et mutation des modules
  restants (W5) ; trous d'assertion et sites `cfg(unix)` sous Linux (W7) ; taux Linux par module et
  fermeture des survivants (W8, checklist 8/8, dont la stabilité enfin satisfaite en vague 9).
- **Accepter =** ratifier les chiffres, pas le produit.
- **Recommandation : OUI pour les quatre.**
- **Effet cumulé** : les huit cartes RF en `todo` deviennent exécutables.

---

## 3. Famille B — décisions produit

### B1 — H2, droits sur le code Ravum (`t-mw9ydgdy-ys8vh5`)
**Question.** Peut-on extraire du code de `Rwanbt/Ravum` vers Flow ?
- **Option 0** — ne rien extraire, Flow reste autonome. Coût 0. Conséquence : certains adaptateurs
  (RF-026) resteront impossibles.
- **Option 1** — extraire avec attribution MPL-2.0. Risque juridique non expertisé : personne n'a
  qualifié les ayants droit. **Non recommandé.**
- **Option 2 — clean-room : spécifier le comportement, réimplémenter sans lire le code Ravum.** Coût
  2–4 h d'audit + la réimplémentation. C'est la seule voie qui ne demande pas de trancher des droits.
- **Recommandation : option 2.** Elle débloque RF-026 sans hypothéquer la licence, et elle est
  compatible avec la MPL-2.0 déjà choisie.

### B2 — H4, seuils et budget S9 (`t-mw9ydwm1-jvv03v`)
**Question.** Quel seuil relatif, quel plancher absolu, quel budget, quel `n` pour la campagne
confirmatoire S9 ?
- **Option 1** — fixer maintenant. Il faut alors : seuil relatif (ex. ≥ 5 % de temps médian en moins),
  plancher absolu, budget heures-machine, `n` par bras, α et puissance. **Problème** : S9 compare Flow à
  Git « nu » ; or l'option H1 n'est pas choisie, donc l'objet comparé n'est pas stable.
- **Option 2** — fixer des valeurs provisoires et accepter qu'elles changent.
- **Option 3 — différer après A1 (H1).** Coût : S9 reste bloquée. **Recommandé.**
- **Recommandation : option 3**, puis une fiche chiffrée en 1 h quand H1 est tranchée. Un protocole
  pré-enregistré contre un objet mouvant ne vaut pas mieux que pas de protocole.

### B3 — H6, handoff P0 vers la session Ravum (`t-mw9yefcd-tr5397`)
**Question.** Transmettez-vous le constat P0 (fidélité Git) à la session qui possède `Rwanbt/Ravum` ?
- **Oui** : un brouillon de handoff, sans aucune écriture dans Ravum. **Recommandé.**
- **Non** : la session Ravum travaille sans l'information.
- **Coût** : 15 min. **Risque** : nul pour Flow, c'est un document.

### B4 — H7, usage interne ou produit (`t-mw9yknb0-qpm9mq`)
- **Option 1 — usage interne d'abord** : Flow sert à sécuriser *vos* publications ; le produit
  indépendant est un sous-produit documenté. **Recommandé** : c'est l'usage pour lequel toutes les
  preuves existent (receipt, CAS, journal, isolation).
- **Option 2 — produit indépendant** : il faut alors support, compatibilité, versions, sécurité multi-
  utilisateurs — rien de tout cela n'est prouvé aujourd'hui.

### B5 — H8, règle d'arrêt des revues (`t-mw9ykuff-tr13u4`)
- **Option 1** — une revue par vague, obligatoire.
- **Option 2 — arrêt quand une revue ne produit aucun constat P0/P1 neuf.** Recommandé : sinon les
  revues se reproduisent indéfiniment (c'est déjà arrivé : G10 citait une prémisse périmée).

---

## 4. Famille C — autorisations (GO bornés)

Un `GO` chez Flow est un contrat : `GO(scope, repository, branch, allowed_paths, budgets, evidence,
stop_conditions)`. Les cartes ci-dessous attendent un GO ; **aucune n'est approuvée par défaut**.

| # | Cartes | GO à accorder | Recommandation | Pourquoi |
|---|---|---|---|---|
| C1 | RF-008 | R2 — conflits et undo | **DIFFÉRER** | dépend de H1 (modèle d'arbitrage) |
| C2 | RF-012→020 | R3/R5 — adaptateurs Cargo, TS, Python, C/C++, scheduler, cache, impact, JSONL | **GO après B2**, un adaptateur à la fois | chaque adaptateur ajoute des surface de confiance |
| C3 | RF-021 | R6 — publication forge | **DIFFÉRER** | écrit sur une forge ; à ne pas ouvrir sans besoin |
| C4 | RF-022 | S9 | **DIFFÉRER** | dépend de B2 |
| C5 | RF-023→025 | R7 — VFS, sync distribuée, CAS distant | **DIFFÉRER** | périmètre recherche, pas production |
| C6 | RF-026 | R6 — adaptateur Ravum opt-in | **DIFFÉRER** | dépend de B1 + B3 |
| C7 | G6 ×2 | ratification des gates candidates | **GO** (ratifier, sans fixer de seuils par agent) | sinon les gates restent non ratifiées |
| C8 | G7 ×2 | builds jetables + épinglage d'actions | **GO** | c'est la seule façon de prouver la reproductibilité et de fermer N1/N2 |

---

## 5. Famille D — les 15 cartes `[US-0xx][STORY]`

Ce ne sont pas des tâches : ce sont des **critères d'acceptation produit** issus de l'import
(`US-001` … `US-015`), écrits à l'infinitif (« Obtenir un reçu… », « Travailler sans worktree… »). Une
carte qui n'a ni périmètre, ni preuve attendue, ni propriétaire ne peut pas être « faite ».

- **Option 1 — adopter comme checklist d'acceptation** : chaque `US-xxx` devient un critère vérifié par
  les tests/preuves existants ou manquants, et le tableau cesse d'avoir 15 cartes mortes.
  **Recommandé.**
- **Option 2 — archiver** : on perd la traçabilité de l'import.
- **Réponse attendue** : `D1 = adopter` (et je produis la matrice US ↔ preuve en une passe).

---

## 6. Ce que je ne peux pas décider, même avec votre accord

- **La seconde revue indépendante (G10)** : elle exige une personne ou un agent **hors équipe**. Aucune
  réponse de ma part ne ferme cette porte.
- **Les droits Ravum (B1)** : ce n'est pas une décision technique ; il faut les ayants droit.
- **La ratification *a posteriori* du GO (A3)** : c'est un acte de gouvernance, pas une correction.
- **La fusion, le tag, la release** : votre acte, jamais le mien.

## 7. Réponse minimale

```
A1 oui ; A2 oui ; A3 oui (b+c) ; A4 oui ; A5 oui ; A6 non ; A7 oui ; A8 oui ; A9 oui ; A10 oui ; A11 oui ; A12 oui
B1 option 2 ; B2 option 3 ; B3 oui ; B4 option 1 ; B5 option 2
C1 différer ; C2 GO après B2 ; C3 différer ; C4 différer ; C5 différer ; C6 différer ; C7 GO ; C8 GO
D1 adopter
```

Avec cette réponse, les 8 cartes RF en `todo` deviennent exécutables et il ne reste plus que des
décisions produit à instruire. Sans elle, le travail d'ingénierie est fait mais la file d'acceptation
reste le goulot.
