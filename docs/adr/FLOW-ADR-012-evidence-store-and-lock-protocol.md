# FLOW-ADR-012 — Store d'évidence transactionnel, verrou à identité d'hôte et contexte versionné

**Status: PROPOSED.** 2026-11-15 · Ouvre D2 (contrat de persistance) · Décisions humaines ouvertes
`H1`, `H3`, `H4`, `H8` · Ne ratifie rien.

## Context

Le contre-audit Fable du 2026-11-15 (SHA `1952325c`) a établi un P0 (`FBL-001`) : deux écrivains
partageant `(agent, task, base, key)` pouvaient tous deux être `ADMITTED` avec des patches différents,
laissant un `.patch` d'un écrivain sous les métadonnées d'un autre. `assemble` appliquait ces octets
sans les re-hacher. Le même audit a établi que le vol d'un verrou mort est un TOCTOU, que les logs de
`run` ne sont pas isolés par exécution, que le contexte de build ignore les valeurs d'environnement et
l'identité des programmes, et que le décodeur de politique accepte des coercitions silencieuses.

La vague de hardening du 2026-11-15 a corrigé ces défauts sur la branche
`feat/flow-production-hardening-fable`. Cet ADR propose de **ratifier les contrats de persistance**
que le code applique désormais. Il ne ratifie ni l'option d'architecture (`H1`), ni la politique de
sandbox (`H3`), ni les seuils S9 (`H4`), ni la gouvernance de release.

## Candidate decision — not ratified

1. **Blobs contenu-adressés.** Les octets d'un patch sont stockés sous
   `.flow/blobs/<2>/<sha256>` ; l'enregistrement de proposition est publié **une seule fois** par
   `create_once` (fichier temporaire fsynced + `hard_link`, échec si le nom existe). Deux écrivains
   d'une même clé produisent exactement un gagnant ; le perdant relit et rend `IDEMPOTENCY_CONFLICT`
   ou `idempotent:true` pour des octets identiques. La métadonnée n'est jamais publiée avant l'objet
   qu'elle référence.
2. **Identités recalculées.** `Proposal::load` recalcule `proposal_id` (agent/task/base/key) et
   re-hache le patch ; `Candidate::load` recalcule `candidate_id` (base/tree/liste ordonnée) ; les
   enregistrements portent un sceau d'intégrité. `assemble` reconstruit chaque arbre et chaque liste de
   changements avant la détection de conflits. Toute divergences → `PROPOSAL_INTEGRITY` /
   `CANDIDATE_INTEGRITY`, jamais une réinterprétation.
3. **Verrou local à identité d'hôte.** Le record `flow-lock/2` porte un jeton d'hôte ; un record
   étranger ou sans jeton est `LOCK_UNKNOWN`, jamais volé. Le vol d'un verrou mort est sérialisé par
   un marqueur par génération (`lock.steal.<nonce>`) : seul son créateur peut supprimer le verrou
   observé. Ce n'est **pas** une garantie multi-hôte ; `.flow` partagé entre machines est hors support.
4. **Évidence par exécution.** Chaque `run` a un `run_id` ; logs sous `logs/<tree>/<run_id>/` ; le reçu
   scellé porte `run_id` et le sha256 de chaque log ; un marqueur `.flow/runs/<run_id>.active` est
   supprimé à toute sortie. `explain` vérifie les digests (`LOG_INTEGRITY_INVALID`). `gc` prend le
   verrou publication et refuse `RUN_ACTIVE`.
5. **Contexte `flow-context/3`.** Le contexte hache les **valeurs** des variables allowlistées (sous
   forme de digests, jamais en clair), le chemin résolu et les octets de chaque programme d'obligation,
   et les `context_probes` déclarées. Un changement d'environnement, de binaire ou de toolchain entre
   `run` et `publish` donne `CONTEXT_MISMATCH`.
6. **Politique stricte.** `Config::from_json` refuse clés inconnues, types inexacts, `timeout_s` hors
   `[1, 86400]`, `execution_boundary` non supportée et `context_probes` malformées (`INVALID_POLICY`).
7. **Attribution candidat ↔ reçu.** `publish --candidate` exige que le reçu nomme ce candidat
   (`RECEIPT_CANDIDATE_MISMATCH`) ; `receipt_candidate_id` est exposé dans le plan, la sortie et le
   journal.

## Alternatives to compare (including O0)

- **O0 — ne rien changer** : le P0 reste ouvert, le multi-agents local n'est pas qualifiable.
- **Un seul fichier par proposition** (patch + métadonnées dans un conteneur) : simple, mais rend la
  lecture partielle et la migration des anciens enregistrements plus coûteuses ; la séparation
  blob/record garde les octets vérifiables indépendamment.
- **Verrou OS (`flock`/`LockFileEx`)** : exclusion plus directe, mais sémantique de disparition au
  crash et portabilité Windows à prouver ; le marqueur de vol est testable et refuse
  conservativement. Un verrou OS reste une évolution possible.
- **Rename atomique du verrou mort** : ne fournit pas de compare-and-delete portable (écrasement par
  renommage variable selon l'OS) ; rejeté.
- **Segmentation du journal** : différée (P3, `FBL-012`) ; la mesure locale donne ~9 ms/append,
  dominé par fsync, jusqu'à 2 000 enregistrements. Une compaction exige un protocole de reprise
  distinct.

## Consequences and tradeoffs

- Le store grandit (blobs orphelins possibles après un crash entre `put_blob` et `create_once`) ;
  `gc` ne les collecte **pas** pour ne pas courir avec un `propose` concurrent. Coût borné, documenté.
- `assemble` double approximativement le nombre d'applications Git (re-dérivation par proposition) :
  coût accepté au nom de la fidélité de l'attribution.
- Les reçus écrits avant `flow-context/3` sont refusés à la publication (`CONTEXT_MISMATCH`) : c'est
  une invalidation volontaire, pas une perte de données ; les propositions legacy restent lisibles.
- Le verrou reste mono-hôte ; `.flow` partagé est refusé, pas sécurisé.

## Evidence required before human acceptance

- Tests de la branche : G1 (15 tours × 8 processus), G2 (runs concurrents), G3 (8 publishers sur
  verrou mort), G4 (changement d'environnement), G6 (matrice de refus), M11 (attribution candidat),
  GC `RUN_ACTIVE`, `PROPOSAL_INTEGRITY`/`CANDIDATE_INTEGRITY`/`LOG_INTEGRITY_INVALID`.
- CI Linux/Windows/macOS verte sur le SHA final, avec les run ids.
- Revue indépendante finale (`G10`) hors équipe.
- Décisions humaines : modèle de menace local-trusted vs hostile, support multi-hôte, sémantique
  « reçu = arbre » vs « reçu = candidat », ensemble de sondes de toolchain, gouvernance de release.

## Human sign-off

Decision: **PENDING** · Actor/date/evidence: **PENDING** · Reopen on adverse data, a change of the
threat model, or a platform that cannot honour `hard_link`/atomic create semantics.
