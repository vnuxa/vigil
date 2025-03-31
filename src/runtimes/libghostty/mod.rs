use std::os::raw::c_int;



pub struct Terminal {

}

impl Terminal {
    pub fn new() -> Self {
        unsafe {
            let _ = ghostty_init();
        }
        Terminal {  }
    }
}


#[link(name = "libghostty")]
extern "C-unwind" {
    pub fn ghostty_init() -> c_int;
}
