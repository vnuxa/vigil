use cosmic::{widget::text_input, Action, Application, Task};

pub struct VigilApp {
    core: cosmic::Core,
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
            },

            Task::none()
        )
    }

    fn view(&self) -> cosmic::Element<Self::Message> {
        text_input("", &self.terminal_buffer)
            .on_input(|new_buffer| VigilMessages::WriteBuffer(new_buffer))
            .into()
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
