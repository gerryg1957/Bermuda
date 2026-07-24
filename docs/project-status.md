MoyoDB Project Status

Updated: 21 July 2026

Overall Status

The project has now moved beyond being a collection of database components and has become a working native Go database engine capable of importing, indexing and searching professional games.

Estimated completion of core engine: ~65%

The remaining work is primarily in usability, search sophistication and GUI development rather than proving the underlying architecture.

Completed
SGF Processing

✔ SGF parser

✔ Main variation extraction

✔ Setup stones (AB/AW)

✔ Pass handling

✔ Capture handling

✔ Ko enforcement

✔ Compact move-file generation

Database

✔ SQLite metadata database

✔ Canonical game hashing

✔ Duplicate detection

✔ Project structure

✔ Database initialisation

✔ Game import

✔ Batch directory import

Position Replay

✔ Complete board reconstruction

✔ Replay to arbitrary move

✔ Position display

✔ Game metadata display

Position Index

✔ Incremental position index

✔ Database versioning

✔ Incremental rebuilds

✔ Fast indexing

Current test results:

100 games
21,474 indexed positions
0 errors
~95 games/second indexing
Pattern Search

Implemented and working:

✔ Extract pattern from any position

✔ Search individual game

✔ Search entire database

✔ Find multiple occurrences

✔ Find occurrences at different board locations

✔ Database-wide search verified on test collection

This is the biggest milestone reached so far.

Command Line Interface

Implemented:

init
import
import-dir
build-position-index
show-position
search-pattern
search-pattern-database

The CLI is now genuinely useful for testing the engine.

Recently Verified

The latest testing confirmed that database-wide search is functioning correctly.

Tests demonstrated:

search of an individual game
search across an entire database
identical results between both where expected
successful detection of patterns in many different games and at multiple board locations
repeated matches across consecutive positions where the local pattern remains unchanged.
Current Limitations

The search engine currently reports every matching position.

For example

Game 5
move 50
move 51
move 52
...
move 78

instead of

Game 5
moves 50–78

This is not incorrect, but it is not yet user-friendly.

Next Development Priorities
1. Improve Search Result Presentation

High priority.

Collapse consecutive matches into ranges such as

Game 23
moves 54–81

instead of dozens of individual entries.

2. Rich Search Results

Include

player names
event
date
result

with every match.

3. Pattern Variations

Support optional searching with

colour swap
rotations
reflections

Eventually also

corner search
edge search
joseki search
4. Performance

Current performance is already good.

Future improvements include

parallel searching
compressed index pages
cached pattern hashes
5. Database Management

Still to add

game deletion
re-index after deletion
import updates
project maintenance commands
6. GUI

No GUI work has begun.

Planned features include

board display
interactive pattern selection
result browser
game replay
database management
Longer-Term Features
Fuseki search
Joseki search
Life-and-death search
Influence search
Shape similarity
AI-assisted pattern search
Statistical reports
Professional game analytics
Current Assessment

The project has now demonstrated all of the critical technical capabilities needed for a serious professional Go database:

robust SGF ingestion
reliable board reconstruction
canonical game identification
scalable position indexing
database-wide pattern search

The remaining work is focused on making these capabilities efficient and pleasant to use. In particular, refining search result presentation, adding richer search options, and eventually building a graphical interface will transform the current engine into a practical replacement for MoyoGo Studio.
