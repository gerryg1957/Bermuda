# Bermuda Public API Design

## Status

This document describes the intended public Rust API for Bermuda.

It is a design document rather than a complete reference. The generated Rust documentation remains the authoritative source for the exact signatures currently implemented.

The API is still evolving. In particular, the search result model and some indexing interfaces may change before the first stable release.

---

# Purpose

Bermuda is designed as a reusable Rust library for professional Go game databases.

The library should allow applications to:

- create and open Bermuda projects;
- import SGF collections;
- inspect and list games;
- replay complete games or individual positions;
- build and maintain search indexes;
- search for exact positions and board patterns;
- retrieve results in a form suitable for command-line and graphical applications.

The command-line application and future Qt application should both use the same public library API.

---

# Design Principles

The public API should follow these principles.

## Library First

Core functionality belongs in the library.

The CLI and Qt application should contain presentation and interaction logic, but should not duplicate database, replay, indexing or search algorithms.

## Stable Domain Types

Applications should work with clearly defined Bermuda types rather than SQLite rows, filesystem paths or internal binary formats.

## Explicit Operations

Operations that may fail should return `Result`.

Operations that modify the database should be clearly distinguishable from read-only operations.

## Search Independence

A client should be able to display search results without needing detailed knowledge of the algorithm that produced them.

Exact-position search, pattern search and future search types should converge on a common result model where practical.

## Incremental Processing

Importing and indexing should support large databases without requiring all games or positions to be loaded into memory at once.

## Backwards Compatibility

Once Bermuda reaches a stable public release, public types and methods should not be changed without a clear migration path.

---

# Main Public Components

The intended public API is organised around the following components.

```text
ProjectManager
    |
    +-- Project
            |
            +-- Importer
            |
            +-- Game queries
            |
            +-- PositionIndexer
                    |
                    +-- Replay
                    +-- Exact-position search
                    +-- Pattern search
                    +-- Future search types
```
Project Management

    
    ProjectManager

ProjectManager is responsible for creating and opening Bermuda projects.

Typical responsibilities include:

validating project paths;
creating the required directory structure;
creating the metadata database;
opening existing projects;
checking project compatibility.

Illustrative usage:

use bermuda::ProjectManager;

let project = ProjectManager::open("/path/to/project")?;

The exact constructor names may differ while the API is being refined.

Project

Project represents an opened Bermuda project.

It should provide access to project-level resources without requiring clients to reconstruct internal paths.

A project may expose:

project name;
root directory;
metadata database location;
game storage location;
temporary storage location;
database or format version information.

Clients should prefer passing a Project value to library services rather than passing raw filesystem paths.

Import API

The import API is responsible for converting SGF files into Bermuda records.

It should support:

importing one SGF file;
recursively importing a directory;
source names and source versions;
duplicate detection;
incremental imports;
error collection;
import statistics.

An import summary should report information such as:

files processed;
games imported;
canonical games added;
duplicate games detected;
source records added;
games skipped;
errors encountered;
elapsed time.

Illustrative usage:

let summary = import_directory(
    &project,
    "GoGoD",
    "2026-07",
    "/path/to/sgf",
)?;

The import API should not require callers to understand the SQLite schema or compact move-file format.

Game Query API

The game query API provides metadata suitable for game lists, filters and search-result displays.

Typical fields include:

game ID;
black player;
white player;
result;
date;
event;
board size;
source information.

Queries should support:

sorting;
filtering;
pagination or bounded result sets;
stable column definitions;
one preferred metadata row per canonical game.

Illustrative usage:

let games = query_games(&project, &query)?;

The public API should not expose raw SQL expressions.

Position and Replay API
PositionIndexer

PositionIndexer is the primary public service for position-based operations.

Its responsibilities include:

reading stored games;
replaying games;
reconstructing individual board positions;
building and maintaining the position index;
searching indexed positions.

It may be opened from an existing project:

let indexer = PositionIndexer::open_project(&project)?;

Opening directly from a database root may remain available for lower-level tools, but project-based construction is preferred.

Reading Games

The API should allow a stored game to be read by its database ID.

let record = indexer.read_game_by_id(game_id)?;

The returned GameRecord contains the move sequence and metadata needed to replay the game.

Replaying a Game

Clients may request every position reached during a game.

let states = indexer.replay_game_states_by_id(game_id)?;

The returned sequence should include:

the initial position;
the position after each move;
move-number information;
side to move;
ko information where applicable.

For very large operations, streaming forms should be preferred over collecting every position into a vector.

Replaying One Position

Clients may reconstruct the board at a specific move number.

let state = indexer.replay_board_position(game_id, move_number)?;

Move number zero represents the initial position before the first move.

This operation is intended for:

board display;
search result navigation;
pattern selection;
analysis tools.
Indexing API

The position index supports fast search without replaying every game for every query.

The API should allow clients to:

determine which games require indexing;
count pending games;
index one game;
rebuild one game;
resume interrupted indexing;
identify the active index version.

Illustrative usage:

let pending = indexer.games_to_index(POSITION_INDEX_VERSION)?;

for game in pending {
    indexer.index_game(&game, POSITION_INDEX_VERSION)?;
}

Index-version details may remain public where needed for command-line maintenance tools, but normal applications should not have to manage internal version numbers directly.

A higher-level method may eventually be introduced:

let summary = indexer.build_pending_index()?;
Search API

Search is one of the central responsibilities of Bermuda.

The public API should distinguish between:

a search query;
an occurrence within a game;
a game-level result;
optional metadata attached to that result.
Common Search Types
SearchOccurrence

A SearchOccurrence identifies one match within one game.

The current design includes:

pub struct SearchOccurrence {
    pub move_number: usize,
    pub side_to_move: Option<Colour>,
    pub ko_point: Option<u16>,
    pub left: Option<u8>,
    pub bottom: Option<u8>,
}

The fields have the following meanings:

move_number identifies the position within the game;
side_to_move records whose turn it is where relevant;
ko_point records the current simple-ko point where relevant;
left and bottom identify the placement of a rectangular pattern match;
absent coordinates indicate a whole-board or non-spatial match.

The type may later gain additional fields if new search modes require them.

SearchResult

A SearchResult groups all relevant occurrences belonging to one game.

The intended structure should include:

game ID;
preferred game metadata;
one or more occurrences.

Illustrative design:

pub struct SearchResult {
    pub game_id: i64,
    pub black_player: Option<String>,
    pub white_player: Option<String>,
    pub date: Option<String>,
    pub event: Option<String>,
    pub result: Option<String>,
    pub occurrences: Vec<SearchOccurrence>,
}

Grouping by game is useful for both the CLI and GUI because one game may contain the same position or pattern more than once.

The final field names should remain consistent with the game-list API.

Exact-Position Search

Exact-position search finds positions with the same complete board state and relevant state information.

The indexed fingerprint should account for information required to distinguish legally or strategically different positions, including:

board contents;
side to move;
ko point where applicable.

Current low-level search:

let matches = indexer.find_exact_position(&fingerprint)?;

Preferred higher-level search:

let results = indexer.search_exact_position(&query)?;

The higher-level form should return common SearchResult values rather than index-specific internal matches.

Searching from a known game position should also be supported:

let results = indexer.find_matches_from_game(game_id, move_number)?;
Pattern Search

Pattern search finds rectangular arrangements of stones within replayed or indexed positions.

The pattern API currently uses:

Pattern;
PatternSearcher;
PatternMatch;
PatternSearchGame.

The long-term aim is to integrate pattern search with the common search result model.

Illustrative usage:

let searcher = PatternSearcher::new();
let matches = searcher.search_database(&indexer, &pattern)?;

A future preferred interface may be:

let results = indexer.search_pattern(&query)?;

or:

let results = searcher.search(&indexer, &query)?;

The final choice should avoid creating multiple competing entry points for the same operation.

Pattern Search Queries

Pattern-search options should be represented by a query type rather than by a growing list of function arguments.

Illustrative design:

pub struct PatternSearchQuery {
    pub pattern: Pattern,
    pub game_ids: Option<Vec<i64>>,
    pub include_rotations: bool,
    pub include_reflections: bool,
    pub colour_mode: ColourMode,
    pub board_region: BoardRegion,
}

Possible future enums include:

pub enum ColourMode {
    Exact,
    SwapColours,
    Relative,
}

pub enum BoardRegion {
    Anywhere,
    Corner,
    Side,
    Centre,
    WholeBoard,
}

These names are provisional.

A query object will be easier to extend than a method with many positional arguments.

Search Result Metadata

Search methods should normally return preferred game metadata with each grouped result.

This avoids requiring every client to perform a second metadata query.

However, lower-level methods returning only IDs and occurrences may remain available for performance-sensitive operations.

The API should make the distinction clear:

find_exact_position(...)
find_exact_position_with_metadata(...)

or, preferably:

search_exact_position(...)
search_exact_position_ids(...)

The naming should be reviewed before the stable API is declared.

Search Scope

Search operations should support restricting the search to:

the entire project;
one game;
a specified set of game IDs;
a filtered game query;
a date, player or tournament subset.

Search filtering should use shared query types rather than duplicate game-filtering logic in each search engine.

Statistics API

Statistics are planned but are not yet part of the stable public API.

Future statistics may include:

number of matching games;
number of matching occurrences;
next-move frequency;
player win rates;
black and white results;
date ranges;
tournament breakdowns;
pattern popularity.

Statistics should be calculated from explicit query inputs and should not be embedded implicitly into every search result.

Possible future design:

let statistics = indexer.pattern_statistics(&query)?;
Error Handling

Public operations should return:

anyhow::Result<T>

during early development.

Before a stable library release, Bermuda should consider introducing a public error type:

pub enum BermudaError {
    InvalidProject,
    UnsupportedVersion,
    Database,
    Io,
    Sgf,
    InvalidMove,
    InvalidPattern,
    Index,
}

A library-specific error type would allow applications to distinguish user errors from internal failures without parsing error strings.

The CLI may continue using anyhow at the application boundary.

Ownership and Borrowing

Public APIs should avoid unnecessary cloning of:

game records;
board positions;
patterns;
metadata strings.

Large result sets should support iterators or streaming where practical.

Simple convenience methods returning Vec<T> may remain available for GUI and CLI clients.

Threading

The public API currently assumes ordinary synchronous use.

The Qt application may perform importing, indexing and large searches on worker threads.

Public service types should therefore avoid hidden global state.

Whether Project, PositionIndexer and search services are Send or Sync should be documented explicitly once the GUI threading model is established.

A single SQLite connection should not be assumed to be safely shared between arbitrary threads.

API Layers

The public library may eventually expose two layers.

High-Level API

Designed for applications:

open a project;
import games;
list games;
replay positions;
build indexes;
perform searches;
retrieve grouped results with metadata.
Low-Level API

Designed for maintenance tools and advanced clients:

compact move-file access;
raw fingerprints;
individual index records;
explicit index versions;
database-root construction;
ungrouped search matches.

The high-level API should be the recommended interface in documentation and examples.

API Stability

Before declaring version 1.0, the following should be completed:

review all public modules;
remove accidental public implementation details;
standardise constructor names;
standardise search result types;
standardise metadata field names;
decide which methods are high-level and low-level;
introduce a stable error model;
document threading expectations;
add library usage examples;
test the API from a separate client crate.

Until then, public APIs may change between 0.x releases.

Example Application Flow

A typical application should eventually be able to use Bermuda approximately as follows:

use bermuda::{
    PatternSearchQuery,
    PositionIndexer,
    ProjectManager,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project = ProjectManager::open("/path/to/project")?;
    let mut indexer = PositionIndexer::open_project(&project)?;

    indexer.build_pending_index()?;

    let position = indexer.replay_board_position(42, 120)?;

    let query = PatternSearchQuery::from_position(&position, 3, 3)?;
    let results = indexer.search_pattern(&query)?;

    for result in results {
        println!(
            "{} vs {}: {} occurrence(s)",
            result.black_player.as_deref().unwrap_or("Unknown"),
            result.white_player.as_deref().unwrap_or("Unknown"),
            result.occurrences.len(),
        );
    }

    Ok(())
}

This example is aspirational. It describes the intended simplicity of the high-level API, not necessarily the exact signatures currently implemented.

Immediate Design Tasks

The next API-design work should be:

Finalise SearchOccurrence.
Finalise SearchResult.
Convert exact-position results into the common result model.
Convert pattern-search results into the common result model.
Introduce a pattern-search query type.
Decide whether search methods belong directly on PositionIndexer or on dedicated search service types.
Review public error handling.
Add examples that compile as tests.
Open Questions

The following decisions remain unresolved.

Central Service or Separate Services

Should PositionIndexer remain the primary entry point for replay, indexing and search, or should Bermuda expose separate types such as:

GameRepository
PositionIndexer
SearchEngine
PatternSearcher

A single central service is easier for clients.

Separate services provide clearer responsibilities and may be easier to test and evolve.

Metadata in Search Results

Should all normal search methods include game metadata automatically, or should metadata loading always be optional?

Streaming Results

Should large searches return iterators, callbacks, channels or collected vectors?

Search Query Types

Should exact-position and pattern searches use separate query types, or a common search enum?

Public Index Versions

Should applications pass explicit index versions, or should this remain an internal maintenance concern?

These questions should be resolved before the Qt application becomes dependent on the API.

Conclusion

The purpose of the Bermuda public API is to provide a stable boundary between the database engine and its user interfaces.

The CLI, Qt application and any future external tools should depend on this public API rather than on internal storage details.

The immediate priority is to unify search results and make the common application workflows simple, explicit and well documented.
