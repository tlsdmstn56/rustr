use std::env;

extern "C" { 
    // int Rf_initialize_R(int ac, char **av); /* in ../unix/system.c */
    fn Rf_initialize_R(ac: i32, av: *mut *mut u8) -> i32;
    fn Rf_mainloop();
    static mut R_running_as_main_program: i32;
}

fn main() {
    unsafe {
        let mut args: Vec<String> = env::args().collect();
        let mut args_raw: Vec<*mut u8> = args.iter_mut()
            .map(|arg| arg.as_mut_ptr())
            .collect();
        R_running_as_main_program = 1;
        Rf_initialize_R(args_raw.len() as i32, args_raw.as_mut_ptr());
        Rf_mainloop();
    }
}
