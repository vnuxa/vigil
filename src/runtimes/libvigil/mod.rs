use std::cell::RefMut;
use std::fs::File;
use std::io::{IsTerminal, Read, Write};
use std::num::NonZeroUsize;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::task::Waker;
use std::{thread, u8};
// use std::os::unix::net::{UnixListener, UnixStream};
use std::{os::fd::RawFd, process::Command, time::Duration};

use cosmic::iced::futures::StreamExt;
use cosmic::iced::{futures, stream, Subscription};
use cosmic::Element;
use lazy_static::lazy_static;
use nix::fcntl::FcntlArg::{F_GETFL, F_SETFL};
use nix::fcntl::{self, OFlag};
use nix::libc::{
    ioctl, signal, O_NONBLOCK, SIGALRM, SIGCHLD, SIGINT, SIGQUIT, SIGTERM, SIG_DFL, TIOCSCTTY,
};
use nix::poll::{self, PollFd, PollFlags, PollTimeout};
use nix::pty::{forkpty, openpty, ForkptyResult};
use nix::sys::socket::MsgFlags;
use nix::sys::termios::{tcgetattr, tcsetattr, InputFlags, SetArg};
use nix::unistd::{close, read, setsid, write};
use polling::{Events, Poller};
use signal_hook::SigId;
use tokio::net::{UnixListener as TokioListener, UnixSocket as TokioSocket};
use tokio::task::spawn_blocking;
use vte::{Params, Parser, Perform};

use crate::app::display::{DisplayBundle, DisplayCell, DisplayStyle, TerminalDisplay};
use crate::app::main::VigilMessages;

lazy_static! {
    static ref WRITE_LIST: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
}

pub struct Terminal<const NUM_ROW: usize, const NUM_COLUMN: usize> {
    pub read_buffer: Vec<u8>,
    pub display: TerminalDisplay<VigilMessages>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub current_style: Option<DisplayStyle>,
    pub previous_bundle_index: usize,
    pub stdout_stream: UnixStream,
    pub stdin_sender: UnixStream,
    pub display_start: usize,
    pub wrapping: bool,
    // pub master_fd: RawFd,
}

impl<const NUM_ROW: usize, const NUM_COLUMN: usize> Terminal<NUM_ROW, NUM_COLUMN> {
    pub fn init(shell: Option<(String, Vec<String>)>) -> Self {
        // IMPORTANT: remove this once done testing
        let default_shell = shell.unwrap_or(
            (
                // std::env::var("SHELL").unwrap(),
                "/run/current-system/sw/bin/fish".to_string(),
                // "/nix/store/bz8zbnsaya0srmhi2k0cbx287krrmqng-nushell-0.102.0/bin/nu".to_string(),
                Vec::new(),
            ),
            // std::env::var("SHELL")
        );
        // TODO: once removed the default shell for testing,
        let pty = Pty::new(Some(default_shell));
        let (stdout_stream, stdin_sender) = pty.read_io();

        // let stdout_fd = spawn_pty_with_shell(default_shell);
        Terminal::<NUM_ROW, NUM_COLUMN> {
            read_buffer: Vec::new(),
            display: TerminalDisplay::new(
                "Lilex Nerd Font".to_string(),
                // "uushi".to_string(),
                16.0,
                Box::new(VigilMessages::StdinInput),
                Box::new(VigilMessages::MouseScroll),
                NUM_ROW,
            ),
            current_style: None,
            cursor_x: 0,
            cursor_y: 0,
            previous_bundle_index: 0,
            display_start: 0,
            wrapping: true,
            // master_fd: pty.file,
            stdout_stream,
            stdin_sender, // make it of type shell
        }
    }

    pub fn cursor_forward(&mut self, amount: usize) {
        if self.cursor_x < NUM_COLUMN || !self.wrapping {
            self.cursor_x += 1;
        } else {
            // println!("cells {:?}", self.display.cells);
            println!("y is {:?}", self.cursor_y);
            self.cursor_x = 0;
            self.cursor_y += 1;
            println!("adding cursor y in forward wrapping");
            // TODO: make scrollback from the lines that arent visible
            // self.display.cells.push(Vec::new());
            // IMPORTANT: might have to add a check if the cursor y s too much then make it
            // increase the scrollback
        }
    }
}

impl<const NUM_ROW: usize, const NUM_COLUMN: usize> Perform for Terminal<NUM_ROW, NUM_COLUMN> {
    fn print(&mut self, c: char) {
        // println!("adding char {:?}", c);
        let mut cell;
        if let Some(obj_cell) = self.display.cells.get_mut(self.cursor_y) {
            cell = obj_cell;
        } else {
            if self.display.cells.len() < self.cursor_y + 1 {
                self.display.cells.resize(self.cursor_y + 1, Vec::new());
            }
            println!("using the non exist method {:?}", self.cursor_y);

            // self.display.cells[self.cursor_y] = Vec::with_capacity(NUM_COLUMN);
            cell = &mut self.display.cells[self.cursor_y];
        }

        // FIX: right now, you cannot add a cell at a specific index
        // big issue if you move cursor left 10 times and want to insert characters there
        if cell.len() < self.cursor_x + 1 {
            cell.resize(
                self.cursor_x + 1,
                DisplayCell {
                    character: ' ',
                    style: None,
                },
            );
        }
        // println!("cursor x is: {:?}", self.cursor_x);
        cell[self.cursor_x] = DisplayCell {
            character: c,
            style: self.current_style,
        };

        self.cursor_forward(1);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            // carriage return
            13 => {
                self.cursor_x = 0;
                self.cursor_y += 1;
                self.display.cells.push(Vec::with_capacity(NUM_COLUMN));
            }
            10 => {
                // self.cursor_y += 1;
                self.cursor_forward(1);

                // TODO: this wont work with scrollback i believe
                if self.cursor_y >= self.display.cells.len() {
                    self.display.cells.push(Vec::with_capacity(NUM_COLUMN));
                }
            }
            _ => {}
        }
        if byte != 00 {
            println!("[execute] {:02x} which is also {:?}", byte, byte);
        }
        // println!("[execute] {:02x}", byte);
        // println!("[thing]: {:?}", self.display.cells)
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
        println!(
            "[osc_dispatch] params={:?} bell_terminated={}",
            params, bell_terminated
        );
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        println!(
            "[csi_dispatch] params={:#?}, intermediates={:?}, ignore={:?}, char={:?}",
            params, intermediates, ignore, c
        );
        let mut params_iter = params.iter();
        let mut next_param_or = |default: u16| match params_iter.next() {
            Some(&[param, ..]) if param != 0 => param,
            _ => default,
        };

        match c {
            'm' => {}
            'J' => match next_param_or(0) {
                // clear screen from cursor to end
                0 => {
                    println!("clearing screen from cursot to end");
                    // let mut index = 0;
                    // for bundle in &mut self.display.cells[self.cursor_y] {
                    //     if bundle.character_start >= self.cursor_x
                    //         && bundle.character_end <= self.cursor_x
                    //     {
                    //         bundle.character_end = self.cursor_x;
                    //         bundle
                    //             .characters
                    //             .truncate(bundle.character_start - self.cursor_x);
                    //         index += 2;
                    //         break;
                    //     }
                    //     index += 1;
                    // }
                    self.display.cells[self.cursor_y].truncate(self.cursor_x);
                    self.display.cells.truncate(self.cursor_y + 1);
                }
                // clear screen from cursor to beggining
                1 => {
                    println!("clearing screen from cursot to beggining");
                    // let mut index = 0;
                    // for bundle in &mut self.display.cells[self.cursor_y] {
                    //     if bundle.character_start >= self.cursor_x
                    //         && bundle.character_end <= self.cursor_x
                    //     {
                    //         bundle.character_start = self.cursor_x;
                    //         bundle.characters = bundle.characters[self.cursor_x..].to_vec();
                    //         index += 2;
                    //         break;
                    //     }
                    //     index += 1;
                    // }
                    self.display.cells[self.cursor_y] =
                        self.display.cells[self.cursor_y][self.cursor_x..].to_vec();

                    self.display.cells = self.display.cells[..self.cursor_y + 1].to_vec();
                }
                //clear entire screen
                2 => {
                    self.display.cells = Vec::with_capacity(NUM_ROW);
                }
                // clear everything
                3 => {
                    self.display.cells = Vec::with_capacity(NUM_ROW);
                    self.cursor_x = 0;
                    self.cursor_y = 0;
                }
                _ => unimplemented!(),
            },
            'K' => match next_param_or(0) {
                // clear from cursor to end
                0 => {
                    println!("clearing from cursot to end");
                    // let mut index = 0;
                    // for bundle in &mut self.display.cells[self.cursor_y] {
                    //     if bundle.character_start >= self.cursor_x
                    //         && bundle.character_end <= self.cursor_x
                    //     {
                    //         bundle.character_end = self.cursor_x;
                    //         bundle
                    //             .characters
                    //             .truncate(bundle.character_start - self.cursor_x);
                    //         index += 2;
                    //         break;
                    //     }
                    //     index += 1;
                    // }
                    self.display.cells[self.cursor_y].truncate(self.cursor_x);
                }
                // clear from cursor to beggining
                1 => {
                    println!("clearing from cursot to beggining");
                    // let mut index = 0;
                    // for bundle in &mut self.display.cells[self.cursor_y] {
                    //     if bundle.character_start >= self.cursor_x {
                    //         if bundle.character_end <= self.cursor_x {
                    //             bundle.character_start = self.cursor_x;
                    //             bundle.characters = bundle.characters[self.cursor_x..].to_vec();
                    //         } else {
                    //             index += 1;
                    //             break;
                    //         }
                    //     }
                    //     index += 1;
                    // }
                    self.display.cells[self.cursor_y] =
                        self.display.cells[self.cursor_y][self.cursor_x..].to_vec();
                }
                //clear entire line
                2 => {
                    self.display.cells[self.cursor_y] = Vec::with_capacity(NUM_COLUMN);
                }
                // clear everything
                3 => {
                    self.display.cells = Vec::with_capacity(NUM_ROW);
                }
                _ => unimplemented!(),
            },
            // move cursor forward by n
            'C' => {
                let amount = next_param_or(1);
                println!("requested to move forward by: {:?}", amount);
                // if (self.cursor_x + amount as usize) > NUM_COLUMN {
                //     // println!(
                //     //     "!!!!!!!! its too much: {:?} + {:?} = {:?} which is also < {:?}",
                //     //     self.cursor_x,
                //     //     amount,
                //     //     self.cursor_x + amount as usize,
                //     //     NUM_COLUMN
                //     // );
                //     self.cursor_x = NUM_COLUMN;
                // } else {
                //     self.cursor_x += amount as usize;
                // }
                self.cursor_x = std::cmp::min(self.cursor_x + amount as usize, NUM_COLUMN);
            }
            // move cursor backward by n
            'D' => {
                let amount = next_param_or(1);
                self.cursor_x = std::cmp::max(self.cursor_x - amount as usize, 0);
                // if self.cursor_x - amount as usize > 0 {
                //     self.cursor_x -= amount as usize;
                // } else {
                //     self.cursor_x = 0;
                // }
            }
            // move cursor up by n
            'A' => {
                let amount = next_param_or(1);
                self.cursor_y = std::cmp::max(self.cursor_y - amount as usize, 0);
                // if self.cursor_y - amount as usize > 0 {
                //     self.cursor_y -= amount as usize;
                // } else {
                //     self.cursor_y = 0;
                // }
            }
            // move cursor down by n
            'B' => {
                let amount = next_param_or(1);
                self.cursor_y += amount as usize;
                // if (self.cursor_y + amount as usize) < NUM_ROW {
                //     self.cursor_y = NUM_ROW;
                // } else {
                //     self.cursor_y += amount as usize;
                // }
            }
            // move cursor to begging of line downward
            'E' => {
                let amount = next_param_or(1);
                self.cursor_y += amount as usize;
                self.cursor_x = 0;
            }
            // move cursor to begging of line upward
            'F' => {
                let amount = next_param_or(1);
                if self.cursor_y != 0 {
                    self.cursor_y -= amount as usize;
                    self.cursor_x = 0;
                }
            }
            //moves to specific column
            'G' => {
                let amount = next_param_or(1);
                // TODO: might need to make it os that this will wrap
                // see alacritty implementation on how to do it i suppose
                self.cursor_x = std::cmp::min(amount as usize, NUM_COLUMN);
                //if amount as usize > NUM_COLUMN {
                //self.cursor_x = NUM_COLUMN;
                //} else {
                //self.cursor_x = amount as usize;
                //}
            }
            // enable alternate buffer
            'l' => match next_param_or(0) {
                7 => {
                    self.wrapping = false;
                }
                _ => {}
            },
            'h' => match next_param_or(0) {
                1049 => {
                    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!! requested alternate screen buffer");
                    // panic!("no more");
                }
                2004 => {
                    // TODO: implement bracketed paste when you have clipboard pasting working
                }
                7 => {
                    self.wrapping = true;
                    // panic!("got 7 restore thingy ");
                }
                _ => {}
            },
            _ => {} // _ => panic!("csi '{}' dispatch not implemented ", c)
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
                let stdout_fd;
                println!("got forkpty result: {:?}", fork_result);
                println!("getting a stdout of {:?}", std::io::stdout().as_raw_fd());
                if let ForkptyResult::Parent { child: _, master } = fork_result {
                    // primary/master part of pty
                    println!("and it should be compared to {:?}", master);
                    stdout_fd = master;
                } else {
                    println!("unfortunately it is of child");
                    // secondary/slave part of pty
                    Command::new(&default_shell)
                        .spawn()
                        .expect("failed to spawn");
                    // std::thread::sleep(Duration::from_millis(2000));
                    println!("exiting?");
                    std::process::exit(0);
                }
                stdout_fd.into_raw_fd()
            }
            Err(e) => panic!("failed to fork {:?}", e),
        }
    }
}

pub struct Pty {
    pub child: Child,
    pub file: File,
    pub signal: UnixStream,
    pub signal_id: SigId,
}

impl Pty {
    fn new(default_shell: Option<(String, Vec<String>)>) -> Self {
        let pty = openpty(None, None).unwrap(); // TODO: make winsize argument not be a none
        let master_fd = pty.master.as_raw_fd();
        let slave_fd = pty.slave.as_raw_fd();

        if let Ok(mut termios) = tcgetattr(&pty.master) {
            termios.input_flags.set(InputFlags::IUTF8, true);

            let _ = tcsetattr(&pty.master, SetArg::TCSANOW, &termios);
        }

        let mut builder = if let Some((shell, args)) = default_shell {
            let mut command = Command::new(shell);
            command.args(args);

            command
        } else {
            Command::new(std::env::var("SHELL").expect("Could not find a default shell to use"))
        };

        builder.stdin(pty.slave.try_clone().unwrap());
        builder.stderr(pty.slave.try_clone().unwrap());
        builder.stdout(pty.slave);

        // TODO: set up shell enviroment based on a config
        // example: https://github.com/alacritty/alacritty/blob/15f1278d695776860ebcd939d30604b253788278/alacritty_terminal/src/tty/unix.rs#L230

        // builder.env("TERM", "dumb");
        builder.env_remove("XDG_ACTIVATION_TOKEN");
        builder.env_remove("DESKTOP_STARTUP_ID");

        unsafe {
            builder.pre_exec(move || {
                // create a new process group
                setsid().expect("Failed to set session id");

                // TODO: set working directory based on config
                // if let Some() =  {
                //
                // }

                set_controlling_terminal(slave_fd.as_raw_fd());

                // remove fds as we do not need the manymore
                let _ = close(slave_fd.into_raw_fd());
                let _ = close(master_fd.into_raw_fd());

                signal(SIGCHLD, SIG_DFL);
                signal(SIGINT, SIG_DFL);
                signal(SIGQUIT, SIG_DFL);
                signal(SIGTERM, SIG_DFL);
                signal(SIGALRM, SIG_DFL);

                Ok(())
            });
        }

        let (signal, signal_id) = {
            let (sender, reciever) = UnixStream::pair().unwrap();

            let signal_id =
                signal_hook::low_level::pipe::register(signal_hook::consts::SIGCHLD, sender)
                    .unwrap();
            let _ = reciever.set_nonblocking(true);

            (reciever, signal_id)
        };

        match builder.spawn() {
            Ok(child) => {
                // let flags = fcntl::fcntl(master_fd.as_raw_fd(), F_GETFL).unwrap();
                // println!("got flags: {:?}", flags);
                // let _ = fcntl::fcntl(
                //     master_fd.as_raw_fd(),
                //     F_SETFL(OFlag::from_bits(flags).expect("wow")),
                // )
                // .unwrap();
                // let _ = fcntl::fcntl(
                //     master_fd.as_raw_fd(),
                //     F_SETFL(
                //         OFlag::from_bits( fcntl::fcntl(
                //             master_fd.as_raw_fd(), F_GETFL
                //         ).expect("epxected 1") | O_NONBLOCK).expect("expected 2")
                //     )
                //
                // );

                Pty {
                    child,
                    file: unsafe { File::from_raw_fd(pty.master.into_raw_fd()) },
                    signal,
                    signal_id,
                }
            }
            Err(err) => {
                panic!("Could not spawn terminal child: {:?}", err)
            }
        }
    }

    fn read_io(mut self) -> (UnixStream, UnixStream) {
        // sender/reciever for stdout
        let (mut out_sender, out_reciever) = UnixStream::pair().unwrap();
        // sender/reciever for stdin
        let (in_sender, mut in_reciever) = UnixStream::pair().unwrap();
        println!("update buffer was called");
        // let _ = self.parser.advance(self, &mut self.read_buffer);
        // let _: Option<i32> = match read_result {
        //     Ok(bytes_read) => {
        //
        //         None
        //         // Some(read_buffer[..bytes_read].to_vec())
        //     },
        //     _ => None
        // };
        // let mut parser = Parser::new();

        thread::spawn(move || {
            // let mut buf  = []
            // futures
            let poller = Poller::new().unwrap();

            // use polling here instead of loop

            let mut polling_interest = polling::Event::readable(0);
            unsafe {
                poller
                    .add_with_mode(
                        &self.file,
                        // polling::Event::readable(0),
                        polling_interest,
                        polling::PollMode::Level,
                    )
                    .unwrap();
                poller
                    .add_with_mode(
                        &in_reciever,
                        polling::Event::readable(1),
                        polling::PollMode::Level,
                    )
                    .unwrap();
            }

            let mut events = Events::with_capacity(NonZeroUsize::new(1024).unwrap());
            let mut buffer = [0];

            loop {
                events.clear();
                let _ = poller.wait(&mut events, None).unwrap();

                for event in events.iter() {
                    match event.key {
                        0 => {
                            if event.readable {
                                match read_from_fd(self.file.as_raw_fd()) {
                                    Some(read_bytes) => {
                                        println!("read bytes {:?}", read_bytes);
                                        println!(
                                            "to convert those bytes to string: {:?}",
                                            String::from_utf8(read_bytes.clone())
                                                .unwrap_or_default()
                                                .replace("\0", "")
                                        );
                                        out_sender.write(&read_bytes).unwrap();
                                    }
                                    None => {
                                        println!("no more to read");
                                    }
                                }
                                // sender.write_all(&read_bytes).unwrap();
                                println!("readable!!");
                            } else {
                                println!("no reading..");
                            }

                            if event.writable {
                                // let mut buffer =readable String::new();
                                // let _ = in_reciever.read_to_string(&mut buffer);
                                println!("writable!");
                                // println!("this writable's readability is: {}", event.readable);
                                let res = self.file.write(&buffer);
                                println!("the result for writable: {:?}", res);
                                // self.write(&buffer);
                                polling_interest.writable = false;
                                let _ = poller.modify_with_mode(
                                    &self.file,
                                    polling_interest,
                                    polling::PollMode::Level,
                                );
                            }
                        }
                        1 => {
                            println!("write list");
                            if event.readable {
                                println!("i see its readable");
                                // let mut buffer = Vec::new();
                                let _ = in_reciever.read(&mut buffer).unwrap();
                                println!("i have obtained: {:?}", buffer);

                                polling_interest.writable = true;
                                let _ = poller.modify_with_mode(
                                    &self.file,
                                    polling_interest,
                                    polling::PollMode::Level,
                                );
                            }
                        }
                        _ => {
                            println!("hey i got some weird key: {:?}", event.key);
                        } // add read write stuff here, and make the thing that sends to main thread
                          // to update
                    }
                }

                // old
                // println!("repeating read fd");
                // match read_from_fd(self.file) {
                //     Some(read_bytes) => {
                //         // println!("more messaged to read! {:?}", read_bytes);
                //         // parser.advance(self, &read_bytes);
                //         println!("read bytes {:?}", read_bytes);
                //         sender.write_all(&read_bytes).unwrap();
                //         // self.read_buffer.append(&mut read_bytes);
                //     }
                //     None => {
                //         // no more data to read
                //         println!(
                //             "no more data to read ",
                //             // String::from_utf8_lossy(&self.read_buffer.clone())
                //         );
                //         panic!("no more data to read?");
                //         // break;
                //     }
                // }
            }
        });

        (out_reciever, in_sender)
    }

    // pub fn write(&self, buffer: &[u8]) -> usize {
    //     unsafe { write(self.file, buffer).unwrap() }
    // }
}

pub fn make_io_subscription(mut stream_stdout: UnixStream) -> Subscription<VigilMessages> {
    Subscription::run_with_id(
        1,
        stream::channel(100, move |mut output| async move {
            spawn_blocking(move || loop {
                let mut buf = [0; 65536];
                // let mut stdout = stream_stdout;
                // if let Err(msg) = stream_stdout.read(&mut buf) {
                //     println!("needs to wait?");
                // }
                let read = stream_stdout.read(&mut buf);

                println!("hey i reead: {:?}", read);

                output
                    .try_send(VigilMessages::StdoutRead(buf.to_vec()))
                    .unwrap();

                // println!("got msg");
            });
            println!("thread after!!");

            loop {
                cosmic::iced_futures::futures::pending!();
            }
        }),
    )
}

fn set_controlling_terminal(fd: RawFd) {
    let res = unsafe {
        // TIOSCTTY changes based on platform and the `ioctl` call is different
        // based on architecture (32/64). So a generic cast is used to make sure
        // there are no issues. To allow such a generic cast the clippy warning
        // is disabled.
        #[allow(clippy::cast_lossless)]
        ioctl(fd, TIOCSCTTY as _, 0)
    };

    if res < 0 {
        panic!("ioctl TIOCSCTTY failed");
    }
}

fn read_from_fd(fd: RawFd) -> Option<Vec<u8>> {
    let mut read_buffer = [0; 65536]; // 0x10_0000
    println!("reading from buffer");
    let read_result = read(fd, &mut read_buffer);
    println!("read result was: {:?}", read_result);

    match read_result {
        Ok(bytes_read) => Some(read_buffer[..bytes_read].to_vec()),
        _ => None,
    }
}

impl<'a, Message> From<TerminalDisplay<Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(terminal_box: TerminalDisplay<Message>) -> Self {
        Self::new(terminal_box)
    }
}
