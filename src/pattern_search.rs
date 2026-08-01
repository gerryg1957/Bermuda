use crate::{Colour, Pattern, indexer::PositionIndexer};
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternMatch {
    pub game_id: i64,
    pub move_number: usize,
    pub side_to_move: Colour,
    pub ko_point: Option<u16>,
    pub left: u8,
    pub bottom: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternSearchScope {
    Game(i64),
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternSearchQuery {
    pub pattern: Pattern,
    pub scope: PatternSearchScope,
}

pub struct PatternSearcher;

impl PatternSearcher {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    fn search_position(
        game_id: i64,
        move_number: usize,
        side_to_move: Colour,
        ko_point: Option<u16>,
        board: &crate::Board,
        pattern: &Pattern,
    ) -> Result<Vec<PatternMatch>> {
        let mut matches = Vec::new();

        if pattern.width > board.size() || pattern.height > board.size() {
            return Ok(matches);
        }

        let max_left = board.size() - pattern.width;
        let max_bottom = board.size() - pattern.height;

        for bottom in 0..=max_bottom {
            for left in 0..=max_left {
                if pattern.matches_at(board, left, bottom)? {
                    matches.push(PatternMatch {
                        game_id,
                        move_number,
                        side_to_move,
                        ko_point,
                        left,
                        bottom,
                    });
                }
            }
        }

        Ok(matches)
    }

    pub fn search(
        &self,
        indexer: &PositionIndexer,
        query: &PatternSearchQuery,
    ) -> Result<Vec<PatternMatch>> {
        match query.scope {
            PatternSearchScope::Game(game_id) => self.search_game(indexer, game_id, &query.pattern),
            PatternSearchScope::Project => self.search_database(indexer, &query.pattern),
        }
    }

    pub fn search_game(
        &self,
        indexer: &PositionIndexer,
        game_id: i64,
        pattern: &Pattern,
    ) -> Result<Vec<PatternMatch>> {
        let states = indexer.replay_game_states_by_id(game_id)?;
        let mut matches = Vec::new();

        for state in states {
            matches.extend(Self::search_position(
                game_id,
                state.occurrence.move_number,
                state.occurrence.side_to_move,
                state.occurrence.ko_point,
                &state.board,
                pattern,
            )?);
        }

        Ok(matches)
    }

    pub fn search_database(
        &self,
        indexer: &PositionIndexer,
        pattern: &Pattern,
    ) -> Result<Vec<PatternMatch>> {
        let mut matches = Vec::new();

        for game_id in indexer.game_ids()? {
            matches.extend(self.search_game(indexer, game_id, pattern)?);
        }

        Ok(matches)
    }
}

impl Default for PatternSearcher {
    fn default() -> Self {
        Self::new()
    }
}
