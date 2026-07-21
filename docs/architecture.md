# MoyoDB Database Design

**Version:** Draft 1
**Status:** Agreed design principles
**Date:** July 2026

---

# 1. Purpose

MoyoDB is a database of professional Go games.

It is **not** an SGF archive.

The primary objective is to provide extremely fast searching of professional games by board position, while preserving information about the origin of every game.

---

# 2. Design Philosophy

The project is built around one fundamental principle:

> **The database stores games, not SGF files.**

An SGF file is simply one representation of a game.

Multiple SGF files may describe exactly the same game.

The database should recognise this and store only one canonical representation of the game.

---

# 3. Definition of a Game

For the purposes of MoyoDB, a game consists of:

* board size
* initial board position
* complete move sequence

Everything else is metadata.

Metadata includes:

* player names
* event
* round
* date
* result
* comments
* annotations
* filename
* SGF formatting
* property ordering
* whitespace

None of these contribute to the identity of the game.

---

# 4. Canonical Game Identity

Each imported game receives a canonical hash.

The canonical hash is calculated from:

* format version
* board size
* canonical initial position
* complete move sequence

The following must *not* influence the canonical hash:

* comments
* labels
* marks
* SGF property order
* whitespace
* character encoding
* source filename

Two SGF files describing the same game must produce the same canonical hash.

---

# 5. Stable Internal Game IDs

Every unique game receives a permanent internal identifier.

Example:

```
Game ID: 47291
```

This identifier never changes.

All future indexes reference this identifier rather than filenames.

Examples include:

* pattern indexes
* joseki indexes
* fuseki statistics
* search results

---

# 6. Supported Board Sizes

The core library supports any legal board size.

This keeps the library reusable and simplifies testing.

The professional database imports only:

* 19×19 games

The importer skips:

* 9×9
* 13×13
* any other board size

Skipped games are reported but are not treated as errors.

---

# 7. Sources

A source identifies where a game originated.

Examples:

* GoGoD
* go4go

A source records:

* name
* version
* import date

Future sources may be added without changing the database design.

---

# 8. Source Metadata

Metadata belongs to the source that supplied it.

Metadata is **never merged automatically**.

If GoGoD and go4go disagree, both versions are preserved.

Example:

```text
Game 47291

GoGoD
------
Event = Honinbo League

go4go
------
Event = Honinbo
```

The database preserves every imported source record exactly as supplied.

When browsing the database, the application presents a single row for each canonical game rather than one row per source. A representative metadata record is selected for display in the game list to provide a concise overview.

All source-specific metadata remains available through the game details view, allowing users to inspect every imported version of a game without introducing duplicate entries into the main browser.

No imported information is discarded or merged automatically.

---

# 9. Database Schema

## games

Stores one record for each unique game.

Fields:

* id
* canonical_hash
* board_size
* move_count
* move_file

---

## sources

Stores information about imported collections.

Fields:

* id
* name
* version
* imported_at

---

## game_sources

Links games to one or more sources.

Fields:

* id
* game_id
* source_id
* original_path
* imported_at

---

## game_metadata

Stores metadata exactly as supplied by each source.

Fields:

* game_source_id
* black_player
* white_player
* event
* round
* date
* result
* komi
* handicap
* rules
* time_limit

---

# 10. Move Files

Each canonical game has exactly one compact move file.

Multiple sources may refer to the same move file.

Move files contain only information required to reconstruct the game.

---

# 11. Import Process

The import process is:

1. Read SGF.
2. Parse SGF.
3. Extract main variation.
4. Validate the game.
5. Reject unsupported board sizes.
6. Generate canonical game representation.
7. Compute canonical hash.
8. Check for existing game.
9. If new:

   * write move file
   * insert game record
10. Record source metadata.

Duplicate games are recognised by canonical hash.

---

# 12. Pattern Searching

Pattern searching is the primary purpose of the database.

Pattern indexes reference:

* Game ID
* move number
* rotation
* reflection

Pattern searches never parse SGF files.

All searching is performed using the indexed game representation.

---

# 13. Application Architecture

The desktop application is organised into four distinct layers.

```text
Kirigami / QML
        │
Rust Qt Models
        │
moyodb
        │
SQLite metadata database + move files
```

Each layer has a clearly defined responsibility.

## Kirigami / QML

Responsible for presentation only.

* Displays the game browser.
* Displays the Go board.
* Presents dialogs and user interaction.
* Never accesses SQLite directly.

## Rust Qt Models

Provide the bridge between the user interface and the core library.

* Expose game lists to QML.
* Notify the interface when data changes.
* Translate user actions into library operations.
* Reuse the same models for browsing, filtering and future pattern-search results.

## moyodb

Implements the application logic.

* Owns all database access.
* Maintains canonical game identity.
* Performs sorting, filtering and searching.
* Selects representative metadata for game browsing.
* Returns canonical game records to the user interface.

## SQLite Database and Move Files

Provide persistent storage.

* Store canonical game records.
* Preserve source provenance.
* Store per-source metadata.
* Store compact move files.
* Maintain position indexes for fast searching.

This separation keeps the user interface independent of the storage layer while ensuring that all applications built on MoyoDB share the same tested core library and database implementation.



---

# 14. Future Extensions

The design allows future additions without changing the core model.

Possible future tables include:

* players
* events
* tournaments
* joseki
* fuseki
* patterns
* pattern_hits

All future tables reference the stable internal Game ID.

---

# 15. Non-Goals

MoyoDB is not intended to:

* preserve SGF formatting
* act as an SGF editor
* preserve comments as part of game identity
* distinguish games by filename
* store duplicate copies of identical games

---

# 16. Guiding Principle

When in doubt, prefer designs that improve:

* correctness
* reproducibility
* search performance
* preservation of source information

over designs that optimise for preserving the original SGF representation.

The database is a long-term research tool for professional Go games. Every design decision should support that objective.
