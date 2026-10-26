# Bermuda navigation

The unified Library / Study layout is now the default and only selectable UI.
The Help-menu trial switch has been removed. Existing preferences for the old
UI are ignored; existing unified-layout divider positions are retained. The
historical trial-prefixed divider keys are kept to preserve those positions.
No database, SGF, study or analysis-result migration is required.

## Layout

- Home has Library on the left (Professional games / My games) and Study on
  the right, with Open, New pattern, Explore joseki, current work and a full
  saved-studies table. Collection counts are shown.
- Open offers professional games, My games, saved studies and an SGF file.
- Tables emphasise Black, White and Date. Horizontal scrolling exposes the
  remaining columns; existing catalogue filters and sorting remain available.
- Professional games, personal games, SGFs and saved studies share the same
  workspace. Professional games are not marked as personal games.
- Pattern search and Search options sit below the compact SGF comments panel,
  above the stone-placement and search controls.
- Matching games fill the space below the continuation summary. Selecting a
  continuation filters this same table. Searching and opening a match keep the
  board and investigation area together, without a separate results screen.
- The top toolbar provides Home, return navigation, Game / notes, KataGo and
  Influence. Flip and rotation buttons are centred below the board, with Return
  to joseki to their left and the move slider beneath them.
- Search options retains area selection, professional/personal searches,
  new search and the handicap setting.
- Return to source restores the query position. Return to joseki restores its
  browser node and clears the temporary match list through the existing path.
- KataGo can investigate a joseki position without requiring a professional
  database or first running a pattern search.
- Play retains existing playing and SGF-saving actions; it does not add new
  playing capabilities or tested strength settings.

Normal edits retain the existing automatic study saving. Current work refers
to the retained workspace in the session, not a persistent recent-files list.

## Validation and remaining checks

The user has tested and accepted the layout and inline pattern-results flow on
Linux. QML syntax, incremental patch application and focused component checks
have also passed. Whole-application Windows and macOS validation remains open.

For release checks, open each source type, return Home and continue, run a
pattern search, filter by a continuation, open a matching game and return to
source. Check joseki return and KataGo review separately. Restart with an
existing profile and with a fresh profile: both must use the unified layout,
with no option to select the old UI.
