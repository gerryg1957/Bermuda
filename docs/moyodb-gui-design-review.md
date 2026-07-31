# MoyoDB GUI Design Review
## Lessons from the MoyoGo Studio starting display

**Date:** 31 July 2026

## Purpose

This document records the discussion about which parts of the original MoyoGo Studio (MGS) interface were useful in practice, which were unnecessary, and which ideas should influence the MoyoDB Qt interface.

The aim is not to reproduce MGS screen-for-screen. The aim is to preserve the genuinely useful study and database functions while producing a simpler, clearer interface.

---

## 1. Overall direction

MoyoDB should concentrate on three persistent areas:

1. **The game catalogue**
2. **The main Go board**
3. **Game details and navigation**

Other tools should appear only when they are relevant. Pattern-search controls, match previews, overlays and administrative functions should not permanently occupy screen space.

The current two-pane Qt layout remains a good basis:

```text
Game catalogue | Main board
               | Game details and navigation
```

The game tree, pattern tools, match preview and other secondary information can appear contextually.

---

## 2. Bottom row of the MGS display

### Good-move and bad-move indicators

MGS included controls that attempted to identify good and bad moves.

**Decision:** Omit.

They were not useful in practice. Any future move-quality judgement would belong to a separate AI-engine analysis feature rather than the database interface.

### Move slider

MGS included a slider for stepping through a game.

**Decision:** Add, provided it remains straightforward.

MoyoDB already has:

- first position;
- previous move;
- back ten moves;
- next move;
- forward ten moves;
- final position;
- current move number and total move count.

A slider would complement these controls by allowing rapid movement through a long game.

### Board-perspective slider

MGS could change the board from a flat view to an angled perspective.

**Decision:** Omit.

It was visually elaborate but not useful for study.

### Stone sound and volume control

MGS could play a sound when a stone was placed and provided a volume slider.

**Decision:** Omit.

This was decorative rather than useful.

---

## 3. The main board

### Orientation

Black should appear at the bottom by convention. This orientation is implicit in normal SGF presentation.

**Decision:** Keep Black at the bottom by default.

Board rotation might eventually be available as a secondary command, but it should not consume permanent interface space.

### Player names

MGS showed the player names around the board. MoyoDB currently displays them below the board, which is visually preferable.

The names may come from:

- a game selected from the MoyoDB catalogue;
- an SGF file opened directly for analysis;
- a pattern-search result;
- a future edited or newly created game.

**Decision:** Keep the names below the board, but ultimately obtain them from the loaded game rather than merely echoing the selected catalogue row.

### Clocks

MGS displayed player clocks.

**Decision:** Omit.

Clock information is unnecessary for the principal database-study workflow.

### Board coordinates

Coordinates are useful when discussing positions and moves.

**Decision:** Add and enable by default.

The horizontal coordinates should follow the normal Go convention and omit the letter `I`.

### Move numbers and last-move indication

Move numbers can be useful, but the most important requirement is identifying the last move played.

**Decision:**

- Add a clear last-move marker soon.
- Consider optional move-number display later.

Possible later modes might include:

- last move only;
- recent moves;
- all visible move numbers;
- no move markings.

Displaying every move number should not be the default because it can make a crowded board difficult to read.

---

## 4. Pattern-search lettering

MGS displayed letters such as `a`, `b` and `c` on the board.

On an empty board, these indicated the most frequent opening moves. During pattern analysis, they indicated common continuations after the matched pattern.

For example:

- `a` = most frequent continuation;
- `b` = second most frequent continuation;
- `c` = third most frequent continuation.

This is not ordinary board annotation. It is statistical information produced from matching games.

**Decision:** Retain this as an optional continuation overlay.

The board should eventually be drawn in separate layers:

1. board grid and star points;
2. stones;
3. last-move marker;
4. coordinates;
5. pattern-selection rectangle;
6. continuation labels or heat maps;
7. mouse-hover marker.

This separation will allow pattern information to be shown or hidden without affecting normal replay.

---

## 5. Database area and tabs

### Comments tab

The MGS Comments tab displayed SGF comment data.

**Decision:** Omit from the permanent database interface.

Comments may still be retained when an SGF is imported or opened, but they are not useful enough to justify permanent screen space.

### Multiple databases

MGS envisaged separate databases such as:

- professional games;
- professional 9×9 games;
- professional handicap games;
- other specialist collections.

In practice, the only rich and important source category is professional 19×19 games. AI engines now serve many of the purposes once imagined for separate specialist collections.

**Decision:** Use one MoyoDB project catalogue rather than multiple conceptual databases.

### Required tabs

Only two catalogue views are needed:

```text
All games | Pattern matches
```

#### All games

Shows the complete canonical-game catalogue.

#### Pattern matches

Shows games identified by the current pattern search.

### Catalogue columns

The current MoyoDB columns are sufficient:

- Black;
- White;
- Date;
- Result;
- Komi;
- Event.

**Decision:** Do not add further default columns.

### Sorting

It should be possible to sort by clicking any visible heading.

A second click should reverse the direction, and the heading should show an ascending or descending indicator.

Even sorting by less useful fields such as komi or event is acceptable because a consistent rule is clearer than arbitrary exceptions.

---

## 6. Miniature pattern-match board

MGS displayed a blank miniature 19×19 board above the database. During pattern-search results, it showed how the searched pattern had been found in the selected game.

It could explain:

- rotation;
- reflection;
- colour exchange;
- the location of the matched pattern;
- the actual colours and orientation in the selected occurrence.

### Colour-exchange example

Suppose Black played san-ren-sei and White responded with the Chinese opening. A search for the Chinese opening played by Black should still be able to find the equivalent formation played by White.

When that result is selected, the miniature board should show the occurrence as it actually appeared in the game.

**Decision:** Keep a miniature match board, but show it only in the **Pattern matches** view.

It should not permanently occupy space above the full database.

The preview should ideally be accompanied by a clear textual explanation such as:

```text
Rotated · Reflected · Colours exchanged
```

### Selecting a match

Selecting a pattern-match result should:

1. load the game;
2. jump to the matched move;
3. show the full position on the main board;
4. mark the matched rectangle;
5. show the transformed occurrence on the miniature board.

A game may contain more than one occurrence. The preferred design is one row per game, with a match count and controls for moving between occurrences.

---

## 7. Top toolbar: right-hand controls

### Separate pattern-frequency and fuseki-frequency buttons

MGS had separate buttons for:

- pattern continuation frequencies;
- identified fuseki frequencies.

**Decision:** Do not preserve two separate controls.

Both represent the same general question:

> In games matching the current search, where was the next move played?

Use one continuation-overlay control regardless of whether the search covers a local pattern or a whole-board opening position.

### Separate fuseki identification

The value of treating fuseki as a separate category is doubtful.

A whole-board opening can be represented as:

- a whole-board pattern;
- an early move range;
- optional rotations and reflections;
- optional colour exchange.

**Decision:** Do not create a separate fuseki subsystem initially.

A dedicated feature should be added only if it later provides something clearly different, such as trustworthy named-opening recognition.

### Multiple pattern-selection methods

MGS provided more than one way to identify a pattern on the current board.

**Decision:** Provide one clear method.

Proposed workflow:

1. enter pattern-selection mode;
2. drag a rectangle on the main board;
3. specify required Black stones, White stones, empty points and ignored points;
4. choose rotation, reflection and colour-exchange options;
5. run the search.

---

## 8. Heat maps and statistical overlays

The discussion identified several distinct overlays that should not be confused.

### Ranked continuation letters

Shows the most frequent next moves as `a`, `b`, `c`, and so on.

### Next-move frequency heat map

Shows where subsequent play tends to occur among matching games.

This represents frequency, not move quality.

### Ownership probability map

Shows how likely each area is eventually to become Black territory, White territory or neutral, based on the matching games.

This is different from a continuation heat map. It describes the likely territorial outcome of the current position rather than the location of the next move.

The interface should avoid implying more certainty than the data supports. SGF records do not always encode final territory and dead stones reliably, so an ownership map may be statistical or approximate.

**Proposed overlay selector:**

```text
Overlay:
    None
    Next-move letters
    Next-move frequency heat map
    Ownership probability
```

The ownership display should probably be called an **ownership map** rather than simply a heat map, to distinguish it from next-move frequency and from AI move-quality analysis.

---

## 9. Game tree and joseki dictionary

The permanently visible game tree in MGS was mainly useful for its bundled joseki dictionary, where one position could lead to several alternative continuations.

For the professional game records that MoyoDB is intended to study, the game tree would normally contain only the main line. It would therefore consume valuable screen space without adding useful information.

A modern joseki dictionary could be valuable, but there is no known current, maintainable source available for inclusion. An old pre-AI joseki dictionary would be of little use and could be actively misleading.

**Decision:**

- Do not include a permanently visible game tree.
- Do not reproduce the old MGS joseki dictionary.
- Reconsider a joseki feature only if a trustworthy modern source becomes available.
- If a directly opened SGF genuinely contains variations, show a contextual variation chooser or tree only while it is needed.

This further supports the simplified main layout:

```text
All games / Pattern matches | Main board
                            | Game information
                            | Move controls
```

---

## 10. Features explicitly omitted

The following MGS features should not be reproduced by default:

- good-move and bad-move indicators;
- board-perspective control;
- stone sounds and volume control;
- clocks;
- permanent SGF Comments tab;
- multiple separate database categories;
- duplicate pattern-selection methods;
- separate fuseki-frequency controls;
- a dedicated fuseki subsystem without a clear use;
- miscellaneous toolbar controls that were unhelpful or duplicated elsewhere;
- a permanently visible game tree;
- the obsolete MGS joseki dictionary.

---

## 11. Emerging MoyoDB pattern-search workflow

```text
Select a pattern on the main board
        ↓
Set transformation and search options
        ↓
Run the search
        ↓
Open the Pattern matches tab
        ↓
Select a matching game
        ↓
Jump to the matched occurrence
        ↓
Show the matched rectangle and miniature preview
        ↓
Optionally display continuation letters,
a next-move heat map, or an ownership map
```

---

## 12. Current priorities suggested by the review

The next uncomplicated improvements to the existing game viewer are:

1. board coordinates;
2. last-move marker;
3. move slider.

The larger design work after that should concentrate on:

1. sortable catalogue headings;
2. All games and Pattern matches tabs;
3. one clear pattern-selection mode;
4. miniature occurrence preview;
5. continuation and ownership overlays.

---

## 13. Open questions for the next discussion

The review has not yet covered the whole MGS interface. Topics still to discuss include:

- the remaining top-left controls and menus;
- opening SGF files directly for analysis;
- pattern-definition controls;
- search-scope controls;
- presentation of multiple matches in one game;
- whether any source or project-management controls belong in the main window;
- the precise visual form of continuation and ownership overlays.
