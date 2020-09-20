use super::board::{Board, Color, Square};
use super::moves::Extra;

use crate::chess::moves::NewMove;
use serde::{Deserialize, Serialize};

#[derive(Eq, PartialEq, Copy, Clone, Hash, Serialize, Deserialize)]
pub enum Type {
    King,
    Queen,
    Rook,
    Bishop,
    Knight,
    Pawn,
}

#[derive(Clone)]
pub struct Piece {
    pub location: Square,
    pub color: Color,
    pub piece_type: Type,
    pub valid_moves: Vec<Square>,
}

impl Piece {
    pub fn new(location: Square, color: Color, piece_type: Type) -> Self {
        let mut piece = Self {
            location,
            color,
            piece_type,
            valid_moves: Vec::new(),
        };

        piece.recalculate_valid_moves();
        piece
    }

    fn get_move_controller(&self) -> &'static dyn MoveController {
        match self.piece_type {
            Type::King => &KING_MOVE_CONTROLLER,
            Type::Queen => &QUEEN_MOVE_CONTROLLER,
            Type::Rook => &ROOK_MOVE_CONTROLLER,
            Type::Bishop => &BISHOP_MOVE_CONTROLLER,
            Type::Knight => &KNIGHT_MOVE_CONTROLLER,
            Type::Pawn => &PAWN_MOVE_CONTROLLER,
        }
    }

    pub fn get_advance_direction(&self) -> i8 {
        if self.color == Color::White {
            1
        } else {
            -1
        }
    }

    pub fn recalculate_valid_moves(&mut self) {
        self.valid_moves.clear();
        self.get_move_controller().recalculate_valid_moves(self);
        self.valid_moves.retain(|&m| m.is_valid());
    }

    pub fn is_move_valid(&self, board: &Board, m: NewMove) -> bool {
        if !m.to.is_valid() {
            return false;
        }

        if !self.valid_moves.iter().any(|&valid_move| valid_move == m.to) {
            return false;
        }

        self.get_move_controller().check_if_move_valid(board, &self, m)
    }

    pub fn after_move(&self, board: &mut Board) {
        self.get_move_controller().after_move(board)
    }
}

impl PartialEq for Piece {
    fn eq(&self, other: &Self) -> bool {
        self.location == other.location && self.color == other.color && self.piece_type == other.piece_type
    }
}

pub trait MoveController {
    fn recalculate_valid_moves(&self, piece: &mut Piece);

    fn check_if_move_valid(&self, board: &Board, piece: &Piece, m: NewMove) -> bool;

    fn after_move(&self, board: &mut Board);
}

pub struct PawnMoveController {}

impl MoveController for PawnMoveController {
    fn recalculate_valid_moves(&self, piece: &mut Piece) {
        let advance_direction = piece.get_advance_direction();

        piece.valid_moves.push(piece.location.get_relative(0, advance_direction));
        piece.valid_moves.push(piece.location.get_relative(1, advance_direction));
        piece.valid_moves.push(piece.location.get_relative(-1, advance_direction));

        if (piece.location.rank_number == 2 && piece.color == Color::White) || (piece.location.rank_number == 7 && piece.color == Color::Black) {
            piece.valid_moves.push(piece.location.get_relative(0, advance_direction * 2));
        }
    }

    fn check_if_move_valid(&self, board: &Board, piece: &Piece, m: NewMove) -> bool {
        let advance_direction = piece.get_advance_direction();
        let destination_rank = ((piece.location.rank_number as i8) + advance_direction) as u8;
        let first_move_destination_rank = ((piece.location.rank_number as i8) + advance_direction * 2) as u8;

        if piece.location.file_number == m.to.file_number {
            // Move forward
            if destination_rank != m.to.rank_number {
                // Not a +1 move, maybe its a first move?
                if piece.location.rank_number == 2 || piece.location.rank_number == 7 {
                    if first_move_destination_rank != m.to.rank_number {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            if board.get_piece(m.to).is_some() {
                // Pawns cannot capture forward
                return false;
            }
        } else if piece.location.file_number == m.to.file_number - 1 || piece.location.file_number == m.to.file_number + 1 {
            // Capture diagonally
            if destination_rank != m.to.rank_number {
                // Can only move 1 when capturing
                return false;
            }

            let mut capture_piece = board.get_piece(m.to);

            if let Some(en_passant_square) = board.state.en_passant_square {
                if en_passant_square == m.to {
                    // En passant capture
                    capture_piece = board.get_piece(Square::new(m.to.file_number, piece.location.rank_number))
                }
            }

            if let Some(captured_piece) = capture_piece {
                if captured_piece.color == piece.color {
                    // Cannot capture own pieces
                    return false;
                }
            } else {
                // Not a capture, cannot go
                return false;
            }
        } else {
            // Invalid move
            return false;
        }

        if m.to.rank_number == 1 || m.to.rank_number == 8 {
            // Promotion rank
            if let Extra::MoveCheck = m.extra {
                return true;
            }

            if let Extra::Promotion(_) = m.extra {
                return true;
            }

            // No promotion arguments
            return false;
        }

        true
    }

    fn after_move(&self, board: &mut Board) {
        let mut capture_square: Option<Square> = None;

        {
            let last_move = board.last_move.unwrap();
            let piece = board.get_piece(last_move.to).unwrap();
            let piece_color = piece.color;

            // Check if move was en passant
            if let Some(en_passant_square) = board.state.en_passant_square {
                if en_passant_square == last_move.to {
                    capture_square = Some(Square::new(last_move.to.file_number, last_move.from.rank_number));
                }
            }

            // Check if move was a first move by 2 squares
            let first_move_destination_rank = ((last_move.from.rank_number as i8) + piece.get_advance_direction() * 2) as u8;

            if first_move_destination_rank == last_move.to.rank_number {
                board.state.en_passant_square = Some(last_move.from.get_relative(0, piece.get_advance_direction()));
            } else {
                board.state.en_passant_square = None;
            }

            // Promotion
            if last_move.to.rank_number == 1 || last_move.to.rank_number == 8 {
                let new_piece_type = match last_move.extra {
                    Extra::Promotion(new_type) => new_type,
                    _ => Type::Queen,
                };

                board.remove_piece(last_move.to);
                board.set_piece(Piece::new(last_move.to, piece_color, new_piece_type));
            }
        }

        if let Some(capture_square) = capture_square {
            board.last_move.as_mut().unwrap().capture = true;
            board.remove_piece(capture_square)
        }
    }
}

pub struct RookMoveController {}

impl MoveController for RookMoveController {
    fn recalculate_valid_moves(&self, piece: &mut Piece) {
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(-1, 0));
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(1, 0));
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(0, 1));
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(0, -1));
    }

    fn check_if_move_valid(&self, board: &Board, piece: &Piece, m: NewMove) -> bool {
        board.is_path_clear(piece.location.find_path_to(&m.to).unwrap())
    }

    fn after_move(&self, board: &mut Board) {
        let last_move = board.last_move.unwrap();

        if last_move.from.file_number == 8 {
            board.state.get_castling_rights_mut_for(last_move.piece_color).short_castle = false;
        }

        if last_move.from.file_number == 1 {
            board.state.get_castling_rights_mut_for(last_move.piece_color).long_castle = false;
        }
    }
}

pub struct KnightMoveController {}

impl MoveController for KnightMoveController {
    fn recalculate_valid_moves(&self, piece: &mut Piece) {
        piece.valid_moves.push(piece.location.get_relative(1, 2));
        piece.valid_moves.push(piece.location.get_relative(-1, 2));
        piece.valid_moves.push(piece.location.get_relative(1, -2));
        piece.valid_moves.push(piece.location.get_relative(-1, -2));
        piece.valid_moves.push(piece.location.get_relative(2, 1));
        piece.valid_moves.push(piece.location.get_relative(-2, 1));
        piece.valid_moves.push(piece.location.get_relative(2, -1));
        piece.valid_moves.push(piece.location.get_relative(-2, -1));
    }

    fn check_if_move_valid(&self, _board: &Board, _piece: &Piece, _m: NewMove) -> bool {
        true
    }

    fn after_move(&self, _board: &mut Board) {}
}

pub struct BishopMoveController {}

impl MoveController for BishopMoveController {
    fn recalculate_valid_moves(&self, piece: &mut Piece) {
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(-1, -1));
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(1, -1));
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(-1, 1));
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(1, 1));
    }

    fn check_if_move_valid(&self, board: &Board, piece: &Piece, m: NewMove) -> bool {
        board.is_path_clear(piece.location.find_path_to(&m.to).unwrap())
    }

    fn after_move(&self, _board: &mut Board) {}
}

pub struct QueenMoveController {}

impl MoveController for QueenMoveController {
    fn recalculate_valid_moves(&self, piece: &mut Piece) {
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(-1, 0));
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(1, 0));
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(0, 1));
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(0, -1));
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(-1, -1));
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(1, -1));
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(-1, 1));
        piece.valid_moves.append(&mut piece.location.get_relatives_until_invalid(-1, -1));
    }

    fn check_if_move_valid(&self, board: &Board, piece: &Piece, m: NewMove) -> bool {
        board.is_path_clear(piece.location.find_path_to(&m.to).unwrap())
    }

    fn after_move(&self, _board: &mut Board) {}
}

pub struct KingMoveController {}

impl MoveController for KingMoveController {
    fn recalculate_valid_moves(&self, piece: &mut Piece) {
        piece.valid_moves.push(piece.location.get_relative(-1, -1));
        piece.valid_moves.push(piece.location.get_relative(-1, 0));
        piece.valid_moves.push(piece.location.get_relative(-1, 1));
        piece.valid_moves.push(piece.location.get_relative(0, -1));
        piece.valid_moves.push(piece.location.get_relative(0, 1));
        piece.valid_moves.push(piece.location.get_relative(1, -1));
        piece.valid_moves.push(piece.location.get_relative(1, 0));
        piece.valid_moves.push(piece.location.get_relative(1, 1));
    }

    fn check_if_move_valid(&self, board: &Board, piece: &Piece, m: NewMove) -> bool {
        if m.to.file_number == piece.location.file_number - 2 {
            return self.can_castle_short(board, piece);
        }

        if m.to.file_number == piece.location.file_number + 2 {
            return self.can_castle_long(board, piece);
        }

        true
    }

    fn after_move(&self, board: &mut Board) {
        let last_move = board.last_move.unwrap();

        let mut rook_from = None;
        let mut rook_to = None;

        if last_move.to.file_number == last_move.from.file_number + 2 {
            // Short castle
            rook_from = Some(Square::new(8, last_move.to.rank_number));
            rook_to = Some(Square::new(6, last_move.to.rank_number));
        }

        if last_move.to.file_number == last_move.from.file_number - 2 {
            // Long castle
            rook_from = Some(Square::new(1, last_move.to.rank_number));
            rook_to = Some(Square::new(4, last_move.to.rank_number));
        }

        if let Some(from) = rook_from {
            if let Some(to) = rook_to {
                board.remove_piece(from);
                board.set_piece(Piece::new(to, last_move.piece_color, Type::Rook));
            }
        }

        let mut castling_rights = board.state.get_castling_rights_mut_for(last_move.piece_color);
        castling_rights.short_castle = false;
        castling_rights.long_castle = false;
    }
}

impl KingMoveController {
    pub fn can_castle_short(&self, board: &Board, king: &Piece) -> bool {
        if !board.state.get_castling_rights_for(king.color).short_castle {
            return false;
        }

        if board.is_attacked(Square::new(6, king.location.rank_number), Some(king.color.get_opposite())) {
            // f1 / f8 is attacked
            return false;
        }

        self.validate_can_castle(board, king, Square::new(1, king.location.rank_number))
    }

    pub fn can_castle_long(&self, board: &Board, king: &Piece) -> bool {
        if !board.state.get_castling_rights_for(king.color).long_castle {
            return false;
        }

        if board.is_attacked(Square::new(4, king.location.rank_number), Some(king.color.get_opposite())) {
            // d1 / d8 is attacked
            return false;
        }

        self.validate_can_castle(board, king, Square::new(1, king.location.rank_number))
    }

    fn validate_can_castle(&self, board: &Board, king: &Piece, rook_location: Square) -> bool {
        if (king.color == Color::White && king.location.to_string() != "E1") || (king.color == Color::Black && king.location.to_string() != "E8") {
            return false;
        }

        if let Some(rook) = board.get_piece(rook_location) {
            if rook.color != king.color || rook.piece_type != Type::Rook {
                return false;
            }
        } else {
            return false;
        }

        if let Some(path) = king.location.find_path_to(&rook_location) {
            return board.is_path_clear(path);
        }

        false
    }
}

static PAWN_MOVE_CONTROLLER: PawnMoveController = PawnMoveController {};
static ROOK_MOVE_CONTROLLER: RookMoveController = RookMoveController {};
static KNIGHT_MOVE_CONTROLLER: KnightMoveController = KnightMoveController {};
static BISHOP_MOVE_CONTROLLER: BishopMoveController = BishopMoveController {};
static QUEEN_MOVE_CONTROLLER: QueenMoveController = QueenMoveController {};
static KING_MOVE_CONTROLLER: KingMoveController = KingMoveController {};
