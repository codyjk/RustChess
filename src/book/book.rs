//! Opening book data structures and operations.

use std::fmt::{Display, Formatter};

use common::bitboard::Square;
use rustc_hash::FxHashMap;

include!(concat!(env!("OUT_DIR"), "/opening_book.rs"));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BookMove(Square, Square);

impl BookMove {
    pub fn new(from: Square, to: Square) -> Self {
        BookMove(from, to)
    }

    pub fn from_square(&self) -> Square {
        self.0
    }

    pub fn to_square(&self) -> Square {
        self.1
    }
}

pub struct OpeningLine {
    pub name: String,
    pub moves: String,
}

#[derive(Default)]
pub struct BookNode {
    lines: FxHashMap<BookMove, Box<BookNode>>,
    line_name: Option<String>,
}

impl BookNode {
    fn new() -> Self {
        Default::default()
    }
}

pub struct Book {
    root: BookNode,
}

impl Default for Book {
    fn default() -> Self {
        // Generated in `precompile/src/book/book_generator.rs`
        create_book()
    }
}

impl Book {
    pub fn new() -> Self {
        Self {
            root: BookNode::default(),
        }
    }

    pub fn add_line(&mut self, line: OpeningLine) {
        let moves = line.moves.split(' ');
        let moves_count = moves.clone().count();
        if moves_count == 0 {
            return;
        }

        let mut curr_node = &mut self.root;

        for (i, raw_move) in moves.clone().enumerate() {
            let raw_from_square: String = raw_move.chars().take(2).collect();
            let raw_to_square: String = raw_move.chars().skip(2).take(2).collect();
            let from_square = Square::from_algebraic(&raw_from_square)
                .unwrap_or_else(|| panic!("Invalid square: {}", raw_from_square));
            let to_square = Square::from_algebraic(&raw_to_square)
                .unwrap_or_else(|| panic!("Invalid square: {}", raw_to_square));
            let book_move = BookMove::new(from_square, to_square);

            let next_node = curr_node
                .lines
                .entry(book_move)
                .or_insert_with(|| Box::new(BookNode::new()));

            if i == moves_count - 1 {
                next_node.line_name = Some(line.name.clone());
            }

            curr_node = next_node;
        }
    }

    pub fn get_next_moves(&self, line: Vec<BookMove>) -> Vec<(BookMove, Option<String>)> {
        let mut curr_node = &self.root;

        for book_move in line {
            let next = curr_node.lines.get(&book_move);
            if next.is_none() {
                return vec![];
            }

            curr_node = next.expect("next should exist after is_none check");
        }

        curr_node
            .lines
            .iter()
            .map(|(move_, node)| (*move_, node.line_name.clone()))
            .collect()
    }

    pub fn get_line(&self, line: Vec<BookMove>) -> Option<String> {
        let mut curr_node = &self.root;
        let mut last_line_name: Option<String> = None;

        for book_move in line {
            curr_node = curr_node.lines.get(&book_move)?;
            // Track the most recent opening name we've seen
            if curr_node.line_name.is_some() {
                last_line_name.clone_from(&curr_node.line_name);
            }
        }

        last_line_name.or(curr_node.line_name.clone())
    }
}

impl Display for Book {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.root)
    }
}

impl Display for BookNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut lines = vec![];
        for (move_, node) in self.lines.iter() {
            let mut line = format!("{}", move_);
            if let Some(name) = node.line_name.clone() {
                line.push_str(&format!(" {}", name));
            }
            lines.push(line);
        }
        write!(f, "{}", lines.join("\n"))
    }
}

impl Display for BookMove {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.0.to_algebraic(), self.1.to_algebraic())
    }
}
