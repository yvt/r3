//! Configures `Timer` as a periodic timer and checks that it fires at expected
//! moments.
//!
//! ```text
//!       __          __               __        __            __
//! Task |__|        |__|             |__|      |__|          |__|
//!      0→1                          2→3       4→5           6→7
//!                                  _        _              _
//! Timer callback                  |_|      |_|            |_|
//!                                 1→2      3→4            5→6
//!      ├──┬──┬──┬──┼──┬──┬──┬──┬──┼──┬──┬──┼──┬──┬──┬──┬──┤
//!      ↑   400ms   ↑     500ms       300ms       500ms
//! system boot    start             ↑
//!                             period ← 400ms
//! ```
use constance::{
    kernel::{cfg::CfgBuilder, Hunk, Task, Timer},
    prelude::*,
    time::{Duration, Time},
};

use super::Driver;
use crate::utils::SeqTracker;

pub struct App<System> {
    timer: Timer<System>,
    task: Task<System>,
    seq: Hunk<System, SeqTracker>,
}

impl<System: Kernel> App<System> {
    pub const fn new<D: Driver<Self>>(b: &mut CfgBuilder<System>) -> Self {
        let timer = Timer::build()
            .delay(Duration::from_millis(500))
            .period(Duration::from_millis(300))
            .start(timer_body::<System, D>)
            .param(42)
            .finish(b);

        let task = Task::build()
            .active(true)
            .start(task_body::<System, D>)
            .priority(1)
            .finish(b);

        let seq = Hunk::<_, SeqTracker>::build().finish(b);

        App { timer, task, seq }
    }
}

fn task_body<System: Kernel, D: Driver<App<System>>>(_: usize) {
    let App { seq, timer, .. } = D::app();

    // Expected current time
    let mut now = Time::from_millis(0);

    seq.expect_and_replace(0, 1);

    let t = Duration::from_millis(400);
    System::sleep(t).unwrap();
    now += t;

    macro_rules! check_time {
        () => {
            let now_got = System::time().unwrap();
            log::debug!("time = {:?} (expected {:?})", now_got, now);
            assert!(now_got.as_micros() >= now.as_micros());
            assert!(now_got.as_micros() <= now.as_micros() + 100_000);
        };
    }

    // Start the timer
    check_time!();
    timer.start().unwrap();

    // First tick
    System::park().unwrap();
    seq.expect_and_replace(2, 3);
    now += Duration::from_millis(500); // delay
    check_time!();

    // Second tick
    System::park().unwrap();
    seq.expect_and_replace(4, 5);
    now += Duration::from_millis(300); // period
    check_time!();

    // Third tick
    System::park().unwrap();
    seq.expect_and_replace(6, 7);
    now += Duration::from_millis(500); // period (new)
    check_time!();

    timer.stop().unwrap();

    D::success();
}

fn timer_body<System: Kernel, D: Driver<App<System>>>(_: usize) {
    let App {
        task, timer, seq, ..
    } = D::app();

    match seq.get() {
        1 => {
            seq.expect_and_replace(1, 2);
            timer.set_period(Some(Duration::from_millis(500))).unwrap();
            task.unpark_exact().unwrap();
        }
        3 => {
            seq.expect_and_replace(3, 4);
            task.unpark_exact().unwrap();
        }
        5 => {
            seq.expect_and_replace(5, 6);
            task.unpark_exact().unwrap();
        }
        _ => unreachable!(),
    }
}