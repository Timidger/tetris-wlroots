#![allow(non_upper_case_globals)]

extern crate rand;
#[macro_use]
extern crate wlroots;

use rand::random;
use std::time::Instant;

use wlroots::{Area, CompositorBuilder, CompositorHandle, InputManagerHandler, KeyboardHandle,
              KeyboardHandler, Origin, OutputBuilder, OutputBuilderResult, OutputHandle,
              OutputHandler, OutputManagerHandler, Size, key_events::KeyEvent,
              xkbcommon::xkb::{KEY_Down, KEY_Escape, KEY_Left, KEY_Right, KEY_x, KEY_z},
              WLR_KEY_PRESSED};

compositor_data!(Tetris);

const BOARD_WIDTH: usize = 10;
const BOARD_HEIGHT: usize = 20;
const BOARD_WIDTH_EDGE: usize = BOARD_WIDTH + 1;
const BOARD_HEIGHT_EDGE: usize = BOARD_HEIGHT + 1;

#[derive(Debug, Clone, Copy)]
enum Color {
    Blue,
    Purple,
    Orange,
    Yellow,
    Red,
    Grey,
    DarkGrey,
    Green,
    Pink
}

#[derive(Default, Debug, Clone, Copy)]
struct PieceData(Origin, Origin, Origin, Origin);

#[derive(Clone, Copy)]
enum PieceType {
    Block,
    L,
    I,
    J,
    T,
    S,
    Z
}

impl PieceType {
    fn origin(self) -> PieceData {
        use PieceType::*;
        match self {
            S => {
                let a = Origin::new(0, 1);
                let b = Origin::new(1, 1);
                let c = Origin::new(1, 0);
                let d = Origin::new(2, 0);
                PieceData(a, b, c, d)

            }
            Block => {
                let a = Origin::new(0, 0);
                let b = Origin::new(1, 0);
                let c = Origin::new(0, 1);
                let d = Origin::new(1, 1);
                PieceData(a, b, c, d)

            },
            L => {
                let a = Origin::new(0, 0);
                let b = Origin::new(0, 1);
                let c = Origin::new(0, 2);
                let d = Origin::new(1, 2);
                PieceData(a, b, c, d)
            },
            I => {
                let a = Origin::new(0, 0);
                let b = Origin::new(0, 1);
                let c = Origin::new(0, 2);
                let d = Origin::new(0, 3);
                PieceData(a, b, c, d)
            },
            J => {
                let a = Origin::new(1, 0);
                let b = Origin::new(1, 1);
                let c = Origin::new(1, 2);
                let d = Origin::new(0, 2);
                PieceData(a, b, c, d)
            },
            T => {
                let a = Origin::new(0, 1);
                let b = Origin::new(1, 1);
                let c = Origin::new(1, 0);
                let d = Origin::new(2, 1);
                PieceData(a, b, c, d)
            },
            Z => {
                let a = Origin::new(0, 0);
                let b = Origin::new(1, 0);
                let c = Origin::new(1, 1);
                let d = Origin::new(2, 1);
                PieceData(a, b, c, d)
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Piece {
    data: PieceData,
    x_offset: i32,
    y_offset: i32,
    ty: PieceType
}

impl Piece {
    fn random() -> Self {
        use PieceType::*;
        let ty = loop {
            match random::<u8>() {
                0 => break L,
                1 => break Block,
                2 => break I,
                3 => break J,
                4 => break T,
                5 => break S,
                6 => break Z,
                _ => continue
            };
        };
        Piece { ty, x_offset: 0, y_offset: 0, data: ty.origin() }
    }

    /// Simulate moving a piece down
    fn move_down(mut self) -> Self {
        self.y_offset += 1;
        self
    }

    fn move_right(mut self) -> Self {
        self.x_offset += 1;
        self
    }

    fn move_left(mut self) -> Self {
        self.x_offset -= 1;
        self
    }

    fn rotate(mut self, dir: Dir) -> Self {
        {
            let mut data = [&mut self.data.0,
                        &mut self.data.1,
                        &mut self.data.2,
                        &mut self.data.3];
            match dir {
                Dir::Right => {
                    for ref mut d in data.iter_mut() {
                        let temp = -d.x;
                        d.x = d.y;
                        d.y = temp;
                    }
                },
                Dir::Left => {
                    for ref mut d in data.iter_mut() {
                        let temp = -d.y;
                        d.y = d.x;
                        d.x = temp;
                    }
                }
            }
        }
        self
    }

    fn color(self) -> Color {
        use Color::*;
        use PieceType::*;
        match self.ty {
            Block => Blue,
            L => Orange,
            I => Pink,
            J => Red,
            T => Yellow,
            S => Purple,
            Z => Green
        }
    }

    /// Get an iterator over the grid-level coordinates.
    fn coords(self) -> [Origin; 4] {
        let mut res = [self.data.0,
                       self.data.1,
                       self.data.2,
                       self.data.3];
        for origin in &mut res {
            origin.x += self.x_offset;
            origin.y += self.y_offset;
        }
        res
    }
}

impl Color {
    fn border() -> Self {
        Color::Grey
    }

    fn background() -> Self {
        Color::DarkGrey
    }
}

impl Into<[f32; 4]> for Color {
    fn into(self) -> [f32; 4] {
        use Color::*;
        match self {
            Blue => [0.0, 0.0, 1.0, 1.0],
            Orange => [1.0, 0.41, 0.0, 1.0],
            Yellow => [1.0, 1.0, 0.0, 1.0],
            Red => [1.0, 0.0, 0.0, 1.0],
            Green => [0.0, 1.0, 0.0, 1.0],
            Pink => [1.0, 0.4117, 0.713, 1.0],
            Purple => [0.9333, 0.50980, 0.9333, 1.0],
            Grey => [0.50, 0.50, 0.50, 1.0],
            DarkGrey => [0.25, 0.25, 0.25, 1.0],
        }
    }
}

#[derive(Clone, Copy)]
enum Dir {
    Left,
    Right
}

#[derive(Default, Clone, Copy)]
struct Handler;

#[derive(Clone)]
struct Tetris {
    board: [[Option<Color>; BOARD_WIDTH]; BOARD_HEIGHT],
    current: Piece,
    time: Instant,
    down: bool
}

impl Default for Tetris {
    fn default() -> Self {
        Tetris { board: [[None; BOARD_WIDTH]; BOARD_HEIGHT],
                 current: Piece::random(),
                 time: Instant::now(),
                 down: false }
    }
}

impl Tetris {
    /// Attempts to move the current piece in the given direction.
    ///
    /// If it would be blocked, then it will not change.
    fn move_dir(&mut self, dir: Dir) {
        let next_move = match dir {
            Dir::Left => self.current.move_left(),
            Dir::Right => self.current.move_right()
        };
        if self.collide(next_move.coords()) {
            return
        }
        self.current = next_move
    }

    /// Attempt to rotate the current piece in the given direction.
    ///
    /// If it would be blocked, then it will not change.
    fn rotate(&mut self, dir: Dir) {
        let next_move = self.current.rotate(dir);
        if self.collide(next_move.coords()) {
            return
        }
        self.current = next_move
    }

    /// Determines if the next step collides it the board with a piece
    fn collide(&self, next: [Origin; 4]) -> bool {
        for coord in next.into_iter() {
            if coord.y >= BOARD_HEIGHT as i32 || coord.y < 0 {
                return true
            }
            if coord.x >= BOARD_WIDTH as i32 || coord.x < 0 {
                return true
            }
            if self.board[coord.y as usize][coord.x as usize].is_some() {
                return true
            }
        }
        false
    }

    /// Clear any full rows that exist
    fn clear_full_rows(&mut self) {
        let mut rows = vec![];
        'row_check: for (index, row) in self.board.iter_mut().enumerate() {
            for block in row.iter_mut() {
                if block.is_none() {
                    continue 'row_check
                }
            }
            rows.push(index);
            *row = [None; BOARD_WIDTH]
        }
        for row_index in rows {
            let mut prev_index = row_index;
            for above_index in (0..(row_index + 1)).rev() {
                self.board[prev_index] = self.board[above_index];
                prev_index = above_index;
            }
        }
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
            if ms > 500 || tetris.down {
                tetris.down = false;
                tetris.time = now;
                let next_move = tetris.current.move_down();
                // Check we don't collide.
                // If we do, add it to the board and gen next falling piece
                if tetris.collide(next_move.coords()) {
                    let color = tetris.current.color();
                    for coord in tetris.current.coords().into_iter() {
                        tetris.board[coord.y as usize][coord.x as usize] = Some(color);
                    }
                    tetris.current = Piece::random()
                } else {
                    tetris.current = next_move
                }
                tetris.clear_full_rows()
            }
            let (x_res, y_res) = output.effective_resolution();
            let (board_start_x, board_start_y) = (x_res / 4, y_res / 4);
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
                    let color = match (row, column) {
                        (0, _) |
                        (_, 0) |
                        (BOARD_WIDTH_EDGE, _) |
                        (_, BOARD_HEIGHT_EDGE) => Color::border(),
                        (_, _) => Color::background()
                    };
                    let mut area = Area::new(Origin::new(board_start_x +
                                                     (block_width as i32 * row as i32)
                                                     - block_width as i32,
                                                     board_start_y +
                                                     (block_height as i32 * column as i32)
                                                     - block_height as i32),
                                         block_size);
                    let mut inner_box = area;
                    inner_box.size.width -= block_width as i32 / 8;
                    inner_box.origin.x -= block_width as i32 / 8;
                    inner_box.size.height -= block_height as i32 / 8;
                    inner_box.origin.y -= block_height as i32 / 8;
                    renderer.render_scissor(inner_box);
                    renderer.render_colored_rect(area, color.into(), transform_matrix);
                    renderer.render_scissor(None);
                }
            }
            // Render the rows in the board
            let mut origin = Origin::new(board_start_x, board_start_y);
            for row in &mut tetris.board {
                origin.x = board_start_x;
                for block in row.iter_mut() {
                    let color = match *block {
                        None => {
                            origin.x += block_width as i32;
                            continue
                        },
                        Some(color) => color
                    };
                    let area = Area::new(origin, block_size);
                    let mut inner_box = area;
                    inner_box.size.width -= block_width as i32 / 8;
                    inner_box.origin.x -= block_width as i32 / 8;
                    inner_box.size.height -= block_height as i32 / 8;
                    inner_box.origin.y -= block_height as i32 / 8;
                    renderer.render_scissor(inner_box);
                    origin.x += block_width as i32;
                    renderer.render_colored_rect(area, color.into(), transform_matrix);
                    renderer.render_scissor(None);
                }
                origin.y += block_height as i32;
            }
            // Render the current falling piece on the board
            let current_color = tetris.current.color();
            for block in tetris.current.coords().into_iter() {
                let x = board_start_x + (block_width as i32 * block.x);
                let y = board_start_y + (block_height as i32 * block.y);
                let area = Area::new(Origin::new(x, y), block_size);
                let mut inner_box = area;
                inner_box.size.width -= block_width as i32 / 8;
                inner_box.origin.x -= block_width as i32 / 8;
                inner_box.size.height -= block_height as i32 / 8;
                inner_box.origin.y -= block_height as i32 / 8;
                renderer.render_scissor(inner_box);
                renderer.render_colored_rect(area, current_color.into(), transform_matrix);
                renderer.render_scissor(None);
            }
        }).unwrap();
    }
}

impl KeyboardHandler for Handler {
    fn on_key(&mut self, compositor: CompositorHandle, _: KeyboardHandle, event: &KeyEvent) {
        with_handles!([(compositor: {compositor})] => {
            let tetris: &mut Tetris = compositor.into();
            if event.key_state() == WLR_KEY_PRESSED {
                for key in event.pressed_keys() {
                    match key {
                        KEY_Escape => wlroots::terminate(),
                        KEY_Down => {
                            let mut prev_move = tetris.current;
                            let mut next_move = tetris.current.move_down();
                            while !tetris.collide(next_move.coords()) {
                                prev_move = next_move;
                                next_move = next_move.move_down();
                            }
                            tetris.current = prev_move;
                            tetris.down = true;
                        },
                        KEY_Left => tetris.move_dir(Dir::Left),
                        KEY_Right => tetris.move_dir(Dir::Right),
                        KEY_z => tetris.rotate(Dir::Left),
                        KEY_x => tetris.rotate(Dir::Right),
                        _ => {}
                    }
                }
            }
        }).unwrap();
    }
}

impl InputManagerHandler for Handler {
    fn keyboard_added(&mut self,
                      _: CompositorHandle,
                      _: KeyboardHandle)
                      -> Option<Box<KeyboardHandler>> {
        Some(Box::new(Handler))
    }
}

impl OutputManagerHandler for Handler {
    fn output_added<'output>(&mut self,
                             _: CompositorHandle,
                             builder: OutputBuilder<'output>)
                             -> Option<OutputBuilderResult<'output>> {
        Some(builder.build_best_mode(Handler))
    }
}
