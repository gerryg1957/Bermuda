# Bermuda Roadmap

## Project Goal

Bermuda is a professional Go game database designed as a modern native replacement for Moyo Go Studio.

The goals are:

- large-scale SGF storage;
- fast exact position search;
- professional game browsing and analysis;
- Linux-native operation;
- long-term maintainable architecture.

The initial focus is 19x19 professional and historical games.

---

# Completed

## 1. Go Game Engine

Completed:

- SGF parser
- Main variation extraction
- Board representation
- Stone placement
- Captures
- Simple ko
- Pass moves
- Setup stones (AB/AW/AE)
- Game replay

The engine can reconstruct valid game states from SGF records.

---

## 2. Compact Game Storage

Completed:

- SGF to compact move-file conversion
- Compact move-file reader/writer
- Canonical game hashing
- Duplicate game identity support

Canonical game identity includes:

- board size;
- setup stones;
- move sequence.

Metadata is excluded from the canonical identity.

---

## 3. Database Architecture

Completed:

- Bermuda project structure
- SQLite metadata database
- Game file storage
- Source collection tracking
- Incremental import framework
- Incremental indexing support

Database layout:

Bermuda project

metadata.sqlite3

games/
compact game files


---

# 4. Exact Position Search

## Completed

The exact position search engine is operational.

Implemented:

- deterministic position fingerprints;
- position stream generation;
- SQLite exact-position index;
- incremental position indexing;
- search by game and move;
- metadata-enriched search results;
- board reconstruction.

The database can answer:

> "Where has this exact board position occurred, and what was the context of the game?"

Search results include:

- game ID;
- move number;
- side to move;
- ko state;
- Black player;
- White player;
- event;
- date;
- result.

Validated against a real go4go database.

## Position Display and Replay Foundation (Completed)

The database can now reconstruct and display individual game positions.

Completed:

- Added `show-position` command for displaying a position from a stored game.
- Reconstructs the board state directly from the compact move file.
- Displays game metadata:
  - Black player
  - White player
  - Event
  - Date
  - Result
- Displays side to move.
- Added tracking of the move that produced each position.
- Displays the last move in standard Go coordinates (for example `Q16`).
- Added board coordinate conversion support.
- Maintains separation between:
  - replay engine (`replay.rs`)
  - board representation (`board.rs`)
  - command-line presentation (`main.rs`)

This provides the foundation for full game replay and future interactive analysis tools.

---

## Next: Database Game Replay

Planned:

- Add a command to replay a complete stored game.
- Display positions sequentially from move 0 onwards.
- Show:
  - move number
  - last move
  - player to move
  - board position
- Allow limiting replay to a selected move range.
- Provide a foundation for later GUI and analysis features.

---

# 5. Analysis Workflow

## 5.1 Board Display

Completed:

- command-line board rendering;
- Go coordinate labels;
- correct skipping of the letter I.

Current output supports readable 19x19 board display.

Future improvements:

- bottom coordinate labels;
- hoshi/star points;
- improved spacing;
- last-move marker.

---

## 5.2 Position Viewing



Completed:

- show-position command
- board reconstruction
- metadata display
- side-to-move display

---

## 5.3 Game Replay Workflow

Planned:

Add improved replay support.

Features:

- replay a complete game;
- step forward/backward;
- show move numbers;
- show captured stones;
- jump to selected positions.

---

# 6. Pattern Search

Major future feature.

Goal:

Search for board patterns rather than only exact positions.

Examples:

- corner joseki searches;
- fuseki patterns;
- local fighting positions;
- opening statistics.

Possible features:

- rectangular pattern matching;
- board transformations;
- frequency analysis;
- professional game statistics.

---

# 7. Game Browsing

Future phase.

Features:

- list games;
- filter by player;
- filter by event;
- filter by date;
- show game information;
- open game directly.

Possible commands:

bermuda list-games
bermuda show-game <id>


---

# 8. User Interface

Future phase.

Possible implementations:

- Qt desktop application;
- web interface;
- integrated Go board viewer.

Principle:

The GUI should use the existing Bermuda engine rather than duplicate database logic.

---

# 9. Large Database Support

Future improvements:

- import full GoGoD collections;
- import full go4go collections;
- background indexing;
- indexing progress display;
- database statistics;
- duplicate reporting;
- faster bulk operations.

---

# Development Principles

## Correctness first

The database must always reproduce the exact game state.

## Small verified changes

Each feature should:

- have tests;
- preserve existing behaviour;
- be committed separately.

## Professional focus

Initial priority:

- 19x19 games;
- historical and professional collections;
- accurate replay and search.

Smaller boards may be supported but are not the primary target.
