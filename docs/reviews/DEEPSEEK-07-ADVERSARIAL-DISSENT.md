# DEEPSEEK-07 — DISSENSUS ADVERSARIAL ET DÉSACCORDS RÉELS

**Statut :** `RAW-OPINIONS / CONSOLIDATED-WITHOUT-ERASURE`. Ce document ne fabrique pas de consensus. Il enregistre les désaccords, dit lesquels sont tranchés, lesquels restent ouverts, et pourquoi.
**Règle :** un désaccord n'est « résolu » que si une **preuve** le tranche ; sinon il reste ouvert, même s'il est inconfortable.

---

## 1. Provenance et indépendance — ce que cette revue peut et ne peut pas prétendre

| Fait | Conséquence |
|---|---|
| **8 sous-agents** lancés, **7 rapports** produits (A, B, C, D, F, G, H) + **1 remplacement** (E, après échec du premier essai) | 8 lectures disjointes, 1 seul échec sans production |
| **Tous du même fournisseur/modèle** (`deepseek-v4.1-flash`, contextes séparés) | L'indépendance est **procédurale** (contextes disjoints, périmètres disjoints, aucune lecture croisée), **pas épistémique**. Ce n'est **pas** un panel d'experts indépendants. |
| Aucun rapport n'a été lu par un autre agent avant rédaction (consigne explicite à G et H ; les autres n'avaient pas accès au dossier) | Pas de contamination croisée, mais **corrélation de famille de modèle** possible : les mêmes biais peuvent se répéter. |
| 4 agents (A, B, D, Lead) ont lu **le même code** au même SHA, séparément | Triangulation réelle sur les P0 : quatre lectures concordantes valent mieux qu'une. |
| Aucun doublon textuel de type Qwen/Gemini | La faute méthodologique documentée dans la V4 n'est **pas** reproduite ici. |
| L'agent E initial a échoué sans livrer ; le remplacement a produit un rapport distinct | Ne pas compter l'échec comme un avis ; ne pas compter le remplacement comme un second avis indépendant. |

**Conclusion de provenance :** la revue vaut par la **traçabilité** (SHA, fichiers, lignes, URLs datées) et la **triangulation du code**, pas par un vote. Toute phrase du type « les agents confirment » serait abusive ; les formulations retenues sont « quatre lectures concordantes », « un agent conteste », etc.

---

## 2. Corrections que les agents ont infligées au Lead (et qui sont retenues)

Un audit qui ne corrige pas son propre Lead n'est pas un audit. Quatre corrections factuelles :

| # | Erreur du Lead | Correction | Effet |
|---|---|---|---|
| 1 | Baseline « 80 issues, 0 PR » côté Ravum | **90 issues, 1 PR (#101 brouillon)** ; l'erreur venait d'un `per_page=80` qui plafonnait le comptage | `DEEPSEEK-00` corrigé ; toute prose non datée sur les compteurs est proscrite |
| 2 | « Le défaut d'ordre des hooks est un écart assumé » | **Non** : l'en-tête de `git_writer.rs` revendique « in Git's order » (`:37-38`) puis liste un autre ordre (`:41-43`) → **auto-contradiction**, pas écart assumé | `DEEPSEEK-00` §3 corrigé ; c'est plus grave qu'un écart documenté |
| 3 | « Aucune protection de branche » (généralisation) | Faux pour Ravum : `dev` **est protégée** (check `mvp`) ; vrai pour Flow ; une lecture non authentifiée renvoie 401, une lecture authentifiée 404 | `DEEPSEEK-00` §2 corrigé |
| 4 | Brief de mission écrivant `ADR-001/005/006/009/010` sans préfixe | Ce sont des **FLOW-ADR**, pas des RAVUM-ADR ; D a refusé la prémisse | `DEEPSEEK-02` §1 ; règle de préfixe obligatoire |

**Ce que cela dit du processus :** les agents ont fait leur travail de contradicteurs. Les corrections sont conservées **dans** le dossier, pas effacées.

---

## 3. Matrice des désaccords

Statut : **T** = tranché par preuve · **O** = ouvert · **R** = reformulé (les deux positions survivent).

| # | Sujet | Position 1 | Position 2 | Statut | Ce qui trancherait |
|---|---|---|---|---|---|
| D1 | **O0 suffit-il ?** | **A** : oui pour la tranche Git/concurrence ; il ne manque que politique + reçu | **H** : non, mais O0 + journal de 10 jours est le vrai test ; **G** : un portail P2 est utile même à 1 agent ; **V6** : O2/O3 restent candidats | **O** | S9-baseline + test de demande |
| D2 | **S8 est-il une preuve ?** | **H** : à refuser entièrement, même en `MEASURED_HISTORICAL` | **E** : utilisable comme ordre de grandeur, jamais comme classement ; **G** : utilisable pour attaquer INV-01 ; **V6** : « pas une supériorité universelle » | **T** (partiel) | l'artefact lui-même : n=3, plages chevauchantes, rapport de médianes → **ni classement ni preuve de sûreté**. H est retenu sur le fond, E sur l'usage borné. |
| D3 | **jj comme O1** | **F** : excellent VCS, mauvais gate ; utilisable comme outil utilisateur | **A** : **disqualifié** comme O1 pour tout projet à hooks (la gate est perdue) | **R** | la liste des projets cibles : si aucun hook requis, F a raison ; sinon A |
| D4 | **Scion « Anthropic »** | **F** : aucune source primaire retrouvée → erreur de source | **V6** : deux Scion distincts (Google Apache-2.0 et un Scion interne Anthropic du 13/10/2026) | **O** | une source primaire Anthropic. En attendant : **ne pas citer**. |
| D5 | **INV-01 (« dossier canonique unique »)** | **V6** : invariant `PROPOSED` (avec amendement INV-01-A) | **G** : à rétrograder en option, car S8 pénalise ce mode en médiane | **O** | S9-baseline comparant B (worktrees) vs C (partagé) — la médiane S8 suggère que G a un point, mais les plages se chevauchent |
| D6 | **Hooks dans le MVP** | **G** : couper les hooks est le trim le plus risqué ; c'est la douleur quotidienne | **B** : un hook est du code arbitraire, jamais « sûr » ; **A** : détecter/refuser suffit au MVP | **R** | H3 : sans sandbox, seul le refus est honnête. Détection/refus au MVP ; isolation conditionnelle. |
| D7 | **Sandbox Windows** | **V6** : « à évaluer, ne pas annoncer comme prêt » | **B** : aucune primitive non-VM/non-admin => `UNSUPPORTED` ; **H** : O3 indisponible sur l'OS de référence | **T** (pour l'état actuel) | un test AppContainer reproductible |
| D8 | **Seuil de gain** | **V6/archives** : 15 %, 20 %, 25 % selon les passages | **G** : exprimer en heures humaines/an ; **E** : trois candidats + plancher absolu ; **H** : amortissement à ~12 tâches/jour | **O** | H4 |
| D9 | **Phase documentaire** | **V6** : « pas de sixième contre-expertise » | **H** : le GOAL en lance une septième le même jour → phase sans règle d'arrêt ; **D** : il ne manque que deux actes humains (H2, H4) | **R** | H8 proposé : geler après 4 corrections + aucune revue sans fait nouveau |
| D10 | **Nombre d'ADR hors périmètre** | **D** : 18 `N` (tableau) / 19 (phrase) — incohérence interne | **G** : sortir les stories qui citent des ADR ravumiens (question différente) | **T** | recalcul : 13 D / 11 I / 18 N ; l'erreur de D est une coquille, corrigée dans `DEEPSEEK-02` |
| D11 | **Ravum a-t-il déjà le noyau ?** | **H** : oui (PR #85/#87, CI d'impact, dev protégée) → trilemme d'ownership | **D** : le coordinateur existe, mais rien ne prouve qu'il soit utilisable hors Ravum | **R** | extraction/CLI testée hors Ravum, sous licence — **pas fait** |
| D12 | **Faut-il un `SourceSnapshotId` ?** | **V6** : identité interne distincte du Git tree OID | **G** : inutile hors adaptateur Ravum ; deux identités = bugs de comparaison | **R** (pour le MVP) | retenu : une seule identité (Git OID) au MVP |
| D13 | **L'ordonnanceur de builds** | **V6** : Flow le prévoit (R5) | **C** : non, Flow possède le graphe d'obligations, pas l'ordonnanceur | **T** (pour le MVP) | S9 pourrait inverser ; pas avant |

---

## 4. Les trois désaccords qui comptent vraiment

### 4.1 « Faut-il construire ? » (A/G/H)

Trois positions irréductibles, toutes argumentées :

- **A** : « O0 suffit pour Git ; seul le reçu manque. » → construire **une couche de politique**, pas un moteur.
- **G** : « Le cas majoritaire (1 agent) rend Flow inutile ; repositionne-le en portail de publication. » → construire **petit**, utile même à 1 agent.
- **H** : « La demande n'est pas prouvée ; 500–700 h pour un composant qui existe déjà à côté. » → **ne pas construire** avant un journal O0 de 10 jours.

**Pourquoi ce n'est pas tranchable ici :** aucune mesure de demande n'existe, et les trois positions reposent sur la même absence. **Décision :** ne pas trancher ; exiger **le test le moins coûteux** (journal O0 + S9-baseline). C'est la seule position commune.

### 4.2 « Le noyau est-il déjà dans Ravum ? » (H vs D)

- **H** apporte des faits vérifiables : PR #85 (81 744 lignes, 329 fichiers, publication transactionnelle, 15 tests de crash), PR #87 (overlays), `mvp.yml` + `mvp-impact.yml`, `dev` protégée.
- **D** objecte que le **couplage** au langage et aux crates n'est pas mesuré : « il existe » ≠ « il est réutilisable hors Ravum ».

**Les deux ont raison.** Le fait est établi ; la conséquence (dupliquer/dépendre/réécrire) est un **trilemme non résolu**, pas une conclusion. Ce qui le trancherait : une CLI minimale du coordinateur Ravum utilisée sur un dépôt non-Ravum, sous licence — c'est un R0 de 2–3 jours, pas une décision d'architecture.

### 4.3 « Le produit est-il pour le solo ou pour l'équipe ? » (G vs corpus)

- Le corpus dit « solo d'abord, équipes ensuite ».
- G montre que **8/15 stories** visent des rôles d'organisation (Responsable CI, Auditeur, Administrateur) et que le cas majoritaire (1 agent) rend Flow inutile.
- **H** ajoute qu'aucun second utilisateur n'existe.

**Conséquences non résolues :** H7 (interne vs produit). La segmentation correcte est le **nombre d'agents concurrents**, pas le nombre d'humains — proposition de G retenue comme hypothèse, non validée.

---

## 5. Ce que chaque rapport ne peut pas prétendre

| Agent | Ne peut pas prétendre | Raison |
|---|---|---|
| A | que `git commit` inclut le staging d'un `pre-commit` (hypothèse de `R-A1`) | inféré des docs, **non reproduit** |
| B | qu'aucune fuite n'existe | zéro test ; tout est `CODE_READ` |
| C | qu'un graphe d'impact est fiable | aucun corpus ; il recommande justement de ne pas le construire |
| D | que les 42 ADR sont fidèles à l'octet | 39/42 lus via V6, pas comparés à l'upstream |
| E | qu'un seuil est le bon | aucun run ; les seuils sont des candidats |
| F | que Scion Anthropic n'existe pas | absence de preuve ≠ preuve d'absence |
| G | que la demande est faible | aucun accès au marché |
| H | que Flow échouera | arguments économiques, pas de mesure |

---

## 6. Divergences de forme à corriger (non résolues par cette revue)

| ID | Problème | Correctif proposé |
|---|---|---|
| `X-05` | état machine de `ARCHITECTURE.md` périmé | aligner sur V6 §4.1 ou marquer `SUPERSEDED` |
| `X-06` | lettres S9 contradictoires | adopter la nomenclature de `DEEPSEEK-04` §3 |
| `X-07` | double numérotation des tests | adopter les IDs V6 canoniques + table d'alias |
| `X-11` | collision `ADR-` 000–011 | préfixes obligatoires |

Ces quatre corrections sont **les seules** que ce dossier recommande d'appliquer immédiatement : elles ne changent aucune décision, seulement la lisibilité, et elles coûtent une heure.

---

## 7. Dissensus maintenu (à ne pas lisser dans la synthèse)

Sont **volontairement laissés ouverts** :

1. O0 vs O2 vs P2 (D1) — dépend de S9 et de la demande.
2. INV-01 comme invariant ou option (D5).
3. Scion Anthropic (D4).
4. Seuil H4 (D8).
5. Ampleur de la phase documentaire (D9).
6. Réutilisabilité du coordinateur Ravum (D11).
7. Interne vs produit (H7).

**Aucun de ces sept points ne doit être présenté comme « tranché » dans un résumé exécutif.** Un résumé qui les masque serait plus dangereux qu'un dossier long.

---

*Sources : rapports bruts A–H (`docs/reviews/raw/`), `DEEPSEEK-00..06`, artefacts GitHub cités. Aucun désaccord n'a été supprimé ; les corrections au Lead sont conservées.*
