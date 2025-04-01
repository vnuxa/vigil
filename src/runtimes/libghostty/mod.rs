use core::str;
use std::{ffi::{c_char, c_void, CStr, CString}, io::Read, os::raw::c_int, slice::from_raw_parts};



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



// struct ghostty_runtime_config_s {
//     userdata: *mut c_void,
//     supports_selection_clipboard: bool,
//     ghostty_runtime_wakeup_cb: (),
//     ghostty_runtime_action_cb: (),
//     ghostty_runtime_clipboard_cb: (),
//     ghostty_runtime_write_clipboard_cb: (),
//     ghostty_runtime_close_surface_cb: ()
//
// }
//
// #[link(name = "ghostty")]
// extern "C-unwind" {
//     pub fn ghostty_init() -> c_int;
//     pub fn ghostty_app_new(opts: *const ghostty_runtime_config_s) ->
// }
