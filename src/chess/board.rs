use std::collections::{hash_map::DefaultHasher, HashMap};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use super::moves::{Extra, HistoryMove, MoveFailureReason, MoveFailureReason::*, MoveParsingError};
use super::pieces::{Piece, Type};

use crate::chess::moves::NewMove;
use std::str::FromStr;

#[derive(Eq, PartialEq, Copy, Clone, Hash, Debug, Serialize, Deserialize)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn get_opposite(&self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Hash)]
pub struct Square {
    pub file_number: u8,
    pub rank_number: u8,
}

impl Square {
    pub fn new(file_number: u8, rank_number: u8) -> Self {
        Self { file_number, rank_number }
    }

    pub fn new_if_valid(file_number: u8, rank_number: u8) -> Option<Self> {
        let square = Self::new(file_number, rank_number);

        if !square.is_valid() {
            return None;
        }

        Some(square)
    }

    pub fn get_file_as_letter(&self) -> char {
        (b'A' + self.file_number - 1) as char
    }

    pub fn is_light(&self) -> bool {
        (self.file_number % 2) ^ (self.rank_number % 2) != 0
    }

    pub fn is_valid(&self) -> bool {
        self.file_number >= 1 && self.file_number <= 8 && self.rank_number >= 1 && self.rank_number <= 8
    }

    pub fn get_relative(&self, file_relative: i8, rank_relative: i8) -> Self {
        Self::new(((self.file_number as i8) + file_relative) as u8, ((self.rank_number as i8) + rank_relative) as u8)
    }

    pub fn get_relatives_until_invalid(&self, file_relative: i8, rank_relative: i8) -> Vec<Self> {
        assert!(file_relative != 0 || rank_relative != 0);

        let mut relatives = Vec::new();
        let mut current: Self = *self;

        loop {
            current = current.get_relative(file_relative, rank_relative);

            if !current.is_valid() {
                break;
            }

            relatives.push(current)
        }

        relatives
    }

    pub fn find_path_to(&self, other: &Self) -> Option<Vec<Self>> {
        if other == self {
            return Some(Vec::new());
        }

        let file_change: i8 = other.file_number as i8 - self.file_number as i8;
        let rank_change: i8 = other.rank_number as i8 - self.rank_number as i8;

        let file_change_reduced = if file_change == 0 { 0 } else { file_change / file_change.abs() };
        let rank_change_reduced = if rank_change == 0 { 0 } else { rank_change / rank_change.abs() };

        if file_change != 0 && rank_change != 0 && file_change_reduced.abs() != rank_change_reduced.abs() {
            return None;
        }

        let mut path = self.get_relatives_until_invalid(file_change_reduced, rank_change_reduced);

        while let Some(element) = path.pop() {
            if element == *other {
                break;
            }
        }

        Some(path)
    }

    pub fn get_unique_index(&self) -> u8 {
        (self.rank_number - 1) * 8 + (self.file_number - 1)
    }
}

impl FromStr for Square {
    type Err = MoveParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let string = s.to_uppercase();
        if string.len() != 2 {
            return Err(MoveParsingError::IncorrectSquareFormat);
        }

        let chars = string.as_bytes();
        let file_character = chars[0];
        let rank_character = chars[1];

        if file_character < b'A' || file_character > b'H' || rank_character < b'1' || rank_character > b'8' {
            return Err(MoveParsingError::InvalidSquare);
        }

        let square = Self::new(file_character - b'A' + 1, rank_character - b'1' + 1);

        if !square.is_valid() {
            return Err(MoveParsingError::InvalidSquare);
        }

        Ok(square)
    }
}

impl Display for Square {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.get_file_as_letter(), self.rank_number)
    }
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct CastlingRights {
    pub short_castle: bool,
    pub long_castle: bool,
}

#[derive(Clone)]
pub struct BoardState {
    pub white_castling_rights: CastlingRights,
    pub black_castling_rights: CastlingRights,
    pub en_passant_square: Option<Square>,
    pub pieces: HashMap<Square, Piece>,
}

impl BoardState {
    pub fn get_castling_rights_mut_for(&mut self, color: Color) -> &mut CastlingRights {
        match color {
            Color::White => &mut self.white_castling_rights,
            Color::Black => &mut self.black_castling_rights,
        }
    }

    pub fn get_castling_rights_for(&self, color: Color) -> &CastlingRights {
        match color {
            Color::White => &self.white_castling_rights,
            Color::Black => &self.black_castling_rights,
        }
    }

    pub fn get_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn hash_castling_rights(rights: &CastlingRights) -> u8 {
        let mut hash: u8 = 0;

        if rights.short_castle {
            hash += 1;
        }

        if rights.long_castle {
            hash += 2;
        }

        hash
    }
}

impl Hash for BoardState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u8(BoardState::hash_castling_rights(&self.white_castling_rights));
        state.write_u8(BoardState::hash_castling_rights(&self.black_castling_rights));
        state.write_u8(self.en_passant_square.map(|square| square.get_unique_index()).unwrap_or(64));

        for file in 1..9 {
            for rank in 1..9 {
                state.write_u8(file);
                state.write_u8(rank);

                if let Some(piece) = self.pieces.get(&Square::new(file, rank)) {
                    state.write_u8(match piece.piece_type {
                        Type::King => 1,
                        Type::Queen => 2,
                        Type::Rook => 3,
                        Type::Bishop => 4,
                        Type::Knight => 5,
                        Type::Pawn => 6,
                    });

                    state.write_u8(match piece.color {
                        Color::White => 1,
                        Color::Black => 2,
                    })
                }
            }
        }
    }
}

impl PartialEq for BoardState {
    fn eq(&self, other: &Self) -> bool {
        if self.white_castling_rights != other.white_castling_rights || self.black_castling_rights != other.black_castling_rights || self.en_passant_square != other.en_passant_square {
            return false;
        }

        if self.pieces.len() != other.pieces.len() {
            return false;
        }

        for (_, this_piece) in self.pieces.iter() {
            match other.pieces.get(&this_piece.location) {
                Some(other_piece) => {
                    if *other_piece != *this_piece {
                        return false;
                    }
                }
                None => {
                    return false;
                }
            }
        }

        true
    }
}

#[derive(PartialEq, Clone)]
pub struct Board {
    pub highlighted_squares: Vec<Square>,
    pub state: BoardState,
    pub last_move: Option<HistoryMove>,
}

impl Board {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_piece(&mut self, piece: Piece) {
        self.state.pieces.insert(piece.location, piece);
    }

    pub fn remove_piece(&mut self, location: Square) {
        self.state.pieces.remove(&location);
    }

    pub fn get_piece_mut(&mut self, location: Square) -> Option<&mut Piece> {
        self.state.pieces.get_mut(&location)
    }

    pub fn get_piece(&self, location: Square) -> Option<&Piece> {
        self.state.pieces.get(&location)
    }

    pub fn clear_board(&mut self) {
        self.state.pieces.clear()
    }

    fn setup_initial_pieces(&mut self, color: Color) {
        let rank = if color == Color::White { 1 } else { 8 };

        self.set_piece(Piece::new(Square::new(1, rank), color, Type::Rook));
        self.set_piece(Piece::new(Square::new(2, rank), color, Type::Knight));
        self.set_piece(Piece::new(Square::new(3, rank), color, Type::Bishop));
        self.set_piece(Piece::new(Square::new(4, rank), color, Type::Queen));
        self.set_piece(Piece::new(Square::new(5, rank), color, Type::King));
        self.set_piece(Piece::new(Square::new(6, rank), color, Type::Bishop));
        self.set_piece(Piece::new(Square::new(7, rank), color, Type::Knight));
        self.set_piece(Piece::new(Square::new(8, rank), color, Type::Rook));
    }

    fn setup_initial_pawns(&mut self, color: Color) {
        let rank = if color == Color::White { 2 } else { 7 };

        for file in 1..9 {
            self.set_piece(Piece::new(Square::new(file, rank), color, Type::Pawn));
        }
    }

    pub fn setup_default_board(&mut self) {
        self.clear_board();

        self.state.white_castling_rights.long_castle = true;
        self.state.white_castling_rights.short_castle = true;
        self.state.black_castling_rights.long_castle = true;
        self.state.black_castling_rights.short_castle = true;
        self.state.en_passant_square = None;

        self.highlighted_squares.clear();
        self.last_move = None;

        self.setup_initial_pieces(Color::White);
        self.setup_initial_pieces(Color::Black);
        self.setup_initial_pawns(Color::White);
        self.setup_initial_pawns(Color::Black);
    }

    pub fn is_path_clear(&self, path: Vec<Square>) -> bool {
        path.iter().all(|&square| self.get_piece(square).is_none())
    }

    pub fn is_attacked(&self, square: Square, color: Option<Color>) -> bool {
        for (_, piece) in self.state.pieces.iter() {
            if let Some(required_color) = color {
                if required_color != piece.color {
                    continue;
                }
            }

            for valid_move in &piece.valid_moves {
                if *valid_move != square {
                    continue;
                }

                let move_check = NewMove {
                    from: piece.location,
                    to: *valid_move,
                    extra: Extra::MoveCheck,
                };

                if !piece.is_move_valid(self, move_check) {
                    continue;
                }

                return true;
            }
        }

        false
    }

    pub fn is_in_check(&self, color: Color) -> bool {
        for (_, piece) in self.state.pieces.iter() {
            if piece.piece_type != Type::King || piece.color != color {
                continue;
            }

            if self.is_attacked(piece.location, Some(piece.color.get_opposite())) {
                return true;
            }
        }

        false
    }

    pub fn get_valid_moves_for_piece(&self, piece: &Piece) -> Vec<HistoryMove> {
        let mut valid_moves = Vec::new();

        for valid_move in &piece.valid_moves {
            let mut board = self.clone();

            let move_check = NewMove {
                from: piece.location,
                to: *valid_move,
                extra: Extra::MoveCheck,
            };

            if board.make_move_if_valid(move_check).is_err() {
                continue;
            }

            if board.is_in_check(piece.color) {
                continue;
            }

            valid_moves.push(board.last_move.unwrap());
        }

        valid_moves
    }

    pub fn get_valid_moves_for(&self, color: Color) -> Vec<HistoryMove> {
        let mut valid_moves = Vec::new();

        for (_, piece) in self.state.pieces.iter() {
            if piece.color != color {
                continue;
            }

            valid_moves.append(&mut self.get_valid_moves_for_piece(piece));
        }

        valid_moves
    }

    pub fn get_pieces_count_by_type(&self, color: Color) -> HashMap<Type, usize> {
        let mut count: HashMap<Type, usize> = HashMap::with_capacity(5);
        count.insert(Type::King, 0);
        count.insert(Type::Queen, 0);
        count.insert(Type::Rook, 0);
        count.insert(Type::Bishop, 0);
        count.insert(Type::Knight, 0);
        count.insert(Type::Pawn, 0);

        for (_, piece) in self.state.pieces.iter() {
            if piece.color == color {
                *count.get_mut(&piece.piece_type).unwrap() += 1;
            }
        }

        count
    }

    pub fn get_material_count(&self, color: Color) -> usize {
        let count = self.get_pieces_count_by_type(color);

        count.get(&Type::Queen).unwrap() * 9 + count.get(&Type::Rook).unwrap() * 5 + count.get(&Type::Bishop).unwrap() * 3 + count.get(&Type::Knight).unwrap() * 3 + count.get(&Type::Pawn).unwrap()
    }

    pub fn make_move_if_valid(&mut self, m: NewMove) -> Result<(), MoveFailureReason> {
        let piece_color: Color;
        let piece_type: Type;

        let piece = match self.get_piece(m.from) {
            Some(piece) => piece,
            None => return Err(NoPiece),
        };

        // Check if move was valid
        if !piece.is_move_valid(self, m) {
            return Err(MoveInvalid);
        }

        piece_color = piece.color;
        piece_type = piece.piece_type;

        let mut was_capture = false;

        // Check if this was a capture
        if let Some(capture) = self.get_piece(m.to) {
            if capture.color == piece_color {
                return Err(CannotCaptureOwnPiece);
            }

            was_capture = true;
        }

        // Remove the piece from old location and the captured piece if any
        self.remove_piece(m.from);
        self.remove_piece(m.to);

        // Setup last move
        self.last_move = Some(HistoryMove {
            piece_color,
            piece_type,
            from: m.from,
            to: m.to,
            capture: was_capture,
            extra: m.extra,
        });

        // Create new piece at the target destination and call after_move
        let piece = Piece::new(m.to, piece_color, piece_type);
        self.set_piece(piece.clone());
        piece.after_move(self);

        // Mark highlighted squares
        self.highlighted_squares.clear();
        self.highlighted_squares.push(m.from);
        self.highlighted_squares.push(m.to);

        Ok(())
    }

    pub fn recalculate_all_pieces_movements(&mut self) {
        for (_, piece) in self.state.pieces.iter_mut() {
            piece.recalculate_valid_moves();
        }
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            highlighted_squares: Vec::new(),
            state: BoardState {
                white_castling_rights: CastlingRights {
                    short_castle: true,
                    long_castle: true,
                },
                black_castling_rights: CastlingRights {
                    short_castle: true,
                    long_castle: true,
                },
                en_passant_square: None,
                pieces: HashMap::with_capacity(64),
            },
            last_move: None,
        }
    }
}
