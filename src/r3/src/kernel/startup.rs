use core::marker::PhantomData;

use crate::utils::Init;

/// Represents a registered startup hook in a system.
///
/// There are no operations defined for startup hooks, so this type
/// is only used for static configuration.
///
/// Startup hooks execute during the boot process with [CPU Lock] active, after
/// initializing kernel structures and before scheduling the first task.
///
/// [CPU Lock]: crate#system-states
///
/// <div class="admonition-follows"></div>
///
/// > **Relation to Other Specifications:** `StartupHook` (AUTOSAR OS,
/// > OSEK/VDX), last function (TI-RTOS), initialization routine (μITRON4.0).
///
#[doc(include = "../common.md")]
pub struct StartupHook<System>(PhantomData<System>);

impl<System> StartupHook<System> {
    pub(super) const fn new() -> Self {
        Self(PhantomData)
    }
}

/// A startup hook.
///
/// This type isn't technically public but needs to be `pub` so that it can be
/// referred to by a macro.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct StartupHookAttr {
    pub(super) start: unsafe fn(usize),
    pub(super) param: usize,
}

impl Init for StartupHookAttr {
    const INIT: Self = Self {
        start: |_| {},
        param: 0,
    };
}
