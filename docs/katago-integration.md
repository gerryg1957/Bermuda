# Bermuda KataGo Integration

## Status

Design document.

## Purpose

KataGo should provide AI analysis to Bermuda without becoming part of Bermuda's core database engine.

The primary use is analysis of personal games and positions, particularly to support discovery of recurring strengths and weaknesses across a personal corpus.

Secondary uses include interactive analysis of any position being viewed in Bermuda, including positions from professional games.

## Architectural boundary

Bermuda should run KataGo as a separate process and communicate with it through KataGo's supported analysis interface.

Conceptually:

    Bermuda
       |
       v
    KataGo adapter
       |
       v
    KataGo process
       |
       +-- configuration
       +-- neural-network model

KataGo should not be linked directly into the Bermuda executable.

This boundary provides several advantages:

- KataGo can be upgraded independently.
- Neural-network models can be changed independently.
- Bermuda remains insulated from KataGo's implementation language and build system.
- failures or termination of KataGo need not corrupt Bermuda;
- analysis can eventually be queued and performed asynchronously;
- advanced users can choose their own KataGo installation and model.

## Bermuda KataGo adapter

A reusable Rust component should own communication with KataGo.

The GUI should not parse KataGo output directly.

The adapter should eventually provide operations resembling:

- start engine;
- stop engine;
- report engine availability;
- analyse position;
- analyse game;
- cancel analysis;
- report progress;
- return structured analysis results.

Both command-line tools and `bermuda-qt` should be able to use the same adapter.

## Discovery and configuration

Bermuda should not assume one fixed KataGo installation path.

The application should eventually support:

1. automatic discovery where practical;
2. an explicitly configured KataGo executable;
3. an explicitly configured neural-network model;
4. an explicitly configured KataGo configuration file.

Bermuda packaging and KataGo packaging should remain separable.

If a distribution supplies KataGo, Bermuda should be able to use it. If not, the user should be able to point Bermuda at another installation.

The first implementation need not download KataGo automatically.

## Analysis modes

Bermuda will need at least two broad analysis modes.

### Interactive analysis

Used while looking at a particular position.

The priorities are:

- responsiveness;
- candidate moves;
- variations;
- score/win estimates;
- the ability to stop and move elsewhere quickly.

### Corpus analysis

Used to analyse many positions or complete personal games.

The priorities are:

- throughput;
- resumability;
- predictable analysis effort;
- persistent results;
- progress reporting;
- avoiding repeated work.

The adapter may use the same KataGo protocol for both, but Bermuda should treat the workflows differently.

## Analysis levels

Not every position needs expensive analysis.

A useful future strategy is staged analysis:

1. a relatively inexpensive pass over a complete personal game;
2. identification of potentially interesting moves;
3. deeper analysis of those moves;
4. still deeper analysis when explicitly requested by the user.

This can make analysis of hundreds or thousands of personal games practical.

The precise visit counts or time limits should be determined experimentally rather than embedded in this design document.

## Stored analysis

Analysis results should be stored rather than recalculated every time a game is opened.

Stored results need sufficient provenance to determine whether they are comparable.

Relevant provenance includes:

- KataGo version;
- model/network identity;
- rules;
- komi;
- board size;
- analysis parameters;
- visit count or equivalent effort;
- any Bermuda analysis-policy version.

An analysis result should be associated with a stable position/game identity.

## Reanalysis and invalidation

Installing a newer KataGo model must not silently reinterpret old numbers as though they came from the new model.

Bermuda should be able to distinguish:

- analysis available with the current configuration;
- analysis available from an older/different configuration;
- analysis not yet performed.

Old analysis need not automatically be deleted. It may be useful historically and reanalysis of a large corpus may be expensive.

## Metrics

Bermuda should retain KataGo's underlying numerical information where useful rather than storing only labels such as "mistake".

Important candidates include:

- score estimate;
- score change;
- win probability;
- candidate moves;
- visits;
- policy;
- principal variations.

For personal behavioural analysis, estimated score loss is expected to be the primary measure.

Move categories displayed to users should be derived by Bermuda from stored numerical results. This allows thresholds and terminology to change without rerunning KataGo.

## Analysis queue

Corpus analysis should eventually be represented as work items in a persistent queue.

This permits:

- stopping Bermuda and resuming later;
- pausing analysis while the computer is busy;
- avoiding duplicate analysis;
- showing meaningful progress;
- prioritising a game the user is currently viewing.

A crash or forced shutdown should not require starting a large corpus analysis from the beginning.

## Resource management

KataGo can consume substantial CPU/GPU and memory resources.

Bermuda should eventually expose simple controls such as:

- analysis enabled/disabled;
- analysis effort;
- background analysis pause/resume;
- CPU/GPU configuration where appropriate.

Advanced KataGo configuration should remain possible without forcing ordinary users to understand every engine option.

The GUI must remain responsive while KataGo is analysing.

## Failure handling

The adapter should treat KataGo as an external service that can fail.

It should handle:

- executable not found;
- model not found;
- invalid configuration;
- startup failure;
- malformed/unexpected output;
- process termination;
- individual analysis failure;
- user cancellation.

Such failures should not damage the professional or personal databases.

Useful diagnostics should be logged.

## Relationship with the personal corpus

The principal automated flow is:

    personal game
        |
        v
    positions/moves
        |
        v
    KataGo analysis
        |
        v
    interesting decisions
        |
        v
    Bermuda pattern comparison
        |
        +--> recurring personal occurrences
        |
        +--> professional corpus matches

KataGo is therefore a detector and evaluator, not the database itself.

## Relationship with professional games

Bermuda should not routinely analyse the complete professional corpus with KataGo.

The professional corpus is evidence of actual professional play and may contain hundreds of thousands of games. Wholesale AI analysis would be expensive and would blur the distinction between professional evidence and machine evaluation.

Interactive analysis of a selected professional position should, however, be available.

Any KataGo analysis of such a position must be visibly identifiable as AI analysis rather than professional evidence.

## User interface principles

AI information should assist game study without overwhelming the board.

A game-review view may provide:

- evaluation/score graph;
- key moves;
- estimated loss;
- candidate moves;
- variations;
- navigation between significant moments;
- links to similar personal occurrences;
- links to matching professional games.

The board remains the principal visual object.

The interface should make it clear whether a displayed recommendation comes from:

- KataGo;
- the player's own game history;
- professional-game statistics.

## Testing

The KataGo adapter should be testable without requiring every automated test to launch a real neural-network analysis.

Protocol parsing and process management should have deterministic tests using captured or synthetic protocol responses.

A smaller integration-test suite can exercise a real KataGo installation when one is available.

## Initial implementation milestone

The first useful milestone is intentionally small:

1. locate/configure KataGo;
2. start it from Rust;
3. send one known position for analysis;
4. parse a structured response;
5. display or print the leading candidate and evaluation;
6. terminate cleanly;
7. cover the adapter with tests.

Only after that works reliably should we add whole-game/background analysis and database persistence.
