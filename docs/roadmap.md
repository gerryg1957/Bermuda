# MoyoDB Roadmap

## Project Goal

MoyoDB is a professional Go game database designed as a modern replacement for Moyo Go Studio.

The primary goals are:

- native Linux support;
- large-scale SGF storage;
- fast exact position search;
- professional game browsing and analysis;
- support for historical and modern Go collections.

The design prioritises:
- correctness;
- deterministic storage;
- incremental indexing;
- long-term maintainability.

---

# Completed

## 1. Core Go Game Engine

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

The engine correctly reconstructs positions from SGF records.

---

## 2. Compact Game Storage

Completed:

- Compact move-file format
- SGF import conversion
- Canonical game hashing
- Duplicate game detection support

Canonical game identity includes:

- board size;
- setup position;
- complete move sequence.

Metadata is deliberately excluded.

---

## 3. Database Project Structure

Completed:

- MoyoDB project creation
- SQLite metadata database
- Game file storage layout
- Incremental import support
- Source tracking

Supported workflow:

SGF collection
|
v
MoyoDB project
|
+-- metadata.sqlite3
|
+-- games/


---

# 4. Exact Position Search

## Completed

The exact-position search engine is now operational.

Implemented:

- deterministic board position fingerprints;
- position stream generation;
- SQLite position index;
- incremental position indexing;
- exact position lookup;
- game/move position selection;
- metadata-enriched search results;
- board reconstruction from indexed positions.

The database can now answer:

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

Example:

Game 1
Move 50
Black to move

Black: Ando Takeo
White: Tozawa Akinobu
Event: 17th Japanese Prime Minister's Cup
Date: 1972-12-27
Result: W+R


---

# 5. Search and Analysis Workflow

## Current priority

Improve usability of position search.

Planned:

### 5.1 Better board display

Add:

- coordinate labels;
- standard Go board notation;
- last move marker;
- move number display;
- improved readability for 19x19 games.

---

### 5.2 Position viewing commands

Add dedicated commands:

moyodb show-position <game> <move>


Capabilities:

- display board;
- show metadata;
- show side to move;
- show move context.

---

### 5.3 Local game context

Add:

- previous moves;
- following moves;
- replay from selected position;
- variation browsing.

---

# 6. Pattern Search

Future major feature.

Goals:

- search for board patterns rather than exact positions;
- support professional joseki/fuseki research;
- allow transformations where appropriate;
- provide frequency information.

Possible features:

- corner pattern search;
- local board-region search;
- occurrence counts;
- move statistics.

---

# 7. User Interface

Future phase.

Possible implementations:

- Qt desktop application;
- web interface;
- integrated analysis board.

The GUI should build on the existing command-line engine rather than duplicate functionality.

---

# 8. Large Database Support

Future improvements:

- efficient import of GoGoD and go4go collections;
- background indexing;
- faster bulk operations;
- database statistics;
- duplicate reporting.

---

# Development Principles

## Correctness first

The database must always reproduce the exact game state.

## Incremental development

Each feature should:

- have tests;
- preserve existing behaviour;
- be committed separately.

## Professional focus

The initial target is 19x19 professional games.

Smaller boards may be supported, but are not the primary focus.
