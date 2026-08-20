# Domain Model

## Status

This document defines the fundamental concepts used throughout Bermuda.

Unlike implementation documents, the domain model describes *what the system represents* rather than *how it is implemented*.

Every public API, database schema and user interface should be understandable in terms of these domain objects.

---

# Purpose

Bermuda manages collections of professional Go games.

Its primary purpose is to import, organise, replay and search those games.

The domain model identifies the concepts required to achieve those goals and defines the responsibility of each.

---

# Domain Overview

```text
Project
    │
    ├── Source
    │
    ├── Game
    │      │
    │      ├── Move
    │      ├── Position
    │      └── Metadata
    │
    ├── Pattern
    │
    ├── SearchQuery
    │
    ├── SearchResult
    │      │
    │      └── SearchOccurrence
    │
    └── Statistics
```

---

# Project

A `Project` represents one Bermuda database.

It owns:

- metadata database
- move files
- temporary files
- configuration
- index versions

A project provides access to all other domain objects.

A project does **not** contain search results or replay state.

---

# Source

A `Source` represents the origin of imported games.

Examples include:

- GoGoD
- Go4Go
- European Go Federation
- British Go Association

A canonical game may exist in multiple sources.

A source owns:

- source name
- version
- import date

A source does not own games.

---

# Game

A `Game` is the central domain object.

It represents one canonical game of Go.

A game owns:

- setup stones
- move sequence
- metadata
- board size

A game never owns:

- indexes
- search results
- statistics

---

# Metadata

Metadata describes a game but is not the game itself.

Typical metadata includes:

- players
- event
- round
- date
- result
- rules
- komi

Metadata may vary between sources.

The project chooses one preferred metadata record for display.

---

# Move

A `Move` represents one action in a game.

A move records:

- colour
- coordinate
- pass (where applicable)

Moves never know about search, replay or statistics.

---

# Position

A `Position` represents the complete board state after a specific move.

It owns:

- board contents
- side to move
- ko point
- move number

Positions are reconstructed by replaying moves.

Positions never own metadata.

---

# Pattern

A `Pattern` represents a rectangular arrangement of intersections.

A pattern owns:

- width
- height
- intersection contents

A pattern never knows:

- where it came from
- which games match it
- search options

Patterns are reusable search objects.

---

# SearchQuery

A `SearchQuery` describes what the user wishes to find.

Examples include:

- exact position
- pattern
- player
- tournament
- date range

Queries describe the search but never execute it.

---

# SearchResult

A `SearchResult` represents one matching game.

It owns:

- game identifier
- preferred metadata
- matching occurrences

It never owns replay state.

---

# SearchOccurrence

A `SearchOccurrence` represents one match within one game.

It records:

- move number
- side to move
- ko point
- pattern coordinates (where applicable)

A game may contain many occurrences.

---

# Statistics

Statistics are derived information.

Examples include:

- frequency
- win rate
- player records
- tournament summaries
- opening popularity

Statistics are calculated from games.

They are not part of the canonical game itself.

---

# Relationships

```
Project
    owns
        Sources
        Games

Game
    owns
        Moves
        Metadata

Position
    derived from
        Game

Pattern
    searched by
        SearchQuery

SearchQuery
    executed by
        SearchEngine

SearchEngine
    returns
        SearchResult

SearchResult
    contains
        SearchOccurrence
```

---

# Ownership Rules

Each concept should have one clear owner.

For example:

- a move belongs to exactly one game;
- a position is derived from one game;
- a search result never becomes part of a game;
- statistics are derived rather than stored.

Avoid duplicating responsibility across multiple objects.

---

# Guiding Principle

Whenever a new feature is added, ask:

> Which domain object owns this concept?

If the answer is unclear, the design should be reconsidered before implementation.

The domain model should remain small, stable and independent of implementation details.
