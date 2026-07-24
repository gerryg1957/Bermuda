use crate::{Pattern, indexer::PositionIndexer};
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternMatch {
    pub game_id: i64,
    pub move_number: usize,
    pub left: u8,
    pub bottom: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternSearchGame {
    pub game_id: i64,
    pub black_player: Option<String>,
    pub white_player: Option<String>,
    pub date: Option<String>,
    pub event: Option<String>,
    pub result: Option<String>,
    pub matches: Vec<PatternMatch>,
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
                        left,
                        bottom,
                    });
                }
            }
        }

        Ok(matches)
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
