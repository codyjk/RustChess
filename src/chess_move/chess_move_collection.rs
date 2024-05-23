use std::slice::Iter;

use super::ChessMove;

#[derive(Default, PartialEq, Debug)]
pub struct ChessMoveCollection {
    moves: Vec<Box<dyn ChessMove>>,
}

// Implements a collection of objects that implement the ChessMove interface.dyn
// This abstracts away the need for Box.
impl ChessMoveCollection {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn push(&mut self, chess_move: Box<dyn ChessMove>) {
        self.moves.push(chess_move);
    }

    pub fn len(&self) -> usize {
        self.moves.len()
    }

    pub fn concat(&mut self, other: &mut ChessMoveCollection) {
        self.moves.append(&mut other.moves);
    }

    pub fn iter(&self) -> Iter<Box<dyn ChessMove>> {
        self.moves.iter()
    }

    pub fn drain(&mut self) -> std::vec::Drain<Box<dyn ChessMove>> {
        self.moves.drain(..)
    }

    pub fn partition<F>(&mut self, predicate: F) -> (ChessMoveCollection, ChessMoveCollection)
    where
        F: Fn(&Box<dyn ChessMove>) -> bool,
    {
        let mut true_collection = Self::new();
        let mut false_collection = Self::new();

        for item in self.moves.drain(..) {
            if predicate(&item) {
                true_collection.push(item);
            } else {
                false_collection.push(item);
            }
        }

        (true_collection, false_collection)
    }

    pub fn append(&mut self, other: &mut ChessMoveCollection) {
        for item in other.moves.drain(..) {
            self.push(item);
        }
    }

    pub fn remove(&mut self, index: usize) -> Box<dyn ChessMove> {
        self.moves.remove(index)
    }

    pub fn sort(&mut self) {
        self.moves.sort_by(|a, b| {
            let from_square_a = a.from_square();
            let from_square_b = b.from_square();

            if from_square_a == from_square_b {
                a.to_square().cmp(&b.to_square())
            } else {
                from_square_a.cmp(&from_square_b)
            }
        });
    }

    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    pub fn contains(&self, chess_move: &Box<dyn ChessMove>) -> bool {
        self.moves.contains(chess_move)
    }
}

#[macro_export]
macro_rules! chess_moves {
    ( $( $t:expr ),* $(,)? ) => {
        {
            let mut collection = ChessMoveCollection::new();
            $(
                collection.push(Box::new($t));
            )*
            collection
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::square::{A2, A3};
    use crate::chess_move::standard::StandardChessMove;
    use crate::std_move;

    #[test]
    fn test_chess_move_collection_equality() {
        let mut collection1 = ChessMoveCollection::new();
        let mut collection2 = ChessMoveCollection::new();

        collection1.push(Box::new(std_move!(A2, A3)));
        collection2.push(Box::new(std_move!(A2, A3)));

        assert_eq!(collection1, collection2);
    }

    #[test]
    fn test_chess_move_collection_concat() {
        let mut collection1 = ChessMoveCollection::new();
        let mut collection2 = ChessMoveCollection::new();

        collection1.push(Box::new(std_move!(A2, A3)));
        collection2.push(Box::new(std_move!(A2, A3)));

        collection1.concat(&mut collection2);

        assert_eq!(collection1.len(), 2);
    }

    #[test]
    fn test_chess_move_collection_partition() {
        let mut collection = ChessMoveCollection::new();

        collection.push(Box::new(std_move!(A2, A3)));
        collection.push(Box::new(std_move!(A2, A3)));

        let (true_collection, false_collection) =
            collection.partition(|chess_move| chess_move.from_square() == A2);

        assert_eq!(true_collection.len(), 2);
        assert_eq!(false_collection.len(), 0);
    }

    #[test]
    fn test_chess_move_collection_append() {
        let mut collection1 = ChessMoveCollection::new();
        let mut collection2 = ChessMoveCollection::new();

        collection1.push(Box::new(std_move!(A2, A3)));
        collection2.push(Box::new(std_move!(A2, A3)));

        collection1.append(&mut collection2);

        assert_eq!(collection1.len(), 2);
        assert_eq!(collection2.len(), 0);
    }

    #[test]
    fn test_chess_move_collection_remove() {
        let mut collection = ChessMoveCollection::new();

        collection.push(Box::new(std_move!(A2, A3)));
        collection.push(Box::new(std_move!(A2, A3)));

        collection.remove(0);

        assert_eq!(collection.len(), 1);
    }

    #[test]
    fn test_iter() {
        let mut collection = ChessMoveCollection::new();

        collection.push(Box::new(std_move!(A2, A3)));
        collection.push(Box::new(std_move!(A2, A3)));

        for chess_move in collection.iter() {
            println!("{:?}", chess_move);
        }
    }
}
