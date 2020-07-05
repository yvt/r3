use parking_lot::Mutex;
use std::{
    mem::MaybeUninit,
    os::raw::c_int,
    ptr::null_mut,
    sync::{
        atomic::{AtomicPtr, Ordering},
        Arc,
    },
    thread,
};

pub use self::thread::ThreadId;

pub unsafe fn exit_thread() -> ! {
    unsafe {
        libc::pthread_exit(std::ptr::null_mut());
    }
}

/// [`std::thread::JoinHandle`] with extra functionalities.
#[derive(Debug)]
pub struct JoinHandle<T> {
    std_handle: thread::JoinHandle<T>,
    thread: Thread,
}

/// Spawn a new thread.
pub fn spawn<T: 'static + Send>(f: impl FnOnce() -> T + Send + 'static) -> JoinHandle<T> {
    let parent_thread = thread::current();

    let data = Arc::new(ThreadData::new());
    let data2 = Arc::clone(&data);

    let std_handle = thread::spawn(move || {
        // Set up a destructor for `THREAD_DATA`
        THREAD_DATA_DTOR.with(|_| {});

        // Move `data2` into `THREAD_DATA`
        THREAD_DATA.store(Arc::into_raw(data2) as _, Ordering::Relaxed);

        parent_thread.unpark();
        drop(parent_thread);

        f()
    });

    let thread = Thread {
        std_thread: std_handle.thread().clone(),
        data,
    };

    // Wait until the just-spawned thread configures its own `THREAD_DATA`.
    thread::park();

    JoinHandle { std_handle, thread }
}

impl<T> JoinHandle<T> {
    pub fn thread(&self) -> &Thread {
        &self.thread
    }
}

// Avoid `pthread_getspecific`, which is not defined as async-signal-safe by
// the POSIX standard.
#[thread_local]
static THREAD_DATA: AtomicPtr<ThreadData> = AtomicPtr::new(null_mut());

// Releases `ThreadData` on thread exit.
thread_local! {
    static THREAD_DATA_DTOR: ThreadDataDestructor = ThreadDataDestructor;
}

struct ThreadDataDestructor;

impl Drop for ThreadDataDestructor {
    fn drop(&mut self) {
        // Take `Arc<_>` back from `THREAD_DATA`.
        let ptr = THREAD_DATA.swap(null_mut(), Ordering::Relaxed);
        if !ptr.is_null() {
            unsafe { Arc::from_raw(ptr) };
        }
    }
}

/// [`std::thread::Thread`] with extra functionalities.
#[derive(Debug, Clone)]
pub struct Thread {
    std_thread: thread::Thread,
    data: Arc<ThreadData>,
}

#[derive(Debug)]
struct ThreadData {
    park_sock: [c_int; 2],
    park_lock: Mutex<()>,
}

impl ThreadData {
    fn new() -> Self {
        let park_sock = unsafe {
            let mut park_sock = MaybeUninit::uninit();
            ok_or_errno(libc::socketpair(
                libc::PF_LOCAL,
                libc::SOCK_STREAM,
                0,
                park_sock.as_mut_ptr() as _,
            ))
            .unwrap();
            park_sock.assume_init()
        };

        let this = Self {
            park_sock,
            park_lock: Mutex::new(()),
        };

        // Enable non-blocking I/O
        ok_or_errno(unsafe {
            libc::fcntl(
                this.park_sock_token_source(),
                libc::F_SETFL,
                libc::O_NONBLOCK,
            )
        })
        .unwrap();

        this
    }

    /// Get the FD to read a park token.
    fn park_sock_token_source(&self) -> c_int {
        self.park_sock[0]
    }

    /// Get the FD to write a park token.
    fn park_sock_token_sink(&self) -> c_int {
        self.park_sock[1]
    }
}

impl Drop for ThreadData {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.park_sock[0]);
            libc::close(self.park_sock[1]);
        }
    }
}

pub fn current() -> Thread {
    let data_ptr = THREAD_DATA.load(Ordering::Relaxed);

    let data = if data_ptr.is_null() {
        // The current thread was created in some other way. Construct
        // `ThreadData` now.
        let data = Arc::new(ThreadData::new());
        let data2 = Arc::clone(&data);
        THREAD_DATA.store(Arc::into_raw(data2) as _, Ordering::Relaxed);

        // Set up a destructor for `THREAD_DATA`
        THREAD_DATA_DTOR.with(|_| {});

        data
    } else {
        let data = std::mem::ManuallyDrop::new(unsafe { Arc::from_raw(data_ptr) });
        Arc::clone(&data)
    };

    Thread {
        std_thread: thread::current(),
        data,
    }
}

pub fn park() {
    let current = current();
    park_inner(&current.data);
}

fn park_inner(data: &ThreadData) {
    let mut buf = 0u8;

    loop {
        // Block the current thread until the token becomes available
        let mut pollfd = libc::pollfd {
            fd: data.park_sock_token_source(),
            events: libc::POLLRDNORM,
            revents: 0,
        };
        let count = ok_or_errno(unsafe { libc::poll(&mut pollfd, 1, c_int::MAX) }).unwrap();

        if count == 0 {
            // It's not available yet. Start waiting again
            continue;
        }

        // Take the token
        match isize_ok_or_errno(unsafe {
            libc::recv(
                data.park_sock_token_source(),
                (&mut buf) as *mut _ as _,
                1,
                0,
            )
        }) {
            Ok(1) => {}
            Ok(0) | Err(errno::Errno(libc::EAGAIN)) => {
                // It was a spurious wakeup (this can be caused by how `unpark`
                // is implemented). Try again.
                continue;
            }
            Ok(i) => panic!("unexpected return value: {}", i),
            Err(e) => panic!("failed to evict park token: {}", e),
        }

        break;
    }
}

impl Thread {
    pub fn id(&self) -> ThreadId {
        self.std_thread.id()
    }

    pub fn unpark(&self) {
        let data = &self.data;
        let _guard = data.park_lock.lock();

        let mut buf = 0u8;

        // Discard any preexisting token. There can be only up to one token
        // because of the use of `park_lock`.
        match isize_ok_or_errno(unsafe {
            libc::recv(
                data.park_sock_token_source(),
                (&mut buf) as *mut _ as _,
                1,
                0,
            )
        }) {
            Ok(_) | Err(errno::Errno(libc::EAGAIN)) => {}
            Err(e) => panic!("failed to evict park token: {}", e),
        }

        // Make a token available
        isize_ok_or_errno(unsafe {
            libc::send(data.park_sock_token_sink(), (&buf) as *const _ as _, 1, 0)
        })
        .unwrap();
    }

    // TODO: `park` (remote park)
}

fn isize_ok_or_errno(x: isize) -> Result<isize, errno::Errno> {
    if x >= 0 {
        Ok(x)
    } else {
        Err(errno::errno())
    }
}

fn ok_or_errno(x: c_int) -> Result<c_int, errno::Errno> {
    if x >= 0 {
        Ok(x)
    } else {
        Err(errno::errno())
    }
}
