use app::VigilApp;
use cosmic::app::Settings;

mod app;
mod runtimes;

fn main() {
    let _ = cosmic::app::run::<VigilApp>(
        Settings::default()
            .antialiasing(true)
            .client_decorations(false)
            .debug(false),
        ()
    );

}
