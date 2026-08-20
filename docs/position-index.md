# Bermuda Position and Pattern Index Design

**Status:** Draft 1
**Target milestone:** Version 0.4

## 1. Purpose

The position index exists to support fast searches across the professional game database.

The main user-facing goal is:

> Find every professional game in which a selected board pattern occurred.

Search results must identify:

* game ID;
* move number;
* board orientation;
* colour relationship;
* source metadata.

The search engine must operate on imported game data without reparsing SGF files.

---

## 2. Exact Positions and Patterns Are Different

An exact-position search asks:

> Did this complete 19×19 board position occur?

A pattern search asks:

> Did this smaller arrangement occur in a corner, on a side, or anywhere on the board?

An exact whole-board hash cannot directly answer arbitrary pattern searches.

Bermuda will therefore separate:

1. **position reconstruction**, which recreates every board state;
2. **candidate indexing**, which rapidly identifies possible pattern matches;
3. **exact pattern verification**, which confirms candidates against the reconstructed board.

---

## 3. Search Occurrences

A searchable occurrence represents the position after a particular move.

Each occurrence is identified by:

* game ID;
* move number.

Move number zero represents the initial position after setup stones and before the first move.

For a game containing 250 moves, occurrences may therefore range from:

```text
0 through 250
```

Passes produce a new occurrence because side-to-move and ko state may change even when the stones do not.

---

## 4. Position State

A position state contains:

* black-stone bitboard;
* white-stone bitboard;
* player to move;
* ko point, if any;
* game ID;
* move number.

The board representation uses the existing six-word bitboards:

```text
black: 6 × u64
white: 6 × u64
```

Only the first 361 bits are used for a 19×19 board.

---

## 5. Side to Move

Side to move is part of position identity.

The same stones with Black to move and White to move are different searchable positions.

For ordinary records, side to move is inferred from the next recorded move.

The index must not assume that colours always alternate because SGFs may contain:

* passes;
* handicap setup;
* edited or unusual move sequences.

---

## 6. Ko State

Ko state is relevant when searching exact positions.

Two positions with identical stones may permit different legal moves because one has a prohibited simple-ko recapture.

The exact-position identity therefore includes the ko point.

Pattern searches may initially ignore ko unless the user explicitly requests legal-move matching.

---

## 7. Symmetry

Go patterns may be equivalent under the eight square-board symmetries:

* identity;
* rotate 90 degrees;
* rotate 180 degrees;
* rotate 270 degrees;
* reflect horizontally;
* reflect vertically;
* reflect across the main diagonal;
* reflect across the anti-diagonal.

Pattern queries may also optionally swap Black and White.

The stored game position remains in its original orientation.

Search results record which transformation matched the query.

---

## 8. Indexing Strategy

Bermuda will use a two-stage pattern search.

### Stage 1: candidate selection

A compact feature index eliminates positions that cannot match.

Candidate features may include:

* occupied-point count within a search region;
* black-stone count;
* white-stone count;
* selected anchor-point states;
* hashes of small local tiles.

The first implementation should remain simple and measurable.

### Stage 2: exact verification

Each candidate position is reconstructed or read from compact indexed state.

The requested pattern is transformed as required and compared point by point.

This stage determines the final correct result.

Candidate indexing may produce false positives, but exact verification must never produce false matches.

---

## 9. Initial Implementation Scope

Version 0.4 should begin with an exact position stream rather than a complete arbitrary-pattern index.

The first implementation will:

1. read every compact move file;
2. replay every game;
3. emit every occurrence;
4. compute an exact-position fingerprint;
5. store game ID and move number;
6. support exact full-board lookup.

This validates position reconstruction and indexing at database scale.

The arbitrary pattern candidate index will be added after measuring:

* total occurrence count;
* index construction speed;
* storage size;
* repeated-position frequency.

---

## 10. Expected Scale

With approximately 116,684 games and an estimated average of 220–260 moves, the database may contain roughly:

```text
25–30 million occurrences
```

The implementation must avoid inserting these rows one transaction at a time.

Index construction should use:

* prepared SQL statements;
* batched transactions;
* progress reporting;
* resumable per-game indexing.

---

## 11. Proposed Database Tables

### indexed_games

Tracks whether a game has been indexed.

```text
game_id
index_version
indexed_at
occurrence_count
```

This makes indexing incremental and restartable.

### exact_positions

Stores exact-position fingerprints and occurrences.

```text
position_hash
game_id
move_number
side_to_move
ko_point
```

The first implementation may store one row per occurrence.

Before committing to that final layout, storage measurements must be taken on a representative sample.

---

## 12. Hashing

Exact-position hashes must include:

* index format version;
* board size;
* black bitboard;
* white bitboard;
* side to move;
* ko point or no-ko marker.

A cryptographic hash is not required for every indexed position.

A fast deterministic 64-bit or 128-bit hash may be used for candidate lookup, provided exact board-state comparison is used to protect against collisions.

Canonical game hashes remain SHA-256 because they identify permanently stored games.

---

## 13. Incremental Indexing

The index builder must be safe to rerun.

For each game:

1. check `indexed_games`;
2. skip it if already indexed with the current index version;
3. replay and insert its occurrences;
4. mark the game indexed only after successful completion.

If indexing is interrupted, rerunning the command continues with the first unindexed game.

Changing the index format increments `index_version`.

---

## 14. Command-Line Interface

The initial commands should be:

```text
bermuda build-position-index <DATABASE>
bermuda position-index-status <DATABASE>
```

A later exact-position query command may be:

```text
bermuda find-position <DATABASE> <POSITION-DESCRIPTION>
```

Pattern-search commands will be designed after the first position-index measurements.

---

## 15. Correctness Requirements

The index builder must:

* reproduce the same positions as normal game replay;
* include the initial setup position;
* include positions after passes;
* preserve move numbers correctly;
* preserve side-to-move information;
* remain idempotent;
* continue after a corrupt move file while logging the error.

Tests must cover:

* ordinary alternating play;
* captures;
* passes;
* handicap setup;
* simple ko;
* identical stones with different side to move;
* interrupted and repeated indexing.

---

## 16. Non-Goals for the First Position Index

The first implementation will not yet provide:

* arbitrary rectangular pattern queries;
* approximate or fuzzy matching;
* joseki classification;
* AI evaluation;
* territory or influence analysis;
* variation-tree indexing.

The first goal is to prove correct, incremental indexing across the complete imported database.

---

## 17. Guiding Principle

Do not optimise an unmeasured index design.

First build a correct position stream, measure its scale and storage cost, and then design the arbitrary-pattern candidate index using real data.
