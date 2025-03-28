use app::VigilApp;
use cosmic::app::Settings;

mod app;

fn main() {
    let _ = cosmic::app::run::<VigilApp>(
        cosmic::app::Settings::default(),
        ()
    );

}
