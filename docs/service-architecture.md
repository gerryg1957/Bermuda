# Domain Model

## Status

This document defines the core domain model of Bermuda.

The domain model describes the concepts represented by the system. It is independent of storage, indexing, user interfaces and implementation details.

---

# Purpose

Bermuda is a database for professional Go games.

Its responsibilities are to:

- import games from multiple sources;
- maintain a canonical representation of each game;
- reconstruct board positions;
- search games efficiently;
- present search results through a stable public API.

The domain model identifies the concepts required to achieve these goals.

---

# Design Principles

The domain model follows four principles.

## Single Responsibility

Every concept has one clear owner.

## Stable Concepts

Domain objects represent concepts that rarely change.

## Separation of Concerns

The domain model is independent of services, storage and user interfaces.

## Explicit Ownership

Relationships between objects should always be clear.

---

# Domain Overview

```text
                   Project
                      │
         ┌────────────┼────────────┐
         │            │            │
      Sources       Games      Statistics
                      │
          ┌───────────┴───────────┐
          │                       │
      Metadata                 Moves
                                  │
                              Positions

Pattern ─────► SearchQuery ─────► SearchEngine
                                      │
                                      ▼
                               SearchResult
                                      │
                                      ▼
                              SearchOccurrence
```

---

# Domain Entities

Entities have a persistent identity.

## Project

Represents one Bermuda database.

Owns:

- configuration
- metadata database
- move files
- indexes
- import history

Provides access to every other domain object.

---

## Game

Represents one canonical Go game.

Owns:

- move sequence
- setup stones
- metadata
- board size

Does not own:

- search results
- indexes
- statistics

Identity never changes.

---

## Source

Represents an imported collection.

Examples:

- GoGoD
- Go4Go
- EGF

Stores:

- source name
- version
- import date

A canonical game may appear in multiple sources.

---

# Value Objects

Value objects describe things but have no independent identity.

## Metadata

Describes a game.

Contains:

- players
- event
- date
- result
- rules
- komi

---

## Move

Represents one move.

Contains:

- colour
- coordinate
- pass

Immutable after creation.

---

## Position

Represents one reconstructed board state.

Contains:

- board
- side to move
- ko point
- move number

Derived from replay.

---

## Pattern

Represents a rectangular arrangement of intersections.

Contains:

- width
- height
- intersection values

Does not know:

- game IDs
- search options
- coordinates within a game

---

## SearchQuery

Describes what should be searched.

Examples include:

- exact position
- pattern
- player
- tournament

Never performs the search.

---

## SearchResult

Represents one matching game.

Contains:

- game identifier
- preferred metadata
- matching occurrences

---

## SearchOccurrence

Represents one individual match.

Contains:

- move number
- side to move
- ko point
- board coordinates (if applicable)

---

## Statistics

Represents derived information.

Examples include:

- win rates
- opening frequencies
- player statistics
- tournament summaries

Always derived.

Never part of a canonical game.

---

# Domain Services

Services perform work on domain objects.

Examples include:

- Importer
- PositionIndexer
- SearchEngine
- GameRepository
- ProjectManager

Services manipulate domain objects but are not themselves part of the domain model.

---

# Relationships

```text
Project
    owns Sources
    owns Games

Game
    owns Metadata
    owns Moves

Position
    derived from Game

Pattern
    used by SearchQuery

SearchQuery
    executed by SearchEngine

SearchEngine
    returns SearchResult

SearchResult
    contains SearchOccurrences

Statistics
    derived from Games
```

---

# Ownership Rules

Every concept has exactly one owner.

Examples:

- a Move belongs to one Game;
- a Position is derived from one Game;
- a SearchResult is never stored inside a Game;
- Statistics are derived, never canonical.

Avoid duplicated responsibility.

---

# Architectural Rule

Whenever adding a feature, ask:

> Which domain object owns this concept?

If the answer is unclear, the design should be reconsidered.

---

# Long-Term Goal

The domain model should remain stable even as:

- search algorithms improve;
- storage formats evolve;
- new indexes are introduced;
- the Qt application grows;
- additional APIs are added.

Implementation may change.

The domain concepts should not.
