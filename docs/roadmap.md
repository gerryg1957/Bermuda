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

# Phase 4 — Position exploration (current)

## Goal

Make search results understandable to humans.

The first priority is to display positions.

Tasks:

## 4.1 Show a board position

Add:


moyodb show-position PROJECT GAME MOVE


Requirements:

- Replay game to requested move
- Display board
- Show side to move
- Show ko point
- Support setup positions

This becomes the foundation for all later search features.

---

## 4.2 Improve search results

Current:


Game 123 Move 50 Black to move


Target:


Game 123

Black:
White:
Event:
Date:
Result:

Position after move 50

[board]


Use existing metadata database.

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
