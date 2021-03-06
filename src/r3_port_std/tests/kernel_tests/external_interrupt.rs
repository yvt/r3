//! Pends an interrupt from an external thread.
use r3::{
    hunk::Hunk,
    kernel::{cfg::CfgBuilder, InterruptHandler, InterruptLine, Task},
    prelude::*,
};
use r3_test_suite::kernel_tests::Driver;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread::{sleep, spawn},
    time::Duration,
};

use r3_port_std::PortInstance;

pub struct App<System> {
    int: Option<InterruptLine<System>>,
    done: Hunk<System, AtomicBool>,
}

impl<System: PortInstance> App<System> {
    pub const fn new<D: Driver<Self>>(b: &mut CfgBuilder<System>) -> Self {
        Task::build()
            .start(task_body1::<System, D>)
            .priority(1)
            .active(true)
            .finish(b);

        let int = if let (&[int_line, ..], &[int_pri, ..]) =
            (D::INTERRUPT_LINES, D::INTERRUPT_PRIORITIES)
        {
            InterruptHandler::build()
                .line(int_line)
                .start(isr::<System, D>)
                .finish(b);

            Some(
                InterruptLine::build()
                    .line(int_line)
                    .priority(int_pri)
                    .enabled(true)
                    .finish(b),
            )
        } else {
            None
        };

        let done = Hunk::<_, AtomicBool>::build().finish(b);

        App { int, done }
    }
}

fn task_body1<System: PortInstance, D: Driver<App<System>>>(_: usize) {
    let int = if let Some(int) = D::app().int {
        int
    } else {
        log::warn!("No interrupt lines defined, skipping the test");
        D::success();
        return;
    };

    // Spawn a host thread
    log::debug!("spawning an external thread");
    spawn(move || {
        sleep(Duration::from_millis(100));
        log::debug!("pending {:?}", int);
        r3_port_std::pend_interrupt_line::<System>(int.num()).unwrap();
    });

    log::debug!("waiting for `done` to be set...");
    while !D::app().done.load(Ordering::Relaxed) {}
    log::debug!("success!");

    D::success();
}

fn isr<System: Kernel, D: Driver<App<System>>>(_: usize) {
    D::app().done.store(true, Ordering::Relaxed);
}
