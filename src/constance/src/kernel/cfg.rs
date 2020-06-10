//! Static configuration mechanism for the kernel
use core::{marker::PhantomData, mem, num::NonZeroUsize};

use super::{event_group, hunk, task, utils::CpuLockCell, Port};
use crate::utils::{Init, ZeroInit, FIXED_PRIO_BITMAP_MAX_LEN};

mod vec;
#[doc(hidden)]
pub use self::vec::ComptimeVec;

/// Define a configuration function.
///
/// Generic parameters are supported with a help of
/// [`parse-generics-shim`](parse_generics_shim).
///
/// The following macros are available inside the function:
///
/// # `set!(prop = value)`
///
/// Set a global propertry.
///
///  - `num_task_priority_levels = NUM_LEVELS: usize` specifies the number of
///    task priority levels. The default value is `16`.
///
/// # `call!(expr, arg1, arg2, ...)`
///
/// Invokes another configuration function `expr`.
///
/// # `build!(expr, name1 = arg1, name2 = arg2, ...)`
///
/// Invokes a builder method `expr`, calls modifying methods `name1, name2, ...`
/// on the builder, and then finally calls `finish`, which is assumed to be a
/// configuration function.
///
/// # `new_task!(start = ENTRY_FN, ...)`
///
/// Defines a task. The following properties can be specified:
///
///  - `start = ENTRY_FN: fn(usize)` (**required**) specifies the task's entry
///    point.
///  - `param = PARAM: usize` specifies the parameter to `start`.
///  - `stack_size = LEN: usize` specifies the task's stack size.
///  - `stack_hunk = HUNK: Hunk<System, [UnsafeCell<u8>]>` specifies the task's
///    hunk.
///  - `priority = PRI: usize` (**required**) specifies the task's initial
///    priority. Tasks with lower priority values execute first. `PRI` must be
///    in range `0..num_task_priority_levels`.
///  - `active = ACTIVE: bool` specifies whether the task should be activated at
///    system startup.
///
/// # `new_event_group!(start = ENTRY_FN, ...)`
///
/// Defines an event group. The following properties can be specified:
///
///  - `initial = BITS: `[`EventGroupBits`] specifies the initial bit pattern.
///
/// [`EventGroupBits`]: crate::kernel::EventGroupBits
///
/// # `new_hunk!(T)`
///
/// Defines a new hunk. `T` must implement [`Init`](crate::utils::Init).
///
/// # `new_hunk!([T], zeroed = true, len = LEN, align = ALIGN)`
///
/// Defines a new zero-initialized hunk of an array of the specified length and
/// alignment.
///
#[macro_export]
macro_rules! configure {
    (
        // (1) Top-level rule - Parse everything before generic parmameters.
        //     Pass the generic parameters to `parse_generics_shim!` and proceed
        //     to (2-1) or (2-2) with the result of `parse_generics_shim!`.
        $( #[$meta:meta] )*
        $vis:vis fn $ident:ident $($gen_tokens:tt)*
    ) => {
        $crate::parse_generics_shim::parse_generics_shim! {
            { constr },
            then $crate::configure! {
                [2]
                meta: $( #[$meta] )*,
                vis: $vis,
                ident: $ident,
            },
            $($gen_tokens)*
        }
    };

    (
        // (2-1) Parse everything between generic parameters and an `where` clause.
        //       Pass the `where` clause to `parse_where_shim!` and proceed to (3)
        //       with the result of `parse_where_shim!`.
        [2]
        // Added by (1)
        meta: $( #[$meta:meta] )*,
        vis: $vis:vis,
        ident: $ident:ident,

        // Generated by `parse_generics_shim`
        $gen_param:tt,

        // Remaining tokens to parse
        (_: CfgBuilder<$sys:ty>) -> $id_map:ty where $($where_tokens:tt)*
    ) => {
        $crate::parse_generics_shim::parse_where_shim! {
            { clause, preds },
            then $crate::configure! {
                [3]
                meta: $( #[$meta] )*,
                vis: $vis,
                ident: $ident,
                sys: $sys,
                id_map: $id_map,
                gen_param: $gen_param,
            },
            where $($where_tokens)*
        }
    };

    (
        // (2-2) Same as (2-1) except `where` is absent.
        [2]
        // Added by (1)
        meta: $( #[$meta:meta] )*,
        vis: $vis:vis,
        ident: $ident:ident,

        // Generated by `parse_generics_shim`
        $gen_param:tt,

        // Remaining tokens to parse
        (_: CfgBuilder<$sys:ty>) -> $id_map:ty { $($body:tt)* }
    ) => {
        $crate::parse_generics_shim::parse_where_shim! {
            { clause, preds },
            then $crate::configure! {
                [3]
                meta: $( #[$meta] )*,
                vis: $vis,
                ident: $ident,
                sys: $sys,
                id_map: $id_map,
                gen_param: $gen_param,
            },
            { $($body)* }
        }
    };

    (
        // (3) Parse everything after an optional `where`. Proceed to (4).
        [3]
        // Added by (1)
        meta: $( #[$meta:meta] )*,
        vis: $vis:vis,
        ident: $ident:ident,

        // Added by (2)
        sys: $sys:ty,
        id_map: $id_map:ty,
        gen_param: $gen_param:tt,

        // Generated by `parse_where_shim`
        $where_param:tt,

        // Remaining tokens to parse
        {
            $($tt:tt)*
        }
    ) => {
        $crate::configure! {
            dollar: [$],
            meta: $( #[$meta] )*,
            vis: $vis,
            ident: $ident,
            sys: $sys,
            id_map: $id_map,
            gen_param: $gen_param,
            where_param: $where_param,

            body: { $($tt)* },
        }
    };

    (
        // (4) Core rule - this is invoked by the top-level rule down below

        // This parameter (`dollar`) is used to produce a dollar token (`$`).
        //
        // When you write something like `$(  )*` in a macro output, the macro
        // transcriber interprets it as a repetition. This conflict with our
        // intent to generate `macro_rules!` because we don't want `$(...)*`
        // inside these generated `macro_rules!` to be processed. We need the
        // expansion of those `$(...)*` to happen when expanding the generated
        // `macro_rules!`, not when expanding `configure!`.
        //
        // We address this problem by receiving a dollar token via a
        // metavariable. The transcriber for `configure!` doesn't interpret the
        // contents of `$dollar` and simply copies them verbadim to the output
        // token stream, so we can use it anywhere in the macro output without
        // worrying about it being processed by the transcriber in an unintended
        // way.
        dollar: [$dollar:tt],
        meta: $( #[$meta:meta] )*,
        vis: $vis:vis,
        ident: $ident:ident,
        sys: $sys:ty,
        id_map: $id_map:ty,
        gen_param: {
            constr: [ $($gen_param_constr:tt)* ],
        },
        where_param: {
            clause: [ $($where_param_clause:tt)* ],
            preds: [ $($where_param_preds:tt)* ],
        },
        body: { $($tt:tt)* },
    ) => {
        // FIXME: `&mut` in `const fn` <https://github.com/rust-lang/rust/issues/57349>
        //        is not implemented yet. Receiving `CfgBuilder` by `&mut _`
        //        would be more cleaner
        $( #[$meta] )*
        #[allow(unused_macros)]
        $vis const fn $ident<$($gen_param_constr)*>(
            cfg: $crate::kernel::CfgBuilder<$sys>
        ) -> $crate::kernel::CfgOutput<$sys, $id_map>
            $($where_param_clause)*
        {
            #[allow(unused_mut)]
            let mut cfg = cfg;

            macro_rules! set {
                ($argname:ident = $arg:expr $dollar(,)*) => {{
                    cfg = cfg.$argname($arg);
                }};
            }

            macro_rules! call {
                ($path:expr $dollar(, $arg:expr)* $dollar(,)*) => {{
                    use $crate::kernel::CfgOutput;

                    let CfgOutput { cfg: new_cfg, id_map } = $path(cfg, $dollar($arg),*);
                    cfg = new_cfg;
                    id_map
                }};
            }

            macro_rules! build {
                ($path:expr $dollar(, $argname:ident = $arg:expr)* $dollar(,)*) => {{
                    use $crate::kernel::CfgOutput;

                    let builder = $path $dollar(. $argname($arg))*;
                    let CfgOutput { cfg: new_cfg, id_map } = builder.finish(cfg);
                    cfg = new_cfg;
                    id_map
                }};
            }

            macro_rules! new_task {
                ($dollar($tt2:tt)*) => {
                    build! { $crate::kernel::CfgTaskBuilder::new(), $dollar($tt2)* }
                };
            }

            macro_rules! new_event_group {
                ($dollar($tt2:tt)*) => {
                    build! { $crate::kernel::CfgEventGroupBuilder::new(), $dollar($tt2)* }
                };
            }

            macro_rules! new_hunk {
                ([u8] $dollar(, zeroed = true)?, len = $len:expr) => {
                    new_hunk!([u8], zeroed = true, len = $len, align = 1)
                };
                ([$ty:ty], zeroed = true, len = $len:expr, align = $align:expr) => {
                    call!($crate::kernel::cfg_new_hunk_zero_array, $len, $align)
                };
                ($ty:ty) => {call!($crate::kernel::cfg_new_hunk::<_, $ty>)};
            }

            // `$ctx` will be updated by the code generated by `call!`

            let id_map = {
                $($tt)*
            };

            $crate::kernel::CfgOutput { cfg, id_map }
        }
    };
}

/// Attach a configuration function (defined by [`configure!`]) to a "system"
/// type by implementing [`KernelCfg2`] on `$sys`.
///
/// [`KernelCfg2`]: crate::kernel::KernelCfg2
#[macro_export]
macro_rules! build {
    ($sys:ty, $configure:expr) => {{
        use $crate::{
            kernel::{
                CfgBuilder, EventGroupCb, HunkAttr, HunkInitAttr, KernelCfg1, KernelCfg2, Port,
                State, TaskAttr, TaskCb,
            },
            utils::{
                intrusive_list::StaticListHead, AlignedStorage, FixedPrioBitmap, Init, RawCell,
                UIntegerWithBound,
            },
        };

        // `$configure` produces two values: a `CfgBuilder` and an ID map
        // (custom type). We need the first one to be `const` so that we can
        // calculate the values of generic parameters based on its contents.
        const CFG: CfgBuilder<$sys> = $configure(CfgBuilder::new()).cfg;

        // The second value can be just `let`
        let id_map = $configure(CfgBuilder::new()).id_map;

        // Set up task priority levels
        type TaskPriority = UIntegerWithBound<{ CFG.num_task_priority_levels as u128 - 1 }>;
        $crate::array_item_from_fn! {
            const TASK_PRIORITY_LEVELS: [TaskPriority; _] =
                (0..CFG.num_task_priority_levels).map(|i| i as _);
        };

        // Safety: We are `build!`, so it's okay to `impl` this
        unsafe impl KernelCfg1 for $sys {
            const NUM_TASK_PRIORITY_LEVELS: usize = CFG.num_task_priority_levels;
            type TaskPriority = TaskPriority;
            const TASK_PRIORITY_LEVELS: &'static [Self::TaskPriority] = &TASK_PRIORITY_LEVELS;
        }

        // Instantiiate task structures
        $crate::array_item_from_fn! {
            const TASK_ATTR_POOL: [TaskAttr<$sys>; _] =
                (0..CFG.tasks.len()).map(|i| CFG.tasks.get(i).to_attr());
            static TASK_CB_POOL:
                [TaskCb<$sys>; _] =
                    (0..CFG.tasks.len()).map(|i| CFG.tasks.get(i).to_state(&TASK_ATTR_POOL[i]));
        }

        // Instantiiate event group structures
        $crate::array_item_from_fn! {
            static EVENT_GROUP_CB_POOL:
                [EventGroupCb<$sys>; _] =
                    (0..CFG.event_groups.len()).map(|i| CFG.event_groups.get(i).to_state());
        }

        // Instantiate hunks
        static HUNK_POOL: RawCell<AlignedStorage<{ CFG.hunk_pool_len }, { CFG.hunk_pool_align }>> =
            Init::INIT;
        const HUNK_INITS: [HunkInitAttr; { CFG.hunks.len() }] = CFG.hunks.to_array();

        // Task ready bitmap
        type TaskReadyBitmap = FixedPrioBitmap<{ CFG.num_task_priority_levels }>;

        // Instantiate the global state
        type KernelState = State<$sys>;
        static KERNEL_STATE: KernelState = State::INIT;

        // Safety: We are `build!`, so it's okay to `impl` this
        unsafe impl KernelCfg2 for $sys {
            type TaskReadyBitmap = TaskReadyBitmap;
            type TaskReadyQueue = [StaticListHead<TaskCb<Self>>; CFG.num_task_priority_levels];

            fn state() -> &'static KernelState {
                &KERNEL_STATE
            }

            const HUNK_ATTR: HunkAttr = HunkAttr {
                hunk_pool: || HUNK_POOL.get() as *const u8,
                inits: &HUNK_INITS,
            };

            #[inline(always)]
            fn task_cb_pool() -> &'static [TaskCb<$sys>] {
                &TASK_CB_POOL
            }

            #[inline(always)]
            fn event_group_cb_pool() -> &'static [EventGroupCb<$sys>] {
                &EVENT_GROUP_CB_POOL
            }
        }

        id_map
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! array_item_from_fn {
    ($(
        $static_or_const:tt $out:ident: [$ty:ty; _] = (0..$len:expr).map(|$var:ident| $map:expr);
    )*) => {$(
        $static_or_const $out: [$ty; { $len }] = {
            let mut values = [$crate::prelude::Init::INIT; { $len }];
            let mut i = 0;
            while i < $len {
                values[i] = {
                    let $var = i;
                    $map
                };
                i += 1;
            }
            values
        };
    )*};
}

// The "real" public interface ends here
// ---------------------------------------------------------------------------

#[doc(hidden)]
pub struct CfgBuilder<System> {
    _phantom: PhantomData<System>,
    pub hunks: ComptimeVec<super::HunkInitAttr>,
    pub hunk_pool_len: usize,
    pub hunk_pool_align: usize,
    pub tasks: ComptimeVec<CfgBuilderTask<System>>,
    pub num_task_priority_levels: usize,
    pub event_groups: ComptimeVec<CfgBuilderEventGroup>,
}

impl<System> CfgBuilder<System> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
            hunks: ComptimeVec::new(),
            hunk_pool_len: 0,
            hunk_pool_align: 1,
            tasks: ComptimeVec::new(),
            num_task_priority_levels: 16,
            event_groups: ComptimeVec::new(),
        }
    }

    pub const fn num_task_priority_levels(mut self, new_value: usize) -> Self {
        if new_value == 0 {
            panic!("`num_task_priority_levels` must be greater than zero");
        } else if new_value > FIXED_PRIO_BITMAP_MAX_LEN {
            panic!("`num_task_priority_levels` must be less than or equal to `FIXED_PRIO_BITMAP_MAX_LEN`");
        } else if new_value >= isize::max_value() as usize {
            // Limiting priority values in range `0..(isize::max_value() - 1)`
            // leaves room for special values outside the extremities.
            //
            // This branch is actually unreachable because
            // `FIXED_PRIO_BITMAP_MAX_LEN` is so small compared to the size of
            // `isize`.
            unreachable!();
        }

        self.num_task_priority_levels = new_value;
        self
    }
}

/// Output of [a configuration function].
///
/// In a configuration function, use `call!` or `build!` to call other
/// configuration functions (i.e., the functions returning this type).
///
/// [a configuration function]: configure
pub struct CfgOutput<System, T> {
    #[doc(hidden)]
    pub cfg: CfgBuilder<System>,

    #[doc(hidden)]
    pub id_map: T,
}

/// Used by `new_hunk!` in configuraton functions
#[doc(hidden)]
pub const fn cfg_new_hunk<System, T: Init>(
    mut cfg: CfgBuilder<System>,
) -> CfgOutput<System, hunk::Hunk<System, T>> {
    let align = mem::align_of::<T>();
    let size = mem::size_of::<T>();

    // Round up `hunk_pool_len`
    cfg.hunk_pool_len = (cfg.hunk_pool_len + align - 1) / align * align;

    let start = cfg.hunk_pool_len;

    cfg.hunks = cfg.hunks.push(hunk::HunkInitAttr {
        offset: start,
        init: |dest| unsafe {
            *(dest as *mut _) = T::INIT;
        },
    });

    cfg.hunk_pool_len += size;
    if align > cfg.hunk_pool_align {
        cfg.hunk_pool_align = align;
    }

    let hunk = unsafe { hunk::Hunk::from_range(start, size) };

    CfgOutput { cfg, id_map: hunk }
}

/// Used by `new_hunk!` in configuraton functions
#[doc(hidden)]
pub const fn cfg_new_hunk_zero_array<System, T: ZeroInit>(
    mut cfg: CfgBuilder<System>,
    len: usize,
    mut align: usize,
) -> CfgOutput<System, hunk::Hunk<System, [T]>> {
    if !align.is_power_of_two() {
        panic!("`align` is not power of two");
    }

    if mem::align_of::<T>() > align {
        align = mem::align_of::<T>();
    }

    let byte_len = mem::size_of::<T>() * len;

    // Round up `hunk_pool_len`
    cfg.hunk_pool_len = (cfg.hunk_pool_len + align - 1) / align * align;

    // The hunk pool is zero-initialized by default
    let start = cfg.hunk_pool_len;
    let hunk = unsafe { hunk::Hunk::from_range(start, byte_len) };
    cfg.hunk_pool_len += byte_len;
    if align > cfg.hunk_pool_align {
        cfg.hunk_pool_align = align;
    }

    CfgOutput { cfg, id_map: hunk }
}

/// Used by `new_task!` in configuraton functions
#[doc(hidden)]
pub struct CfgTaskBuilder<System> {
    _phantom: PhantomData<System>,
    start: Option<fn(usize)>,
    param: usize,
    stack: Option<TaskStack<System>>,
    priority: Option<usize>,
    active: bool,
}

enum TaskStack<System> {
    Auto(usize),
    Hunk(task::StackHunk<System>),
    // TODO: Externally supplied stack? It's blocked by
    //       <https://github.com/rust-lang/const-eval/issues/11>, I think
}

impl<System: Port> CfgTaskBuilder<System> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
            start: None,
            param: 0,
            stack: None,
            priority: None,
            active: false,
        }
    }

    pub const fn start(self, start: fn(usize)) -> Self {
        Self {
            start: Some(start),
            ..self
        }
    }

    pub const fn param(self, param: usize) -> Self {
        Self { param, ..self }
    }

    pub const fn stack_size(self, stack_size: usize) -> Self {
        // FIXME: `Option::is_some` is not `const fn` yet
        if let Some(_) = self.stack {
            panic!("the task's stack is already specified");
        }

        Self {
            stack: Some(TaskStack::Auto(stack_size)),
            ..self
        }
    }

    pub const fn stack_hunk(self, stack_hunk: task::StackHunk<System>) -> Self {
        // FIXME: `Option::is_some` is not `const fn` yet
        if let Some(_) = self.stack {
            panic!("the task's stack is already specified");
        }

        Self {
            stack: Some(TaskStack::Hunk(stack_hunk)),
            ..self
        }
    }

    pub const fn priority(self, priority: usize) -> Self {
        Self {
            priority: Some(priority),
            ..self
        }
    }

    pub const fn active(self, active: bool) -> Self {
        Self { active, ..self }
    }

    pub const fn finish(
        self,
        mut cfg: CfgBuilder<System>,
    ) -> CfgOutput<System, task::Task<System>> {
        // FIXME: `Option::unwrap_or` is not `const fn` yet
        let stack = if let Some(stack) = self.stack {
            stack
        } else {
            TaskStack::Auto(System::STACK_DEFAULT_SIZE)
        };
        let stack = match stack {
            TaskStack::Auto(size) => {
                let CfgOutput {
                    cfg: new_cfg,
                    id_map: hunk,
                } = cfg_new_hunk_zero_array(cfg, size, System::STACK_ALIGN);
                cfg = new_cfg;

                // Safety: We just created a hunk just for this task, and we
                // don't use this hunk for other purposes.
                unsafe { task::StackHunk::from_hunk(hunk) }
            }
            TaskStack::Hunk(hunk) => hunk,
        };

        cfg.tasks = cfg.tasks.push(CfgBuilderTask {
            start: if let Some(x) = self.start {
                x
            } else {
                panic!("`start` (task entry point) is not specified")
            },
            param: self.param,
            stack,
            priority: if let Some(x) = self.priority {
                x
            } else {
                panic!("`priority` is not specified")
            },
            active: self.active,
        });

        let task = unsafe { task::Task::from_id(NonZeroUsize::new_unchecked(cfg.tasks.len())) };

        CfgOutput { cfg, id_map: task }
    }
}

#[doc(hidden)]
pub struct CfgBuilderTask<System> {
    start: fn(usize),
    param: usize,
    stack: task::StackHunk<System>,
    priority: usize,
    active: bool,
}

impl<System> Clone for CfgBuilderTask<System> {
    fn clone(&self) -> Self {
        Self {
            start: self.start,
            param: self.param,
            stack: self.stack,
            priority: self.priority,
            active: self.active,
        }
    }
}

impl<System> Copy for CfgBuilderTask<System> {}

impl<System: Port> CfgBuilderTask<System> {
    pub const fn to_state(&self, attr: &'static task::TaskAttr<System>) -> task::TaskCb<System> {
        task::TaskCb {
            port_task_state: System::PORT_TASK_STATE_INIT,
            attr,
            priority: if self.priority < System::NUM_TASK_PRIORITY_LEVELS {
                System::TASK_PRIORITY_LEVELS[self.priority]
            } else {
                panic!("task's `priority` must be less than `num_task_priority_levels`");
            },
            st: CpuLockCell::new(if self.active {
                task::TaskSt::PendingActivation
            } else {
                task::TaskSt::Dormant
            }),
            link: CpuLockCell::new(None),
            _force_int_mut: crate::utils::RawCell::new(()),
        }
    }

    pub const fn to_attr(&self) -> task::TaskAttr<System> {
        task::TaskAttr {
            entry_point: self.start,
            entry_param: self.param,
            stack: self.stack,
        }
    }
}

/// Used by `new_event_group!` in configuraton functions
#[doc(hidden)]
pub struct CfgEventGroupBuilder<System> {
    _phantom: PhantomData<System>,
    initial_bits: event_group::EventGroupBits,
}

impl<System: Port> CfgEventGroupBuilder<System> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
            initial_bits: 0,
        }
    }

    pub const fn initial(self, initial: event_group::EventGroupBits) -> Self {
        Self {
            initial_bits: initial,
            ..self
        }
    }

    pub const fn finish(
        self,
        mut cfg: CfgBuilder<System>,
    ) -> CfgOutput<System, event_group::EventGroup<System>> {
        cfg.event_groups = cfg.event_groups.push(CfgBuilderEventGroup {
            initial_bits: self.initial_bits,
        });

        let event_group = unsafe {
            event_group::EventGroup::from_id(NonZeroUsize::new_unchecked(cfg.event_groups.len()))
        };

        CfgOutput {
            cfg,
            id_map: event_group,
        }
    }
}

#[doc(hidden)]
pub struct CfgBuilderEventGroup {
    initial_bits: event_group::EventGroupBits,
}

impl Clone for CfgBuilderEventGroup {
    fn clone(&self) -> Self {
        Self {
            initial_bits: self.initial_bits,
        }
    }
}

impl Copy for CfgBuilderEventGroup {}

impl CfgBuilderEventGroup {
    pub const fn to_state<System: Port>(&self) -> event_group::EventGroupCb<System> {
        event_group::EventGroupCb {
            bits: CpuLockCell::new(self.initial_bits),
        }
    }
}
