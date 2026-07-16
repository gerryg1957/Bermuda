# MoyoDB Position Explorer Design

**Status:** Draft 1
**Target milestone:** Version 0.5

## 1. Purpose

The Position Explorer allows a user to locate every indexed occurrence of the exact position currently displayed in a game.

The user should not need to know or enter a position fingerprint.

The primary workflow is:

```text
Select game
    ↓
Select move number
    ↓
Reconstruct board position
    ↓
Find identical indexed positions
    ↓
Display matching games and move numbers
```

---

## 2. Initial Command-Line Interface

The first implementation will provide:

```text
moyodb explore-position <DATABASE> <GAME_ID> <MOVE_NUMBER>
```

Example:

```text
moyodb explore-position ~/go-database-index-test 30 174
```

The command will:

1. find game 30 in the database;
2. read its compact move file;
3. replay it to move 174;
4. obtain the position fingerprint;
5. search `exact_positions`;
6. print all matching game IDs and move numbers.

---

## 3. Position Selection

Move number zero means the initial position after setup stones and before the first move.

For a game containing `N` moves, valid positions are:

```text
0 through N
```

If the requested move number is greater than `N`, the command must return a clear error.

---

## 4. Exact Position Identity

A match requires the same:

* board size;
* Black stones;
* White stones;
* side to move;
* simple-ko state.

The existing exact-position fingerprint already includes these values.

The Position Explorer will reuse the existing fingerprint and lookup implementation rather than defining a second form of position identity.

---

## 5. Search Results

Each result initially contains:

* game ID;
* move number;
* side to move;
* ko point, if present.

Example:

```text
Position: game 30 after move 174
Matches: 3

Game 30 — move 174 — Black to move
Game 812 — move 63 — Black to move
Game 1456 — move 201 — Black to move
```

Later versions may add:

* player names;
* game date;
* event;
* result;
* source collection;
* direct links to game display.

---

## 6. Self-Matches

The occurrence used as the search input will normally appear in the results.

The initial implementation will include it.

A later option may allow the user to exclude the source occurrence:

```text
--exclude-self
```

---

## 7. Index Requirements

The selected game must exist in the database.

The selected position can be reconstructed even if the game has not yet been indexed. However, it will only appear among search results after its exact-position rows have been created.

If no matching positions are present, the command should print:

```text
Matches: 0
```

This is not an error.

---

## 8. Core API

The command-line layer should remain thin.

The indexer should provide a reusable method conceptually equivalent to:

```text
find_position_from_game(game_id, move_number)
```

The method should:

1. load the game’s move file;
2. generate its position stream;
3. select the requested occurrence;
4. query the exact-position index;
5. return the selected position and its matches.

This API can later be called directly by the Qt application.

---

## 9. Future Graphical Interface

The Qt Position Explorer will provide:

* game board display;
* move navigation;
* “Find identical positions” action;
* result list;
* metadata columns;
* double-click navigation to a matching game;
* previous and next matching occurrence controls.

The user-facing action should be available from:

* a toolbar button;
* a board context menu;
* a keyboard shortcut.

---

## 10. Relationship to the Fuseki Explorer

The Position Explorer searches for a complete exact board position.

The Fuseki Explorer will instead aggregate opening continuations and statistics.

Both features will reuse:

* compact move files;
* position streams;
* exact-position fingerprints;
* indexed game occurrences;
* metadata lookup.

The Position Explorer should therefore be completed before the Fuseki Explorer.

---

## 11. Correctness Tests

Tests must cover:

* move zero;
* a normal middle-game position;
* the final position;
* an out-of-range move number;
* an unknown game ID;
* a position with multiple matches;
* a position with no indexed matches;
* side-to-move differences;
* pass-generated positions;
* ko-state differences.

---

## 12. First Implementation Scope

The first version will not include:

* board diagrams in terminal output;
* interactive navigation;
* filtering by player or date;
* symmetry matching;
* partial-pattern matching;
* colour reversal.

The first goal is a correct reusable search from a database game ID and move number.
