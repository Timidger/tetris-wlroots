extern crate rand;
#[macro_use]
extern crate wlroots;

use rand::random;
use std::time::Instant;

use wlroots::{Area, CompositorBuilder, CompositorHandle, InputManagerHandler, KeyboardHandle,
              KeyboardHandler, Origin, OutputBuilder, OutputBuilderResult, OutputHandle,
              OutputHandler, OutputManagerHandler, Size, key_events::KeyEvent,
              xkbcommon::xkb::KEY_Escape, WLR_KEY_PRESSED};

const BOARD_WIDTH: usize = 10;
const BOARD_HEIGHT: usize = 20;

#[derive(Debug, Clone, Copy)]
enum Color {
    Cyan,
    Blue,
    Orange,
    Yellow,
    Lime,
    Purple,
    Red,
    Grey
}

#[derive(Default, Debug, Clone, Copy)]
struct PieceData(Origin, Origin, Origin, Origin);

impl PieceData {
    fn move_down(mut self) -> Self {
        self.0.y += 1;
        self.1.y += 1;
        self.2.y += 1;
        self.3.y += 1;
        PieceData(self.0, self.1, self.2, self.3)
    }

    fn coords(self) -> [Origin; 4] {
        [self.0, self.1, self.2, self.3]
    }
}

#[derive(Debug, Clone, Copy)]
enum Piece {
    Block(PieceData),
    L(PieceData)
}

impl Piece {
    fn random() -> Self {
        loop {
            return match random::<u8>() {
                0 => {
                    let a = Origin::new(0, 0);
                    let b = Origin::new(0, 1);
                    let c = Origin::new(0, 2);
                    let d = Origin::new(1, 2);
                    Piece::L(PieceData(a,b,c,d))
                },
                1 => {
                    let a = Origin::new(0, 0);
                    let b = Origin::new(1, 0);
                    let c = Origin::new(0, 1);
                    let d = Origin::new(1, 1);
                    Piece::Block(PieceData(a,b,c,d))
                }
                _ => continue
            }
        }
    }
    /// Simulate moving a piece down
    fn move_down(self) -> Self {
        use Piece::*;
        match self {
            Block(data) => Block(data.move_down()),
            L(data) => L(data.move_down())
        }
    }

    fn color(self) -> Color {
        use Piece::*;
        match self {
            Block(..) => Color::Blue,
            L(..) => Color::Orange
        }
    }

    /// Get an iterator over the grid-level coordinates.
    fn coords(self) -> [Origin; 4] {
        use Piece::*;
        match self {
            L(data) | Block(data) => data.coords(),
        }
    }
}

impl Color {
    fn border() -> Self {
        Color::Grey
    }
    fn random() -> Self {
        use Color::*;
        loop {
            return match random::<u8>() {
                0 => Cyan,
                1 => Blue,
                2 => Orange,
                3 => Yellow,
                4 => Lime,
                5 => Purple,
                6 => Red,
                _ => continue
            }
        }
    }
}

impl Into<[f32; 4]> for Color {
    fn into(self) -> [f32; 4] {
        use Color::*;
        match self {
            Cyan => [0.0, 1.0, 1.0, 1.0],
            Blue => [0.0, 0.0, 1.0, 1.0],
            Orange => [1.0, 0.0, 0.41, 1.0],
            Yellow => [1.0, 1.0, 0.0, 1.0],
            Lime => [0.196, 0.80, 0.196, 1.0],
            Purple => [0.33, 0.10, 0.545, 1.0],
            Red => [1.0, 0.0, 0.0, 1.0],
            Grey => [0.50, 0.50, 0.50, 1.0]
        }
    }
}

#[derive(Default, Clone, Copy)]
struct Handler;
#[derive(Clone, Copy)]
struct Tetris {
    board: [[Option<Color>; BOARD_WIDTH]; BOARD_HEIGHT],
    current: Piece,
    time: Instant
}

impl Default for Tetris {
    fn default() -> Self {
        Tetris { board: [[None; BOARD_WIDTH]; BOARD_HEIGHT],
                 current: Piece::random(),
                 time: Instant::now() }
    }
}

fn main() {
    CompositorBuilder::new().gles2(true)
                            .output_manager(Box::new(Handler))
                            .input_manager(Box::new(Handler))
                            .build_auto(Tetris::default())
                            .run()
}

impl OutputHandler for Handler {
    fn on_frame(&mut self, compositor: CompositorHandle, output: OutputHandle) {
        with_handles!([(compositor: {compositor}), (output: {output})] => {
            let tetris: &mut Tetris = compositor.data.downcast_mut().unwrap();
            let now = Instant::now();
            let delta = now.duration_since(tetris.time);
            let seconds_delta = delta.as_secs();
            let nano_delta = delta.subsec_nanos() as u64;
            let ms = (seconds_delta * 1000) + nano_delta / 1000000;
            // Every half second simulate gravity
            if ms > 500 {
                tetris.time = now;
                let next_move = tetris.current.move_down();
                // Check we don't collide.
                // If we do, add it to the board and gen next falling piece
                if collide(&tetris.board, next_move.coords()) {
                    let color = tetris.current.color();
                    for coord in tetris.current.coords().into_iter() {
                        tetris.board[coord.y as usize][coord.x as usize] = Some(color);
                    }
                    tetris.current = Piece::random()
                } else {
                    tetris.current = next_move
                }
            }
            let (x_res, y_res) = output.effective_resolution();
            let (board_start_x, board_start_y) = (x_res / 4, y_res / 4);
            let (board_end_x, board_end_y) = (x_res - x_res / 4, y_res - y_res / 4);
            let renderer = compositor.renderer.as_mut().expect("No renderer");
            let mut renderer = renderer.render(output, None);
            let transform_matrix = renderer.output.transform_matrix();
            renderer.clear([0.0, 0.0, 0.0, 1.0]);
            let scale = 4.0;
            let block_width = scale * 5.0;
            let block_height = scale * 5.0;
            // Render the border of the board
            let block_size = Size::new(block_width as i32, block_height as i32);
            for row in 0..(BOARD_WIDTH + 2) {
                for column in 0..(BOARD_HEIGHT + 2) {
                    let area = Area::new(Origin::new(board_start_x +
                                                     (block_width as i32 * row as i32)
                                                     - block_width as i32,
                                                     board_start_y +
                                                     (block_height as i32 * column as i32)
                                                     - block_height as i32),
                                         block_size);
                    renderer.render_colored_rect(area, Color::border().into(), transform_matrix);
                }
            }
            // Render the rows in the board
            let mut origin = Origin::new(board_start_x, board_start_y);
            for row in &mut tetris.board {
                origin.x = board_start_x;
                for block in row.iter_mut() {
                    let color = match *block {
                        None => continue,
                        Some(color) => color
                    };
                    let area = Area::new(origin, block_size);
                    origin.x += block_width as i32;
                    renderer.render_colored_rect(area, color.into(), transform_matrix);
                }
                origin.y += block_height as i32;
            }
            // Render the current falling piece on the board
            let current_color = tetris.current.color();
            for block in tetris.current.coords().into_iter() {
                let x = board_start_x + (block_width as i32 * block.x);
                let y = board_start_y + (block_height as i32 * block.y);
                let area = Area::new(Origin::new(x, y), block_size);
                renderer.render_colored_rect(area, current_color.into(), transform_matrix);
            }
        }).unwrap();
    }
}

impl KeyboardHandler for Handler {
    fn on_key(&mut self, compositor: CompositorHandle, keyboard: KeyboardHandle, event: &KeyEvent) {
        with_handles!([(compositor: {compositor})] => {
            if event.key_state() == WLR_KEY_PRESSED {
                for key in event.pressed_keys() {
                    if key == KEY_Escape {
                        compositor.terminate();
                        return
                    }
                }
            }
        }).unwrap();
    }
}

impl InputManagerHandler for Handler {
    fn keyboard_added(&mut self,
                      compositor: CompositorHandle,
                      keyboard: KeyboardHandle)
                      -> Option<Box<KeyboardHandler>> {
        Some(Box::new(Handler))
    }
}

impl OutputManagerHandler for Handler {
    fn output_added<'output>(&mut self,
                             compositor: CompositorHandle,
                             builder: OutputBuilder<'output>)
                             -> Option<OutputBuilderResult<'output>> {
        Some(builder.build_best_mode(Handler))
    }
}

/// Determines if the next step collides it the board with a piece
fn collide(board: &[[Option<Color>; BOARD_WIDTH]; BOARD_HEIGHT], next: [Origin; 4]) -> bool {
    for coord in next.into_iter() {
        if coord.y >= BOARD_HEIGHT as i32 {
            return true
        }
        if board[coord.y as usize][coord.x as usize].is_some() {
            return true
        }
    }
    false
}
