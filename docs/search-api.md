# Search API Design

## Status

This document describes the intended architecture of the MoyoDB search system.

It is a design document and does not necessarily reflect the exact implementation at the current stage of development.

---

# Purpose

Searching is the primary purpose of MoyoDB.

Every feature of the library ultimately exists to make searching professional Go games easier, faster and more flexible.

The search API should provide a single, consistent interface for every search supported by MoyoDB.

Applications should not need to know how a search is implemented internally.

---

# Design Principles

The search API should satisfy the following principles.

## One Search Model

Every search should follow the same conceptual model:

```
Query
    ↓
Search Engine
    ↓
Search Results
```

Regardless of whether the search is:

- exact position
- pattern
- joseki
- fuseki
- player
- tournament
- statistics

the client should receive results in a consistent form.

---

## Queries Describe What

A query describes **what** is being searched.

Examples include:

- an exact board position
- a rectangular pattern
- a player name
- an event
- a date range

A query should contain no implementation details.

---

## Search Engines Describe How

The search engine determines **how** a query is executed.

For example:

- replaying positions
- using fingerprints
- scanning indexes
- filtering metadata

Clients should not depend upon the chosen implementation.

---

# Search Architecture

```
Application
      │
      ▼
SearchQuery
      │
      ▼
SearchEngine
      │
      ▼
SearchResult
      │
      ▼
SearchOccurrence
```

---

# Search Queries

Each search should be represented by a query object.

Examples:

```
ExactPositionQuery
PatternSearchQuery
PlayerQuery
TournamentQuery
JosekiQuery
```

This avoids functions with long parameter lists and allows future options to be added without changing method signatures.

---

# Search Results

Every search should ultimately return:

```rust
Vec<SearchResult>
```

A `SearchResult` represents one game.

A game may contain multiple matching positions.

---

# Search Occurrences

Each `SearchResult` contains one or more occurrences.

An occurrence records where the match was found.

Typical fields include:

- move number
- side to move
- ko point
- board coordinates (where applicable)

---

# Search Types

## Exact Position Search

Finds identical board positions.

Uses the position index.

Returns grouped `SearchResult` values.

---

## Pattern Search

Finds rectangular board patterns.

Initially implemented by replaying indexed games.

Future versions may use specialised indexes.

---

## Joseki Search

Future feature.

Searches opening sequences.

May combine move order and position information.

---

## Fuseki Search

Future feature.

Searches whole-board opening patterns.

---

## Metadata Search

Future feature.

Searches:

- player
- tournament
- date
- result

May be combined with position searches.

---

# Combining Searches

Queries should eventually be composable.

For example:

```
Pattern
    AND
Black Player = Go Seigen
    AND
Date < 1950
```

The search API should support combining search criteria without requiring separate search functions for every combination.

---

# Responsibilities

## PositionIndexer

Responsible for:

- replay
- fingerprints
- position reconstruction
- index maintenance

It should not become responsible for every future search algorithm.

---

## SearchEngine

Responsible for:

- executing searches
- grouping results
- combining filters
- returning common search results

It should become the primary public interface for searching.

---

# Performance

The search API should hide implementation details while allowing different search strategies.

Possible implementations include:

- replay
- fingerprint indexes
- specialised pattern indexes
- cached statistics

Applications should not need to change when search algorithms improve.

---

# Future Extensions

The design should support:

- rotations
- reflections
- colour-independent matching
- wildcard intersections
- influence searches
- AI-assisted searches

without changing the fundamental API.

---

# Example

```rust
let query = PatternSearchQuery::from_position(...)?;

let engine = SearchEngine::new(&project)?;

let results = engine.search_pattern(&query)?;
```

The application should not know whether the search used replay, indexing or another internal algorithm.

---

# Long-Term Goal

The search API should become the defining feature of MoyoDB.

Every search—current and future—should feel like a natural extension of the same interface.

The implementation may evolve, but the public search model should remain stable.
