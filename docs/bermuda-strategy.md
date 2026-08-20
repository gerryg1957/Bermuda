# Bermuda Strategy

## 1. Purpose

Bermuda is a database for studying professional Go games through position,
pattern and sequence search.

Its purpose is not merely to locate geometrically similar arrangements of
stones. It should help a player investigate questions such as:

- How have strong human players handled comparable positions?
- Which continuations occurred most often?
- What alternatives did professionals choose?
- Did professionals normally continue locally, tenuki, or return later?
- How did the position arise?
- Which examples are genuinely useful for understanding the shape?
- How does professional practice compare with modern engine analysis?

Bermuda should preserve the strengths of professional-game databases while
being explicit about their limitations.

It is neither a replacement for KataGo nor simply a graphical front end for
an engine. Human-game evidence and engine analysis answer different
questions. The value of Bermuda lies partly in bringing those forms of
evidence together without confusing them.

## 2. Human precedent, not proof

A professional-game database records what strong players actually played in
real games.

That evidence is valuable, but it does not prove that a move was best.

If five professionals chose one continuation and eight chose alternatives,
Bermuda may report those choices and their frequency. It should not say that
the five players knew their continuation was optimal, nor that the eight
alternatives were inferior.

A professional may choose a move because:

- it is believed to be strongest;
- it suits the whole-board position;
- it is familiar;
- it is easier to play;
- it avoids a difficult fight;
- it fits the player's preferred style;
- it is appropriate when ahead or behind;
- it creates practical problems for the opponent;
- or a better move was simply missed.

The database normally cannot recover the player's actual reasoning.

Bermuda should therefore distinguish carefully between:

- **observed fact** — what was played;
- **statistical association** — what tended to occur with it;
- **engine evaluation** — what KataGo assesses;
- **interpretation** — a plausible explanation;
- **player commentary** — direct evidence of intention, when available.

## 3. The value and limits of game results

Win and loss records provide useful context, but they do not directly
evaluate a local continuation.

A continuation may appear frequently in won games because:

- stronger players chose it;
- the player was already ahead;
- the whole-board position favoured that continuation;
- the opponent later made an unrelated mistake;
- or the local choice genuinely contributed to the result.

Likewise, a sound continuation may be associated with losses because it was
played mainly from already difficult positions.

Bermuda should present results descriptively:

> This continuation occurred in 13 games: 5 wins and 8 losses.

It should avoid unsupported causal language:

> This continuation causes losses.

Useful future comparisons may control or annotate for:

- player strength;
- opponent strength;
- colour;
- komi and rules;
- era;
- game state before the continuation;
- engine evaluation before and after the local sequence.

Even with those additions, the statistics remain evidence rather than proof.

An uneven result distribution is nevertheless worth investigating. If one
continuation appears mainly in won games and another mainly in lost games,
Bermuda should help the user ask why. The difference may reflect move quality,
prior board state, player strength, strategic choice or later play. It is a
lead for investigation, not a verdict.

## 4. Candidate discovery and pattern search

Pattern matching is a candidate-generation process.

The practical learning problem is often not choosing between several moves
that have already been analysed. It is the earlier question:

> Where should I even consider playing next?

A stronger player recognises more plausible candidates through tactics,
strategy, book knowledge and accumulated intuition. A less familiar position
may leave the student with no candidate at all. When a professional move is
then shown, the response is often “oh yes” even though the move was not
previously within the student's field of vision.

Bermuda should support a non-linear expansion of that field of vision. If eight
professionals played at one point, five at another and three elsewhere, those
figures identify recurring professional candidates. They do not establish an
absolute ranking, but they show which moves deserve investigation.

> Bermuda does not tell the user what must be played. It shows what the user
> should be thinking about.

The intended investigation workflow is:

1. find comparable professional positions;
2. reveal the candidate continuations that were actually played;
3. select one candidate and filter to its supporting games;
4. inspect what happened locally and elsewhere;
5. compare the contexts and descriptive outcomes of alternative candidates;
6. later compare the human evidence with KataGo's assessment of the exact
   whole-board position.

Pattern search itself answers:

> Where does this arrangement of required black, white and empty points occur?

Ordinary study searches should include:

- rotations;
- reflections;
- reversed colours;
- deduplication of equivalent symmetric forms.

The orientation of the board and the nominal colour of the player are
normally irrelevant when comparing strategic ideas.

Each result must nevertheless retain its transformation metadata so that
moves and aggregate maps can be normalised into the orientation and colour
frame of the original query.

A result should be described as:

> Position after move N

This is more accurate than saying merely “move N”. The position may have
been created by a placement inside the rectangle, by a capture, or by a
larger tactical event outside it.

## 5. Formation, existence and continuation

A pattern occurrence has a lifecycle:

1. precursor position;
2. pattern-forming event;
3. first matching position;
4. period during which the pattern remains present;
5. local continuation, tenuki or delayed return;
6. disappearance or transformation of the pattern.

These stages support different study questions.

### Formation

> How and why did this shape arise?

Formation study examines the moves before the first matching position. This
is especially important when a capture produces the pattern.

### Continuation

> What did professionals play after the shape existed?

Continuation study examines subsequent play, including immediate local
moves, tenuki and later returns.

### Persistence

> Was the pattern active, settled or incidental?

Persistence study considers how long the pattern remained, whether the
surrounding area stayed active and whether the match occurred only as an
accidental configuration on a crowded board.

Bermuda should not treat these as the same question.

## 6. Correct matches are not always useful matches

A geometrically exact result may still be a poor study example.

This commonly occurs when:

- the board is already crowded;
- the local position is settled;
- the pattern arose incidentally through a capture;
- the next important play is elsewhere;
- distant stones determine the meaning of the position;
- or the match occurs late in a game with little relevant continuation.

Such results should normally remain available. Their lack of further local
play can itself be informative. However, they should not necessarily rank
alongside examples containing an active and instructive continuation.

Bermuda should distinguish:

- search correctness;
- contextual similarity;
- instructional usefulness.

## 7. Continuation, activity, influence and ownership maps

Several visually similar board overlays answer different questions. Bermuda
should name and present them separately.

### Continuation map

The continuation map is a corpus-derived **immediate-next-move distribution**.
For each distinct appearance, it records the move played immediately after
the matched position and normalises that point into the query's orientation
and colour frame.

It answers:

> Where did professionals play immediately next in comparable positions?

The display may use larger or stronger circles for more frequent moves, but
frequency must not be presented as proof of quality. A continuation point
should lead directly to the games that support it. Passes, moves outside the
displayed area and games that ended at the matched position should remain
visible in the summary rather than disappearing from the evidence.

### Formation and local-activity maps

Later aggregate views may examine wider time windows. Post-pattern activity
maps may show:

- the next 5 moves;
- the next 10 moves;
- the next 20 moves;
- the next local sequence;
- later returns to the area.

Pre-pattern formation maps may show:

- common pattern-forming moves;
- common approach sequences;
- captures that produce the shape;
- whether the shape was deliberately constructed or appeared as an outcome
  of another fight.

These maps should complement, not conceal, the source games. A user must
always be able to open a representative occurrence and replay the complete
sequence.

### Influence map

An influence map is not derived from professional continuation frequencies.
It is a heuristic field calculated from the current board position. It may
help a player see the balance between secure territorial “cash”, outward-facing
influence and contested areas, and how influence gradually converts—or fails
to convert—into territory as the game develops.

A suitable model may calculate a continuous field internally, but the visible
display should use only broad qualitative bands:

- strong Black or White influence;
- weak or provisional Black or White influence;
- neutral or contested space.

This avoids suggesting more precision than the heuristic can justify. Strong
influence must not be described as settled territory or as a probability of
final ownership. Influence remains a future, separate feature after the
candidate-investigation workflow has been developed further.

### KataGo ownership map

A future KataGo ownership map is separate again. It estimates eventual point
ownership in the exact whole-board position. It should not silently replace
the influence map or be confused with historical continuation evidence.

## 8. Ranking and classification

Search results should eventually be ranked by explicit, inspectable
features rather than an unexplained score.

Possible measurements include:

- distance from the pattern to the next move;
- number of nearby moves among the next 5, 10 or 20 moves;
- time until the next local move;
- length of the following local sequence;
- activity before the pattern appeared;
- whether the pattern was produced by a capture;
- whether the area was revisited;
- board occupancy;
- amount of open space around the pattern;
- number of comparable professional examples.

Useful descriptive categories may include:

- immediate local continuation;
- local answer followed by tenuki;
- delayed return;
- formation example;
- capture-created outcome;
- settled position;
- incidental crowded-board match;
- opening or fuseki example;
- active fighting example;
- endgame example.

A result may belong to more than one category.

The interface should explain ranking in ordinary language, for example:

> 7 of the next 10 moves were nearby. Local play continued for 12 moves.

or:

> No nearby move occurred in the next 20 turns. The area was not revisited.

## 9. Raw matches, appearances and local episodes

Bermuda should preserve several levels of grouping.

### Raw match

Every indexed position satisfying the pattern.

### Appearance

A continuous period in which the same physical occurrence remains present.
Repeated unchanged positions should not appear as unrelated examples.

### Local episode

Several nearby or overlapping appearances arising during one local sequence.

A rectangle may shift by one line between consecutive moves and technically
produce two appearances. For study purposes, those appearances may still
belong to one local episode.

This distinction will help prevent result lists from overstating the amount
of independent professional evidence.

## 10. Fuseki and game stage

Opening positions may benefit from specialised treatment because their
meaning often depends on large areas or the whole board.

Bermuda should not reproduce an opaque “fuseki harvest” process merely
because an earlier program used one. Nor should it assume that one fixed
move number cleanly separates useful pattern search from positions better
suited to engine analysis.

Move number may be one feature, but game stage is better described through
context such as:

- board occupancy;
- distribution of stones;
- amount of open space;
- local contact and fighting;
- established territories;
- whole-board sensitivity;
- availability of comparable examples.

Pattern search will often be more useful earlier in a game, while KataGo may
become more useful as exact whole-board details dominate. This is a tendency,
not a universal cutoff.

A late life-and-death or endgame pattern may still have valuable precedents.
An early position may already depend too strongly on distant stones for a
small local pattern to be meaningful.

## 11. Relationship with KataGo

KataGo and professional-game search should be complementary.

KataGo asks:

> What move appears strongest in this exact whole-board position?

Professional-game search asks:

> What did strong human players actually play in comparable positions?

Those answers may differ.

An engine may prefer tenuki. Professionals may have continued locally
because:

- they did not identify the engine move;
- the local sequence was easier to understand;
- it reduced practical risk;
- it suited their judgement of the game;
- or it posed more difficult human problems.

For players below professional strength, a clear and robust human sequence
may sometimes be more educational than an unexplained engine tenuki, even
when the engine move is objectively superior.

Future Bermuda analysis may compare:

- frequency in professional games;
- professional alternatives;
- local and whole-board game results;
- KataGo policy;
- KataGo score or win-rate change;
- ownership change;
- difficulty and complexity of resulting variations.

The purpose is not to declare humans right or wrong in every case. It is to
show how human practice and machine evaluation relate.

## 12. The problem of “why”

The absence of a comprehensible “why” is one limitation of raw AI analysis.

KataGo may identify a best move and provide variations, but this does not
automatically produce an explanation suited to a human learner.

A professional corpus also cannot normally recover why a player chose a
move. It can, however, provide evidence from which useful questions and
interpretations emerge:

- Does the move defend a weakness?
- Does it retain sente?
- Does it simplify?
- Does it invite or avoid fighting?
- Does it favour territory or influence?
- Is it commonly chosen only when the player is ahead?
- Do professionals with different tendencies choose different continuations?
- Is a slightly inferior engine move much easier to play?

Bermuda should support explanations grounded in visible evidence while
labelling inference honestly.

When contemporary commentary, annotations or player statements are
available, they provide a stronger source for intention than either pattern
frequency or engine output.

## 13. Player style

Player style may help interpret variation in professional choices, but it
must be handled cautiously.

Bermuda may eventually identify corpus-level tendencies such as:

- frequency of tenuki;
- preference for influence or territory;
- willingness to enter complex fights;
- tendency to simplify;
- preferred opening systems;
- local versus whole-board emphasis.

Such tendencies may help compare groups of games or players.

They should not be used to claim certainty about a particular decision:

> This player chose the move because of their style.

Style is contextual, changes over time and is difficult to reduce to one
label. Any classifications should be transparent about their source and
method.

## 14. Strategic division of labour

Bermuda should develop three connected layers.

### Human evidence

- professional occurrences;
- continuation frequencies;
- local and tenuki distributions;
- representative games;
- qualified win/loss statistics;
- player and era distributions.

### Machine evidence

- KataGo candidate moves;
- policy probabilities;
- score and win-rate changes;
- ownership changes;
- tactical variations;
- comparison of played and recommended moves.

### Interpretive context

- pattern formation;
- capture-created positions;
- local activity;
- game stage;
- whole-board sensitivity;
- possible strategic functions;
- available human commentary.

The interface should allow these layers to support one another without
presenting any one of them as the complete truth.

A heuristic influence map belongs to interpretive context rather than human
or machine evidence. Its assumptions should be documented, and it should be
visually distinct from both the continuation map and KataGo ownership.

## 15. Implementation priorities

### Current foundation

- reliable SGF import;
- canonical deduplication;
- position indexing;
- exact pattern matching;
- rotations, reflections and colour reversal;
- distinct-appearance counting;
- responsive asynchronous search;
- accurate navigation to each occurrence;
- a normalised immediate continuation map;
- selectable continuation points;
- in-memory filtering to the games supporting a selected candidate;
- a frequency-ordered Professional continuations list without treating
  frequency as quality;
- separate appearance and distinct-game counts;
- explicit retention of passes, off-map moves and ended games in the
  continuation evidence;
- A/B comparison of two candidate continuations;
- direct access to each candidate's supporting games;
- descriptive SGF outcome summaries using recorded Black/White game colours.

The candidate-investigation workflow is therefore established: a user can
discover professional candidates, inspect how often they occurred, compare
two of them, and move directly into the games that provide the evidence.

Historical outcomes remain prompts for investigation rather than evaluations.
Because ordinary searches include colour reversal, the current Black/White
outcome counts describe the recorded supporting games; they are not presented
as a chooser-relative candidate win rate.

### Next priority: occurrence context

- first and last matching positions;
- pattern duration;
- forming move or capture;
- previous local moves;
- next local move;
- tenuki and delayed-return detection;
- board occupancy and local activity measurements;
- grouping into appearances and local episodes.

This is the next useful step because candidate frequency tells the user
**what** professionals considered, while occurrence context begins to explain
**what kind of situation** produced each example and whether it is a useful
example for study.

### Broader aggregate views

- pre-pattern formation maps;
- post-pattern local-activity maps;
- explainable ranking of useful examples;
- result categories and representative examples;
- appearance-location and move-number distributions;
- an experimental, transparent influence-map prototype only after occurrence
  context and candidate investigation are mature.

### Engine integration

- optional KataGo analysis;
- comparison of human choices with engine candidates;
- evaluation before and after continuations;
- a separately labelled KataGo ownership map;
- identification of robust human alternatives;
- support for personal-game analysis.

## 16. Long-term direction

Bermuda should become a tool for exploring the relationship between:

- local shape;
- whole-board context;
- professional practice;
- practical game outcomes;
- player tendencies;
- and machine evaluation.

It should not pretend that frequency proves quality, that victory proves a
local move was correct, or that an engine recommendation explains itself.

Its distinctive contribution should be to expand the user's candidate
knowledge and make the supporting evidence explorable:

> What happened in professional games?

> How did the position arise?

> What alternatives were played?

> What happened next locally and elsewhere?

> How does KataGo assess those choices?

> What plausible strategic explanations fit the evidence?

The aim is not merely to find matching stones. It is to help players develop
a better-founded understanding of professional choices while remaining
honest about what the available evidence can and cannot establish.
