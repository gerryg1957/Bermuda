# Database Design

## Status

This document describes the logical and physical design of the Bermuda database.

It explains how canonical games, metadata, sources and indexes are stored and why the database has been structured in this way.

This document describes the design rather than a specific implementation. Internal details may evolve while preserving the overall architecture.

---

# Design Goals

The database has five primary goals.

- Store canonical professional Go games.
- Support efficient metadata queries.
- Support fast position and pattern searches.
- Preserve provenance from multiple data sources.
- Scale to hundreds of thousands of games.

---

# Architectural Overview

Bermuda separates storage into two parts.

```
Project
│
├── metadata.sqlite3
│
├── games/
│      *.moves
│
└── indexes/
```

Each part has a different responsibility.

---

# Metadata Database

SQLite stores structured information that is frequently queried.

Examples include:

- players
- events
- dates
- results
- source information
- canonical game identifiers
- index metadata

SQLite is chosen because it provides:

- reliability
- transactions
- SQL querying
- excellent performance for metadata

Large move sequences are intentionally stored elsewhere.

---

# Move Files

Every canonical game is stored as a compact move file.

Move files contain:

- board size
- setup stones
- move sequence

Move files deliberately exclude metadata that already exists in SQLite.

Advantages include:

- compact storage
- sequential replay
- simple versioning
- easy backup

---

# Canonical Games

Every unique game has one canonical representation.

Canonical identity is determined by:

- board size
- initial position
- move sequence

Metadata does not affect canonical identity.

Different SGF files representing the same game therefore share one canonical game record.

---

# Sources

Games may originate from multiple collections.

Examples include:

- GoGoD
- Go4Go
- EGF archives

The database records:

- source name
- source version
- original filename
- import date

Multiple sources may refer to the same canonical game.

---

# Position Index

The position index accelerates exact position searches.

Each indexed position stores:

- game identifier
- move number
- position fingerprint

Fingerprints are derived from reconstructed board positions.

The index can always be rebuilt from the canonical games.

---

# Pattern Search

Pattern searches currently replay indexed games.

The architecture deliberately separates the search API from the search implementation.

Future implementations may introduce specialised pattern indexes without changing the public API.

---

# Metadata Relationships

Conceptually the database contains relationships such as:

```
Source
    │
    ├── Imported Games
    │
Canonical Game
    │
    ├── Metadata
    ├── Move File
    └── Indexed Positions
```

Canonical games form the centre of the database.

---

# Normalisation

Metadata should avoid unnecessary duplication.

For example:

- player names
- event names
- tournament names

may be normalised into separate tables where appropriate.

Large binary data should never be duplicated.

---

# Transactions

Imports should be transactional.

A successful import updates:

- metadata
- source information
- move files
- indexes

If an error occurs, the project should remain internally consistent.

---

# Rebuildability

All derived data should be rebuildable.

Examples include:

- position indexes
- statistics
- search caches

Only canonical games and metadata are considered authoritative.

---

# Performance Principles

Optimisation should focus on:

- metadata queries
- replay speed
- position indexing
- search throughput

Storage efficiency is important but secondary to correctness and maintainability.

---

# Future Extensions

The design allows future support for:

- specialised pattern indexes
- statistics caches
- joseki databases
- fuseki databases
- AI analysis
- opening books

These should be implemented as additional derived structures rather than changes to the canonical game representation.

---

# Guiding Principles

The database should remain:

- canonical
- rebuildable
- scalable
- maintainable
- implementation-independent

Changes to indexing or search algorithms should never require changes to the canonical game representation.
