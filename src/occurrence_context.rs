use crate::{GameRecord, Pattern, PatternMatch};

/// Maximum rectangle distance measured by the first local-activity pass.
///
/// Distance zero means inside the matched rectangle. Distances one, two and
/// three are successive rectangular rings around it.
pub const LOCAL_ACTIVITY_MAX_DISTANCE: u8 = 3;

/// A played move found near a pattern appearance.
///
/// Coordinates are absolute coordinates in the matched game's board
/// orientation. `distance` is the rectangular-ring distance from the actual
/// matched rectangle, after applying the match transformation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NearbyMove {
    pub move_number: usize,
    pub x: u8,
    pub y: u8,
    pub distance: u8,
}

impl NearbyMove {
    /// Number of moves after the appearance first exists.
    #[must_use]
    pub fn delay_moves_from(self, appearance_move_number: usize) -> usize {
        self.move_number.saturating_sub(appearance_move_number)
    }
}

/// First subsequent played move found at each local-distance threshold.
///
/// The thresholds are cumulative: a move inside the rectangle also counts as
/// being within one, two and three intersections.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocalActivity {
    pub first_inside: Option<NearbyMove>,
    pub first_within_one: Option<NearbyMove>,
    pub first_within_two: Option<NearbyMove>,
    pub first_within_three: Option<NearbyMove>,
}

impl LocalActivity {
    /// Return the first subsequent move within a requested distance 0..=3.
    #[must_use]
    pub fn first_within(&self, distance: u8) -> Option<NearbyMove> {
        match distance {
            0 => self.first_inside,
            1 => self.first_within_one,
            2 => self.first_within_two,
            3 => self.first_within_three,
            _ => None,
        }
    }
}

/// Measure the first subsequent played move at distances 0, 1, 2 and 3 from
/// a pattern appearance.
///
/// Measurement starts immediately after the *first* matching position, not
/// after the appearance ends. This deliberately allows nearby play to be
/// recorded while the exact pattern is still unchanged.
///
/// Passes are ignored. If no played move is found within a threshold before
/// the game ends, that threshold remains `None`.
#[must_use]
pub fn measure_local_activity(
    record: &GameRecord,
    pattern: &Pattern,
    appearance: &PatternMatch,
) -> LocalActivity {
    if record.board_size == 0 || pattern.width == 0 || pattern.height == 0 {
        return LocalActivity::default();
    }

    let (width, height) = appearance
        .transformation
        .transformed_dimensions(pattern.width, pattern.height);

    let board_size = u16::from(record.board_size);
    let board_points = board_size * board_size;

    let mut activity = LocalActivity::default();

    /*
     * Position N is followed by move N + 1 at zero-based record.moves[N].
     */
    for (move_index, mv) in record.moves.iter().enumerate().skip(appearance.move_number) {
        let Some(point) = mv.point else {
            continue;
        };

        if point >= board_points {
            continue;
        }

        let x = u8::try_from(point % board_size).expect("board x must fit in u8");
        let y = u8::try_from(point / board_size).expect("board y must fit in u8");

        let distance = rectangle_distance(x, y, appearance.left, appearance.bottom, width, height);

        if distance > LOCAL_ACTIVITY_MAX_DISTANCE {
            continue;
        }

        let nearby = NearbyMove {
            move_number: move_index + 1,
            x,
            y,
            distance,
        };

        if distance == 0 && activity.first_inside.is_none() {
            activity.first_inside = Some(nearby);
        }

        if distance <= 1 && activity.first_within_one.is_none() {
            activity.first_within_one = Some(nearby);
        }

        if distance <= 2 && activity.first_within_two.is_none() {
            activity.first_within_two = Some(nearby);
        }

        if distance <= 3 && activity.first_within_three.is_none() {
            activity.first_within_three = Some(nearby);
        }

        if activity.first_inside.is_some()
            && activity.first_within_one.is_some()
            && activity.first_within_two.is_some()
            && activity.first_within_three.is_some()
        {
            break;
        }
    }

    activity
}

fn rectangle_distance(x: u8, y: u8, left: u8, bottom: u8, width: u8, height: u8) -> u8 {
    let x = i16::from(x);
    let y = i16::from(y);

    let left = i16::from(left);
    let bottom = i16::from(bottom);
    let right = left + i16::from(width) - 1;
    let top = bottom + i16::from(height) - 1;

    let dx = if x < left {
        left - x
    } else if x > right {
        x - right
    } else {
        0
    };

    let dy = if y < bottom {
        bottom - y
    } else if y > top {
        y - top
    } else {
        0
    };

    u8::try_from(dx.max(dy)).unwrap_or(u8::MAX)
}
