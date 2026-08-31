# Bermuda Application Design

## Status

Design direction.

## Purpose

Bermuda should become a polished native desktop Go database and study application.

The application is built on Qt/QML and should develop into an application that feels at home on KDE Plasma while remaining usable on other Linux desktop environments and, where practical, Windows.

Functionality and visual design should reinforce Bermuda's central purpose: exploring Go positions and games.

## Design objectives

The application should feel:

- calm;
- fast;
- visually coherent;
- board-centred;
- discoverable;
- suitable for prolonged study;
- powerful without appearing cluttered.

"Delightful" should come from responsiveness, clarity, thoughtful details and attractive presentation rather than decorative complexity.

## Core activities

The application should eventually be organised around a small number of natural activities:

### Library

Browse and filter games.

This may include professional sources and, separately, personal corpora.

### Search

Construct and run Bermuda position/pattern searches and inspect their results.

### Game

Replay and explore an individual game, including variations and metadata.

### Review

Study a personal game with KataGo analysis and connections to recurring personal patterns and professional examples.

These are conceptual activities, not necessarily four permanent tabs.

## The board

The Go board is the primary object in Bermuda.

Whenever practical:

- it should receive the largest useful portion of the window;
- resizing should preserve a good board aspect ratio;
- controls should not compete visually with it;
- panels should provide context rather than permanently consume unnecessary space.

The board must remain fully usable without KataGo.

## Three kinds of evidence

A central interface requirement is to distinguish three sources of information.

### Professional evidence

What occurred in professional games.

Examples:

- matching games;
- continuation frequencies;
- player/event/date information.

### Personal evidence

What occurred in the user's own games.

Examples:

- repeated occurrences;
- outcomes;
- historical trends;
- recurring choices.

### KataGo analysis

What the AI engine estimates about a position.

Examples:

- score;
- win probability;
- candidate moves;
- variations;
- estimated move loss.

The interface should allow these to interact without presenting them as equivalent sources.

## Personal review

Online Go Server's game-analysis presentation provides useful inspiration: an evaluation graph, key moments, move quality and direct navigation make AI review understandable.

Bermuda should adopt useful interaction ideas without attempting to reproduce another application's interface.

A possible Bermuda review arrangement is:

    +-----------------------------------------------+
    |                               | REVIEW        |
    |                               |               |
    |             BOARD             | Key moments   |
    |                               |               |
    |                               | Similar       |
    |                               | positions     |
    |                               |               |
    |                               | Pro examples  |
    +-------------------------------+---------------+
    | evaluation / score graph       move navigation|
    +-----------------------------------------------+

Selecting a key move changes the board.

Selecting a KataGo candidate explores an AI variation.

Selecting "Similar positions" opens occurrences from the personal corpus.

Selecting "Professional examples" runs or opens the corresponding professional pattern search.

This makes AI review a gateway into Bermuda's distinctive database capabilities.

## KDE integration

Qt/QML is already an appropriate foundation.

Bermuda already uses KDE Kirigami and should progressively adopt further KDE technologies and conventions where they improve integration. This should remain incremental rather than requiring a wholesale GUI rewrite.

Areas for KDE integration include:

- standard application actions;
- menus;
- keyboard shortcuts;
- settings;
- recent files/databases;
- standard dialogs;
- About information;
- desktop notifications where useful;
- icons;
- application metadata;
- Plasma appearance integration.

KDE-specific GUI dependencies should not leak into the Bermuda core libraries.

## Linux desktop integration

A properly installed Bermuda should eventually provide:

- application launcher entry;
- application icon at standard sizes;
- AppStream metadata;
- appropriate categories;
- MIME association with SGF where desirable;
- command-line executable(s);
- documentation;
- standard installation paths.

Opening an SGF from the desktop should eventually be able to open it in Bermuda.

## Responsiveness

Long-running operations must not freeze the interface.

This includes:

- importing large databases;
- building indexes;
- pattern searches;
- importing personal corpora;
- KataGo analysis.

Operations should expose progress and cancellation where meaningful.

This reinforces the architectural requirement that the GUI use stable core interfaces rather than parse human-oriented command-line output.

## Progressive enhancement

The current working GUI should not be discarded merely to obtain a new visual design.

Development should proceed incrementally:

1. preserve existing functionality;
2. improve application structure;
3. introduce standard KDE integration;
4. improve layout and navigation;
5. add personal-corpus views;
6. add KataGo review;
7. polish animations, transitions and visual details after workflows are stable.

Large visual rewrites should not block database development.

## Accessibility and scale

The interface should work with:

- system font settings;
- HiDPI displays;
- keyboard navigation;
- light and dark desktop themes;
- different window sizes.

Information must not depend on colour alone.

Go stones and board markings need sufficient contrast under supported themes.

## Configuration

Application settings will eventually include areas such as:

- professional database location;
- personal corpus locations;
- KataGo executable/model/configuration;
- analysis preferences;
- board/display preferences;
- search preferences.

Defaults should make the common case straightforward while retaining advanced configuration.

## Windows

The application design should avoid unnecessary assumptions that it is running under KDE.

On Windows, KDE/Qt components used by Bermuda must either be deployable with the application or have suitable cross-platform behaviour.

Linux desktop integration features such as `.desktop` files and AppStream metadata are packaging concerns and should not be prerequisites for the core application.

## Visual-design work

Before major cosmetic implementation, representative screens should be designed for:

- initial/library view;
- pattern search;
- search results;
- professional game replay;
- personal game review;
- recurring-behaviour overview;
- KataGo position analysis;
- settings.

This allows the application to develop a coherent visual language rather than accumulating controls opportunistically.

## Guiding principle

The user should be able to begin with a stone or position on the board and move naturally between:

    the game I am studying
           |
           +--> similar professional games
           |
           +--> similar positions in my games
           |
           +--> KataGo analysis
           |
           +--> recurring personal behaviour

Bermuda's front end should make those relationships feel immediate.
