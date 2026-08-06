# MoyoDB Pattern Search Design

## Purpose

MoyoDB pattern search should help a Go player find positions that are equivalent
for a stated purpose without losing the ability to perform exact technical
searches. It should also help a player who does not know what to consider next
discover candidate moves repeatedly chosen by strong players and investigate
the games behind those choices.

The phrase **same pattern** can mean several different things:

- exactly the same stones and empty intersections;
- the same local formation in another corner or on another side;
- the same formation with Black and White exchanged;
- the same position reached through a different move order;
- the same defining stones with surrounding intersections ignored;
- the same formation at a corresponding distance from the board edges;
- the same formation appearing during a particular stage of the game;
- one continuous appearance rather than every move for which it persists.

These meanings must not be hidden inside one ambiguous operation. MoyoDB should
represent them as independent, visible search choices.

This document defines the intended concepts and direction of travel. It does
not commit the project to implementing every option at once.

---

## Design principles

### Exactness should remain available

Broad, strategically useful searches should not replace exact search. Exact
position and exact local-pattern searches remain valuable for testing,
transpositions, known positions and diagnostic work.

### Similarity should be explicit

MoyoDB should not silently guess what the user means by *similar*. The user
should be able to specify:

- which stones matter;
- which empty points matter;
- which points do not matter;
- which geometric transformations are allowed;
- whether colours may be exchanged;
- whether board-edge relationships matter;
- whether the game stage matters;
- how repeated matches are counted.

### Search facts should be separated from Go interpretation

The engine can establish observable facts such as:

- where a pattern occurs;
- when it first appears;
- which transformation matched;
- how often it occurs;
- what moves followed;
- how results are distributed across the board.

It should not assume that geometry alone determines the strategic meaning of a
shape.

### Candidate frequencies are evidence, not verdicts

Where several continuations occur, their frequencies identify recurring
professional candidates. They do not prove which move is best. A move played
eight times deserves investigation, but it should not be labelled superior
merely because another move appeared five times.

The interface should lead from each candidate to the supporting games,
contexts and continuations. Historical outcomes may be shown descriptively,
but must not be converted into causal claims or an unexplained move-quality
score.

### Broad searches should support refinement

A useful workflow may begin with a broad search, inspect a continuation map,
appearance-location map or move-number distribution, and then add explicit
restrictions. Result analysis
should inform refinement rather than silently discarding matches.

### Core semantics belong in Rust

Pattern definitions, transformations, matching, deduplication and occurrence
counting belong in the Rust core library. The GUI and CLI should consume the
same search model.

---

## Current foundation

MoyoDB currently has an exact rectangular position search.

The existing search:

- extracts a rectangle from a board position;
- records every black stone, white stone and empty intersection within it;
- records its relationship to board edges;
- preserves stone colours;
- searches in the same orientation;
- finds the moves at which the exact rectangle exists;
- groups matches by game for display.

The current large-result work also introduces a bounded result path:

- the database-wide scan retains one summary per matching game;
- the summary records the match count and first occurrence;
- result metadata is prepared without retaining every occurrence for every
  game in the GUI;
- a selected game's complete occurrences are regenerated on demand.

This exact-search foundation should remain available while broader
position-pattern semantics are added.

---

## Search modes

### 1. Exact pattern

An exact pattern requires:

- the same black stones;
- the same white stones;
- the same empty intersections;
- the same dimensions;
- the same orientation;
- the same board-edge relationships.

Typical uses include:

- finding an exact local position;
- locating a known joseki position;
- finding exact repetitions or transpositions;
- verifying a database transcription;
- testing the search implementation;
- comparing exact continuations.

Exact search may retain access to every matching board state for diagnostic
purposes.

### 2. Position pattern

A position pattern describes the strategically significant parts of a
formation while allowing irrelevant intersections to be ignored.

It may optionally allow:

- rotations;
- reflections;
- colour reversal;
- explicitly required empty points;
- unspecified points;
- unrelated stones elsewhere on the board;
- preserved corner or side relationships;
- a board-region restriction;
- a move-number restriction;
- distinct continuous appearances rather than every matching move.

This should become the normal mode for practical Go study.

Typical uses include:

- finding an opening formation on either side of the board;
- comparing the same formation when played by Black or White;
- finding a local shape despite unrelated moves elsewhere;
- finding positions reached through different move orders;
- examining professional continuations from equivalent positions.

### 3. Sequence pattern

A future sequence search would describe how a position was reached rather
than only the resulting arrangement.

It might support:

- a required order of moves;
- intervening moves elsewhere;
- transpositions;
- optional tenuki;
- colour reversal;
- rotations and reflections;
- continuation statistics.

Sequence search should be designed only after position-pattern semantics are
mature.

---

## Pattern point states

A position-pattern editor needs four possible states for each intersection.

### Required black

A black stone must occupy the intersection.

### Required white

A white stone must occupy the intersection.

### Required empty

The intersection must be empty.

Emptiness can be strategically significant. Examples include:

- a cutting point that must remain open;
- an invasion point;
- a required liberty;
- the open side of an enclosure;
- a point whose occupation would change the local position.

Required emptiness should therefore be explicit.

### Unspecified

The intersection may contain:

- a black stone;
- a white stone;
- no stone.

Unspecified points are not part of the search condition.

This is essential when only a few defining stones matter. In position-pattern
mode, unmarked intersections should normally default to unspecified rather
than required empty.

---

## Pattern boundary and surrounding context

A selected rectangle is a convenient editing region, but it must not always
mean that every point inside it is part of the condition.

The editor should distinguish:

- the area in which the user is defining the pattern;
- the points explicitly constrained;
- the pattern's relationship to board edges;
- the surrounding board, which may remain irrelevant.

This allows a user to draw a large enough region to express a formation
without accidentally requiring every unused point to be empty.

The interface should make the active constraints visible. It should be
possible to tell at a glance whether a point is:

- required black;
- required white;
- required empty;
- unspecified.

---

## Geometry and transformations

Geometric transformations should be independent search options.

### Exact orientation

Only the orientation drawn by the user is searched.

### Reflections

The pattern may be reflected horizontally, vertically or diagonally where
meaningful.

### Rotations

The pattern may be rotated through 90, 180 or 270 degrees.

For a rectangular pattern, rotation may exchange its width and height.

### Board-edge transformations

When edge relationships matter, transformations must apply both to the stones
and to their relationship with the board edges.

A corner pattern rotated into another corner should preserve its corresponding
distances from the two adjacent edges.

A side pattern reflected to another side should preserve its relationship to
that side.

### Duplicate transformations

Symmetrical patterns may produce identical transformed versions.

The engine should deduplicate equivalent transformed patterns before
searching. A symmetric pattern should not generate duplicate appearances
merely because more than one transformation produces the same condition.

### Reporting transformations

Each appearance should record which transformation matched. This permits:

- correct board highlighting;
- navigation between appearances;
- transformation counts;
- later filtering by orientation;
- debugging of symmetry handling.

---

## Colour treatment

Colour handling should be independent of geometry.

### Preserve colours

Required black stones remain black and required white stones remain white.

### Reverse colours

The search also accepts the same formation with Black and White exchanged.

Colour reversal should not imply rotation or reflection, and a geometric
transformation should not imply colour reversal.

They are separate choices.

This distinction is necessary for opening searches in which the same
formation may be constructed by either player.

### Duplicate colour results

Where colour reversal produces an equivalent condition, duplicate appearances
should be removed.

---

## Spatial context

A pattern's meaning may depend on where it occurs on the board.

Some shapes are meaningful almost anywhere. Others depend on:

- a corner;
- one or more board edges;
- exact distances from an edge;
- a broad board region;
- a whole-board opening relationship.

A geometrically identical arrangement in another region may therefore be:

- a strategically equivalent pattern;
- a useful transformed example;
- an accidental geometric match;
- a false positive for the user's research question.

MoyoDB should make spatial context explicit.

### Anywhere

Geometry alone matters.

The pattern may match in a corner, on a side or in the centre unless
individual edge relationships were explicitly included.

This is appropriate for many local tactical and connection shapes.

### Preserve exact edge relationships

The pattern retains the exact distances from every relevant board edge.

This is the closest spatial equivalent to the current exact search.

### Corner-anchored

The pattern derives part of its identity from two adjacent board edges.

Examples include:

- corner enclosures;
- corner joseki;
- corner invasions;
- corner approach formations.

A geometrically identical arrangement in the centre is not another
corner-anchored occurrence.

A corner-anchored search may still allow rotations and reflections into the
other corners.

### Side-anchored

The pattern derives part of its identity from one board edge.

A side-anchored search should preserve:

- its distance from the edge;
- its orientation relative to the edge;
- any explicitly required extent along the side.

It may optionally match corresponding positions on the other sides.

### Selected board region

The user may restrict a search to:

- one corner;
- one side;
- one half of the board;
- one quadrant;
- another selected region.

This is distinct from anchoring. A search may preserve a shape's edge
relationship while also restricting results to the right side of the board.

### Whole-board opening context

Some opening formations are local enough that unrelated stones elsewhere
should be ignored, but large enough that their broad relationship to a corner
and side remains essential.

A whole-board opening pattern may combine:

- required defining stones;
- unspecified unrelated intersections;
- preserved edge distances;
- optional transformations;
- colour reversal where appropriate;
- an optional move-number range.

It should not require the whole board position to match exactly.

---

## Temporal context

The same geometry may have a different significance depending on when it
appears.

An opening formation normally has its intended meaning during the fuseki. A
similar arrangement appearing very late after captures may be a valid
geometric match but not a useful opening example.

MoyoDB should therefore record the move at which each distinct appearance
begins.

### Move-number filters

The user may optionally restrict results by:

- first appearance before a specified move;
- first appearance after a specified move;
- first appearance within a move range.

The core search representation should use explicit move numbers.

Broad labels such as *opening*, *middle game* and *endgame* may be offered by
the interface only if their definitions are documented.

### Move order remains separate

A move-number restriction does not impose move order.

A position-pattern search may still find:

- transpositions;
- intervening tenuki;
- unusual corner ordering;
- delayed completion of a formation.

Move-order restrictions belong to sequence search.

---

## Shape identity and strategic function

Recognising a pattern does not by itself determine its strategic meaning.

A shimari is structurally a corner enclosure, but it may also provide an
active base for fighting against an approach, pressing an opponent,
restricting development or otherwise making the approach uncomfortable.

A one-space jump between stones may often be used to defend, connect or
solidify, but its actual purpose depends on the surrounding position.

Descriptions such as:

- aggressive;
- defensive;
- solid;
- light;
- territorial;
- influential;

must therefore be treated as context-dependent interpretations rather than
fixed properties of a geometric pattern.

Spatial anchoring describes where a pattern belongs. It does not determine
what the pattern means strategically.

MoyoDB should initially report observable information:

- where the pattern occurs;
- when it first appears;
- the surrounding position;
- subsequent moves;
- continuation frequencies;
- game results where statistically meaningful.

It should not assign strategic labels automatically from shape alone.

Later analysis, including possible KataGo assistance, may help a player study
the function of a pattern in context. That would remain analysis rather than
a rigid property of the search definition.

---

## Counting occurrences

The current exact engine can report the same unchanged formation at many
successive moves.

For most Go study, this is not useful.

A pattern that appears at move 20 and remains unchanged until move 45 should
normally count as one continuous appearance rather than 26 separate matches.

### Appearance identity

For position-pattern search, one appearance should be identified by:

- game;
- transformed orientation;
- colour assignment;
- board location;
- continuous period during which the pattern remains present.

The displayed move should initially be the first move at which the complete
pattern exists.

### Reappearance

If the pattern is broken and later reconstructed at the same location, the
reconstructed pattern may count as a new appearance.

Whether a one-move interruption should count as a new appearance remains an
open design question.

### Multiple simultaneous locations

The same pattern appearing simultaneously at two different locations in one
game should count as two appearances.

### Overlapping matches

Overlapping transformed matches may be:

- genuinely distinct appearances;
- duplicates caused by symmetry;
- alternate descriptions of the same local formation.

The engine should remove exact duplicates while preserving genuinely distinct
locations or transformations.

### Raw matches

Exact search should retain an option to expose every matching board state for
testing and diagnostic use.

The normal position-pattern result display should favour distinct
appearances.

---

## Search results

The result list should normally contain one row per matching game.

Each row should show:

- Black player;
- White player;
- date;
- result;
- event;
- number of distinct appearances;
- first matching move.

Selecting a game should load or regenerate its complete appearance list on
demand.

The user should then be able to navigate:

- previous appearance;
- next appearance;
- the move at which each appearance begins;
- the board location;
- the transformation used;
- whether colours were reversed.

The full occurrence list should not need to be retained for every matching
game during the database-wide scan.

This is important for searches that produce millions of raw board-state
matches.

---

## Continuation analysis and candidate investigation

The continuation display should turn a result set into a route for studying
professional choices rather than a passive summary.

### Immediate continuation distribution

For each distinct appearance, MoyoDB should record at most one immediate next
move: the recorded move after the displayed position. The points must be
normalised into the query's orientation and colour frame before aggregation.

The aggregate should distinguish:

- points within the displayed pattern area and margin;
- moves outside the displayed area;
- passes;
- games that ended at the matched position.

These categories must total the number of appearances used for the aggregate.
No evidence should disappear merely because it cannot be drawn as a point on
the board.

The board overlay for this distribution is called the **continuation map**.
Larger or stronger circles may represent greater frequency, but the map shows
what professionals played, not an evaluation of what they should have played.

### Candidate counts

Each candidate should be able to report:

- number of appearances;
- number of distinct supporting games;
- later, where useful, number of distinct players or player pairs.

Appearances and games must remain separate because several occurrences in one
game are not equivalent to several independent games. Local-episode grouping
may later reduce further over-counting.

### Interactive investigation

Selecting a continuation point should create an explicit filter showing the
games in which that candidate was played. A frequency-ordered candidate list
may provide the same operation for keyboard and non-board use.

For a selected candidate, the user should be able to:

- open every supporting game;
- jump to the matched position;
- replay the subsequent local and whole-board sequence;
- compare another candidate without reconstructing the search;
- clear the candidate filter and return to the complete result set.

The selected point is an object of investigation, not a declaration that the
move is best.

### Historical outcomes

Win and loss counts may be shown from the perspective of the player who chose
the candidate, together with the sample size. The language must remain
descriptive, for example:

> This continuation occurred in 13 games: 5 wins and 8 losses.

The display should invite inspection of the games rather than imply that the
continuation caused those results. Later annotations may include player
strength, era, colour, rules, komi and engine evaluation before and after the
sequence.

### Map terminology

MoyoDB should keep several board overlays distinct:

- **continuation map** — corpus-derived immediate-next-move frequencies;
- **appearance-location map** — where matching appearances occurred;
- **formation or activity map** — moves before or after a pattern over a wider
  time window;
- **influence map** — a future heuristic field derived from the current board;
- **KataGo ownership map** — a future engine estimate of eventual ownership.

An influence or ownership map must not be presented as another form of
professional continuation statistics.

---

## Aggregate result analysis

A result set should also support aggregate analysis.

### Appearance-location map

MoyoDB should be able to summarise result locations as an appearance-location
map.

The map may show:

- where distinct appearances begin;
- how often each board location is used;
- whether matches cluster in corners or along sides;
- whether unexpected centre matches occur;
- whether transformed versions occupy predictable regions.

The appearance-location map should not be an invisible rule that decides
which matches are meaningful.

Its purpose is to help the user discover whether location appears intrinsic
to the pattern and then refine the search deliberately.

### Move-number distribution

A companion display should summarise the moves at which distinct appearances
begin.

It may be shown as:

- a histogram;
- configurable move-number bands;
- a cumulative distribution;
- documented opening, middle-game and endgame groupings.

For an opening formation, a concentration in the first few dozen moves would
support its interpretation as a fuseki pattern.

Late appearances should remain inspectable as possible:

- delayed transpositions;
- unusual move orders;
- reconstruction after local fighting;
- false positives;
- genuinely interesting exceptions.

### Transformation and colour counts

The analysis may also show:

- counts by rotation or reflection;
- counts with original colours;
- counts with reversed colours;
- counts by corner or side.

### Interactive refinement

A later interface may allow the user to refine a search by:

- selecting appearance-location cells;
- selecting one or more board regions;
- selecting a move-number histogram range;
- selecting transformations;
- comparing Black and White use of a formation.

Such refinements must create explicit, visible filters. They should not
silently remove results.

---

## Search setup board

The pattern setup board is not a legal game editor.

Users must be able to place:

- several consecutive black stones;
- several consecutive white stones;
- setup stones in any order;
- required-empty markers;
- unspecified markers.

No artificial pass moves should be necessary.

This distinction is conceptually important. Constructing a search condition
is not the same operation as recording an alternating legal game.

---

## User interface principles

Search controls should describe their effects plainly.

### Search mode

- Exact pattern
- Position pattern

### Point meaning

- Required black
- Required white
- Required empty
- Unspecified

### Transformations

- Same orientation only
- Include rotations
- Include reflections

### Colours

- Preserve colours only, for exact or diagnostic searches
- Include reversed colours, the ordinary study default

### Spatial scope

- Anywhere
- Preserve exact edge relationships
- Corner-anchored
- Side-anchored
- Restrict to a selected board region

### Game stage

- Any move
- First appearance before a specified move
- First appearance within a specified move range

### Result counting

- Distinct appearances
- Every matching position

The initial practical default for position-pattern search should probably be:

- unmarked intersections are unspecified;
- rotations and reflections are included;
- reversed colours are included;
- edge relationships are preserved when the selected definition explicitly
  depends on an edge;
- move order is ignored;
- all move numbers are allowed;
- distinct continuous appearances are counted.

Exact and diagnostic searches may explicitly preserve colours and orientation.
Ordinary study searches should include colour reversal because the purpose is
to compare strategic ideas rather than the nominal colour of the player.

The active search definition should be inspectable before and after the
search runs.

---

## Acceptance examples

### Example 1: simple two-stone shape

A selected 3 x 2 rectangle contains two white stones.

The interface must make the following questions explicit:

- Are the other four intersections required to be empty?
- Are they unspecified?
- Should a reflected version match?
- Should a rotated version match?
- Should two black stones in the same relation match?
- Should an unchanged shape persisting for many moves count once or many
  times?
- Should the shape be allowed anywhere on the board?

The current exact search answers:

- the other points are required empty;
- no rotations or reflections;
- colours are preserved;
- every matching move may be counted;
- edge relationships are preserved.

Position-pattern search must allow different answers.

### Example 2: Chinese-opening reference position

A whole-board exact search based on the discussed Chinese-opening position
may find only games with the identical complete position.

The Moyo Go Studio example produced three move-12 results from 2018. This is a
useful exact-position reference case, but it does not by itself establish
that the strategic variation disappeared after 2018.

A broader position-pattern search is required to investigate the formation
rather than the exact whole-board position.

### Example 3: Chinese formation on either side

A search based on the defining stones should be able to find:

- the formation on either side of the board;
- reflected versions;
- rotated versions where appropriate;
- the formation played by Black;
- the formation played by White when colour reversal is enabled;
- positions reached through different move orders;
- positions with unrelated stones elsewhere.

### Example 4: White formation against another opening

White may construct the same formation:

- in response to Black's San-ren-sei;
- against an old-school Japanese opening;
- against another unrelated opening framework.

With suitable reflection and colour reversal enabled, these should be
recognised as related formations.

Unrelated stones elsewhere on the board should not prevent a match when they
are outside the defining condition.

### Example 5: Chinese shape translated into the centre

A broad geometry-only search might find the defining stones translated into
the centre.

This is a geometric match but not a strategically equivalent Chinese
opening.

A suitable position-pattern definition should be able to require:

- the defining stones;
- their distances from the relevant side and corner;
- optional reflection to the opposite side;
- colour reversal where appropriate;
- unrelated stones elsewhere to be ignored.

The centre translation should then be excluded by an explicit spatial rule,
not by a hidden heuristic.

A deliberately broad preliminary search may still include it. In that case,
the appearance-location map should reveal it as a centre occurrence that can be
inspected or filtered.

### Example 6: shimari and strategic function

A shimari should be recognised as a corner-anchored structural pattern.

The search result should not automatically label it defensive or aggressive.

Its function may depend on:

- the direction of an approach;
- nearby strength and weakness;
- ladders;
- whole-board priorities;
- subsequent fighting.

MoyoDB should show the position and continuation rather than asserting a
fixed strategic meaning.

### Example 7: location-independent local shape

A local tactical or connection shape may be meaningful in several board
regions.

With spatial scope set to Anywhere, the same geometry should be able to
match:

- in a corner;
- on a side;
- in the centre.

The appearance-location map should display the distribution without treating one
region as more correct than another.

### Example 8: persistent opening position

If a completed formation first exists at move 12 and remains unchanged until
move 25, normal position-pattern results should report one appearance
beginning at move 12.

It should not produce fourteen strategically identical appearances.

### Example 9: reappearance

If the formation is broken at move 30 and reconstructed at move 40, the
second continuous period may be reported as another appearance beginning at
move 40.

### Example 10: two locations in one game

If the same local shape exists in two corners at the same time, both
locations should be available as separate appearances within that game.

### Example 11: opening-stage distribution

A search for an opening formation should normally show first appearances
concentrated early in the game.

A move-number distribution should help distinguish:

- ordinary fuseki examples;
- delayed transpositions;
- late reconstructions;
- accidental late-game matches.

A move-range filter may then be applied explicitly.

---

## Proposed implementation stages

### Stage 1: stabilise exact search

- retain current exact semantics;
- complete reliable large-result handling;
- keep cancellation responsive;
- display results reliably;
- load per-game occurrences lazily;
- document current behaviour clearly.

### Stage 2: distinct appearances

- collapse consecutive matches at the same transformed location;
- report the first move of each continuous appearance;
- preserve access to raw matches for testing;
- handle breakage and reappearance;
- distinguish simultaneous locations.

### Stage 3: transformations

- generate rotations and reflections;
- transform board-edge constraints correctly;
- deduplicate symmetrical transformed patterns;
- record which transformation produced each appearance.

### Stage 4: position-pattern point states

- add required black;
- add required white;
- add required empty;
- add unspecified;
- make unmarked intersections unspecified in position-pattern mode;
- show active point states clearly in the editor.

### Stage 5: colour equivalence

- search the original colour assignment;
- include the reversed assignment in ordinary study searches;
- allow exact or diagnostic searches to preserve colours;
- record the colour assignment used;
- prevent duplicate results where reversal makes no difference.

### Stage 6: explicit spatial scope

- support Anywhere;
- support exact edge relationships;
- support corner anchoring;
- support side anchoring;
- support selected board regions;
- keep spatial rules visible in the search definition.

### Stage 7: continuation and candidate investigation

- aggregate one immediate next move per distinct appearance;
- normalise continuations across transformations and colour reversal;
- distinguish local points, off-map moves, passes and ended games;
- make continuation points selectable;
- filter results to the games supporting a selected candidate;
- show appearances and distinct supporting-game counts;
- present historical outcomes descriptively.

### Stage 8: temporal filters and broader aggregate analysis

- record the first move of each appearance;
- support explicit move-number filters;
- produce an appearance-location map;
- produce a first-appearance move-number distribution;
- produce formation and post-pattern activity maps;
- produce transformation and colour counts;
- allow explicit refinement from aggregate views.

### Stage 9: search interface refinement

- expose independent search choices;
- show the active search definition;
- allow settings to be changed without reconstructing the board;
- provide clear result and candidate summaries;
- support navigation between appearances;
- support comparison of candidate continuations;
- make map and histogram refinements inspectable.

### Stage 10: sequence search and external analysis

- design sequence search only after position-pattern search is mature;
- support move order and optional intervening moves;
- investigate longer local continuations and delayed returns;
- consider later KataGo-assisted analysis;
- keep KataGo ownership separate from any heuristic influence map.

---

## Architectural principles

Pattern semantics belong in the Rust core library.

The core should define:

- pattern point states;
- geometric transformations;
- colour treatment;
- spatial constraints;
- temporal constraints;
- appearance identity;
- deduplication;
- result summaries;
- aggregate result data.

The Qt interface should:

- construct a search definition;
- invoke the Rust search API;
- display progress and results;
- request per-game occurrence details;
- visualise aggregate analysis;
- create explicit refinements.

QML should not:

- implement transformations;
- interpret pattern semantics;
- query SQLite;
- reproduce search logic;
- assign strategic meaning to shapes.

The CLI and GUI should use the same core definitions.

Search result types should remain stable enough for:

- the GUI;
- command-line tools;
- tests;
- future analysis utilities;
- possible personal-game analysis;
- possible KataGo-assisted study.

---

## Performance principles

Search semantics must remain usable on large databases.

The implementation should avoid:

- retaining millions of raw occurrences when summaries are sufficient;
- one metadata query per matching game;
- blocking the GUI while preparing results;
- serialising all occurrence data before any rows can be shown.

The preferred pattern is:

1. scan games;
2. retain bounded summaries;
3. prepare result rows in batches;
4. display one row per game;
5. load or recompute detailed appearances for a selected game;
6. compute aggregate and candidate analysis from compact appearance data.

Progress reporting should distinguish at least:

- preparing the database;
- scanning games;
- preparing result summaries;
- displaying results.

Cancellation should remain meaningful until the result set is ready.

---

## Non-goals for the immediate milestone

The next milestone does not need to include:

- fuzzy shape similarity;
- machine-learning pattern recognition;
- automatic opening-name recognition;
- tactical evaluation;
- automatic strategic labels such as aggressive or defensive;
- an unexplained ranking of candidate move quality;
- causal conclusions from game outcomes;
- an influence or territory heuristic;
- KataGo analysis;
- full sequence search.

The immediate goal is to make exact and transformed position searches lead
into a clear candidate-investigation workflow: reveal professional
continuations, select one, inspect its supporting games and compare the
evidence without overstating what it proves.

---

## Open design questions

The following questions should be settled through examples and testing rather
than assumption:

1. Should touching an edge automatically suggest anchoring?
2. Should the user always confirm the spatial scope explicitly?
3. How should overlapping appearances be presented?
4. Should a pattern broken for one move and immediately restored count as a
   new appearance?
5. Should result sorting initially use date, game ID, appearance count or first
   matching move?
6. How should required-empty and unspecified points be edited visually?
7. Should exact search and position-pattern search use separate setup tools or
   one editor with a mode selector?
8. How much of a selected rectangle should default to unspecified?
9. How should transformed match locations be highlighted?
10. Which spatial scopes belong in the first position-pattern interface?
11. How should selected board regions be represented?
12. Should move-number analysis use fixed bands, configurable bands or a
    continuous histogram?
13. How should appearance-location filtering combine with transformations and
    colour reversal?
14. Should late appearances of an opening formation be included by default and
    shown as outliers?
15. How should aggregate analysis count simultaneous appearances?
16. When should several appearances be grouped into one local episode?
17. Should candidate summaries count distinct players as well as appearances
    and games?
18. How should outcome counts be displayed without implying move quality or
    causation?
19. How should two candidate continuations be compared without crowding the
    board and result list?
20. How should the interface explain that a recognised shape does not imply a
    fixed strategic function?
21. Which examples should become permanent regression and acceptance tests?

---

## Direction of travel

MoyoDB should not attempt to guess what the user means by *similar*.

It should let the user define similarity through independent, visible
choices:

- which stones matter;
- which empty points matter;
- which points do not matter;
- which geometries are equivalent;
- whether colours may be exchanged;
- whether edge relationships matter;
- whether a board region matters;
- whether the game stage matters;
- whether move order matters;
- how repeated positions are counted.

Continuation maps, appearance-location maps and move-number distributions
should help the user understand the result set and refine the search. They
should not silently redefine the search or rank move quality.

The principal study path should lead from a candidate continuation to the
supporting games, subsequent play and descriptive context. Frequency and game
results identify questions worth investigating; they do not supply final
verdicts.

Pattern recognition should establish structural and historical facts.
Strategic interpretation should remain contextual, inspectable and open to
Go judgement.
