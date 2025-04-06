use std::os::fd::IntoRawFd;
use std::{os::fd::RawFd, process::Command, time::Duration};

use cosmic::iced::{stream, Subscription};
use cosmic::Element;
use nix::pty::{forkpty, ForkptyResult};
use nix::unistd::read;
use vte::{Params, Parser, Perform};

use crate::app::display::{DisplayBundle, DisplayCell, DisplayStyle, TerminalDisplay};
use crate::app::main::VigilMessages;




#[derive(Clone)]
pub struct Terminal<const NUM_ROW: usize, const NUM_COLUMN: usize> {
    pub read_buffer: Vec<u8>,
    pub display: TerminalDisplay<NUM_ROW, NUM_COLUMN>,
    pub stdout_fd: RawFd,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub current_style: DisplayStyle,
    pub previous_bundle_index: usize,
}


impl<const NUM_ROW: usize, const NUM_COLUMN: usize> Terminal<NUM_ROW, NUM_COLUMN> {
    pub fn init(shell: Option<String>) -> Self {
        let default_shell = shell
            .unwrap_or(
                "/nix/store/bz8zbnsaya0srmhi2k0cbx287krrmqng-nushell-0.102.0/bin/nu".to_string()
                // "/run/current-system/sw/bin/fish".to_string()
                // std::env::var("SHELL")
                    // .expect("could not find default $SHELL enviroment variable")
            );

        let stdout_fd = spawn_pty_with_shell(default_shell);
        let mut term = Terminal::<NUM_ROW, NUM_COLUMN> {
            read_buffer: Vec::new(),
            display: TerminalDisplay::new("Lilex Nerd Font".to_string(), 16.0 ),
            current_style: DisplayStyle {
                background: None,
                foreground: None,
                style_metadata: 0,
            },
            cursor_x: 0,
            cursor_y: 0,
            previous_bundle_index: 0,
            stdout_fd,
        };


        term
    }

    pub fn subscription(self) -> Subscription<VigilMessages>
    {


        Subscription::run_with_id(1, stream::channel(100, move |mut output| async move {
            tokio::task::spawn_blocking(move || {

                loop {
                    match read_from_fd(self.stdout_fd) {
                        Some(read_bytes) => {
                            // println!("more messaged to read! {:?}", read_bytes);
                            output
                                .try_send(VigilMessages::StdoutRead(read_bytes))
                                .expect("Could not send buffer message");
                            // self.read_buffer.append(&mut read_bytes)
                        },
                        None => {
                            // no more data to read
                            // println!("{:?}", String::from_utf8_lossy(&self.read_buffer.clone()));
                        }
                    }
                }
            }).await.unwrap();

            loop {
                cosmic::iced_futures::futures::pending!()
            }
        }))
    }

    pub fn make_display(&mut self) {
    // -> TerminalDisplay<NUM_ROW, NUM_COLUMN>

        println!("got make_display call!");
        let mut parser = Parser::new();
        // let mut read_buffer = [0; 65536];
        // let read_result = read(self.stdout_fd, &mut read_buffer);

        let _ = parser.advance(&mut self.clone(), &self.read_buffer);
        // let _: Option<i32> = match read_result {
        //     Ok(bytes_read) => {
        //         parser.advance(self, &read_buffer[..bytes_read]);
        //
        //         None
        //         // Some(read_buffer[..bytes_read].to_vec())
        //     },
        //     _ => None
        // };

    }

    pub fn cursor_forward(&mut self, amount: usize) {
        if self.cursor_x < NUM_COLUMN {
            self.cursor_x += 1;
        } else {
            println!("cells {:?}", self.display.cells);
            println!("y is {:?}", self.cursor_y);
            self.cursor_x = 0;
            self.cursor_y += 1;
            println!("adding cursor y in forward wrapping");
            // TODO: make scrollback from the lines that arent visible
            // self.display.cells.push(Vec::new());
            if self.cursor_y > self.display.cells.len() {
                println!("making it bigger here");
                self.display.cells.push(Vec::new());
            }
            // IMPORTANT: might have to add a check if the cursor y s too much then make it
            // increase the scrollback
        }
    }


    fn make_bundle(&self, c: char) -> DisplayBundle{
        DisplayBundle {
            characters: vec![c],
            style: self.current_style,
            character_start: self.cursor_x,
            character_end: self.cursor_x,
            unicode_positions: Vec::new(),
        }
    }
}

impl<const NUM_ROW: usize, const NUM_COLUMN: usize> Perform for Terminal<NUM_ROW, NUM_COLUMN> {
    fn print(&mut self, c: char) {
        // self.out_buffer.push(c as u8);
        // println!("got the cell thiing: {:?} with cursor y {:?}", self.display.cells, self.cursor_y);
        if let Some(cell) = self.display.cells[self.cursor_y].get_mut(self.previous_bundle_index) {
            // merge current cell to a bundle if its style matches
            if cell.style == self.current_style {
                cell.characters.push(c);
                cell.character_end = self.cursor_x;
                if !c.is_ascii() {
                    cell.unicode_positions.push(cell.characters.len() - 1);
                }
            } else {
                // make a new bundle if style does not match
                self.display.cells[self.cursor_y].push(DisplayBundle {
                    characters: vec![c],
                    style: self.current_style,
                    character_start: self.cursor_x,
                    character_end: self.cursor_x,
                    unicode_positions: if c.is_ascii() {
                        Vec::new()
                    } else {
                        vec![0]
                    },
                });
                self.previous_bundle_index = self.display.cells.len();
            }
        } else {
            self.display.cells[self.cursor_y].push(DisplayBundle {
                characters: vec![c],
                style: self.current_style,
                character_start: self.cursor_x,
                character_end: self.cursor_x,
                unicode_positions: if c.is_ascii() {
                    Vec::new()
                } else {
                    vec![0]
                },
            });
            self.previous_bundle_index = self.display.cells.len();
        }
        // self.display.cells[self.cursor_y][self.cursor_x] = DisplayCell {
        //     character: c,
        //     style: self.current_style
        // };
        self.cursor_forward(1);
        println!("[print] {:?}", c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            13 => {
                self.cursor_x = 0;
                self.cursor_y += 1;
                self.display.cells.push(Vec::new());
            },
            10 => {
                self.cursor_y += 1;
                // TODO: this wont work with scrollback i believe
                if self.cursor_y >= self.display.cells.len() {
                    self.display.cells.push(Vec::new());
                }

            },
            _ => {}
        }
        println!("[execute] {:02x}", byte);
        println!("[thing]: {:?}", self.display.cells)
    }

    fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        println!(
            "[hook] params={:?}, intermediates={:?}, ignore={:?}, char={:?}",
            params, intermediates, ignore, c
        );
    }

    fn put(&mut self, byte: u8) {
        println!("[put] {:02x}", byte);
    }

    fn unhook(&mut self) {
        println!("[unhook]");
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        println!("[osc_dispatch] params={:?} bell_terminated={}", params, bell_terminated);
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        println!(
            "[csi_dispatch] params={:#?}, intermediates={:?}, ignore={:?}, char={:?}",
            params, intermediates, ignore, c
        );

        match c {
            'm' => {
            }
            _ => {},
            // _ => panic!("csi '{}' dispatch not implemented ", c)
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        println!(
            "[esc_dispatch] intermediates={:?}, ignore={:?}, byte={:02x}",
            intermediates, ignore, byte
        );
    }
}

fn spawn_pty_with_shell(default_shell: String) -> RawFd {
    unsafe {
        match forkpty(None, None) {
            Ok(fork_result) => {
                let mut stdout_fd;
                if let ForkptyResult::Parent{ child, master  } = fork_result {
                    // primary/master part of pty
                    stdout_fd = master;
                } else {
                    // secondary/slave part of pty
                    Command::new(&default_shell)
                        .spawn()
                        .expect("failed to spawn");
                    std::thread::sleep(Duration::from_millis(2000));
                    std::process::exit(0);
                }
                stdout_fd.into_raw_fd()
            },
            Err(e) => panic!("failed to fork {:?}", e),

        }

    }
}

fn read_from_fd(fd: RawFd) -> Option<Vec<u8>> {
    let mut read_buffer = [0; 65536];
    let read_result = read(fd, &mut read_buffer);

    match read_result {
        Ok(bytes_read) => Some(read_buffer[..bytes_read].to_vec()),
        _ => None
    }
}


impl<Message, const NUM_ROW: usize, const NUM_COLUMN: usize> From<TerminalDisplay<NUM_ROW, NUM_COLUMN>> for Element<'_, Message>
where
    Message: Clone,
{
    fn from(terminal_box: TerminalDisplay<NUM_ROW, NUM_COLUMN>) -> Self {
        Self::new(terminal_box)
    }
}
