use thiserror::Error;

pub const MAX_BOARD_SIZE: u8 = 19;
pub const MAX_POINTS: usize = 361;
const WORDS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black,
    White,
}

impl Color {
    pub fn opponent(self) -> Self {
        match self {
            Self::Black => Self::White,
            Self::White => Self::Black,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub color: Color,
    pub point: Option<u16>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BoardError {
    #[error("unsupported board size {0}; expected 1..=19")]
    InvalidSize(u8),
    #[error("point {0} lies outside the board")]
    PointOutside(u16),
    #[error("point {0} is occupied")]
    Occupied(u16),
    #[error("move at {0} violates simple ko")]
    Ko(u16),
    #[error("move at {0} is suicide")]
    Suicide(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    size: u8,
    black: [u64; WORDS],
    white: [u64; WORDS],
    ko: Option<u16>,
}

impl Board {
    pub fn new(size: u8) -> Result<Self, BoardError> {
        if !(1..=MAX_BOARD_SIZE).contains(&size) {
            return Err(BoardError::InvalidSize(size));
        }
        Ok(Self {
            size,
            black: [0; WORDS],
            white: [0; WORDS],
            ko: None,
        })
    }

    pub fn size(&self) -> u8 { self.size }
    pub fn ko_point(&self) -> Option<u16> { self.ko }
    pub fn black_words(&self) -> &[u64; WORDS] { &self.black }
    pub fn white_words(&self) -> &[u64; WORDS] { &self.white }

    pub fn point(&self, x: u8, y: u8) -> Result<u16, BoardError> {
        if x >= self.size || y >= self.size {
            return Err(BoardError::PointOutside(u16::from(y) * u16::from(self.size) + u16::from(x)));
        }
        Ok(u16::from(y) * u16::from(self.size) + u16::from(x))
    }

    pub fn color_at(&self, point: u16) -> Option<Color> {
        if !self.is_valid_point(point) { return None; }
        if get_bit(&self.black, point) { Some(Color::Black) }
        else if get_bit(&self.white, point) { Some(Color::White) }
        else { None }
    }

    pub fn set_setup(&mut self, color: Color, point: u16) -> Result<(), BoardError> {
        self.require_point(point)?;
        self.clear(point);
        self.set(color, point);
        self.ko = None;
        Ok(())
    }

    pub fn clear_setup(&mut self, point: u16) -> Result<(), BoardError> {
        self.require_point(point)?;
        self.clear(point);
        self.ko = None;
        Ok(())
    }

    pub fn play(&mut self, mv: Move) -> Result<Vec<u16>, BoardError> {
        let Some(point) = mv.point else {
            self.ko = None;
            return Ok(Vec::new());
        };
        self.require_point(point)?;
        if self.color_at(point).is_some() { return Err(BoardError::Occupied(point)); }
        if self.ko == Some(point) { return Err(BoardError::Ko(point)); }

        let previous = self.clone();
        self.set(mv.color, point);
        let mut captured = Vec::new();

        for neighbour in self.neighbours(point) {
            if self.color_at(neighbour) == Some(mv.color.opponent()) {
                let group = self.group(neighbour);
                if self.liberty_count(&group) == 0 {
                    captured.extend(group);
                }
            }
        }
        captured.sort_unstable();
        captured.dedup();
        for &stone in &captured { self.clear(stone); }

        let own_group = self.group(point);
        if self.liberty_count(&own_group) == 0 {
            *self = previous;
            return Err(BoardError::Suicide(point));
        }

        self.ko = if captured.len() == 1 && own_group.len() == 1 && self.liberty_count(&own_group) == 1 {
            Some(captured[0])
        } else {
            None
        };
        Ok(captured)
    }

    fn is_valid_point(&self, point: u16) -> bool {
        point < u16::from(self.size) * u16::from(self.size)
    }
    fn require_point(&self, point: u16) -> Result<(), BoardError> {
        if self.is_valid_point(point) { Ok(()) } else { Err(BoardError::PointOutside(point)) }
    }
    fn set(&mut self, color: Color, point: u16) {
        self.clear(point);
        match color {
            Color::Black => set_bit(&mut self.black, point),
            Color::White => set_bit(&mut self.white, point),
        }
    }
    fn clear(&mut self, point: u16) {
        clear_bit(&mut self.black, point);
        clear_bit(&mut self.white, point);
    }
    fn neighbours(&self, point: u16) -> Vec<u16> {
        let size = u16::from(self.size);
        let x = point % size;
        let y = point / size;
        let mut out = Vec::with_capacity(4);
        if x > 0 { out.push(point - 1); }
        if x + 1 < size { out.push(point + 1); }
        if y > 0 { out.push(point - size); }
        if y + 1 < size { out.push(point + size); }
        out
    }
    fn group(&self, start: u16) -> Vec<u16> {
        let Some(color) = self.color_at(start) else { return Vec::new(); };
        let mut visited = [false; MAX_POINTS];
        let mut stack = vec![start];
        let mut result = Vec::new();
        while let Some(point) = stack.pop() {
            if visited[usize::from(point)] { continue; }
            visited[usize::from(point)] = true;
            result.push(point);
            for neighbour in self.neighbours(point) {
                if self.color_at(neighbour) == Some(color) { stack.push(neighbour); }
            }
        }
        result
    }
    fn liberty_count(&self, group: &[u16]) -> usize {
        let mut liberties = [false; MAX_POINTS];
        for &point in group {
            for neighbour in self.neighbours(point) {
                if self.color_at(neighbour).is_none() { liberties[usize::from(neighbour)] = true; }
            }
        }
        liberties.into_iter().filter(|value| *value).count()
    }
}

fn get_bit(words: &[u64; WORDS], point: u16) -> bool {
    let p = usize::from(point);
    words[p / 64] & (1u64 << (p % 64)) != 0
}
fn set_bit(words: &mut [u64; WORDS], point: u16) {
    let p = usize::from(point);
    words[p / 64] |= 1u64 << (p % 64);
}
fn clear_bit(words: &mut [u64; WORDS], point: u16) {
    let p = usize::from(point);
    words[p / 64] &= !(1u64 << (p % 64));
}
