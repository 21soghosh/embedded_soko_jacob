use core::arch::asm;
use core::panic::PanicInfo;
use cortex_m::interrupt;
use cortex_m_semihosting::hprintln;
use rt::{exception, ExceptionFrame};

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    hprintln!("{:?}", ef);
    halt()
}

#[exception]
unsafe fn DefaultHandler(_irqn: i16) {}

#[panic_handler]
fn panic_handler(pi: &PanicInfo) -> ! {
    hprintln!("{}", pi);
    halt()
}

fn halt() -> ! {
    interrupt::disable();
    loop {
        unsafe { asm!("wfi") }
    }
}
