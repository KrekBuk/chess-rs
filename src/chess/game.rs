use serde::{Deserialize, Serialize};

use super::board::{Board, Color};
use super::moves::{HistoryMove, MoveFailureReason};

use super::pieces::Type;

use crate::chess::moves::NewMove;
use GameResult::*;
use MoveFailureReason::*;

#[derive(Eq, PartialEq, Copy, Clone, Hash, Debug, Serialize, Deserialize)]
pub enum GameResult {
    Ongoing,
    CheckMate(Color),
    Resignation(Color),
    OutOfTime(Color),
    Stalemated,
    InsufficientMaterial,
    ThreefoldRepetition,
    FiftyMoves,
    DrawAgreed,
}

impl GameResult {
    pub fn get_winner(&self) -> Option<Color> {
        use GameResult::*;

        match self {
            Ongoing | Stalemated | InsufficientMaterial | ThreefoldRepetition | FiftyMoves | DrawAgreed => None,
            CheckMate(color) | Resignation(color) | OutOfTime(color) => Some(color.get_opposite()),
        }
    }

    pub fn pretty_message(&self) -> String {
        match self {
            Ongoing => String::from("The game is still ongoing."),
            CheckMate(color) => format!("{:?} is checkmated.", color),
            Resignation(color) => format!("{:?} has resigned.", color),
            OutOfTime(color) => format!("{:?} has resigned.", color),
            Stalemated => String::from("Stalemate."),
            InsufficientMaterial => String::from("Insufficient material. "),
            ThreefoldRepetition => String::from("Three-fold repetition."),
            FiftyMoves => String::from("50-move rule violation."),
            DrawAgreed => String::from("Both players agreed to a draw. "),
        }
    }
}

#[derive(Clone)]
pub struct GameState {
    pub board: Board,
    pub half_move_clock: u32,
    pub current_turn: Color,
    pub draw_offers: Vec<Color>,
    pub takeback_offers: Vec<Color>,
    pub board_hash: u64,
}

impl GameState {
    pub fn new(board: Board, half_move_clock: u32, current_turn: Color) -> Self {
        Self {
            board,
            half_move_clock,
            current_turn,
            draw_offers: Vec::with_capacity(2),
            takeback_offers: Vec::with_capacity(2),
            board_hash: 0,
        }
    }
}

#[derive(Clone)]
pub struct Game {
    pub state: GameState,
    pub state_history: Vec<GameState>,
    pub result: Option<GameResult>,
}

impl Game {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.state.board.setup_default_board();
        self.state.half_move_clock = 0;
        self.state.current_turn = Color::White;
        self.state.draw_offers.clear();
        self.state.takeback_offers.clear();

        self.state_history.clear();
        self.result = None;
    }

    pub fn takeback_move(&mut self) -> Result<(), MoveFailureReason> {
        if self.result.is_some() {
            return Err(GameEnded);
        }

        match self.state_history.pop() {
            Some(state) => {
                self.state = state;
                self.state.board.recalculate_all_pieces_movements();

                Ok(())
            }
            None => Err(NoPreviousPositions),
        }
    }

    pub fn draw(&mut self) -> Result<GameResult, MoveFailureReason> {
        if self.result.is_some() {
            return Err(GameEnded);
        }

        self.result = Some(DrawAgreed);
        Ok(self.result.unwrap())
    }

    pub fn make_move(&mut self, m: NewMove) -> Result<HistoryMove, MoveFailureReason> {
        if self.result.is_some() {
            return Err(GameEnded);
        }

        // Check if move is valid
        let piece = match self.state.board.get_piece(m.from) {
            Some(piece) => piece,
            None => return Err(NoPiece),
        };

        if piece.color != self.state.current_turn {
            return Err(NotYourPiece);
        }

        let mut new_board = self.state.board.clone();

        new_board.make_move_if_valid(m)?;

        if new_board.is_in_check(self.state.current_turn) {
            return Err(InCheckAfterTurn);
        }

        // Clone this state
        let mut previous_state = self.state.clone();

        // Make a new state
        self.state.board = new_board;
        self.state.half_move_clock = previous_state.half_move_clock;
        self.state.current_turn = previous_state.current_turn.get_opposite();
        self.state.draw_offers.clear();
        self.state.takeback_offers.clear();

        // Save the previous state to history
        previous_state.board_hash = previous_state.board.state.get_hash();
        self.state_history.push(previous_state);

        // Reset half-move counter if a pawn move or a capture was made
        let last_move = &self.state.board.last_move.unwrap();
        if last_move.capture || last_move.piece_type == Type::Pawn {
            self.state.half_move_clock = 0;
        }

        // check for mate or draw
        if self.state.board.get_valid_moves_for(self.state.current_turn).is_empty() {
            // current player has no moves
            if self.state.board.is_in_check(self.state.current_turn) {
                // they are in check so its checkmate
                self.result = Some(CheckMate(self.state.current_turn))
            } else {
                // they are not in check so its stalemate
                self.result = Some(Stalemated);
            }
        } else if self.check_for_insufficient_material() {
            self.result = Some(InsufficientMaterial);
        } else if self.check_for_threefold_repetition() {
            self.result = Some(ThreefoldRepetition);
        } else if self.state.half_move_clock >= 50 {
            self.result = Some(FiftyMoves);
        }

        Ok(self.state.board.last_move.unwrap())
    }

    pub fn resign(&mut self, color: Color) -> Result<GameResult, MoveFailureReason> {
        if self.result.is_some() {
            return Err(GameEnded);
        }

        self.result = Some(Resignation(color));
        Ok(self.result.unwrap())
    }

    pub fn offer_draw(&mut self, color: Color) -> Result<GameResult, MoveFailureReason> {
        if self.result.is_some() {
            return Err(GameEnded);
        }

        self.state.draw_offers.push(color);
        self.state.draw_offers.dedup();

        if self.state.draw_offers.len() == 2 {
            return self.draw();
        }

        Ok(Ongoing)
    }

    pub fn offer_takeback(&mut self, color: Color) -> Result<bool, MoveFailureReason> {
        if self.result.is_some() {
            return Err(GameEnded);
        }

        self.state.takeback_offers.push(color);
        self.state.takeback_offers.dedup();

        if self.state.takeback_offers.len() == 2 {
            self.takeback_move()?;
            return Ok(true);
        }

        Ok(false)
    }

    fn validate_has_sufficient_material(&self, color: Color) -> bool {
        let count = self.state.board.get_pieces_count_by_type(color);

        if count[&Type::Rook] != 0 || count[&Type::Queen] != 0 || count[&Type::Pawn] != 0 {
            // there are rooks, queens or pawns, game is not drawn
            return true;
        }

        // 2 bishops, 3 knights or 1 bishop and 1 knight are enough to force a mate
        if count[&Type::Bishop] >= 2 || count[&Type::Knight] >= 3 {
            return true;
        }

        count[&Type::Bishop] >= 1 && count[&Type::Knight] >= 1
    }

    pub fn check_for_insufficient_material(&self) -> bool {
        !self.validate_has_sufficient_material(Color::White) && !self.validate_has_sufficient_material(Color::Black)
    }

    pub fn check_for_threefold_repetition(&self) -> bool {
        let mut positions_count = 1;

        let current_hash = self.state.board.state.get_hash();

        for previous_state in &self.state_history {
            if previous_state.board_hash != current_hash {
                continue;
            }

            if self.state.board.state != previous_state.board.state {
                continue;
            }

            positions_count += 1;
        }

        positions_count >= 3
    }
}

impl Default for Game {
    fn default() -> Self {
        let mut new = Self {
            state: GameState::new(Board::new(), 0, Color::White),
            state_history: Vec::new(),
            result: None,
        };

        new.reset();
        new
    }
}
