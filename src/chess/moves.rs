use super::board::{Color, Square};
use super::pieces::Type;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum MoveFailureReason {
    NoPiece,
    MoveInvalid,
    CannotCaptureOwnPiece,
    NotYourPiece,
    IllegalPieceMove,
    InCheckAfterTurn,
    NoPreviousPositions,
    GameEnded,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum Extra {
    Promotion(Type),
    MoveCheck,
    None,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct Move {
    pub piece_color: Color,
    pub piece_type: Type,
    pub from: Square,
    pub to: Square,
    pub capture: bool,
    pub extra: Extra,
}
