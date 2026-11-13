# DEEPSEEK — GOAL D'EXÉCUTION PROPOSÉ (BROUILLON, NON LANCÉ)

> **STATUT : `DRAFT-ONLY / NOT LAUNCHED / NOT AUTHORIZED`.**
> Ce document est une **proposition** de mandat pour la session suivante. Il ne doit être ni exécuté, ni copié dans une nouvelle session, ni présenté comme approuvé tant que l'humain n'a pas coché H2, H4 et H5.
> **Ce GOAL n'autorise aucun code Flow, aucune modification de Ravum, aucune CI, aucun benchmark hors fixtures jetables.**

---

## 1. Préconditions bloquantes (à vérifier avant d'ouvrir la session)

| Condition | Si absente |
|---|---|
| **H2** — décision écrite sur licence/droits (Flow + Ravum) | **STOP** : aucune extraction, aucune publication |
| **H4** — seuil δ, marge d'équivalence `δ_eq`, plancher absolu, budget | **STOP** : le protocole ne peut pas être pré-enregistré |
| **H5** — `GO_R0_FIXTURES` signé avec périmètre et budget | **STOP** : aucune exécution |
| **H3** — position écrite (même « non tranchée, fail-closed ») | **STOP** pour tout lot touchant hooks/sandbox |
| SHA Ravum re-épinglé et blob SHAs re-vérifiés à l'ouverture | **STOP** : les constats P0 ne sont plus localisés |

Aucune de ces conditions n'est remplie à la date de rédaction.

---

## 2. Objet du GOAL (si et seulement si les préconditions sont remplies)

Établir, **sur fixtures synthétiques jetables uniquement**, (a) la réalité des chemins de risque P0, (b) le coût réel des stratégies de travail concurrent (copies, vrais worktrees, dossier partagé, séquentiel, O0), et (c) si un écart justifie la construction d'un noyau Flow. **Aucune ligne de code Flow n'est écrite dans ce GOAL.**

Le résultat attendu est une **décision informée** : construire un noyau minimal, réduire à un portail, ou arrêter.

---

## 3. Périmètre autorisé / interdit

| Autorisé | Interdit |
|---|---|
| Fixtures synthétiques jetables (`%TEMP%` ou répertoire dédié), secrets **fictifs** | Le dépôt Ravum actif, son clone local, le workspace utilisateur, tout dépôt réel non copié |
| Reproduction des tests P0 sur fixtures | Correctifs dans Ravum, PR, branche, commit côté Ravum |
| Harnais de mesure neutre (aucun code Flow) | Écrire du code produit Flow (R1+) |
| S9-baseline A/B/C/D/E (+F si disponible), `n = 8` exploratoire | Campagnes confirmatoires, publication, CI Flow |
| Mesures CPU/RAM/disque/temps | Compilations lourdes concurrentes de la session Ravum |
| Rapports, JSON, logs, SHA | Toute conclusion présentée comme `PROVED` ou comme garantie de sûreté |

**Ressources :** 16 Go, une seule campagne à la fois, une seule compilation lourde à la fois. Budget : celui fixé par H4.

---

## 4. Lots de travail

### R0-a — Harnais et fixtures (≈ 1–2 jours)

**Livrables :** dépôt de fixtures (Rust multi-crate minimal ; un cas TS/Python si budget) avec SHA épinglé ; script de mesure neutre (horloge monotone, `T_cap`, capture CPU/RSS/disque) ; checklist de pré-enregistrement remplie et **hachée**.

**Critère de sortie :** le harnais produit un JSON reproductible sur 2 runs consécutifs à vide ; aucune dépendance à Flow.

### R0-b — Reproduction P0 (≈ 2–3 jours)

Pour chaque test, produire `REPRODUCED | DISPROVED | NOT_REPRODUCED | BLOCKED` avec commande exacte, SHA, logs et limite :

| Test | Attendu en cas de reproduction |
|---|---|
| `TREE-01` | un dirty hors patch rend le build vert, le tree publié rouge |
| `HOOK-02a` | le staging d'un `pre-commit` est absent de `commit^{tree}` |
| `HOOK-02b` | un `pre-commit` modifie un fichier dirty du workspace réel |
| `HOOK-02c` | l'ordre observé est `prepare-commit-msg → pre-commit → commit-msg` |
| `ENV-04a/b/c` | canari visible dans le hook / test ; fuite éventuelle par les logs |
| `FENCE-07a/b/c/d` | comportement de deux writers, d'un `git` externe, crash entre CAS et outcome |
| `FILE-09` | perte de `100755` sur un chemin neuf ; refus non-UTF-8 |
| `PATH-08` | écriture hors workspace via parent symlink/jonction ; noms réservés |

**Règle :** un test `NOT_REPRODUCED` n'infirme **pas** le constat de lecture ; il indique que la conséquence n'est pas déclenchée par la fixture.

### R0-c — S9-baseline (≈ 2–3 jours)

Comparateurs **A** copies, **B** vrais `git worktree` (add/remove chronométrés), **C** partagé + garde, **D** séquentiel, **E** O0, **F** outil existant si honnêtement comparable. Pas de **G** (Flow n'existe pas).
Workloads : W0–W16 de [`DEEPSEEK-04`](DEEPSEEK-04-S9-PROTOCOL.md) §4, sous budget.
**Sorties :** `σ_d`, ratio médian apparié + IC bootstrap, `T_coord/T_total`, coût worktree calibré (`t_wt`, `t_rm`, `v_copy`, `v_fs`, `t_av`), taux d'échec, zéro condition éliminatoire ou disqualification.

**Critère de sortie :** le rapport permet de répondre **oui/non** à « un coordinateur peut-il produire δ sur au moins une classe ? » Sans G, la réponse est **indirecte** : elle borne `T_coord/T_total`.

---

## 5. Format de preuve exigé

Chaque affirmation : `CODE_READ | SOURCE_READ | MEASURED | MEASURED_HISTORICAL | PROPOSED | NOT_REPRODUCED | UNKNOWN | NOT_REVIEWED`.
Chaque mesure : machine, OS, toolchains, SHA fixture, commande, `n`, ordre, charge, limites. **Aucun chiffre sans son artefact.** Aucun agrégat sans les runs individuels.

---

## 6. Conditions d'arrêt (le GOAL s'arrête et rend compte)

1. Une condition éliminatoire est observée (perte silencieuse, arbre non qualifié publié, secret fictif fuité, E2E masqué, `UNKNOWN` promu) → arrêt du bras et rapport immédiat.
2. Le budget H4 est atteint.
3. `n` confirmatoire est inatteignable → rapport « non démontrable », sans conclure.
4. Une précondition (§1) manque ou change.
5. La session Ravum signale une gêne de ressources → suspension.
6. Le harnais ne peut pas mesurer `T_verified` sans ambiguïté → arrêt, pas de substitut.

---

## 7. Ce que ce GOAL ne fait pas

- Il **n'implémente pas** Flow (pas de R1).
- Il **ne corrige pas** Ravum et **n'émet aucun patch**.
- Il **ne prouve aucune sécurité** : les tests P0 sont des reproductions de chemins, pas des preuves.
- Il **ne tranche pas** H1, H2, H3, H6, H7.
- Il **ne mesure pas** la demande utilisateur (test G, une semaine, séparé et quasi gratuit).

---

## 8. Sortie attendue (un seul dossier)

1. `R0-BASELINE.md` — harnais, fixtures, SHA, pré-enregistrement haché.
2. `R0-P0-REPRO.md` — tableau des huit tests avec statut et preuves.
3. `R0-S9-BASELINE.md` — résultats, `σ_d`, `T_coord`, coût worktree, limites.
4. `R0-DECISION-MEMO.md` — **≤ 2 pages** : construire / réduire / arrêter, avec les chiffres et les incertitudes.
5. Artefacts bruts (JSON, logs) et scripts.

---

## 9. Bloc à copier dans une future session — **UNIQUEMENT après signature H2/H4/H5**

```text
GOAL — R0 FIXTURES (Ravum Flow), borné.
Contexte : Ravum-flow@dev, dossier DEEPSEEK-00..07. Ravum = LECTURE SEULE (SHA à re-épingler).
Autorisé : fixtures synthétiques jetables, secrets fictifs, reproduction P0, S9-baseline A/B/C/D/E (+F),
          une campagne à la fois, budget H4.
Interdit : code Flow, modification Ravum, CI Flow, publication, licence, PR, benchmark sur dépôt réel.
Livrables : R0-BASELINE.md, R0-P0-REPRO.md, R0-S9-BASELINE.md, R0-DECISION-MEMO.md + artefacts.
Règle : MEASURED ou NOT_REPRODUCED, jamais PASSED par défaut ; arrêt sur condition éliminatoire.
Sortie : construire / réduire / arrêter. Ne pas lancer R1.
```

---

*Sources : `DEEPSEEK-04` (protocole), `DEEPSEEK-06` (préconditions), `docs/DEEPSEEK_GOAL.md` (format du mandat). Ce document n'est pas une autorisation.*
