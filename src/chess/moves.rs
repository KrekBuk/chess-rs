use super::board::{Color, Square};
use super::pieces::Type;

use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Eq, PartialEq, Copy, Clone, Debug, Error)]
pub enum MoveFailureReason {
    #[error("No piece")]
    NoPiece,
    #[error("Invalid move")]
    MoveInvalid,
    #[error("You cannot capture your own piece")]
    CannotCaptureOwnPiece,
    #[error("It is not your piece")]
    NotYourPiece,
    #[error("Illegal piece move")]
    IllegalPieceMove,
    #[error("In check after turn")]
    InCheckAfterTurn,
    #[error("No previous positions")]
    NoPreviousPositions,
    #[error("Game ended")]
    GameEnded,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum Extra {
    Promotion(Type),
    MoveCheck,
    None,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct NewMove {
    pub from: Square,
    pub to: Square,
    pub extra: Extra,
}

#[derive(Eq, PartialEq, Debug)]
pub enum MoveParsingError {
    IncorrectMoveFormat,
    IncorrectSquareFormat,
    InvalidSquare,
}

impl FromStr for NewMove {
    type Err = MoveParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 4 {
            return Err(MoveParsingError::IncorrectMoveFormat);
        }

        let from = Square::from_str(&s[0..2].to_uppercase())?;
        let to = Square::from_str(&s[2..4].to_uppercase())?;
        // TODO: Extra

        Ok(NewMove { from, to, extra: Extra::None })
    }
}

impl Display for MoveParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct HistoryMove {
    pub piece_color: Color,
    pub piece_type: Type,
    pub from: Square,
    pub to: Square,
    pub capture: bool,
    pub extra: Extra,
}
