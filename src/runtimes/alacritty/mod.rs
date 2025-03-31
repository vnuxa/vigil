use std::{collections::HashMap, path::PathBuf};

use alacritty_terminal::{event::WindowSize, grid::Dimensions, term::Config, tty::{self, Options, Pty, Shell}, Term};
use event::EventProxy;

mod event;

pub struct Terminal {
    pty: Pty,
    terminal: Term<EventProxy>,
    options: TerminalOptions
}

pub struct TerminalOptions {
    pub size: WindowSize,
    pub id: u64,
    pub options: Options,

    pub scrolling_history: usize
}


impl Terminal {
    pub fn new(options: TerminalOptions) -> Self {
        let pty = tty::new(&options.options, options.size, options.id);
        let terminal = Term::new(
            Config {
                scrolling_history: options.scrolling_history,
                kitty_keyboard: true,
                ..Default::default()
            },
            &Size {
                cell_width: options.size.cell_width as f32,
                cell_height: options.size.cell_height as f32,
                height: (options.size.cell_height * options.size.num_lines) as u32,
                width: (options.size.cell_width* options.size.num_cols) as u32,
            },
            EventProxy {}
        );

        Self {
            pty: pty.expect("Expected pseudo terminal"),
            terminal,
            options
        }
    }

    pub fn buffer(&self) -> HashMap<i32, String> {
        let mut output: HashMap<i32, String> = HashMap::new();
        let grid = self.terminal.grid();
        for indexed in grid.display_iter() {
            if let Some(line) = output.get_mut(&indexed.point.line.0) {
                line.push(indexed.cell.c);
            } else {
                output.insert(indexed.point.line.0, indexed.cell.c.to_string());
            }
        }

        output
    }

}

struct Size {
    pub width: u32,
    pub height: u32,
    pub cell_width: f32,
    pub cell_height: f32,
}

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }

    fn screen_lines(&self) -> usize {
        ((self.height as f32) / self.cell_height).floor() as usize
    }

    fn columns(&self) -> usize {
        ((self.width as f32) / self.cell_width).floor() as usize
    }
}
