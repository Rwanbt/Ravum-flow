# O3 — propositions d'architecture complète (2026-11-15)

**Statut : propositions, non décidées, non implémentées.** `H1` a retenu **O3 complet** comme cible
(voir [DECISION_REGISTER](../DECISION_REGISTER.md)). La branche livrée reste le sous-ensemble
**O1 + O2** (CLI locale, arbre vérifié, publication CAS) : aucune des options ci-dessous n'existe dans
le code, aucune n'est budgétée, et aucune ne doit être présentée comme livrée.

Chaque proposition indique ce qu'elle ajoute, ses avantages, ses inconvénients, son coût et ce qu'elle
exigerait avant d'être autorisée. Le choix n'est pas technique seulement : il change le modèle de
menace et l'exploitation.

---

## O3-A — Orchestrateur local mono-hôte (`flowd`)

Un démon local (ou un mode « coordinateur » du binaire) qui possède la file des propositions, les
baux d'exécution, les worktrees et les reprises ; les agents lui parlent en JSON local ; `run`/
`publish` restent les primitives de vérification/publication.

| | |
|---|---|
| **Avantages** | Réutilise le store, le verrou et les reçus actuels ; pas de surface réseau ; testable sur une machine et dans la CI existante ; résout réellement l'orchestration multi-agents sur un poste (files, priorités, reprise après crash) ; incrémental (chaque étape est utilisable seule). |
| **Inconvénients** | Nouveau cycle de vie de démon (démarrage, arrêt, mise à jour, orphelins, Windows service) ; sémantique de bail à concevoir (le verrou actuel est mono-écrivain court, pas un bail long) ; surface d'attaque locale accrue ; duplicate partiellement les runtimes d'agents ; reste mono-hôte. |
| **Coût / risque** | Élevé (3–6 semaines d'ingénierie) ; risque de concurrence et de reprise après crash ; risque produit : un démon local est plus lourd à opérer qu'une CLI. |
| **Prérequis avant GO** | ADR d'orchestration (baux, epochs, reprise, annulation) ; modèle de menace local révisé ; tests de crash/restart ; budget explicite ; décision sur le mode « bibliothèque » vs démon. |

## O3-B — Serveur de coordination réseau (multi-hôte)

Service central (HTTP/JSON-RPC) : baux distribués, état des propositions, agents sur plusieurs
machines, reçus signés, intégration forge/CI.

| | |
|---|---|
| **Avantages** | Vraie collaboration multi-hôte et multi-utilisateur ; centralise l'audit et les politiques ; peut servir d'API à des orchestrateurs existants ; correspond à l'ambition « O3 complet » au sens produit. |
| **Inconvénients** | Coût le plus élevé : authentification/autorisation, TLS, baux avec epochs, cohérence, migration de schémas, signatures, exploitation, sauvegardes ; contredit le modèle actuel `local-trusted` mono-hôte (FBL-008 : l'identité d'hôte est un hachage de nom, insuffisant) ; nécessite une revue sécurité dédiée et une infrastructure d'hébergement ; dépendance opérationnelle forte. |
| **Coût / risque** | Très élevé (plusieurs mois) ; risque sécurité majeur ; risque de sur-ingénierie si le besoin réel est mono-poste. |
| **Prérequis avant GO** | Modèle de menace hostile documenté ; protocole de bail/epoch et preuve d'exclusion ; signatures des reçus ; plan d'exploitation (SLA, sauvegardes, rotation de clés) ; budget et équipe ; décision explicite sur l'hébergement. |

## O3-C — Fédération asynchrone par Git/forge (sans serveur)

Les agents poussent des branches de propositions ; un « assembleur » (CI ou poste dédié) valide,
assemble, vérifie et publie ; la coordination passe par le remote Git et les API de forge.

| | |
|---|---|
| **Avantages** | Pas de nouveau verrou distribué : Git/forge fournissent authentification, audit et disponibilité ; fonctionne entre machines et organisations ; latence acceptable pour du travail asynchrone ; le moins d'exploitation à porter ; aligné sur « Git natif ». |
| **Inconvénients** | Latence (push + CI) incompatible avec la coordination fine ; dépendance à la forge (permissions, quotas, pannes) ; la confiance se déplace vers la CI et ses secrets ; les reçus non signés restent falsifiables par qui peut écrire la CI ; résolution de conflits asynchrone plus lente ; exige des droits d'écriture sur le dépôt (non couverts par le GO actuel). |
| **Coût / risque** | Moyen (2–4 semaines pour un assembleur CI minimal) ; risque de sécurité CI (secrets, code non fiable) ; risque de saturation des runners. |
| **Prérequis avant GO** | Un dépôt/forge autorisé explicitement (distinct du GO actuel, qui interdit les écritures Ravum) ; conception des secrets CI ; signatures ou isolation de l'assembleur ; politique de branches ; budget CI. |

## O3-D — Bibliothèque/protocole intégré (orchestration déléguée)

Flow reste un vérificateur/publicateur et expose un contrat (crate Rust + JSON) que Ravum, un CI ou un
orchestrateur existant intègre ; l'orchestration n'est pas un produit Flow.

| | |
|---|---|
| **Avantages** | Plus petite surface produit ; pas de démon ni de serveur ; réutilisable par plusieurs hôtes (Ravum, CI, agents) ; préserve le modèle `local-trusted` et les garanties actuelles ; coût le plus faible et risque le plus contenu ; peut se livrer par étapes (API stable, adaptateurs). |
| **Inconvénients** | N'est pas un produit « complet » de bout en bout ; l'expérience multi-agents dépend de l'intégrateur ; pas de scheduler ni de file intégrés ; adoption conditionnée à un tiers ; peut ne pas correspondre à l'intention « O3 complet » du propriétaire. |
| **Coût / risque** | Faible à moyen (1–2 semaines pour stabiliser l'API et un adaptateur) ; risque d'API prématurée ; risque d'éparpillement de la responsabilité d'orchestration. |
| **Prérequis avant GO** | Décision sur le propriétaire de l'orchestration ; versionnement du contrat (semver) ; tests d'intégration avec au moins un hôte réel (CI ou Ravum, en lecture seule côté Ravum) ; ADR d'interface. |

---

## Comparaison

| Critère | O3-A local | O3-B réseau | O3-C fédéré Git | O3-D bibliothèque |
|---|---|---|---|---|
| Couvre le multi-hôte | non | **oui** | oui (asynchrone) | selon l'hôte |
| Nouveau démon/service | oui | oui (serveur) | assembleur | non |
| Coût d'ingénierie | élevé | très élevé | moyen | faible–moyen |
| Risque sécurité | moyen (local) | **élevé** | moyen (CI/secrets) | faible |
| Réutilise le store/verrou actuels | oui | partiellement | oui | oui |
| Compatible avec `H3` fail-closed | oui | à prouver | à prouver | oui |
| Time-to-first-value | moyen | long | moyen | **court** |
| Change le modèle de menace | oui (démon) | **oui (réseau)** | oui (CI) | non |

## Recommandation (à trancher par le propriétaire)

Séquence minimisant le risque et maximisant la valeur :

1. **O3-D d'abord** : stabiliser le contrat (API/JSON) et un adaptateur CI, sans nouveau démon. Cela
   rend Flow utilisable dans une orchestration existante et prépare O3-A/C sans dette.
2. **O3-A ensuite** si le besoin est mono-poste : orchestrateur local, avec ADR de baux/epochs et
   tests de crash. C'est le prolongement naturel du store actuel.
3. **O3-C seulement si** la collaboration inter-machines devient un besoin réel et qu'un dépôt/forge
   est explicitement autorisé.
4. **O3-B en dernier** (ou jamais) : uniquement si le multi-hôte synchronisé est une exigence dure,
   avec une revue sécurité et un plan d'exploitation complets.

Aucune de ces options ne peut être déclarée « livrée » avant son ADR, son budget, son modèle de
menace et ses tests. Le S9 gelé ([BENCHMARK_S9.md](../BENCHMARK_S9.md)) mesure G vs E pour la ligne
actuelle ; une option O3 qui change les arms exigera une nouvelle version du protocole.
