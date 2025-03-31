use alacritty_terminal::tty::{Options, Shell};
use cosmic::{widget::{column, text_input, Column}, Action, Application, Task};

use crate::runtimes::libghostty::Terminal;

// use crate::runtimes::alacritty::{Terminal, TerminalOptions};

pub struct VigilApp {
    core: cosmic::Core,
    terminal: Terminal,
    terminal_buffer: String
}

#[derive(Clone, Debug)]
pub enum VigilMessages {
    WriteBuffer(String)
}


impl Application for VigilApp {
    type Message = VigilMessages;
    type Executor = cosmic::executor::Default;
    type Flags = ();


    const APP_ID: &'static str = "vigil_terminal";

    fn init(core: cosmic::Core, _flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
        (
            Self {
                core,
                terminal_buffer: "".to_string(),
                // terminal: Terminal::new(TerminalOptions {
                //     size: alacritty_terminal::event::WindowSize {
                //         num_lines: 50,
                //         num_cols: 50,
                //         cell_width: 10,
                //         cell_height: 10,
                //     },
                //     id: 0,
                //     options: Options {
                //         shell: Some(Shell::new("echo".to_string(), vec![ "'hi there'".to_string() ])),
                //         working_directory: None,
                //         ..Default::default()
                //     },
                //     scrolling_history: 100
                // })
                terminal: Terminal::new()
            },

            Task::none()
        )
    }

    fn view(&self) -> cosmic::Element<Self::Message> {
        let mut column_children = Vec::new();
        // for (line, text) in self.terminal.buffer() {
        //     println!("hey i got text for line: {} |||| with text: {}", line, text);
        //     column_children.push(cosmic::widget::text(text).into());
        // }
        column_children.push(
            text_input("", &self.terminal_buffer)
                .on_input(|new_buffer| VigilMessages::WriteBuffer(new_buffer)).into()
        );

        Column::from_vec(column_children).into()
    }

    fn update(&mut self, message: Self::Message) -> cosmic::Task<Action<Self::Message>> {
        match message {
            VigilMessages::WriteBuffer(new_buffer) => self.terminal_buffer = new_buffer,
        }
        println!("hey i got buffer {:?}", self.terminal_buffer);

        Task::none()
    }


    fn core(&self) -> &cosmic::Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }
}
