use std::collections::HashMap;
use image::{ImageFormat, Rgba};

use chess_bot::chess::board::Color;
use chess_bot::chess::pieces::Type;
use chess_bot::util::board_visualizer::{BoardVisualizer, Config};

use chess_bot::discord::bot::{start_bot, BotData};
use chess_bot::system::game::GameManager;
use serenity::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let data = BotData {
        visualizer: setup_visualizer(),
        game_manager: GameManager::new(),
    };

    start_bot(data).await?;

    Ok(())
}

fn setup_visualizer() -> BoardVisualizer {
    let mut piece_mappings = HashMap::new();
    let mut piece_mappings_white = HashMap::new();
    let mut piece_mappings_black = HashMap::new();

    piece_mappings_white.insert(Type::Queen, (1, 0));
    piece_mappings_black.insert(Type::Queen, (1, 1));
    piece_mappings_white.insert(Type::King, (0, 0));
    piece_mappings_black.insert(Type::King, (0, 1));
    piece_mappings_white.insert(Type::Rook, (4, 0));
    piece_mappings_black.insert(Type::Rook, (4, 1));
    piece_mappings_white.insert(Type::Knight, (3, 0));
    piece_mappings_black.insert(Type::Knight, (3, 1));
    piece_mappings_white.insert(Type::Bishop, (2, 0));
    piece_mappings_black.insert(Type::Bishop, (2, 1));
    piece_mappings_white.insert(Type::Pawn, (5, 0));
    piece_mappings_black.insert(Type::Pawn, (5, 1));

    piece_mappings.insert(Color::White, piece_mappings_white);
    piece_mappings.insert(Color::Black, piece_mappings_black);

    let config = Config {
        tile_size: 64,
        bottom_fill_size: 50,
        bottom_fill_color: Rgba([0x36, 0x39, 0x3f, 0xFF]),
        light_tile_color: Rgba([0x36, 0x39, 0x3f, 0xFF]),
        dark_tile_color: Rgba([0x32, 0x35, 0x3b, 0xFF]),
        light_tile_color_highlighted: Rgba([0x56, 0x59, 0x5f, 0xFF]),
        dark_tile_color_highlighted: Rgba([0x52, 0x55, 0x5b, 0xFF]),
        text_on_light_color: Rgba([0xFF, 0xFF, 0xFF, 0xFF]),
        text_on_dark_color: Rgba([0xFF, 0xFF, 0xFF, 0xFF]),
        text_font: Vec::from(include_bytes!("res/DejaVuSans.ttf") as &[u8]),
        text_font_size: 20,
        pieces_image: Vec::from(include_bytes!("res/pieces.png") as &[u8]),
        pieces_image_format: ImageFormat::Png,
        pieces_mappings: piece_mappings,
        piece_size: 60,
    };

    BoardVisualizer::new(config)
}
