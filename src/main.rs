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
    current: Option<(usize, usize, usize, usize)>,
    time: Instant
}

impl Default for Tetris {
    fn default() -> Self {
        let mut board = [[None; BOARD_WIDTH]; BOARD_HEIGHT];
        for row in &mut board {
            for block in row.iter_mut() {
                *block = Some(Color::random())
            }
        }
        Tetris { board: [[None; BOARD_WIDTH]; BOARD_HEIGHT],
                 current: None,
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
            let (x_res, y_res) = output.effective_resolution();
            let (board_start_x, board_start_y) = (x_res / 4, y_res / 4);
            let (board_end_x, board_end_y) = (x_res - x_res / 4, y_res - y_res / 4);
            let tetris: &mut Tetris = compositor.data.downcast_mut().unwrap();
            let renderer = compositor.renderer.as_mut().expect("No renderer");
            let mut renderer = renderer.render(output, None);
            let transform_matrix = renderer.output.transform_matrix();
            renderer.clear([0.0, 0.0, 0.0, 1.0]);
            let scale = 4.0;
            let block_width = scale * 5.0;
            let block_height = scale * 5.0;
            let mut origin = Origin::new(board_start_x, board_start_y);
            // Render the border of the board
            for row in 0..(BOARD_WIDTH + 2) {
                for column in 0..(BOARD_HEIGHT + 2) {
                    let area = Area::new(Origin::new(board_start_x +
                                                     (block_width as i32 * row as i32)
                                                     - block_width as i32,
                                                     board_start_y +
                                                     (block_height as i32 * column as i32)
                                                     - block_height as i32),
                                         Size::new(block_width as i32, block_height as i32));
                    renderer.render_colored_rect(area, Color::border().into(), transform_matrix);
                }
            }
            // Render the rows in the board
            for row in &mut tetris.board {
                origin.x = board_start_x;
                for block in row.iter_mut() {
                    let block = Area::new(origin,
                                          Size::new(block_width as i32, block_height as i32));
                    origin.x += block_width as i32;
                    renderer.render_colored_rect(block, Color::random().into(), transform_matrix);
                }
                origin.y += block_height as i32;
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
