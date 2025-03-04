use crate::sbi::set_timer;
use crate::register::{
    time,
    sie
};

pub static mut TICKS: usize = 0;

static TIMEBASE: u64 = 100000;
pub fn init() {
    unsafe {
        TICKS = 0;
        sie::set_stimer();
    }
    clock_set_next_event();
    println!("++++ setup timer!     ++++");
}

pub fn clock_set_next_event() {
    let current_cycle = get_cycle();
    println!("Current cycle: {}", current_cycle);  
    set_timer(current_cycle + TIMEBASE);

}


fn get_cycle() -> u64 {
    time::read() as u64
}