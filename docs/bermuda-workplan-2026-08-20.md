# Bermuda Workplan — 20 August 2026

## Current position

Bermuda is now the project name throughout the active codebase, Qt application, documentation, local repository and GitHub repository. Legacy MoyoDB storage and format identifiers remain in place where necessary for compatibility with existing databases and indexes.

The core professional-game study workflow is working: browse games, select a position or pattern, search the professional corpus, inspect matching games and continuations, and use professional precedent as evidence rather than as an engine evaluation.

The project-wide pattern search now:
- searches freely across board locations rather than being pinned to the source rectangle's exact board coordinates;
- excludes handicap games by default while keeping them in the database;
- uses the conservative geometric **Bermuda** heuristic for long, shallow edge-oriented selections;
- preserves exact stones and empty intersections as information;
- groups multiple occurrences under one game and exposes the number in the **Matches** column.

The Bermuda heuristic should now be treated as good enough unless normal use produces a concrete counterexample. It is deliberately geometric, not a model of fuseki or Go strategy.

## Immediate work

### 1. Settings

Add an application Settings facility.

First setting:

**Include handicap games in pattern searches**
- default: Off;
- when Off, project-wide pattern searches exclude any canonical game for which any source metadata reports a positive handicap;
- when On, those games are included;
- handicap games remain present in the database regardless of this setting.

This should expose the behaviour already implemented in the search core rather than adding a second filtering mechanism.

### 2. Search Results ordering

Give **Search Results** the same useful ordering capabilities as **Game database**.

Important sortable fields include:
- Black;
- White;
- Date;
- Result;
- Event;
- Matches.

This is especially useful for research: finding the earliest examples, following changes in professional practice over time, investigating particular players or events, and spotting games with multiple occurrences.

### 3. Continue ordinary Go use

Use Bermuda for real study rather than constructing synthetic tests unless a problem appears.

In particular:
- exercise small tactical patterns as well as opening/fuseki selections;
- note cases where Bermuda hides a useful precedent or allows a geometrically misleading one;
- note workflow friction in Search Results, continuation selection, replay and database browsing;
- prefer concrete examples from actual use over speculative rule refinement.

Do not continue tuning the Bermuda thresholds merely to optimise result counts.

## Next design work

### 4. Player identity and aliases

Design a non-destructive player-identity layer.

The imported source names should remain untouched, but one real player should be able to have a preferred display name and multiple aliases, for example differing Korean/Japanese romanisations.

Likely model:
- `player_id`;
- preferred/canonical display name;
- source aliases;
- curated confirmation before aliases are merged.

Benefits:
- reliable player search;
- consistent sorting and display;
- better player statistics and historical research;
- preservation of original source metadata.

Do not automatically merge people merely because names look similar.

### 5. Fuseki filter — separate from Bermuda

Keep this as a distinct future feature.

The idea emerged while experimenting with Bermuda: once the program starts asking whether stones were really part of the opening, when they were played, or whether a shape is Chinese, mini-Chinese, san-ren-sei, etc., it is no longer doing geometric filtering.

A future **Fuseki filter** could explicitly help answer questions such as:
- show this pattern when it occurred as part of the opening rather than as a later tactical coincidence;
- compare treatments of an opening formation across periods;
- distinguish surviving opening stones from later stones occupying typical opening coordinates.

Do **not** put this knowledge into Bermuda. Bermuda remains a simple geometric suppression heuristic.

The design should be considered carefully because move number alone is insufficient: an opening stone may remain on the board into the middle game, while a later stone may occupy a normal opening coordinate.

## Subsequent major work

### 6. Personal-game corpus and analysis architecture

Design the architecture for a separate corpus/database of the user's own games.

The long-term purpose is collective analysis rather than merely replay:
- pattern-search recurring situations across personal games;
- identify recurring mistakes or habits;
- compare personal continuations with professional-game precedents;
- optionally use KataGo later as a complementary analytical source.

Keep professional evidence and engine evaluation conceptually distinct.

Architecture/design comes before implementation.

### 7. CI and packaging

After the personal-games/KataGo architecture is clear:
- establish CI;
- verify clean builds/tests automatically;
- prepare Linux packaging;
- review desktop integration, application metadata and icons;
- retain the possibility of Windows/macOS later without treating them as currently supported platforms.

## Release-readiness work

When ordinary use stops exposing important functional problems:

- review current documentation under the Bermuda name;
- add a short compatibility note explaining retained `MOYODB-*`, `moyodb-project.toml`, existing data-directory and settings identifiers;
- review experimental/development CLI commands and remove or clearly label those not intended for ordinary users;
- confirm database/index versioning and migration expectations;
- review backup documentation, which still records the former MoyoDB paths and commands;
- make a first meaningful Bermuda release.

## Deliberately deferred

Do not turn Bermuda into a cut-price KataGo.

For now, defer:
- automatic move ranking or verdicts;
- score estimation;
- whole-board strategic evaluation;
- automatic claims that recent professional play is better;
- automatic strategic classification based only on pattern density or opening coordinates;
- further Bermuda threshold tuning without a concrete failure from normal use.

## Guiding principle

Bermuda is a professional precedent finder.

Its central question remains:

> **We got here somehow — who has got a map?**

The program should expose the professional evidence clearly and let the user investigate what the professionals did next.
