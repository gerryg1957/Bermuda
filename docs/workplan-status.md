# Bermuda — Status and Workplan

_Last updated: 2026-09-20 · branch: `master` · checkpoint: `3d89d94`_

## Current state

Bermuda is a native desktop Go application built around a Rust core and a Qt/QML interface. Its purpose is broader than a game database: it combines a professional-game corpus, pattern search, personal games, Study, joseki reference material, and eventually analysis and online play in one coherent desktop environment.

The application name is simply **Bermuda**.

### Professional database and pattern search

The professional-game database remains a user-built corpus rather than data bundled with Bermuda. The intended corpus is professional 19×19 games. Bermuda supports importing SGF collections into projects and searching the resulting database for board patterns.

Pattern selection uses a rectangular selection area. The intended semantics are important:

- the rectangle defines the **context of the pattern search**;
- stones outside the rectangle are deliberately excluded;
- board edges outside the rectangle are also excluded;
- a tight rectangle around stones asks for the local shape regardless of its original board location;
- including one board edge makes that edge part of the pattern context;
- including two board edges makes the corner itself part of the context.

The search behaviour is therefore intentional, but the UI does not yet communicate these semantics clearly enough.

### Study

Study is now a permanent Bermuda workspace rather than something that only becomes available after selecting a database game.

Study contains three destinations:

- **Current study**
- **Study Library**
- **Joseki Library**

Study documents are copy-on-write. Professional games, My Games, and imported SGFs remain historical source material. Once substantive Study work is added, Bermuda creates or uses a separate Study Library document rather than modifying the original source.

Implemented Study functionality includes:

- replayable SGF game tree;
- comments and source SGF markup;
- Bermuda annotations;
- crosses, triangles, circles, squares, labels and move-number annotations;
- semantic Letter annotations;
- source SGF labels are protected from accidental replacement;
- Bermuda-added letters continue around source labels;
- semantic letters compact after deletion;
- Study annotations survive navigation;
- Study Library persistence across application restarts;
- Study Library browser;
- opening and deleting Study documents;
- safe Study Library deletion rules;
- provenance metadata stored separately in Bermuda's private SGF metadata.

The next major Study feature is a proper SGF editor.

### Native OGS Joseki Explorer

The Joseki Library now reads the current community-curated OGS Joseki Explorer natively inside Bermuda.

Implemented behaviour includes:

- on-demand OGS position loading;
- local caching of visited positions;
- cached fallback when offline;
- no bundled OGS database snapshot;
- OGS attribution and an external **View on OGS** action;
- OGS source information and commentary;
- lightweight rendering of OGS Markdown conventions;
- board references such as `<A:Q17>` rendered as readable labels;
- OGS position links rendered as links;
- OGS move categories retained rather than flattened.

The board distinguishes OGS categories:

- **Ideal**
- **Good**
- **Trick**
- **Mistake**
- **Question**

Variation labels are retained on the goban. Ideal moves are visually prominent, while other categories remain distinguishable without being given identical weight.

The original flat textual continuation list has been removed.

### Lazy Joseki variation tree

The OGS Joseki Explorer now has a lazy variation tree integrated into the central Study workspace.

The tree:

- starts from the currently known OGS position;
- includes its documented children;
- loads a child only when the user explores it;
- caches explored OGS positions;
- retains explored nodes for the current Bermuda session;
- keeps OGS category distinctions on the tree;
- keeps variation labels;
- lives beside the goban rather than in the commentary pane;
- uses a Bermuda-style tree presentation;
- stacks alternative joseki branches vertically while move depth progresses across the centre pane.

This now feels consistent with the rest of Bermuda and is a natural checkpoint.

## Immediate next work

### 1. SGF editor inside Study

Study should become a genuine SGF editing environment rather than only a replay and annotation environment.

The editor should build on the existing Study tree rather than introduce a second representation.

Likely initial scope:

- add moves;
- add variations;
- delete moves and variations safely;
- edit comments;
- edit relevant SGF properties;
- retain setup stones and markup;
- preserve legal move/replay behaviour;
- preserve full tree topology;
- keep source material immutable through the existing copy-on-write Study model.

Editing semantics should be agreed before implementation, particularly around deleting branch points, replacing continuations, setup properties, and provenance.

### 2. Make pattern-selection context explicit

The current pattern-search semantics are sound but need clearer communication.

First improvement:

> Only stones and board edges inside the selected area are part of the search pattern.

Possible follow-up visual improvement:

- visually emphasise any board edge included by the selected rectangle;
- make it immediately obvious whether the query is free-standing, side-constrained, or corner-constrained.

The terminology should distinguish the transient drag rectangle from the persistent selected search area, but the more important issue is the semantics of the selected context.

### 3. Improve “Study this position” for OGS

At present the Joseki explorer can create a local Study from the current OGS position, but the longer-term goal is better:

- copy the **explored OGS subtree** into the new local Study;
- retain current-path commentary and markup;
- retain provenance;
- turn the result into ordinary editable local Study material;
- keep OGS as the external reference catalogue and Study as the user's persistent working copy.

## Wider Bermuda workplan

### Study and document model

Further Study refinements:

- first-class Study titles;
- clear identity when several studies originate from the same source game;
- richer provenance for OGS-derived material rather than generic detached provenance;
- ensure only genuine Study Library documents are treated as Bermuda-owned writable files;
- improve metadata and source display where useful;
- keep generic SGF export semantically faithful.

### Pattern search

Potential remaining pattern-search work includes:

- clarify rubber-band/search-area semantics;
- visually communicate included board edges;
- review symmetry behaviour and search controls;
- review whether colour-independent or friendly/enemy matching is still desirable;
- review whether wildcard / “don't care” points are still desirable;
- optional exclusion/inclusion of handicap games, with a sensible default;
- a distinct fuseki-oriented search/filter where useful;
- richer result sorting;
- player-name and alias normalisation.

Some older pattern-search ideas may already have been superseded by the current Bermuda quadrilateral workflow and should be re-checked before implementation.

### My Games

My Games should ultimately be more than personal SGF storage.

Longer-term goals:

- maintain a separate personal-game corpus;
- update unfinished or revised games sensibly;
- show clearly when a game is already in My Games;
- run KataGo-assisted review;
- identify recurring mistakes or recurring difficult shapes;
- find “interesting moments” automatically;
- compare the user's continuation with professional continuations in the database;
- use pattern search across the personal corpus;
- support long-term analysis of habits and recurring weaknesses.

### KataGo

KataGo integration should become a polished optional Bermuda capability rather than a developer-oriented setup.

Outstanding work includes:

- GUI settings for KataGo executable;
- model selection;
- configuration-file selection;
- clear environment-variable override behaviour;
- **Test KataGo** diagnostics;
- graceful first-run behaviour when KataGo is absent;
- current-position analysis;
- whole-game analysis;
- evaluation graph;
- robust cancellation/supersession when the user moves rapidly between positions;
- possible play-against-KataGo mode.

Bermuda itself should remain useful without KataGo.

### Database and project workflow

The project/database experience should continue to become more polished:

- simple project creation;
- opening existing projects;
- importing and updating SGF corpora;
- clear import progress and errors;
- reliable maintenance of a personal professional-game collection;
- documentation showing users how to obtain/build their own corpus rather than shipping one with Bermuda;
- coherent database/project menu structure.

The user's own target corpus is professional 19×19 games, with particular interest in keeping post-1-Jan-2026 games current.

### Statistics and explorers

Possible future database-facing features:

- opening/fuseki statistics;
- player statistics;
- win rates;
- tournament summaries;
- richer joseki exploration;
- richer fuseki exploration;
- links between statistical views, pattern search, Study and source games.

These are useful but come after the core editing, Study, search and analysis workflows.

### UI and application structure

As Bermuda matures, the major workflows should remain coherent rather than accumulating ad hoc controls.

Likely top-level concepts remain:

- Game
- Search / Pattern
- My Games
- Study
- Database
- Settings
- Help

Exact menu organisation can be refined later, but similar actions should live in similar places throughout the application.

### Documentation, packaging and release readiness

Before a 1.0-quality release Bermuda will need:

- user documentation;
- first-run guidance;
- corpus-building/import instructions;
- KataGo setup instructions;
- Study and pattern-search documentation;
- Linux packaging outside the development tree;
- CI/build automation;
- release automation;
- diagnostics;
- graceful handling of unavailable optional services;
- final UI consistency and accessibility review.

## Online play — deliberately later

Online play remains part of the Bermuda vision, but it is deliberately postponed because networking can become an “internet swamp” and consume the project before the local application is mature.

There are two distinct possibilities.

### Bermuda-to-Bermuda peer-to-peer play

Two Bermuda users could play directly using the Bermuda board and interface.

A complete implementation would eventually have to address:

- invitations / discovery;
- connection establishment;
- NAT traversal or relay strategy;
- rules and game-state synchronisation;
- clocks;
- pass and resign;
- undo requests if supported;
- disconnect/reconnect;
- resume;
- security;
- final SGF generation and saving.

The apparently simple feature “let two Bermuda users play one another” therefore has substantial infrastructure behind it.

### Bermuda as an OGS playing client

A potentially more useful route is to make Bermuda a native front end for OGS.

Possible capabilities:

- authenticate an OGS account;
- see challenges and active games;
- issue and accept challenges;
- play live or correspondence games using Bermuda's board;
- handle clocks and game state through OGS;
- pass, resign and other supported game actions;
- reconnect safely;
- save completed games directly into My Games;
- move completed games naturally into Study.

This would give Bermuda access to an existing player community instead of requiring Bermuda to create its own network.

The native OGS Joseki integration makes OGS interoperability less foreign to Bermuda than it once was, but live play is much more demanding than reading Joseki data. It may require authentication, persistent connections/WebSockets, protocol tracking, clocks, reconnect logic, challenge handling, chat/observer decisions, and careful state reconciliation.

Analysis and Study tools must not be exposed in ways that violate server or fair-play rules during live games. They belong naturally after a game has finished.

### Online-play sequencing

Do not begin online play until Bermuda's local workflows are mature.

In particular, complete or stabilise:

- Study editing;
- pattern-search semantics;
- Study Library;
- My Games;
- KataGo setup and analysis;
- database/project workflows.

Then reassess OGS client integration and peer-to-peer play with the benefit of a stable board, SGF and Study architecture.

## Suggested development sequence

The current preferred sequence is:

1. SGF editor in Study.
2. Clarify pattern-selection/rubber-band semantics.
3. Improve OGS → Study subtree transfer.
4. Remaining Study document-model refinements.
5. KataGo settings, diagnostics and broader analysis.
6. My Games analysis workflow.
7. Pattern-search refinements.
8. Database/project workflow polish.
9. Statistics and explorers.
10. Documentation, packaging and release readiness.
11. Reassess online play:
    - OGS native playing client;
    - Bermuda-to-Bermuda peer-to-peer play.

This order is intentionally conservative: finish the bounded local problems first, then approach the networking work once Bermuda's core concepts are stable.
