// use core::sync::atomic::{AtomicBool, Ordering};
// use core::cell::UnsafeCell;
// use core::hint::spin_loop;
// use core::ops::{Deref, DerefMut, Drop};
// use core::marker::Sync;

// pub struct Spinlock<T>
// where
//     T: Send,        
// {
//     lock: AtomicBool,
//     data: UnsafeCell<T>,
// }

// pub struct SpinlockGuard<'a, T: 'a>
// {
//     lock: &'a AtomicBool,
//     data: &'a mut T,
// }

// unsafe impl<T: Send> Sync for Spinlock<T> {}

// pub const INIT_STATIC_SPINLOCK: Spinlock<()> = Spinlock {
//     lock: AtomicBool::new(false),
//     data: UnsafeCell::new(()),
// };

// impl<T: Send> Spinlock<T>
// {
//     pub fn new(user_data: T) -> Spinlock<T>
//     {
//         Spinlock {
//             lock: AtomicBool::new(false),
//             data: UnsafeCell::new(user_data),
//         }
//     }

//     fn obtain_lock(&self)
//     {
//         while self.lock.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err()
//         {
//             // Simple back-off strategy to reduce contention
//             for _ in 0..10 {
//                 spin_loop();
//             }
//         }
//     }

//     pub fn lock(&self) -> SpinlockGuard<T>
//     {
//         self.obtain_lock();
//         SpinlockGuard {
//             lock: &self.lock,
//             data: unsafe { &mut *self.data.get() },
//         }
//     }
// }

// impl<'a, T> Deref for SpinlockGuard<'a, T>
// {
//     type Target = T;
//     fn deref<'b>(&'b self) -> &'b T { &*self.data }
// }

// impl<'a, T> DerefMut for SpinlockGuard<'a, T>
// {
//     fn deref_mut<'b>(&'b mut self) -> &'b mut T { &mut *self.data }
// }

// impl<'a, T> Drop for SpinlockGuard<'a, T>
// {
//     fn drop(&mut self)
//     {
//         self.lock.store(false, Ordering::Release);
//     }
// }
