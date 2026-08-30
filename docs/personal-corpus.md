# Bermuda Personal Corpus

## Status

Design document.

## Purpose

Bermuda's personal corpus is a collection of a player's own games intended for longitudinal study. Its purpose is not merely to provide another SGF library. It should allow Bermuda to discover recurring strengths, weaknesses, decisions, and patterns of behaviour across many games.

The personal corpus is deliberately distinct from Bermuda's professional-game corpus.

The professional corpus answers questions such as:

- How have professional players handled this position or local pattern?
- What continuations occurred in comparable professional games?
- Which players, dates, events, or results are associated with those examples?

The personal corpus answers different questions:

- What do I repeatedly do in positions of this kind?
- Which kinds of positions repeatedly cause me difficulty?
- Are there mistakes that recur across otherwise unrelated games?
- Are there areas in which my decisions are consistently strong?
- Am I improving?
- What can professional games teach me about a recurring personal weakness?

KataGo provides a third source of evidence: machine analysis of positions in the personal corpus.

These three sources must remain conceptually distinct even when Bermuda presents them together.

## Corpus separation

Personal games must not be imported into the professional corpus.

The professional corpus is intended to remain a corpus of professional games. Historical amateur games that happen already to exist in source collections are source-data oddities and do not justify deliberately mixing modern personal games into it.

A Bermuda installation may therefore contain:

1. a professional corpus;
2. one or more personal corpora;
3. KataGo analysis associated with personal games and selected other positions.

They may share reusable Bermuda infrastructure such as SGF parsing, board representation, canonicalisation and pattern geometry, but their database identities and purposes remain separate.

This separation also allows a professional database to be replaced or rebuilt without affecting a user's personal games or analysis.

## Initial personal-corpus capabilities

The first implementation should be deliberately modest.

It should support:

- creation of a personal corpus;
- explicitly adding a game played inside Bermuda;
- importing individual SGF files;
- importing directories of SGF files;
- duplicate detection;
- preserving original SGF metadata;
- listing and replaying personal games;
- recording the player's identity within personal games;
- requesting KataGo analysis of one position;
- requesting KataGo analysis of an entire personal game;
- storing analysis for later use.

A game played inside Bermuda must not be added to the personal corpus
automatically merely because it was played or saved. The user should make an
explicit decision to add it to My Games.

The first implementation does not need to discover sophisticated Go concepts automatically.

## Game representation and ingestion

`GameRecord` is the neutral interchange object between Bermuda's game-producing
and game-consuming components.

Conceptually:

    SGF file ------------------+
                               |
    game played in Bermuda ----+--> GameRecord --> corpus ingestion
                               |
    future game sources -------+

SGF is therefore an external interchange and save format, not Bermuda's
internal glue.

An SGF import is parsed into a `GameRecord` and then passed to the same
source-independent ingestion path that can accept a game played directly
inside Bermuda. A played game should not be serialised to SGF and parsed again
merely in order to enter the personal corpus.

The professional corpus and a personal corpus may both use Bermuda's ordinary
`Project` infrastructure: canonical game storage, source/provenance records,
metadata, replay, catalogues and position indexes. They remain separate
projects/databases because their purposes and ownership differ.

The same canonical game may have more than one source occurrence. Source
metadata and provenance remain attached to those occurrences rather than being
folded into canonical game identity.

For an imported SGF, provenance naturally includes the source file path. For a
game created inside Bermuda, provenance should identify it as a Bermuda-played
game without inventing a fictitious SGF source file.

## Identity

A personal corpus needs to know which player is the owner or subject of analysis.

Player identity should not depend on a single literal SGF spelling. A user may appear under different names or server accounts.

The personal-corpus layer should eventually support a curated personal identity with aliases, while preserving the original SGF metadata unchanged.

For example:

    Personal identity
        ├── "Gerry"
        ├── "gerrysmith"
        └── another server account

This is analogous to the broader curated-player-identity problem already anticipated for Bermuda, but personal identity is especially important because colour, opponent and result must be interpreted from the user's point of view.

## KataGo-derived move observations

For an analysed personal game Bermuda should be able to retain, as appropriate:

- game identity;
- move number;
- position identity;
- colour to play;
- move actually played;
- KataGo candidate moves;
- KataGo preferred move or moves;
- score estimate before the move;
- score estimate after the move;
- estimated point loss;
- win probability before and after;
- policy information where useful;
- analysis visit count or equivalent effort;
- analysis settings;
- KataGo version;
- neural-network/model identity.

The precise schema should be designed when implementation begins.

Analysis provenance is essential. A numerical judgement is not timeless truth: changing the model, rules, komi or analysis effort may change it.

## Score loss and move classification

For behavioural analysis, score loss should normally be the primary quantitative measure. Win probability should remain available, but it can be misleading as a measure of the magnitude of a mistake, particularly in games that are already strongly won or lost.

Human-readable classifications such as:

- excellent;
- good;
- inaccuracy;
- mistake;
- blunder;

may be useful in the interface, but their thresholds are Bermuda presentation policy rather than intrinsic properties of KataGo output.

Thresholds should therefore be configurable or revisable without reanalysing the underlying games.

## From individual errors to recurring behaviour

The central personal-corpus feature is cross-game analysis.

A conventional AI review asks:

> What happened in this game?

Bermuda should additionally ask:

> What do I repeatedly do across my games?

The initial pipeline can be:

1. Analyse personal games with KataGo.
2. Identify moves with interesting evaluation changes.
3. Extract local or larger-board patterns around those positions.
4. Use Bermuda's geometric/pattern machinery to compare those positions.
5. Cluster or otherwise identify repeated similar situations.
6. Present candidate recurring behaviours to the user.
7. Allow the user to explore every contributing game and position.
8. Search the professional corpus independently for comparable patterns.

This is deliberately a discovery process. Bermuda should not require a hand-written taxonomy of every possible Go mistake before it can find useful recurring behaviour.

## Example

Suppose KataGo identifies 47 decisions in a player's corpus with a loss greater than a chosen threshold.

Bermuda may discover that nine occur in sufficiently similar local positions.

It might present:

    Possible recurring weakness

    Similar occurrences: 9
    Games involved: 7
    Average estimated loss: 4.1 points

    Explore occurrences
    Compare with professional games

The user can then inspect the actual positions rather than accepting a statistical label without evidence.

A professional-corpus search might reveal that the user's preferred continuation is uncommon among professionals and show the alternatives professionals actually played.

KataGo therefore helps Bermuda *find* potentially interesting behaviour; the professional corpus supplies independent human evidence about comparable positions.

## Strengths as well as weaknesses

The system must not be designed only as an error detector.

The same infrastructure should identify:

- consistently good decisions;
- types of fighting position handled well;
- effective opening choices;
- good endgame judgement;
- improvements over time;
- previously recurring mistakes that have disappeared.

For example:

    Improvement detected

    Earlier 25 games: average loss 3.8 points
    Recent 25 games:  average loss 1.4 points

Such comparisons need statistical care, particularly because opponents, time controls and game contexts may differ.

## Pattern discovery

There are several possible levels of similarity:

- exact position;
- existing Bermuda pattern match;
- local geometric similarity;
- whole-board contextual similarity;
- semantic Go concepts inferred later.

The first implementation should exploit the pattern machinery Bermuda already has rather than introducing machine-learning clustering prematurely.

As Bermuda's pattern representation develops, the personal-corpus analyser can make progressively better use of it.

The existing Bermuda opening/pattern rules should not be assumed to describe every personal-game situation. Behavioural analysis will occur throughout the game and needs a more general position-comparison mechanism.

## Professional comparison

A personal pattern and a professional pattern search are connected by a query, not by merging their corpora.

Conceptually:

    personal positions
           |
      recurring pattern
           |
           v
       Bermuda query
           |
           v
    professional corpus
           |
      matching examples

Professional examples should expose the underlying games, continuations and metadata so that the user can study them directly.

The system should avoid claims such as "professionals prove this move is correct". Frequency, selection effects, era and surrounding board context all matter.

## Longitudinal analysis

A major advantage of a persistent personal corpus is time.

Future Bermuda versions should support questions such as:

- What weaknesses have appeared recently?
- Which old weaknesses have diminished?
- Which mistakes cost me the most points over the last six months?
- Do I repeatedly mishandle the same shape?
- Do I perform differently as Black and White?
- Which opening situations lead to poor later outcomes?
- Which kinds of decisions are improving?

The raw games and stored analysis should make these questions possible without dictating the exact future interface now.

## Privacy and portability

Personal games and derived analysis belong to the user.

The initial design should be local-first. No Bermuda service should require uploading personal games to a remote Bermuda server.

The corpus should be backupable and portable. Database formats should be versioned and migration paths provided when schemas change.

## Non-goals for the first implementation

The first personal-corpus implementation does not need:

- automatic natural-language Go coaching;
- a complete taxonomy of Go mistakes;
- cloud synchronisation;
- online accounts;
- automatic opponent profiling;
- training a neural network on the user's games;
- automatic alteration of SGF source files.

The goal is first to create reliable data infrastructure on which richer analysis can later be built.

## Architectural principle

The core relationship is:

    Professional corpus ---- pattern evidence ----+
                                                  |
    Personal corpus ---- recurring behaviour -----+--> Bermuda
                                                  |
    KataGo -------- position evaluation -----------+

Each source answers a different question. Bermuda's value comes from allowing the user to move naturally between them without confusing one kind of evidence with another.

A personal corpus should initially be implemented as a separate ordinary
Bermuda project rather than as a parallel database technology. Personal-only
features such as annotations, ownership information and KataGo analysis can be
added alongside the shared game-storage foundation as their schemas are
designed.
