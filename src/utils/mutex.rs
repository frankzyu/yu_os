use core::sync::atomic::{AtomicBool, Ordering};
use core::hint::spin_loop;
use core::cell::UnsafeCell;
use core::marker::Sync;
use core::ops::{Drop, Deref, DerefMut};
use core::fmt;
use core::option::Option::{self, None, Some};
use core::default::Default;

pub struct Mutex<T: ?Sized>
{
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

#[derive(Debug)]
pub struct MutexGuard<'a, T: ?Sized + 'a>
{
    lock: &'a AtomicBool,
    data: &'a mut T,
}

unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}

impl<T> Mutex<T>
{
    pub const fn new(user_data: T) -> Mutex<T>
    {
        Mutex
        {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(user_data),
        }
    }

    pub fn into_inner(self) -> T {
        let Mutex { data, .. } = self;
        data.into_inner()
    }
}

impl<T: ?Sized> Mutex<T>
{
    fn obtain_lock(&self)
    {
while self.lock.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
    while self.lock.load(Ordering::Relaxed) {
        spin_loop();
    }
}
    }

    pub fn lock(&self) -> MutexGuard<T>
    {
        self.obtain_lock();
        MutexGuard
        {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() },
        }
    }
    pub unsafe fn force_unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }
    pub fn try_lock(&self) -> Option<MutexGuard<T>>
    {
        if self.lock.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            Some(MutexGuard {
                lock: &self.lock,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            None
        }
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Mutex<T>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        match self.try_lock()
        {
            Some(guard) => write!(f, "Mutex {{ data: ")
				.and_then(|()| (&*guard).fmt(f))
				.and_then(|()| write!(f, "}}")),
            None => write!(f, "Mutex {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized + Default> Default for Mutex<T> {
    fn default() -> Mutex<T> {
        Mutex::new(Default::default())
    }
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T>
{
    type Target = T;
    fn deref<'b>(&'b self) -> &'b T { &*self.data }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T>
{
    fn deref_mut<'b>(&'b mut self) -> &'b mut T { &mut *self.data }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T>
{
    fn drop(&mut self)
    {
        self.lock.store(false, Ordering::Release);
    }
}
