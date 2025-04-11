use std::{
    io::{Read, Write},
    thread,
};

use cosmic::{
    widget::{column, text_input, Column},
    Action, Application, Task,
};
use vte::Parser;

use crate::runtimes::libvigil::{self, make_io_subscription, Terminal};

pub struct VigilApp<const NUM_ROW: usize, const NUM_COLUMN: usize> {
    core: cosmic::Core,
    // TODO: better solution for the width of the termianl, as this seems to be not a good solution
    // if size were to change
    terminal: Terminal<NUM_ROW, NUM_COLUMN>,
    terminal_buffer: String,
}

#[derive(Clone, Debug)]
pub enum VigilMessages {
    WriteBuffer(String),
    StdoutRead(Vec<u8>),
    StdinInput(char),
}

impl<const NUM_ROW: usize, const NUM_COLUMN: usize> Application for VigilApp<NUM_ROW, NUM_COLUMN> {
    type Message = VigilMessages;
    type Executor = cosmic::executor::Default;
    type Flags = ();

    const APP_ID: &'static str = "vigil_terminal";

    fn init(
        mut core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let terminal = Terminal::init(None);

        // terminal.update_buffer();
        // terminal.make_display();
        core.window.show_headerbar = false;

        (
            Self {
                core,
                terminal_buffer: "".to_string(),
                terminal,
            },
            Task::none(),
        )
    }

    fn view(&self) -> cosmic::Element<Self::Message> {
        self.terminal.display.clone().into()
    }

    fn update(&mut self, message: Self::Message) -> cosmic::Task<Action<Self::Message>> {
        match message {
            VigilMessages::WriteBuffer(new_buffer) => self.terminal_buffer = new_buffer,
            // VigilMessages::StdoutRead(mut read_bytes) => { self.terminal.read_buffer.append(&mut read_bytes); self.terminal.make_display();}
            VigilMessages::StdoutRead(mut buf) => {
                // let mut buffer = [0u8; 0x10_0000];
                let mut parser = Parser::new();
                println!("reading?");
                parser.advance(&mut self.terminal, &buf);
                self.terminal.read_buffer.append(&mut buf);
                // self.terminal.make_display();
                // let res = self.terminal.update_buffer();
                // println!("got result of {:?}", res);
                // self.terminal.read_buffer.append(&mut read_bytes);
                // println!("before display")
                // self.terminal.make_display();
                println!("after update term")
            }
            VigilMessages::StdinInput(char) => {
                println!("got input {:?}", char);
                // let stream_clone = self.terminal.stdout_stream.clone();
                // let mut stream = *stream_clone;
                println!("got stream");
                let mut buffer = [0, 0, 0, 0];
                // println!("file descriptor: {:?}", self.terminal.master_fd);
                self.terminal
                    .write_pty(char.encode_utf8(&mut buffer).as_bytes());
                // let result = stream.write(char.encode_utf8(&mut buffer).as_bytes());
                // println!("got result {:?}", result);
            }
        }
        println!("hey i got buffer {:?}", self.terminal_buffer);

        Task::none()
    }

    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        make_io_subscription(self.terminal.stdout_stream.clone())
        //     // println!("subscription called!");
        //     self.terminal.subscription(self.terminal.pty.file)
        //     // self.terminal.subscription()
    }

    fn core(&self) -> &cosmic::Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }
}
