# Bermuda Architecture

**Version:** Draft 2  
**Status:** Current architecture and agreed development direction  
**Date:** July 2026

---

## 1. Purpose

Bermuda is a native, library-first Go research system for professional game collections.

Its purpose is not merely to store SGF files. It is intended to let a user:

- import large professional game collections;
- preserve the provenance and metadata supplied by each source;
- replay any stored game accurately;
- search by exact board position;
- select a local pattern and find it throughout the database;
- inspect the matching games at the relevant positions;
- analyse the continuations and historical outcomes associated with a pattern;
- use the same core library from a command-line interface and a future Qt application.

Bermuda is therefore both a database engine and the foundation of an interactive Go research environment.

Detailed rules for canonical games, source metadata, compact move files and the SQLite schema belong in `database-design.md`. This document describes how the parts of the whole system fit together.

---

## 2. Architectural Principles

Bermuda follows these principles:

1. **Library first.**  
   Core behaviour belongs in the Rust library. The command-line interface and future graphical interface are clients of that library.

2. **Correctness before optimisation.**  
   Replay, capture, ko, passes, setup stones and pattern matching must be correct before search is accelerated.

3. **One owner for each responsibility.**  
   Parsing, importing, replay, indexing, searching, statistics and presentation must not become entangled.

4. **Raw evidence before presentation.**  
   Search services return occurrences and associated facts. Grouping, sorting, labels, charts and other presentation choices belong in higher layers.

5. **Source preservation.**  
   Imported metadata remains associated with the source that supplied it. Bermuda does not silently merge conflicting source records.

6. **Rebuildable derived data.**  
   Position indexes, future pattern indexes and statistics are derived from canonical games and may be rebuilt.

7. **Front-end independence.**  
   No core API should assume a terminal, Qt, a particular board widget or a particular screen layout.

8. **Measure before optimising.**  
   The current brute-force pattern search remains a reference implementation against which later indexes and optimisations can be tested.

---

## 3. System Overview

The intended structure is:

```text
+------------------------------------------------------------+
| Front ends                                                 |
|                                                            |
|  Command-line interface       Future Qt application        |
+--------------------------+---------------------------------+
                           |
                           v
+------------------------------------------------------------+
| Public application API                                     |
|                                                            |
|  Project access       Game queries       Search queries     |
|  Replay operations    Result models      Statistics         |
+------------------------------------------------------------+
                           |
                           v
+------------------------------------------------------------+
| Domain services                                             |
|                                                            |
|  ProjectManager   Importer          GameRepository          |
|  Replay           PositionIndexer   SearchEngine            |
|  ContinuationStatistics             AnalysisEngine (later)  |
+------------------------------------------------------------+
                           |
                           v
+------------------------------------------------------------+
| Domain model                                                |
|                                                            |
|  Project   Source   Game   Metadata   Move   Position       |
|  Pattern   SearchQuery   SearchOccurrence   SearchResult    |
+------------------------------------------------------------+
                           |
                           v
+------------------------------------------------------------+
| Infrastructure                                              |
|                                                            |
|  SGF parser   Compact move files   SQLite metadata          |
|  Exact-position index   Future search indexes              |
+------------------------------------------------------------+
```

Some of these components already exist under different names or with broader responsibilities. The diagram describes the direction in which the current implementation should evolve.

---

## 4. Library-First Boundary

The `bermuda` crate is the authoritative implementation of:

- project management;
- SGF parsing and import;
- canonical game identity;
- compact move storage;
- replay;
- position generation;
- indexing;
- exact-position search;
- pattern extraction and matching;
- future continuation statistics.

The CLI should:

1. parse command-line arguments;
2. construct library request objects;
3. call public library operations;
4. format and print the returned results.

The CLI must not contain database logic, replay logic, search algorithms or statistical calculations.

The future Qt application should follow the same rule. It will translate user actions into library requests and render the returned data.

---

## 5. Domain Model

### 5.1 Project

A `Project` represents one Bermuda collection and owns the paths to its database, move files and temporary storage.

A project does not perform every operation itself. It provides access to the services that operate on its data.

### 5.2 Source

A `Source` identifies an imported collection or update, for example GoGoD or go4go.

It records provenance such as:

- source name;
- source version;
- import date.

### 5.3 Game

A `Game` is the canonical identity of a played game.

Its identity is based on:

- board size;
- final initial position after setup edits;
- complete ordered move sequence.

Metadata and SGF formatting do not define game identity.

### 5.4 Metadata

Metadata belongs to a particular imported source record.

Examples include:

- players;
- date;
- event;
- result;
- round;
- komi;
- handicap;
- rules.

A browser may select representative metadata for a concise game-list row, while retaining every source-specific record.

### 5.5 Move

A `Move` records colour and point, including a pass.

It is a domain value and must not depend on SGF syntax or GUI coordinates.

### 5.6 Position

A `Position` or `PositionState` represents a replayed board state together with information needed to interpret it, including:

- board contents;
- side to move;
- simple-ko point where applicable;
- move number.

### 5.7 Pattern

A `Pattern` is a rectangular selection extracted from a board position.

It contains the selected intersections and the relevant board-edge information. Empty intersections inside the selected rectangle are part of the pattern.

A pattern does not know:

- which database it will be searched in;
- which GUI selected it;
- whether it came from a professional game, an imported SGF or a manually constructed board.

### 5.8 Search occurrence

A search occurrence identifies one place where a query matched.

For a pattern search it will eventually need to carry information such as:

- game ID;
- move number;
- matched rectangle in the original game orientation;
- transformation used to obtain the match;
- next recorded move, when one exists.

### 5.9 Search result

A search result contains raw matching occurrences and enough information for a caller to group, sort or display them.

The core search API should not assume that one displayed row equals one occurrence. A GUI may show one row per game while retaining all occurrences for that game.

---

## 6. Services and Responsibilities

### 6.1 ProjectManager

`ProjectManager` is responsible for:

- creating projects;
- opening projects;
- validating the expected project structure;
- refusing unsafe overwrites.

It should not import games or execute searches.

### 6.2 Importer

The importer is responsible for:

- reading SGF input;
- parsing collections;
- extracting the supported game variation;
- validating games;
- calculating canonical identity;
- detecting duplicates;
- writing compact move files;
- recording source and metadata rows;
- reporting skipped files and errors.

Directory traversal is orchestration around the importer, not part of game identity.

### 6.3 GameRepository

A `GameRepository` is the intended home for database-backed game access.

Its responsibilities should include:

- retrieving games by ID;
- listing games;
- sorting and filtering game lists;
- selecting representative metadata for display;
- retrieving all source-specific metadata;
- resolving move-file locations.

Some of this behaviour currently exists in `game_list`, `indexer` and database queries. It should move behind a coherent public API rather than being duplicated by front ends.

### 6.4 Replay

Replay is responsible for reconstructing board states from a `GameRecord`.

All consumers must use the same replay rules. Import validation, position indexing, search previews, CLI output and GUI replay must not implement separate versions of capture or ko logic.

Database games are replayed read-only. A future GUI may also provide a separate manually constructed analysis position, but that must not alter the stored game.

### 6.5 PositionIndexer

`PositionIndexer` is responsible for derived exact-position index data:

- replaying games into position occurrences;
- writing index rows transactionally;
- tracking index versions;
- finding exact-position occurrences efficiently;
- rebuilding stale index data.

It should gradually cease to be the general entry point for unrelated game queries or local pattern searching.

### 6.6 PatternSearcher

`PatternSearcher` is the current reference implementation for local pattern search.

It is responsible for:

- testing a pattern at candidate board locations;
- searching one replayed game;
- searching the complete database by replaying candidate games;
- returning raw matches.

The brute-force implementation is valuable even after optimisation because it provides a correctness reference.

### 6.7 SearchEngine

A public `SearchEngine` is the intended unified search service.

It should provide a stable API for:

- exact-position search;
- pattern search in one game;
- pattern search across a project;
- future transformed and wildcard searches.

The `SearchEngine` should coordinate repositories, replay and search algorithms without exposing storage details to callers.

It should accept request objects rather than long parameter lists.

Conceptually:

```rust
pub struct PatternSearchQuery {
    pub pattern: Pattern,
    pub scope: SearchScope,
    pub transformations: TransformationOptions,
}

pub enum SearchScope {
    Game(i64),
    Project,
}
```

The exact public types will be finalised during the public search API phase.

### 6.8 ContinuationStatistics

Continuation analysis is separate from pattern matching.

Given a collection of occurrences, this service should be able to determine:

- how often each next move was played;
- how many matching games contributed;
- historical Black and White results;
- result breakdowns associated with continuations;
- later filters such as date range, player, event or rank.

These figures describe the imported professional-game evidence. They are not an objective evaluation of the board position and must not be presented as AI judgement.

A presentation layer may group different next-move coordinates under the same displayed statistical category. The grouping policy belongs to statistics or presentation, not to the pattern matcher.

### 6.9 AnalysisEngine

External AI analysis is a later, optional service.

It should be kept separate from historical database statistics.

An engine adapter may eventually provide:

- recommended moves;
- win probability;
- score estimate;
- visit counts;
- principal variations.

The interface should not hard-code KataGo, even if KataGo is the first supported engine.

Conceptually:

```rust
pub trait AnalysisEngine {
    fn analyse(&self, position: &PositionState) -> Result<EngineAnalysis>;
}
```

No external engine is part of the current core architecture.

---

## 7. Storage Architecture

Bermuda uses two complementary forms of storage.

### 7.1 SQLite metadata database

SQLite stores:

- canonical game records;
- sources;
- links between games and sources;
- source-specific metadata;
- derived exact-position index rows;
- index version information;
- future derived search and statistics tables where justified.

### 7.2 Compact move files

Each canonical game has one compact move file containing the information required to reconstruct it.

Move files are:

- independent of source filenames;
- referenced by stable game IDs and database rows;
- shared by every source that supplied the same canonical game.

SQLite is used for relational queries. Compact files are used for efficient replay and long-term separation between game content and metadata.

---

## 8. Import Architecture

The import flow is:

```text
SGF file
   |
   v
parse collection
   |
   v
extract supported game
   |
   v
validate and canonicalise
   |
   +---- duplicate ----> attach new source record and metadata
   |
   +---- new game -----> write compact move file
                         insert canonical game
                         attach source record and metadata
```

Import must be incremental. Adding a later source version should not require recreating the entire project.

Import errors should be reported per file so that one malformed SGF does not invalidate a large directory import.

The professional project currently accepts 19×19 games. The core board and replay code may continue to support other legal sizes for reuse and testing.

---

## 9. Replay and Position Streams

A position stream contains:

- the initial position;
- one position after every recorded move, including passes;
- side-to-move information;
- ko state where required;
- a stable move number.

Setup stones are applied before the initial occurrence is emitted.

Non-alternating move sequences are interpreted from the recorded colours rather than assumed alternation.

Position streams support:

- board replay;
- exact-position indexing;
- pattern search;
- displaying a selected search occurrence;
- continuation analysis.

There must be one authoritative route from a stored game to its position stream.

---

## 10. Search Architecture

### 10.1 Pattern sources

A user may define the source position in three ways:

1. manually place stones on a board;
2. load an external SGF;
3. select a position from a game in the database.

In every case, the search input becomes the same domain data:

```text
board position + selected rectangle
```

The source of the position is not part of pattern identity.

### 10.2 Graphical selection

In a future GUI, a rubber-band rectangle will capture the selected region.

The user should not need to think in terms of numeric `left`, `bottom`, `width` and `height` arguments. Those coordinates are an implementation representation produced by the board widget.

The CLI may continue to expose numeric coordinates for testing and automation.

### 10.3 Exact-position search

Exact-position search compares the complete board state and relevant state information through the position index.

It is appropriate when the entire position must be identical.

### 10.4 Local pattern search

Pattern search compares only the selected rectangle and its board-edge constraints.

It is appropriate for:

- joseki shapes;
- fuseki structures;
- tactical formations;
- whole-board fragments;
- manually constructed research questions.

### 10.5 Transformations

Pattern search should later support rotations and reflections.

Search orientation and display orientation are separate concerns:

- the matcher may transform the pattern to find an occurrence;
- the stored game must always replay in its original orientation;
- the result must record the matched rectangle and transformation;
- the GUI uses that information to highlight the correct area without rotating the game.

A displayed board retains its normal coordinates, with the lower-left corner remaining the lower-left corner of the recorded game.

Colour reversal may be added as an explicit search option rather than assumed automatically.

### 10.6 Wildcards and semantic cells

Future pattern queries may distinguish:

- required black stone;
- required white stone;
- required empty point;
- any point;
- friendly stone;
- enemy stone;
- ignored point.

These are search semantics and must not be added by weakening the meaning of the current exact `PatternCell` values.

---

## 11. Search Results and Study Workflow

The complete research workflow is:

```text
construct or open an interesting position
                |
                v
select a rectangular pattern
                |
                v
search the project
                |
                v
receive raw matching occurrences
                |
                v
group them into a sortable matching-game list
                |
                v
select a game and open it at the matching position
                |
                v
step through the recorded game and compare continuations
```

### 11.1 Occurrences and games

A pattern may occur:

- in many games;
- more than once in one game;
- in different corners or orientations.

The search layer returns occurrences.

The application layer may group those occurrences into one row per game and retain:

- first or selected matching move;
- total match count;
- all match locations;
- associated metadata.

### 11.2 Selected-game replay

When a matching game is selected, the application should:

- open the original recorded game;
- jump to the relevant move;
- show where the pattern matched;
- allow read-only stepping backwards and forwards;
- mark the next recorded move distinctly.

The selected game's original orientation must be preserved.

### 11.3 Continuation overlay

The application may overlay continuation information derived from all matches while displaying one selected historical game.

These are separate layers:

1. recorded stones in the selected game;
2. matched rectangle;
3. marker for the selected game's actual next move;
4. labels for statistically observed continuations.

The core library supplies the data. The GUI decides how letters, symbols, bars and highlighting are rendered.

### 11.4 Historical outcome information

Statistics may summarise what happened in the matching professional games.

They must be labelled as historical database evidence. Terms such as “predicted outcome” should be used cautiously because the sample may be affected by:

- player strength;
- date;
- rules;
- komi;
- selection bias;
- small sample size;
- changes in professional understanding.

AI evaluation, when available, must be shown as a separate source of information.

---

## 12. Front Ends

### 12.1 Command-line interface

The CLI is a development, testing and automation interface.

It should remain useful for:

- project creation;
- import;
- index building;
- exact-position searches;
- pattern searches;
- diagnostic replay;
- future benchmarking.

CLI handlers should use small request structs and public library calls.

### 12.2 Qt application

The future Qt application will provide the integrated research workflow.

Likely responsibilities include:

- main board display;
- manual stone placement;
- imported-SGF browsing;
- database game browsing;
- rubber-band pattern selection;
- sortable full-database and match lists;
- small-board occurrence preview;
- selected-game replay;
- continuation labels;
- outcome charts;
- filters and sort state.

The Qt layer must not perform replay, SQL queries, pattern matching or statistics directly.

### 12.3 Study-session state

A study session is useful as a GUI concept but is not a durable core domain object.

It may contain:

- current source position;
- selected rectangle;
- active search results;
- selected occurrence;
- current game-list ordering and filters;
- current replay move;
- optional continuation and AI overlays.

This state belongs in the application or presentation layer. The core library remains stateless apart from project-backed services and returned values.

---

## 13. Module Direction

The current modules provide a sound foundation:

```text
board
canonical
database
game
game_list
import_directory
importer
indexer
move_file
pattern
pattern_search
position
position_stream
project
project_manager
replay
sgf
```

The likely evolution is:

- retain `board`, `game`, `pattern`, `position` and related value modules;
- retain `importer` and `project_manager` as services;
- narrow `indexer` to derived position-index responsibilities;
- introduce a coherent game repository API;
- introduce a public `search` or `search_engine` module;
- introduce statistics only after search results have stable public types;
- keep UI code in a separate Qt crate or package.

Module boundaries should follow domain responsibility rather than historical file placement.

---

## 14. Performance and Indexing

The first implementation of local pattern search is deliberately brute-force.

This provides:

- simple behaviour;
- a trusted correctness reference;
- a basis for benchmarks;
- a way to validate later optimised implementations.

Optimisation may later include:

- candidate filtering;
- compact board encodings;
- regional fingerprints;
- precomputed transformed patterns;
- parallel replay;
- persistent pattern indexes;
- caching frequently replayed games;
- batched metadata retrieval.

No optimisation should change public search semantics.

Every derived index must have an explicit version and a safe rebuild path.

---

## 15. Transactions, Errors and Recovery

Operations that alter project state should be transactional where practical.

In particular:

- one game's index replacement must be atomic;
- import should not leave a partially registered canonical game;
- duplicate detection and source attachment must remain consistent;
- interrupted derived-index builds must be resumable or safely restartable.

Library errors should carry context such as:

- project path;
- game ID;
- source file;
- move number;
- index version;
- requested rectangle.

The CLI and GUI decide how those errors are presented.

---

## 16. Testing Strategy

Tests should cover:

- canonical game identity;
- SGF parsing and main-variation extraction;
- setup stones, captures, passes and ko;
- compact move-file round trips;
- project creation and opening;
- duplicate import behaviour;
- game-list metadata selection and sorting;
- position-stream generation;
- exact-position indexing and lookup;
- pattern extraction at corners, sides and centre;
- single-game and database-wide pattern search;
- future transformation invariance;
- future continuation statistics.

Optimised search implementations must be compared against the brute-force reference on generated and real-game samples.

Public API tests should exercise the library without going through the CLI.

---

## 17. Development Direction

The agreed sequence is:

### Public search API

- stable query objects;
- stable occurrence and result types;
- a `SearchEngine` boundary;
- thin CLI handlers;
- library-level integration tests.

### Pattern-search evolution

- rotations;
- reflections;
- optional colour reversal;
- wildcard and ignored intersections;
- benchmarking and optimisation.

### Continuation statistics

- next-move frequencies;
- historical result breakdowns;
- game and occurrence grouping;
- filters and sample-size reporting.

### Qt research interface

- board-centred study workflow;
- graphical pattern capture;
- sortable match lists;
- contextual replay;
- continuation and outcome displays.

### Optional external analysis

- generic engine adapter;
- KataGo integration as one implementation;
- clear visual separation between historical evidence and AI judgement.

---

## 18. Non-Goals

Bermuda is not intended to:

- preserve SGF formatting as part of game identity;
- distinguish identical games by filename;
- store duplicate canonical game content;
- become primarily an SGF editor;
- embed search logic in the user interface;
- treat historical win percentages as objective board evaluation;
- make an external AI engine mandatory;
- distribute third-party game or joseki collections without permission.

---

## 19. Guiding Principle

Bermuda should support the way a Go player studies:

> Start with an interesting position, find where it occurred, inspect the games, and understand what happened next.

The database, search engine, statistics and user interfaces all exist to support that workflow.

When design choices conflict, prefer the option that improves:

- correctness;
- reproducibility;
- provenance;
- testability;
- search usefulness;
- front-end independence;
- long-term maintainability.
