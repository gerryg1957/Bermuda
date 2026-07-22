# MoyoDB Roadmap

MoyoDB is a professional Go game database designed to replace legacy tools such as MoyoGo Studio and provide a modern foundation for searching, analysing, and exploring large SGF collections.

The project is organised around a core database engine with command-line tools and a future graphical interface.

---

# Phase 1 — Core game representation ✅ Complete

## Goals

Create a reliable internal representation of Go games.

Implemented:

- SGF parser
- Main variation extraction
- Setup stones
- Captures
- Pass moves
- Simple ko handling
- Compact move file format

Result:

SGF files can be converted into a compact internal representation suitable for database storage and replay.

---

# Phase 2 — Game identity and database foundation ✅ Complete

## Goals

Create a database architecture suitable for professional Go collections.

Implemented:

- MoyoDB project structure
- SQLite metadata database
- Canonical game hashing
- Duplicate game detection
- Source tracking
- Import from individual SGF files
- Import from SGF directories

Project layout:

MoyoDB project
|
├── moyodb-project.toml
├── database/
│ ├── metadata.sqlite3
│ └── games/
├── indexes/
└── cache/


Validated with:

- GoGoD test collections
- go4go collection
- 116,684 games imported

---

# Phase 3 — Exact position search engine ✅ Complete

## Goals

Allow MoyoDB to answer:

"Has this exact board position occurred before?"

Implemented:

- Position fingerprints
- Position stream generation
- Incremental indexing
- SQLite position index
- Schema migration system
- Exact position lookup

Database tables:

indexed_games

exact_positions

Validation: 

Games indexed : 116684
Positions : 25085473
Errors : 0


Command examples:


moyodb build-position-index PROJECT

moyodb find-position PROJECT GAME MOVE

---

## 4. Position Search and Exploration

### 4.1 Exact Position Indexing ✅ COMPLETE

Implemented:

- Deterministic exact-position fingerprints.
- Position stream generation during game replay.
- SQLite position index.
- Incremental position indexing.
- Schema migration support.
- Search by game and move number.
- Duplicate position detection across the database.

Current scale tested:

- Database: go4go
- Games imported: 116,684
- Positions indexed: 25,085,473
- Indexing errors: 0

The index can now answer:

- "Where does this exact position occur?"
- "Which games contain this position?"
- "At what move number does it occur?"

---

### 4.2 Position Reconstruction and Display Foundation ✅ COMPLETE

Implemented:

- Replay of a game into a sequence of complete board states.
- Position state objects combining:
  - board position;
  - side to move;
  - ko information;
  - position fingerprint.
- Basic ASCII board rendering.

This provides the foundation for moving from a database result to an actual Go position view.

---

### 4.3 Connect Search Results to Board Display 🔄 NEXT

Goal:

Turn position search into a usable player-facing tool.

Implement:

1. `find-position` returns matching game and move information.
2. Load the associated move file.
3. Replay the game to the requested position.
4. Display the resulting board.
5. Show:
   - game metadata;
   - move number;
   - player to move;
   - board diagram.

Example target:

Game: 12345
Move: 50
Black: Player A
White: Player B

A B C D E F G ...
19 . . . X . . . ...
18 . O X . . . . ...
...

Black to move


---

### 4.4 Pattern Search Layer

After position exploration is usable:

- support searching for board patterns;
- allow local board regions rather than whole-board equality;
- support rotations and reflections where appropriate;
- rank results by game metadata.

This becomes the foundation for a modern replacement for MoyoGo Studio pattern search.



---

# Phase 5 — Pattern search engine

## Goal

Provide the feature that made tools like Kombilo valuable.

Answer:

"Where has this shape appeared before?"

Initial implementation:

- Corner pattern search
- 5x5 patterns
- 7x7 patterns

Possible schema:

pattern_positions

pattern_hash
game_id
move_number
x
y
width
height


Later:

- arbitrary board regions
- pattern similarity
- professional joseki search

---

# Phase 6 — Graphical interface

## Goal

Provide a modern replacement for MoyoGo Studio.

The GUI should use the existing MoyoDB engine.

Architecture:

Qt interface

 |
 v

MoyoDB library

 |
 +-- database
 +-- importer
 +-- position search
 +-- pattern search

Features:

- Game browser
- Go board display
- SGF viewer
- Position search
- Pattern search
- Metadata search

---

# Phase 7 — Large collection optimisation

## Goal

Support professional-scale databases.

Tasks:

- Faster indexing
- Parallel import
- Better caching
- Incremental updates
- Collection statistics

Target collections:

- GoGoD
- go4go
- personal SGF archives

---

# Development principle

Each phase should produce a working, testable system.

Priority order:

1. Correctness
2. Database integrity
3. Search capability
4. User interface
5. Performance optimisation

The database engine comes first; the GUI is built on top of it.
