//! Primitives for doing something exactly once.

use std::cell::UnsafeCell;
use std::fmt::{Debug, Display, Formatter};
use std::sync::atomic::{AtomicBool, Ordering};


/// A simple primitive for ensuring something is done exactly once. Not thread-safe.
///
/// e.g.
/// ```
/// let mut once_task = DoOnce::new();
/// loop {
///     once_task.do_once(|| {
///         // this closure only runs once
///     })
///     //...
/// }
/// ```
pub struct DoOnce(bool);
impl DoOnce {
    pub const fn new() -> Self { DoOnce(false) }

    /// The passed closure will be called only once, after that calling this will be a no-op.
    pub fn do_once<F: FnOnce()>(&mut self, func: F) {
        if !self.0 {
            func();
            self.0 = true;
        }
    }

    /// Returns true if the task has been run once already
    pub fn done(&self) -> bool { self.0 }
}


/// A simple primitive for ensuring something is done exactly once. DoOnceSync is thread-safe
/// and uses internal mutability, so you can `do_once` with an immutable reference.
///
/// e.g.
/// ```
/// let once_task = DoOnceSync::new();
/// loop {
///     once_task.do_once(|| {
///         // this closure only runs once
///     })
///     //...
/// }
/// ```
pub struct DoOnceSync(AtomicBool);
impl DoOnceSync {
    pub fn new() -> Self { DoOnceSync(AtomicBool::new(false)) }

    /// The passed closure will be called only once, after that calling this will be a no-op.
    pub fn do_once<F: FnOnce()>(&self, func: F) {
        let prev = self.0.swap(true, Ordering::SeqCst);
        if !prev {
            func();
        }
    }

    /// Returns true if the task has been run once already
    pub fn done(&self) -> bool { self.0.load(Ordering::SeqCst) }
}
unsafe impl Send for DoOnceSync {}
unsafe impl Sync for DoOnceSync {}


/// A simple once-initialized immutable-ish reference for easy global statics.
///
/// ```rs
/// static SOME_CONSTANT: InitOnce<SomeType> = SOME_CONSTANT::uninitialized();
/// SOME_CONSTANT.initialize(value);
/// SOME_CONSTANT.get()
/// ```
///
/// No `is_initialized` function is provided, since providing an accurate, thread-safe result is
/// is non-trivial, and you should never need to directly check whether the value is initialized.
/// If you aren't sure, use `get_or_init` if you want to initialize it, or `try_get` if not.
/// There is also no `Deref<T>` as implicitly getting an unchecked reference to the value is unsafe.
///
/// # Safety
///
/// InitOnce uses a lot of unsafe code internally to access the contents of the UnsafeCell,
/// but the end user doesn't need to worry about &mut aliasing because the API only exposes
/// immutable references.
pub struct InitOnce<T> {
    inner: UnsafeCell<Option<T>>,
    lock: AtomicBool,
}
impl<T> InitOnce<T> {
    /// Creates a new empty InitOnce. This `fn` is `const` so it can be used in statics.
    pub const fn uninitialized() -> Self {
        InitOnce { inner: UnsafeCell::new(None), lock: AtomicBool::new(false) }
    }

    /// Attempt to get a reference to the value contained within.
    /// Safely returns `None` if uninitialized or currently being initialized elsewhere.
    pub fn try_get(&self) -> Option<&T> {
        if self.lock.swap(true, Ordering::SeqCst) { return None }
        let inner: &Option<T> = unsafe { &*self.inner.get() };
        self.lock.store(false, Ordering::SeqCst);
        inner.as_ref()
    }

    /// Retrieves a reference to the contained value without checking. Panics if uninitialized.
    pub fn get(&self) -> &T {
        unsafe {
            let r = self.inner.get().as_ref().unwrap();
            match r {
                Some(r) => r,
                None => panic!("Tried to access InitOnce<{}> before initialization", std::any::type_name::<T>())
            }
        }
    }

    /// Retrieves a reference to the value contained within, calling the given closure to provide
    /// the initial value if uninitialized. Utilizes interior mutability so only `&self` is
    /// required. The closure will not be called if the value has already been initialized. Returns
    /// Err only if the value is currently being initialized on another thread, since we can neither
    /// initialize it ourselves nor return a valid reference. Always safe to `unwrap()` in a
    /// synchronous, single-threaded context.
    pub fn get_or_init<F>(&self, func: F) -> Result<&T, String> where F: Fn() -> T {
        let prev = self.lock.swap(true, Ordering::SeqCst);
        if prev {
            return Err(format!("Tried to initialize InitOnce<{}> twice at the same time", std::any::type_name::<T>()));
        }

        unsafe {
            if (*self.inner.get()).is_none() {
                self.initialize(func())?;
            }
        }
        self.lock.store(false, Ordering::SeqCst);
        Ok(self.get())
    }

    /// Inserts a value into this InitOnce if it's not already initialized.
    /// Utilizes interior mutability so only `&self` is required.
    /// If already initialized, ignores the new value and returns Err.
    pub fn initialize(&self, value: T) -> Result<(), String> {
        let prev = self.lock.swap(true, Ordering::SeqCst);
        if prev {
            return Err(format!("Tried to initialize InitOnce<{}> twice concurrently", std::any::type_name::<T>()));
        }
        unsafe {
            let ptr = self.inner.get();
            if (*ptr).is_some() {
                return Err(format!("Tried to initialize InitOnce<{}> a second time", std::any::type_name::<T>()));
            }
            ptr.write(Some(value));
        }
        self.lock.store(false, Ordering::SeqCst);
        Ok(())
    }
}

impl <T: Debug> Debug for InitOnce<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        T::fmt(self.get(), f)
    }
}

impl <T: Display> Display for InitOnce<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        T::fmt(self.get(), f)
    }
}

unsafe impl<T: Send> Send for InitOnce<T> {}
unsafe impl<T: Sync> Sync for InitOnce<T> {}
