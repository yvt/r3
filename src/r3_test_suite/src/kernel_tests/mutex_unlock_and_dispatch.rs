//! Unlocks a mutex, waking up a task.
use r3::{
    hunk::Hunk,
    kernel::{cfg::CfgBuilder, Mutex, Task},
    prelude::*,
};

use super::Driver;
use crate::utils::SeqTracker;

pub struct App<System> {
    task2: Task<System>,
    task3: Task<System>,
    mtx: Mutex<System>,
    seq: Hunk<System, SeqTracker>,
}

impl<System: Kernel> App<System> {
    pub const fn new<D: Driver<Self>>(b: &mut CfgBuilder<System>) -> Self {
        Task::build()
            .start(task1_body::<System, D>)
            .priority(2)
            .active(true)
            .finish(b);
        let task2 = Task::build()
            .start(task2_body::<System, D>)
            .priority(1)
            .finish(b);
        let task3 = Task::build()
            .start(task3_body::<System, D>)
            .priority(0)
            .finish(b);

        let mtx = Mutex::build().finish(b);
        let seq = Hunk::<_, SeqTracker>::build().finish(b);

        App {
            task2,
            task3,
            mtx,
            seq,
        }
    }
}

fn task1_body<System: Kernel, D: Driver<App<System>>>(_: usize) {
    D::app().seq.expect_and_replace(0, 1);

    D::app().mtx.lock().unwrap();

    D::app().seq.expect_and_replace(1, 2);
    D::app().task2.activate().unwrap();
    D::app().seq.expect_and_replace(3, 4);
    D::app().task3.activate().unwrap();

    D::app().seq.expect_and_replace(5, 6);
    // Unblock `task3` following the task priority order, not the FIFO order
    D::app().mtx.unlock().unwrap();

    D::app().seq.expect_and_replace(8, 9);

    D::success();
}

fn task2_body<System: Kernel, D: Driver<App<System>>>(_: usize) {
    D::app().seq.expect_and_replace(2, 3);

    D::app().mtx.lock().unwrap(); // start waiting, switching to `task1`

    D::app().seq.expect_and_replace(7, 8);
}

fn task3_body<System: Kernel, D: Driver<App<System>>>(_: usize) {
    D::app().seq.expect_and_replace(4, 5);

    D::app().mtx.lock().unwrap(); // start waiting, switching to `task1`

    D::app().seq.expect_and_replace(6, 7);
    D::app().mtx.unlock().unwrap(); // unblock `task2`
}
