// use std::ops::Bound;

fn main() {
    println!("cargo:rustc-link-lib=ghostty");

    let bindings = bindgen::Builder::default()
        .header("ghostty.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("unable to generate ghostty c bindings to rust");
}

//
// fn main() {
//
//
//     let ghostty = pkg_config::Config::new()
//         // .range_version((Bound::Included(incl_bound), Bound::Excluded(excl_bound)))
//         .cargo_metadata(true)
//         .probe("ghostty");
//
//
//     ghostty.unwrap_or_else(|err| panic!("cannot find ghostty {err}"));
// }
