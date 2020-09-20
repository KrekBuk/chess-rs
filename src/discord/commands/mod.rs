use crate::chess::moves::MoveFailureReason;

pub mod admin;
pub mod game;
pub mod util;

#[derive(Error, Debug)]
pub enum GeneralError {
    #[error("Failed to create a game, maybe you're already in one?")]
    FailedToCreateGame,
    #[error("This player is not in a game.")]
    PlayerNotInGame,
    #[error("Failed to move: {0}")]
    FailedToMove(MoveFailureReason),
    #[error("Failed to resign.")]
    FailedToResign,
}
