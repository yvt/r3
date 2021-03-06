//! Checks that [`InterruptHandlerTable::get`] returns `None` for interrupt
//! lines that have no registered interrupt handlers.
use core::marker::PhantomData;
use r3::{
    kernel::{cfg::CfgBuilder, InterruptHandler, InterruptLine, StartupHook},
    prelude::*,
};
use r3_test_suite::kernel_tests::Driver;

pub struct App<System> {
    _phantom: PhantomData<System>,
}

impl<System: Kernel> App<System> {
    pub const fn new<D: Driver<Self>>(b: &mut CfgBuilder<System>) -> Self {
        StartupHook::build().start(hook_body::<System, D>).finish(b);

        InterruptLine::build().line(2).priority(64).finish(b);
        InterruptLine::build().line(3).priority(64).finish(b);

        unsafe {
            InterruptHandler::build().line(2).start(|_| {}).finish(b);
            InterruptHandler::build().line(3).start(|_| {}).finish(b);
            InterruptHandler::build()
                .line(5)
                .unmanaged()
                .start(|_| {})
                .finish(b);
            InterruptHandler::build()
                .line(7)
                .unmanaged()
                .start(|_| {})
                .finish(b);
        }

        App {
            _phantom: PhantomData,
        }
    }
}

fn hook_body<System: Kernel, D: Driver<App<System>>>(_: usize) {
    log::debug!("INTERRUPT_HANDLERS = {:#?}", System::INTERRUPT_HANDLERS);
    assert_eq!(System::INTERRUPT_HANDLERS.get(0), None);
    assert_eq!(System::INTERRUPT_HANDLERS.get(1), None);
    assert!(System::INTERRUPT_HANDLERS.get(2).is_some());
    assert!(System::INTERRUPT_HANDLERS.get(3).is_some());
    assert_eq!(System::INTERRUPT_HANDLERS.get(4), None);
    assert!(System::INTERRUPT_HANDLERS.get(5).is_some());
    assert_eq!(System::INTERRUPT_HANDLERS.get(6), None);
    assert!(System::INTERRUPT_HANDLERS.get(7).is_some());
    assert_eq!(System::INTERRUPT_HANDLERS.get(8), None);
    D::success();
}
