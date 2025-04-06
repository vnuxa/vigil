use app::main::VigilApp;
// use app::VigilApp;
use cosmic::app::Settings;
use runtimes::libvigil::Terminal;

mod app;
mod runtimes;

fn main() {
    // let mut term: Terminal<80, 80> = Terminal::init(None);
    // term.update_buffer();

    println!("after all of that i got a display like this");
    //
    // for row in term.display.cells {
    //
    //     for column in row {
    //         print!("{}", column.character);
    //     }
    //     println!();
    // }

    let _ = cosmic::app::run::<VigilApp<80, 80>>(
        Settings::default()
            .antialiasing(true)
            .client_decorations(false)
            .debug(false),
        ()
    );

}
