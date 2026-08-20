# Bermuda Developer Handbook (Working Draft)

## Purpose

This document records the design decisions, architecture, milestones,
and roadmap for the Bermuda project as developed during our ChatGPT
sessions. It is intended to live alongside the source code (for example
in `docs/developer-handbook.md`) and evolve as the project grows.

------------------------------------------------------------------------

# Project Vision

Bermuda is a native Rust Go (Baduk/Weiqi) database aimed at
professional-scale collections such as GoGoD and go4go.

Primary goals:

-   Native Linux implementation.
-   Fast SGF import.
-   Compact game storage.
-   SQLite metadata.
-   Exact position search.
-   Local pattern search.
-   Long-term support for wildcard patterns, symmetry, and continuation
    search.

------------------------------------------------------------------------

# Major Milestones Completed

## Core infrastructure

-   SGF parser.
-   Compact move file format.
-   Replay engine.
-   Board implementation.
-   Capture handling.
-   Ko handling.
-   Pass handling.
-   Canonical game hashing.
-   SQLite-backed project layout.

## Database

-   Project manager.
-   Import of single SGF.
-   Directory import.
-   Metadata storage.
-   Replay from database.

## Exact Position Search

Implemented:

-   Position replay.
-   SHA-256 fingerprints.
-   Position index.
-   Position lookup by fingerprint.

## Pattern Search

Implemented:

-   Pattern extraction.
-   Rectangle validation.
-   Edge detection.
-   Pattern matching.
-   PatternSearcher.
-   Single-game brute-force search.
-   CLI command `search-pattern`.

Verified manually:

-   Self-match within the same game.
-   No false positives across different games.
-   Repeated matches while a local region remains unchanged.

------------------------------------------------------------------------

# Current Architecture

-   board
-   replay
-   pattern
-   pattern_search
-   indexer
-   database
-   project_manager
-   CLI

Each module has a single responsibility.

------------------------------------------------------------------------

# Design Principles

-   Prefer correctness before optimisation.
-   Keep replay logic in one place.
-   Pattern objects know nothing about databases.
-   Searchers know nothing about SGF parsing.
-   Optimise only after measuring.

------------------------------------------------------------------------

# Current Status

Completed:

-   SGF import
-   Replay
-   Position indexing
-   Exact position search
-   Pattern extraction
-   Pattern matching
-   Single-game pattern search

Planned:

1.  Refactor PatternSearcher internals.
2.  Add automated tests.
3.  Search entire database.
4.  Improve CLI and output.
5.  Wildcards.
6.  Symmetry.
7.  Optimised indexing.

------------------------------------------------------------------------

# Notes

The project has intentionally been developed in small, tested
increments. The brute-force pattern search is retained as the reference
implementation against which future optimisations can be validated.

This handbook is intended to be expanded after each major development
session.
