# Questions about the meaning of pattern and continuation

**Status:** working design note, not a specification
**Date:** 7 August 2026

**Purpose of sharing:** this document is intended to invite criticism, elaboration and pruning. It records current questions and distinctions, not conclusions that collaborators are expected to accept.

This note collects questions that have emerged during the development of MoyoDB. They are deliberately left partly unresolved. The purpose is to preserve the reasoning behind the project while the implementation makes the underlying ideas more precise.

A recurring danger is to let a convenient technical definition become the meaning of the Go concept it is intended to approximate. Exact pattern matching, occurrence duration, next-move statistics and other mechanisms are useful because they give us evidence. They are not, by themselves, the thing MoyoDB is ultimately trying to understand.

The fundamental intention remains broader: to help a player enlarge the set of things they know to consider — not simply to say which move is “right”, but to reveal what strong players thought worth considering and to make the ideas behind those possibilities available for investigation.

## 1. What is a pattern?

At the implementation level, a pattern is currently a rectangular set of occupied and empty intersections. That is precise and searchable.

At the level of Go understanding, however, “the same pattern” can mean several different things.

### Exact positional pattern

The strictest meaning is:

- the same stones are present;
- the same intersections are empty;
- the selected rectangle is otherwise unchanged.

This is the meaning currently used by exact pattern search and by occurrence-duration measurement.

It is useful because it is objective. But it can be much narrower than the pattern a human player thinks they are looking at.

### Structural pattern

A player may regard a formation as still being the same shape even after extra stones have been added.

A framework such as a Chinese-opening formation can illustrate this. A handful of stones may define a recognisable strategic structure even after later play has filled points that were empty when the original position was selected.

This raises an unresolved question:

> Which intersections are essential to the identity of a pattern, and which merely describe the position at the moment the pattern was selected?

A future structural search might therefore need some notion of:

- required stones;
- required empty points;
- unconstrained or “don't care” points;
- perhaps relationships between stones rather than only exact occupancy.

That would be a different search semantics, not merely an extension of exact matching.

### Sparse does not mean underspecified

The discussion of structural patterns must not be read as implying that a sparse selection is necessarily a badly specified exact pattern.

A sparse early-board position may be exactly what the user intends to study. A beginner looking at six stones in an opening may quite reasonably be asking:

> “Here is this position. Where do strong players play next?”

In that question the empty intersections are part of the position. The point of the search is not necessarily to recognise a named opening system; it may be to discover the candidate moves from which strategic understanding can begin.

The same visible selection can therefore support two different and equally legitimate questions:

1. **Exact sparse-position search** — what did professionals play next from this precise early-board position?
2. **Structural-framework search** — when these defining stones were present, what ideas developed around the framework even if other points differed?

Pattern density cannot tell MoyoDB which question the user intends. The distinction belongs in the search semantics, not in an automatic assumption that sparse areas are “don't care”.

## 2. What counts as “the same” pattern?

For ordinary study, orientation and colour usually should not make strategically equivalent positions separate cases.

MoyoDB therefore treats rotations, reflections and colour reversal as ordinary equivalences for pattern search. This reflects the purpose of the search: to investigate ideas and continuations, not to distinguish positions merely because they occurred in another corner or with the colours exchanged.

But geometric equivalence is only the simplest kind of equivalence.

Questions that remain include:

- Can two patterns be strategically equivalent despite small differences in nearby stones?
- When does an added stone change the identity of the pattern rather than merely add context?
- Can a framework or joseki “family” be recognised without requiring exact local occupancy?
- How much surrounding context is needed before two locally identical shapes cease to be meaningfully comparable?

These are questions about Go meaning, not merely about search algorithms.

## 3. What is an occurrence?

Raw exact matching can produce the same match at many consecutive board positions. If nothing in the selected rectangle changes while moves are played elsewhere, the pattern continues to match after every one of those moves.

Treating each snapshot as a new occurrence would exaggerate the evidence.

MoyoDB therefore currently treats consecutive matching positions with the same location and equivalence transformation as one **appearance**.

For an appearance we can measure:

- first matching position;
- last matching position;
- duration = last position − first position.

Thus a pattern present only after move 50 has duration 0, while one present after moves 50 through 55 has duration 5.

This is deliberately a measurement, not an interpretation.

## 4. What does duration mean?

A long exact-pattern duration tells us something precise:

> Nothing inside the selected rectangle changed during that interval.

It does **not** by itself tell us why.

A long duration may occur because:

- the local position is settled;
- both players have tenukied temporarily;
- forcing or asking moves are being exchanged elsewhere;
- important play continues just outside the selected rectangle;
- the selected rectangle is sparse enough that local strategic development does not alter its exact contents;
- the pattern has simply become irrelevant to the remainder of the game.

The observation that an appearance survives to the end of the game is potentially useful additional evidence. We can distinguish:

- exact pattern duration;
- whether it persisted to game end;
- the fraction of the remaining game for which it survived.

For example, an appearance beginning after move 148 in a 290-move game and lasting through move 290 survives **all 142 remaining moves**.

Even that does not prove “settled”. It becomes much more informative when combined with measurements of later activity near the pattern.

The converse matters too. An exact sparse opening position can have duration zero simply because the next move is played somewhere inside a large selected rectangle. That does not make the search unhelpful. For an opening inquiry, the immediate next move may be exactly the evidence the user wanted. Duration describes persistence of the exact configuration; it does not determine whether the search question was meaningful.

## 5. What is a continuation?

This question has become less simple as the project has developed.

### Chronological continuation

The easiest definition is:

> the next move in the game record after the first matching position.

This is what the current continuation map measures.

It is objective, reproducible and useful. It tells us what professionals actually played next.

But it does not always correspond to what a Go player means by “the continuation of this pattern”.

### Local continuation

Suppose a joseki position occurs after move 30. The players then exchange asking moves elsewhere:

- move 31: elsewhere;
- move 32: answer elsewhere;
- move 33: elsewhere;
- move 34: answer elsewhere;
- move 35: return to the joseki.

Chronologically, move 31 is the next move.

Strategically, move 35 may be the continuation the student is trying to discover.

This suggests that MoyoDB may need to distinguish:

- immediate next move;
- next move inside the selected region;
- next move within a margin around the region;
- delay before local return;
- sequence of local moves after return.

The important point is not to redefine the immediate continuation away. Both facts are useful; they answer different questions.

There is also a whole-board opening use of “continuation” that is neither a defect nor merely a local-sequence problem. In a sparse opening position, the immediate next move may expose a strategic choice before the student has learned the conventional vocabulary for it. For example, understanding why White chooses one sixth move may require considering whether White cares about allowing Black to establish a Kobayashi-style continuation, or whether White could instead occupy the point Black would otherwise take next. Two such moves may have comparable strategic value while leading to very different games.

This is close to MoyoDB's central educational purpose: the user can reach the question “where should I be thinking about playing?” before they have read the books, watched the videos, or learned the names by which stronger players discuss the position.

## 6. Asking moves, tenuki and interrupted sequences

Joseki and other local sequences need not be played as uninterrupted blocks.

Players may leave the local position in order to:

- make an asking move;
- force an answer;
- test an opponent's intentions;
- settle move-order questions elsewhere;
- take sente before returning;
- postpone the local decision until more whole-board information is available.

A database that assumes “the next move in the SGF is the continuation” risks breaking apart a sequence that a human player understands as one strategic thread.

This suggests a future concept of a **local episode**: a sequence of related local activity that can contain global interruptions.

An episode might include:

- the formation of the pattern;
- a period of inactivity;
- one or more returns;
- a local continuation sequence;
- eventual abandonment or settlement.

The boundaries of such an episode should be measured from explicit spatial and temporal evidence before MoyoDB tries to classify it.

## 7. Exact persistence versus structural persistence

The Chinese-opening and joseki examples expose two different ideas of persistence.

### Exact persistence

The selected rectangle remains exactly unchanged.

This is what current duration measures.

### Structural persistence

The defining formation remains recognisably present even though new stones are added.

A sparse opening framework may persist strategically for dozens of moves while its exact rectangle stops matching almost immediately.

This distinction may eventually require a richer pattern language. Possible ingredients include:

- “don't care” intersections;
- required anchors;
- optional local stones;
- relative rather than exact relationships;
- perhaps explicit user marking of which stones define the pattern.

For now, this should remain a question rather than an implementation commitment.

## 8. Local activity as context

Duration becomes more informative when combined with nearby activity.

For each appearance, useful measurements may include:

- number of moves played inside the selected rectangle after appearance;
- number of moves within 1, 2 or 3 intersections of it;
- time until the next local move;
- whether either player returns locally at all;
- number of local moves after the return;
- whether local activity is continuous or interrupted;
- whether the appearance persists to game end.

This would distinguish cases such as:

**Long duration + no nearby return**
Strong evidence that the local configuration was no longer being actively contested.

**Long duration + substantial nearby play**
The rectangle remained unchanged, but the surrounding position continued to develop.

**Long duration + delayed local return**
Possible tenuki, asking-move sequence or postponed continuation.

These remain observations. Labels such as “settled”, “tenuki” or “asking move” should come later, and only where the evidence justifies them.

## 9. Formation matters as well as continuation

A pattern is not only something from which play continues. It also came from somewhere.

The same final shape may have arisen through:

- the normal sequence;
- a different move order;
- a capture;
- a transposition;
- setup stones;
- a locally interrupted sequence;
- moves that arrived from different strategic purposes.

Therefore useful occurrence context may eventually include:

- the moves immediately before the first appearance;
- local activity before appearance;
- whether a capture created the pattern;
- whether the pattern emerged gradually or suddenly;
- whether the apparent sequence was interrupted by play elsewhere.

Two identical exact positions may have different study value because their histories differ.

## 10. A match is a candidate for investigation, not an answer

Pattern search should generate comparable professional examples. It should not silently turn frequency into judgement.

A continuation played by many professionals is important evidence that it is a recurring candidate. It does not follow that it is universally best.

The useful question is closer to:

> “Is this a move or idea I should know to consider here?”

rather than:

> “Which move wins the frequency count?”

This is central to the purpose of MoyoDB. Professional practice can expand the player's candidate set non-linearly: sometimes seeing one move produces the reaction “of course — now that I see it”, even though the move was outside the player's previous set of possibilities.

This can be especially valuable for less experienced players. Pattern search can expose candidate moves before the user possesses the established opening or joseki terminology. The database can therefore be a route *into* understanding, rather than merely a reference tool used after the theory has already been learned.

### Absence can also be evidence

Professional practice can refine or reject candidates as well as discover them.

There is an important difference between saying:

> “Professionals played A more often than B, therefore A is better.”

and observing:

> “This comparable position occurs many times, yet professionals apparently never chose C.”

The first statement turns frequency too quickly into judgement. The second can be strong **negative evidence**, especially when the number of genuinely comparable positions is large.

The denominator matters. Zero examples out of four comparable games says little. Zero examples out of hundreds is much more informative. Comparability matters too: an early sparse whole-board position may provide unusually clean evidence because relatively little hidden local context differs between occurrences.

This still does not make absence a mathematical proof that a move is bad. But MoyoDB should not become so cautious that it refuses to support ordinary practical judgement when the corpus evidence is strong. A useful conclusion may be:

> “There is probably something strategically wrong with treating this move as equivalent to the professional candidates.”

That conclusion leads naturally to the more valuable question: **why?**

In the Chinese-opening discussion, an apparently similar White move in the other corner produced no professional continuation in the searched corpus. That absence challenges the initial intuition that the two moves have equal strategic meaning. Direction of play, prospective frameworks, balance between corners, sente, or another whole-board consideration may explain the difference; the corpus tells us that there is a difference worth investigating before it tells us what the explanation is.

This gives MoyoDB two complementary educational mechanisms:

- **candidate discovery** — “I had not thought of that move”;
- **candidate rejection or refinement** — “I thought these moves were equivalent, but professional practice suggests that they are not.”

Both can enlarge the player's understanding of what should be considered in a position.

## 11. Human practice, outcomes and engine analysis are different evidence

Several evidence layers should remain distinct.

### Professional corpus evidence

This can tell us:

- what professionals played;
- how often a continuation occurred;
- which plausible candidates were absent, and from how many comparable positions;
- what followed;
- in what contexts;
- how sequences developed.

Presence and absence are both corpus evidence. Their strength depends on the size and comparability of the sample.

### Historical outcomes

Game results can be displayed as descriptive context.

They do not establish that a local continuation caused the result, and should not be converted into an implicit move-quality score.

### KataGo or other engine analysis

Engine analysis asks a different question:

> What does the engine prefer in this exact whole-board position?

That is not the same as:

> What did professional players choose in comparable positions?

The two evidence sources can be placed beside one another, but neither should be silently substituted for the other.

## 12. Professional practice changes over time

Professional-game frequencies are historical observations, not timeless evaluations.

A move or opening may become less common without becoming “wrong”. Changes in frequency can reflect several influences that may be difficult or impossible to separate cleanly:

- changes in strategic understanding;
- engine influence;
- fashion;
- what leading players or teams are currently studying;
- preparation for particular opponents or events;
- changes in the openings that professionals choose to enter in the first place.

The Chinese opening is a useful example. Searches of older professional material show substantial use of large frameworks, while such openings appear less frequently in contemporary elite practice. There would be nothing inherently anomalous about a professional choosing the Chinese opening today; reduced frequency does not by itself amount to a refutation of the opening.

A working observation from our discussions is that post-AI professional play can *appear* more territory-oriented and less willing to build very large frameworks. That should remain a hypothesis to investigate rather than a premise built into MoyoDB. Corpus counts alone cannot distinguish engine-driven strategic reassessment from fashion, preparation, or the current focus of professional study.

This matters educationally because historical professional games may remain highly relevant to positions that still occur frequently in amateur play. A strategic family can become unfashionable at the top level while continuing to be something an ordinary player needs to understand.

MoyoDB should therefore avoid silently treating “recent” as synonymous with “relevant” or “better”. A useful future view might show how professional treatment of a pattern changes over time, for example:

- continuation frequency by period;
- when a candidate becomes more or less common;
- whether the position itself becomes rarer;
- historical versus recent continuations shown side by side.

Such a view would still report evidence rather than explain its cause. The explanation may involve strategic evolution, fashion, current study, or several factors together.

Absence should also be interpreted historically. A move absent from recent professional games may simply have gone out of fashion or arise from an opening professionals now avoid. A move absent across a large body of comparable games spanning different periods is a different and potentially stronger observation. MoyoDB should make the relevant denominator and time range visible rather than treating every zero as equivalent.

## 13. The selected rectangle is an analytical choice

The meaning of a pattern depends partly on what the user selected.

A tight rectangle may isolate a joseki shape but omit surrounding strategic context.

A large rectangle may include enough board context to make matches more meaningful, but exact empty intersections then become increasingly restrictive.

A sparse large selection is particularly ambiguous in meaning. It may be an exact early-board position in which the empty intersections are deliberately part of the question, or it may be a way of pointing at a framework whose defining stones matter more than the intervening empty points.

This leads to another important question:

> Does selection mean “everything in this rectangle matters”, or “these are the features I am pointing at”?

At present MoyoDB implements the former. That is a legitimate and useful search semantics, including for sparse openings. A future structural search could implement the latter as an explicit alternative. MoyoDB should not infer one from the visual density of the selection.

## 14. Position identity and strategic identity are not the same

Several distinctions now recur:

- exact position vs recognisable structure;
- chronological next move vs strategic continuation;
- consecutive matching snapshots vs one appearance;
- inactivity inside a rectangle vs local inactivity;
- local inactivity vs settlement;
- game result vs local move quality;
- corpus frequency vs recommendation;
- corpus absence vs proof of inferiority;
- weak absence evidence vs strong absence evidence with a large comparable denominator;
- current popularity vs strategic validity;
- recent professional fashion vs historical educational relevance;
- engine preference vs historical professional practice.

These distinctions are likely to remain important even if the implementation changes.

They suggest a general principle:

> MoyoDB should preserve the difference between what it can observe exactly and what a Go player may infer from those observations.

## 15. A possible hierarchy of evidence

Without treating this as a fixed architecture, the discussions so far suggest a useful progression:

1. **Exact match** — where did this selected configuration occur?
2. **Appearance** — when did one continuous occurrence begin and end?
3. **Immediate continuation** — what was played next chronologically?
4. **Candidate distribution** — which continuations recur, which plausible candidates are absent, and what is the denominator?
5. **Local activity** — when and where did play next return nearby?
6. **Local episode** — what sequence of related local activity surrounds the appearance?
7. **Structural relationship** — does the underlying formation persist despite added stones?
8. **Context** — what was happening elsewhere on the board?
9. **Historical context** — when was the game played, and did professional treatment of this position change over time?
10. **Outcome** — how did the game eventually end?
11. **Engine evidence** — how does an analysis engine assess the exact whole-board position?

The earlier levels are easier to define objectively. The later levels require progressively more interpretation.

That argues for the design principle already emerging in MoyoDB:

> **Measure first, interpret second.**

## 16. Questions to preserve

These questions should remain open while MoyoDB develops.

- What features make two positions meaningfully “the same pattern” to a Go player?
- Which empty intersections are part of a pattern's identity?
- Should the user be able to mark intersections as unconstrained?
- Can structural patterns be defined without making search vague or unpredictable?
- When does a local sequence end?
- How should MoyoDB recognise a return after tenuki or asking moves?
- What spatial margin best represents “local” activity?
- Should that margin depend on the size or density of the selected pattern?
- Can local episodes be detected from measurements rather than hand-written Go categories?
- When is a long-lived pattern evidence of settlement, and what additional evidence is needed?
- How should capture-created patterns be represented in their formation history?
- How much whole-board context should be presented alongside a local match?
- When is a sparse opening selection an exact whole-board question, and when is it intended as a structural framework search?
- How should sparse opening frameworks differ from dense joseki patterns?
- How should MoyoDB show changes in professional practice over time?
- Can frequency changes be presented without pretending to distinguish strategic reassessment from fashion, preparation, or current study when the corpus cannot establish the cause?
- How can historical games remain visible when they are educationally relevant to positions still common in amateur play but less common in current professional play?
- Can professional continuation frequencies be presented prominently without inviting the interpretation “most frequent = best”?
- When is a zero-frequency candidate strong negative evidence rather than merely a small-sample accident?
- How should MoyoDB display the denominator and degree of positional comparability behind an apparent absence?
- How should absence be separated by historical period so that “not played recently” is not confused with “never considered in comparable professional play”?
- How should corpus evidence and KataGo analysis be compared without collapsing them into one judgement?
- What information helps a player discover a candidate move that was outside their previous knowledge?
- Can MoyoDB help reconstruct not merely the order of moves, but the strategic thread the players were pursuing?
- Which of these questions matter in practice, and which only appear important because of the current implementation?

## 17. Why keep these questions separate from the specification?

The implementation will necessarily choose precise definitions. Those definitions allow code to be written and tested.

But the project is still teaching us what its important concepts mean.

A specification says, for example, that an exact appearance runs from the first to the last consecutive matching board position. That can be correct and stable.

This document asks a different question:

> Does that measurement correspond to the kind of persistence a Go player cares about in this situation?

Keeping the two kinds of document separate allows MoyoDB to be rigorous without prematurely freezing its conceptual model.

Some questions here may eventually become features. Some may turn out not to matter. Some may be replaced by better questions.

That uncertainty is intentional.
