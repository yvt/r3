//! Type-safe hunks
use core::{
    fmt,
    marker::PhantomData,
    mem,
    ops::Deref,
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
};

use crate::{
    kernel::{self, cfg::CfgBuilder, Kernel, Port, StartupHook},
    utils::{Init, ZeroInit},
};

/// The priority of the [startup hooks] used to initialize [typed hunks]. It has
/// a negative value so that startup hooks with non-negative priorities (which
/// can be created without `unsafe` blocks) will never see an uninitialized
/// value in a typed hunk.
///
/// [startup hooks]: crate::kernel::StartupHook
/// [typed hunks]: Hunk
pub const INIT_HOOK_PRIORITY: i32 = -0x7000_0000;

/// Represents a single typed hunk in a system.
///
/// Hunks are nothing more than static variables defined in a kernel
/// configuration. They come in handy when you are designing a component that
/// can be instantiated by a kernel configuration and wanting each instance to
/// have its own separate state data.
///
/// This type is implemented on top of [`r3::kernel::Hunk`], the untyped
/// hunk type.
///
/// [`r3::kernel::Hunk`]: crate::kernel::Hunk
#[doc(include = "./common.md")]
pub struct Hunk<System, T: ?Sized> {
    /// The offset of the hunk. `System::HUNK_ATTR.hunk_pool_ptr()` must be
    /// added before dereferencing.
    offset: *const T,
    _phantom: PhantomData<System>,
}

unsafe impl<System, T: ?Sized + Send> Send for Hunk<System, T> {}
unsafe impl<System, T: ?Sized + Sync> Sync for Hunk<System, T> {}

impl<System: Kernel, T: ?Sized> Hunk<System, T> {
    /// Construct a `CfgTaskBuilder` to define a hunk in [a configuration
    /// function](crate#static-configuration).
    pub const fn build() -> CfgHunkBuilder<System, T, DefaultInitTag> {
        CfgHunkBuilder {
            _phantom: PhantomData,
            len: 1,
            align: 1,
        }
    }
}

/// As a generic parameter of [`CfgHunkBuilder`], indicates that the [hunk]
/// should be initialized with [`Init`].
///
/// [`Init`]: crate::utils::Init
/// [hunk]: crate::kernel::Hunk
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct DefaultInitTag;

/// As a generic parameter of [`CfgHunkBuilder`], indicates that the [hunk]
/// should be zero-initialized.
///
/// [hunk]: crate::kernel::Hunk
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct ZeroInitTag;

/// Implemented on [`DefaultInitTag`] and [`ZeroInitTag`] when `T` can be
/// initialized in this way.
pub trait HunkIniter<T> {
    /// A flag indicating whether [`Self::init`] should be called for
    /// initialization.
    const NEEDS_INIT: bool;

    /// Initialize the specified memory region.
    fn init(dest: &mut mem::MaybeUninit<T>);
}

impl<T: Init> HunkIniter<T> for DefaultInitTag {
    const NEEDS_INIT: bool = true;
    fn init(dest: &mut mem::MaybeUninit<T>) {
        *dest = mem::MaybeUninit::new(T::INIT);
    }
}

impl<T> HunkIniter<T> for ZeroInitTag {
    const NEEDS_INIT: bool = false;
    fn init(_: &mut mem::MaybeUninit<T>) {
        // Do nothing - a hunk pool is zero-initialized by default
    }
}

/// Configuration builder type for [`Hunk`].
///
/// `InitTag` is either [`DefaultInitTag`] or [`ZeroInitTag`].
///
/// [`Hunk`]: crate::kernel::Hunk
#[must_use = "must call `finish()` to complete registration"]
pub struct CfgHunkBuilder<System, T: ?Sized, InitTag> {
    _phantom: PhantomData<(System, InitTag, T)>,
    len: usize,
    align: usize,
}

impl<System: Kernel, T: ?Sized, InitTag> CfgHunkBuilder<System, T, InitTag> {
    /// Specify the element count. Defaults to `1`. Must be `1` for a non-array
    /// hunk.
    pub const fn len(self, len: usize) -> Self {
        Self { len, ..self }
    }

    /// Specify the minimum alignment. Defaults to `1`.
    pub const fn align(self, align: usize) -> Self {
        Self { align, ..self }
    }

    /// Zero-initialize the hunk.
    pub const fn zeroed(self) -> CfgHunkBuilder<System, T, ZeroInitTag>
    where
        T: ZeroInit,
    {
        // Safety: `T: ZeroInit`, so it's zero-initializable
        unsafe { self.zeroed_unchecked() }
    }

    /// Zero-initialize the hunk even if it might be unsafe.
    ///
    /// # Safety
    ///
    /// If zero initialization is not a valid bit pattern for `T`, accessing the
    /// hunk's contents may result in an undefined behavior.
    pub const unsafe fn zeroed_unchecked(self) -> CfgHunkBuilder<System, T, ZeroInitTag> {
        CfgHunkBuilder {
            _phantom: PhantomData,
            len: self.len,
            align: self.align,
        }
    }
}

impl<System: Kernel, T, InitTag: HunkIniter<T>> CfgHunkBuilder<System, T, InitTag> {
    /// Complete the definition of a hunk, returning a reference to the hunk.
    pub const fn finish(self, cfg: &mut CfgBuilder<System>) -> Hunk<System, T> {
        let untyped_hunk = kernel::Hunk::<System>::build()
            .len(mem::size_of::<T>())
            .align(max(mem::align_of::<T>(), self.align))
            .finish(cfg);

        assert!(self.len == 1, "Non-array hunk must have `len` of `1`");

        let start = untyped_hunk.offset();

        // Insert an initializer
        if InitTag::NEEDS_INIT {
            unsafe {
                StartupHook::build()
                    .priority(INIT_HOOK_PRIORITY)
                    .start(|start| {
                        let untyped_hunk = kernel::Hunk::<System>::from_offset(start).as_ptr();
                        // Safety: The destination is large enough to contain `T`
                        InitTag::init(&mut *(untyped_hunk as *mut mem::MaybeUninit<T>));
                    })
                    .unchecked()
                    .param(start)
                    .finish(cfg);
            }
        }

        Hunk {
            offset: start as _,
            _phantom: PhantomData,
        }
    }
}

impl<System: Port, T, InitTag: HunkIniter<T>> CfgHunkBuilder<System, [T], InitTag> {
    /// Complete the definition of a hunk, returning a reference to the hunk.
    pub const fn finish(self, cfg: &mut CfgBuilder<System>) -> Hunk<System, [T]> {
        assert!(self.align.is_power_of_two(), "`align` is not power of two");

        let untyped_hunk = kernel::Hunk::<System>::build()
            .len(mem::size_of::<T>() * self.len)
            .align(max(mem::align_of::<T>(), self.align))
            .finish(cfg);

        let start = untyped_hunk.offset();

        // Insert an initializer
        if InitTag::NEEDS_INIT {
            // TODO: There is no way to pass a length into the initializer
            todo!();
        }

        Hunk {
            offset: slice_from_raw_parts_mut(start as _, self.len),
            _phantom: PhantomData,
        }
    }
}

impl<System, T> Init for Hunk<System, [T]> {
    // Safety: This is safe because it points to nothing
    const INIT: Self = Self {
        offset: slice_from_raw_parts_mut(core::ptr::null_mut(), 0),
        _phantom: PhantomData,
    };
}

impl<System: Kernel, T: fmt::Debug + ?Sized> fmt::Debug for Hunk<System, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Hunk")
            .field(&Self::as_ptr(*self))
            .field(&&**self)
            .finish()
    }
}

impl<System, T: ?Sized> Clone for Hunk<System, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<System, T: ?Sized> Copy for Hunk<System, T> {}

impl<System, T: ?Sized> Hunk<System, T> {
    /// Reinterpret the hunk as another type.
    ///
    /// # Safety
    ///
    ///  - Similarly to [`core::mem::transmute`], this is **incredibly** unsafe.
    ///  - The byte offset must be valid for the destination type.
    ///
    pub const unsafe fn transmute<U>(self) -> Hunk<System, U> {
        Hunk {
            offset: self.offset.cast(),
            _phantom: PhantomData,
        }
    }
}

impl<System: Kernel, T: ?Sized> Hunk<System, T> {
    /// Get the untyped hunk.
    #[inline]
    pub fn untyped_hunk(this: Self) -> kernel::Hunk<System> {
        kernel::Hunk::from_offset(this.offset as *const u8 as usize)
    }

    // FIXME: The following methods are not `const fn` on account of
    //        <https://github.com/rust-lang/const-eval/issues/11> being
    //        unresolved

    /// Get a raw pointer to the hunk's contents.
    #[inline]
    pub fn as_ptr(this: Self) -> *const T {
        this.offset.set_ptr_value(Self::untyped_hunk(this).as_ptr())
    }

    /// Get a raw pointer to the raw bytes of the hunk.
    #[inline]
    pub fn as_bytes_ptr(this: Self) -> *const [u8] {
        slice_from_raw_parts(Self::untyped_hunk(this).as_ptr(), mem::size_of_val(&*this))
    }

    /// Get a reference to the raw bytes of the hunk.
    ///
    /// # Safety
    ///
    /// The result might include uninitialized bytes and/or interior mutability,
    /// so it might be unsafe to access.
    #[inline]
    pub unsafe fn as_bytes(this: Self) -> &'static [u8] {
        // Safety: The caller is responsible for making sure interpreting the
        // contents as `[u8]` is safe
        unsafe { &*Self::as_bytes_ptr(this) }
    }
}

impl<System: Kernel, T: ?Sized> AsRef<T> for Hunk<System, T> {
    fn as_ref(&self) -> &T {
        unsafe { &*Self::as_ptr(*self) }
    }
}

impl<System: Kernel, T: ?Sized> Deref for Hunk<System, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

/// FIXME: `Ord::max` is not available in `const fn`
const fn max(x: usize, y: usize) -> usize {
    if x > y {
        x
    } else {
        y
    }
}
