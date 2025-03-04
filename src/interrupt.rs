use core::arch::global_asm;
use crate::register::scause::{Trap, Exception, Interrupt};
use crate::register::{stvec, sscratch, sstatus};
use crate::context::TrapFrame;
use crate::timer::{TICKS,clock_set_next_event};

global_asm!(include_str!("trap/trap.asm"));

pub fn init() {
    unsafe {
        extern "C" {
            fn __alltraps();
        }        
        sscratch::write(0);
        stvec::write(__alltraps as usize, stvec::TrapMode::Direct);
	    sstatus::set_sie();
    }
    println!("++++ setup interrupt! ++++");
}

#[no_mangle]
pub fn rust_trap(tf: &mut TrapFrame) {
    println!("Trap occurred with cause: {:?}", tf.scause.cause());
    match tf.scause.cause() {
        Trap::Exception(Exception::Breakpoint) => {
            println!("Handling breakpoint...");
            breakpoint(&mut tf.sepc);
        },
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            println!("Handling supervisor timer...");
            super_timer();
        },
        _ => panic!("undefined trap!")
    }
}


fn breakpoint(sepc: &mut usize) {
    println!("a breakpoint set @0x{:x}", sepc);
    *sepc += 2;
}

fn super_timer() {
    clock_set_next_event();
    unsafe {
        TICKS += 1;
        if TICKS == 100 {
            TICKS = 0;
            println!("* 100 ticks *");
        }
    }
}

