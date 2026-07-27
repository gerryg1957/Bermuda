MoyoDB progress report — 27 July 2026
Architecture

MoyoDB now has a project-centred library architecture:

ProjectManager
    ↓
Project
    ├── GameCatalogue
    ├── GameStore
    ├── Importer
    └── PositionIndexer

The CLI works with project directories rather than exposing the internal database layout.

Game catalogue

A project-aware catalogue API is now available:

let catalogue = project.catalogue()?;

let games = catalogue.list(&query)?;
let count = catalogue.count(&query)?;
let game = catalogue.get(game_id)?;

Implemented catalogue features:

stable multi-column sorting;
pagination using limit and offset;
exact player filtering;
Black, White, or either-colour player searches;
inclusive date-range filtering;
result filtering for Black wins, White wins, jigo/draws, and void games;
filtered counts that ignore pagination;
retrieval of one canonical game’s selected metadata;
clear errors for unknown game IDs.

Result conventions are based on the imported data:

Black win  B+...
White win  W+...
Jigo       Jigo..., Draw, or 0
Void       Void...
Game retrieval

A project-aware GameStore has been added:

let store = project.game_store()?;
let record = store.load(game_id)?;

It:

finds the compact move file through the database;
resolves its path relative to the project database root;
reads and returns a complete GameRecord;
reports unknown game IDs;
reports missing or unreadable move files with game-specific context.
Removal of duplicated logic

Game-record loading is now centralised in one internal implementation:

GameStore::load()
        ↓
load_game_record()

PositionIndexer::read_game_by_id()
        ↓
load_game_record()

PositionIndexer::replay_board_position() now uses read_game_by_id() rather than repeating the database and move-file lookup.

PositionIndexer::game_by_id() remains because indexing operations still require a GameToIndex containing the move-file path.

Terminology

MoyoDB-owned Rust terminology now consistently uses British spelling:

Colour
InvalidColour
parse_colour
colour_at
colour_byte

External dependency names and QML APIs retain their required color spelling.

Quality and testing

The work is covered by tests for:

catalogue listing;
metadata selection;
player filters;
date filters;
result filters;
filtered counts;
single-game catalogue lookup;
project-aware game loading;
unknown game IDs;
missing move files;
existing position-index and replay behaviour.

The complete test suite and Clippy checks pass.

Next stage

The next planned feature is a position-at-move API on GameStore:

let store = project.game_store()?;
let position = store.position_at(game_id, move_number)?;

Planned semantics:

move 0 is the position after setup stones;
move 1 is the position after the first move;
passes advance the move number;
recorded move colours remain authoritative;
out-of-range move numbers produce a clear error.

After that, the likely sequence is:

catalogue CLI commands;
project statistics;
richer loaded-game API combining catalogue metadata and GameRecord;
Qt project browser and game list;
Qt board display using the position-at-move API.
