# MoyoDB Algorithms

## Purpose

This document describes the algorithms used throughout MoyoDB.
It complements `architecture.md`, which explains how the software is
organised. This document explains *how the individual algorithms work*.

Algorithms should be described independently of their Rust implementation
where possible.

---

# Contents

1. Board Representation
2. Move Application
3. Capture Detection
4. Liberty Search
5. Ko Detection
6. Replay Engine
7. Canonical Game Hashing
8. Position Hashing
9. Position Indexing
10. Pattern Representation
11. Pattern Extraction
12. Pattern Matching
13. Pattern Search
14. Future Algorithms

---

# 1. Board Representation

## Goal

Represent a Go board efficiently while allowing fast move generation,
capture detection and replay.

## Current implementation

- Fixed-size board.
- Intersections stored in a contiguous array.
- Each intersection stores:
  - Empty
  - Black
  - White

### Complexity

- Memory: O(board size²)
- Read: O(1)
- Write: O(1)

---

# 2. Move Application

## Goal

Apply a legal move to the board.

### Steps

1. Verify legality.
2. Place stone.
3. Merge friendly groups.
4. Remove captured enemy groups.
5. Detect suicide.
6. Update ko.
7. Record move.

---

# 3. Capture Detection

Describe the flood-fill algorithm used to determine captured chains.

---

# 4. Liberty Search

Describe the depth-first search used to determine liberties.

---

# 5. Ko Detection

Describe simple ko.

---

# 6. Replay Engine

Explain reconstruction of board positions from compact move files.

---

# 7. Canonical Game Hashing

Describe:

- domain separator
- canonical format
- setup stones
- moves
- exclusions
- SHA-256

---

# 8. Position Hashing

Describe the position hash used by the position index.

---

# 9. Position Indexing

Explain:

- replay
- hashing
- SQLite index

---

# 10. Pattern Representation

Describe:

Pattern

PatternCell

PatternRect

BoardEdges

---

# 11. Pattern Extraction

Explain how a rectangular region is extracted.

---

# 12. Pattern Matching

Explain:

matches_at()

comparison algorithm

boundary handling

---

# 13. Pattern Search

Describe the current brute-force algorithm.

Pseudo-code:

for every game

    replay

    for every position

        for every legal rectangle

            compare

---

Complexity

O(G × P × B)

where

G = games

P = positions

B = board locations

---

# 14. Future Algorithms

Planned:

- wildcard matching
- colour inversion
- rotations
- reflections
- continuation search
- indexed pattern search
- bitboard optimisation
- SIMD comparison
