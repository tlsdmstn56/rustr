use std::env;
use rustr::bindings::*;
// use rustr::unix::system::initialze_r;

fn main() {
    // unsafe {
    //     let args: Vec<String> = env::args().collect();
    //     R_running_as_main_program = 1;
    //     initialze_r(&args);
    //     Rf_mainloop();
    // }
    unsafe {
        let mut args: Vec<String> = env::args().collect();
        let mut args_raw: Vec<*mut u8> = args.iter_mut()
            .map(|arg| arg.as_mut_ptr())
            .collect();
        R_running_as_main_program = 1;
        Rf_initialize_R(args_raw.len() as i32, args_raw.as_mut_ptr() as *mut *mut i8);
        Rf_mainloop();
    }
}
