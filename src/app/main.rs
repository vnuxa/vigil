use cosmic::{widget::{column, text_input, Column}, Action, Application, Task};

use crate::runtimes::libvigil::Terminal;



pub struct VigilApp<const NUM_ROW: usize, const NUM_COLUMN: usize> {
    core: cosmic::Core,
    // TODO: better solution for the width of the termianl, as this seems to be not a good solution
    // if size were to change
    terminal: Terminal<NUM_ROW, NUM_COLUMN>,
    terminal_buffer: String
}

#[derive(Clone, Debug)]
pub enum VigilMessages {
    WriteBuffer(String),
    StdoutRead(Vec<u8>)
}

impl<const NUM_ROW: usize, const NUM_COLUMN: usize> Application for VigilApp<NUM_ROW, NUM_COLUMN> {
    type Message = VigilMessages;
    type Executor = cosmic::executor::Default;
    type Flags = ();


    const APP_ID: &'static str = "vigil_terminal";

    fn init(mut core: cosmic::Core, _flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let mut terminal = Terminal::init(None);
        terminal.make_display();
        core.window.show_headerbar = false;
        (
            Self {
                core,
                terminal_buffer: "".to_string(),
                terminal
            },

            Task::none()
        )
    }

    fn view(&self) -> cosmic::Element<Self::Message> {
        self.terminal.display.clone().into()
    }

    fn update(&mut self, message: Self::Message) -> cosmic::Task<Action<Self::Message>> {
        match message {
            VigilMessages::WriteBuffer(new_buffer) => self.terminal_buffer = new_buffer,
            VigilMessages::StdoutRead(mut read_bytes) => { self.terminal.read_buffer.append(&mut read_bytes); self.terminal.make_display();}
        }
        println!("hey i got buffer {:?}", self.terminal_buffer);

        Task::none()
    }

    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        self.terminal.clone().subscription()
    }


    fn core(&self) -> &cosmic::Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }
}
