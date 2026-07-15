use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection {
    pub trees: Vec<GameTree>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameTree {
    pub sequence: Vec<Node>,
    pub variations: Vec<GameTree>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Node {
    pub properties: BTreeMap<String, Vec<String>>,
}

impl Node {
    pub fn first(&self, property: &str) -> Option<&str> {
        self.properties
            .get(property)
            .and_then(|values| values.first())
            .map(String::as_str)
    }

    pub fn values(&self, property: &str) -> &[String] {
        self.properties
            .get(property)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SgfError {
    #[error("unexpected end of SGF input")]
    UnexpectedEnd,
    #[error("expected '{expected}' at byte {offset}")]
    Expected { expected: char, offset: usize },
    #[error("expected property identifier at byte {0}")]
    ExpectedIdentifier(usize),
    #[error("game tree contains no nodes at byte {0}")]
    EmptySequence(usize),
    #[error("trailing non-whitespace content at byte {0}")]
    TrailingContent(usize),
    #[error("SGF contains no game trees")]
    EmptyCollection,
}

pub fn parse_collection(bytes: &[u8]) -> Result<Collection, SgfError> {
    let mut parser = Parser { bytes, pos: 0 };
    parser.skip_whitespace();

    let mut trees = Vec::new();
    while parser.peek() == Some(b'(') {
        trees.push(parser.parse_game_tree()?);
        parser.skip_whitespace();
    }

    if trees.is_empty() {
        return Err(SgfError::EmptyCollection);
    }
    if parser.pos != bytes.len() {
        return Err(SgfError::TrailingContent(parser.pos));
    }

    Ok(Collection { trees })
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn parse_game_tree(&mut self) -> Result<GameTree, SgfError> {
        self.expect(b'(')?;
        self.skip_whitespace();

        let mut sequence = Vec::new();
        while self.peek() == Some(b';') {
            sequence.push(self.parse_node()?);
            self.skip_whitespace();
        }
        if sequence.is_empty() {
            return Err(SgfError::EmptySequence(self.pos));
        }

        let mut variations = Vec::new();
        while self.peek() == Some(b'(') {
            variations.push(self.parse_game_tree()?);
            self.skip_whitespace();
        }

        self.expect(b')')?;
        Ok(GameTree {
            sequence,
            variations,
        })
    }

    fn parse_node(&mut self) -> Result<Node, SgfError> {
        self.expect(b';')?;
        self.skip_whitespace();
        let mut node = Node::default();

        while matches!(self.peek(), Some(b'A'..=b'Z')) {
            let identifier = self.parse_identifier()?;
            self.skip_whitespace();

            let mut values = Vec::new();
            while self.peek() == Some(b'[') {
                values.push(self.parse_value()?);
                self.skip_whitespace();
            }

            if values.is_empty() {
                return Err(SgfError::Expected {
                    expected: '[',
                    offset: self.pos,
                });
            }
            node.properties
                .entry(identifier)
                .or_default()
                .extend(values);
        }

        Ok(node)
    }

    fn parse_identifier(&mut self) -> Result<String, SgfError> {
        let start = self.pos;
        while matches!(self.peek(), Some(b'A'..=b'Z')) {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(SgfError::ExpectedIdentifier(self.pos));
        }
        Ok(String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned())
    }

    fn parse_value(&mut self) -> Result<String, SgfError> {
        self.expect(b'[')?;
        let mut output = Vec::new();

        loop {
            let byte = self.next().ok_or(SgfError::UnexpectedEnd)?;
            match byte {
                b']' => break,
                b'\\' => {
                    let escaped = self.next().ok_or(SgfError::UnexpectedEnd)?;
                    match escaped {
                        b'\r' => {
                            if self.peek() == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
                        b'\n' => {}
                        other => output.push(other),
                    }
                }
                b'\r' => {
                    if self.peek() == Some(b'\n') {
                        self.pos += 1;
                    }
                    output.push(b'\n');
                }
                other => output.push(other),
            }
        }

        Ok(String::from_utf8_lossy(&output).into_owned())
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), SgfError> {
        if self.peek() == Some(expected) {
            self.pos += 1;
            Ok(())
        } else {
            Err(SgfError::Expected {
                expected: expected as char,
                offset: self.pos,
            })
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        Some(byte)
    }
}
