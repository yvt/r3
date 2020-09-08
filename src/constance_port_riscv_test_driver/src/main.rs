#![feature(asm)]
#![feature(const_fn)]
#![feature(const_mut_refs)]
#![feature(naked_functions)]
#![feature(llvm_asm)]
#![feature(decl_macro)]
#![feature(unsafe_block_in_unsafe_fn)] // `unsafe fn` doesn't imply `unsafe {}`
#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(feature = "run", no_std)]
#![cfg_attr(feature = "run", no_main)]

#[cfg(feature = "output-rtt")]
mod logger_rtt;
#[cfg(feature = "output-uart")]
mod logger_uart;

#[cfg(feature = "output-rtt")]
mod panic_rtt;
#[cfg(feature = "output-uart")]
mod panic_uart;

#[cfg(feature = "output-e310x-uart")]
#[path = "uart_e310x.rs"]
mod uart;

#[cfg(feature = "interrupt-e310x")]
mod interrupt_e310x;

#[cfg(any(feature = "board-e310x-red-v", feature = "board-e310x-qemu"))]
mod e310x;

#[allow(unused_macros)]
macro_rules! instantiate_test {
    // If a test case is specified, instantiate the test case
    ({ path: $path:path, name_ident: $ident:ident, $($tt:tt)* }, $($excess:tt)*) => {
        // Only one test case can be specified
        reject_excess!($($excess)*);

        use constance::kernel::{StartupHook, InterruptPriority, InterruptNum,
            cfg::CfgBuilder};
        #[cfg(feature = "kernel_tests")]
        use constance_test_suite::kernel_tests;
        #[cfg(feature = "kernel_benchmarks")]
        use constance_test_suite::kernel_benchmarks;
        use constance_port_riscv as port;
        use $path as test_case;

        fn report_success() {
            // The test runner will catch this
            #[cfg(feature = "output-rtt")]
            rtt_target::rprintln!("!- TEST WAS SUCCESSFUL -!");

            #[cfg(feature = "output-uart")]
            uart::stdout_write_fmt(format_args!("!- TEST WAS SUCCESSFUL -!"));

            loop {
                // prevent the loop from being optimized out
                // <https://github.com/rust-lang/rust/issues/28728>
                unsafe { asm!("") };
            }
        }

        fn report_fail() {
            panic!("test failed");
        }

        port::use_port!(unsafe struct System);
        port::use_rt!(unsafe System);
        port::use_timer!(unsafe impl PortTimer for System);

        impl port::ThreadingOptions for System {}

        #[cfg(feature = "interrupt-e310x")]
        use_interrupt_e310x!(unsafe impl InterruptController for System);

        impl port::TimerOptions for System {
            const MTIME_PTR: usize = 0x0200_bff8;
            const MTIMECMP_PTR: usize = 0x0200_4000;
            const FREQUENCY: u64 = e310x::MTIME_FREQUENCY;

            // Updating `mtime` is not supported by QEMU.
            const RESET_MTIME: bool = false;
        }

        struct Driver;

        #[cfg(feature = "kernel_benchmarks")]
        impl kernel_benchmarks::Driver<test_case::App<System>> for Driver {
            fn app() -> &'static test_case::App<System> {
                &COTTAGE
            }
            fn success() {
                report_success();
            }
            fn performance_time() -> u32 {
                unsafe {
                    let mcycle;
                    asm!("csrr {}, mcycle", out(reg)mcycle);
                    mcycle
                }
            }

            const PERFORMANCE_TIME_UNIT: &'static str = "cycle(s)";

            #[cfg(feature = "interrupt-e310x")]
            const INTERRUPT_LINES: &'static [InterruptNum] = &[
                crate::interrupt_e310x::INTERRUPT_GPIO0,
                // `USE_NESTING` is only enabled on QEMU
                #[cfg(feature = "board-e310x-qemu")]
                crate::interrupt_e310x::INTERRUPT_GPIO1,
            ];
            const INTERRUPT_PRIORITY_LOW: InterruptPriority = 2;
            const INTERRUPT_PRIORITY_HIGH: InterruptPriority = 6;
        }

        #[cfg(feature = "kernel_tests")]
        impl kernel_tests::Driver<test_case::App<System>> for Driver {
            fn app() -> &'static test_case::App<System> {
                &COTTAGE
            }
            fn success() {
                report_success();
            }
            fn fail() {
                report_fail();
            }
            #[cfg(feature = "interrupt-e310x")]
            const INTERRUPT_LINES: &'static [InterruptNum] = &[
                crate::interrupt_e310x::INTERRUPT_GPIO0,
                // `USE_NESTING` is only enabled on QEMU
                #[cfg(feature = "board-e310x-qemu")]
                crate::interrupt_e310x::INTERRUPT_GPIO1,
            ];
            const INTERRUPT_PRIORITY_LOW: InterruptPriority = 2;
            const INTERRUPT_PRIORITY_HIGH: InterruptPriority = 6;
        }

        static COTTAGE: test_case::App<System> =
            constance::build!(System, configure_app => test_case::App<System>);

        const fn configure_app(b: &mut CfgBuilder<System>) -> test_case::App<System> {
            // Initialize the clock
            #[cfg(any(feature = "board-e310x-red-v", feature = "board-e310x-qemu"))]
            StartupHook::build().start(|_| {
                e310x::clocks();
            }).finish(b);

            System::configure_interrupt(b);
            System::configure_timer(b);

            // Initialize RTT (Real-Time Transfer) with two up channels and set
            // the first one as the print channel for the printing macros, and
            // the second one as log output
            #[cfg(feature = "output-rtt")]
            StartupHook::build().start(|_| {
                let channels = rtt_target::rtt_init! {
                    up: {
                        0: {
                            size: 512
                            mode: BlockIfFull
                            name: "Terminal"
                        }
                        1: {
                            size: 1024
                            mode: NoBlockSkip
                            name: "Log"
                        }
                    }
                };

                unsafe {
                    rtt_target::set_print_channel_cs(
                        channels.up.0,
                        &((|arg, f| f(arg)) as rtt_target::CriticalSectionFunc),
                    )
                };
                logger_rtt::init(channels.up.1);
            }).finish(b);

            // Redirect the log output to stderr
            #[cfg(feature = "output-uart")]
            StartupHook::build().start(|_| {
                logger_uart::init();
            }).finish(b);

            test_case::App::new::<Driver>(b)
        }
    };

    () => {
        compile_error!("no test is specified");
    }
}

#[allow(unused_macros)]
macro_rules! reject_excess {
    () => {};
    ($($tt:tt)*) => {
        compile_error!("can't specify more than one test");
    };
}

// Get the selected test case and instantiate
#[cfg(feature = "kernel_benchmarks")]
constance_test_suite::get_selected_kernel_benchmarks!(instantiate_test!());
#[cfg(feature = "kernel_tests")]
constance_test_suite::get_selected_kernel_tests!(instantiate_test!());

#[cfg(not(feature = "run"))]
fn main() {
    panic!("This executable should be invoked directly");
}