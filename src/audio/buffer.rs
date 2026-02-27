//! Lock-free double buffer for passing state from a writer thread to a reader thread.
//!
//! The writer prepares new state and atomically swaps the pointer.
//! The reader always reads from the current pointer without blocking.

use std::sync::atomic::{AtomicPtr, Ordering};

/// Lock-free double buffer.
///
/// Writer (main thread) prepares new state and calls [`swap`](DoubleBuffer::swap).
/// Reader (audio thread) calls [`get`](DoubleBuffer::get) to access the current state.
pub struct DoubleBuffer<T> {
    ptr: AtomicPtr<T>,
}

impl<T> DoubleBuffer<T> {
    /// Create a new double buffer with the given initial value.
    pub fn new(initial: T) -> Self {
        let boxed = Box::new(initial);
        Self {
            ptr: AtomicPtr::new(Box::into_raw(boxed)),
        }
    }

    /// Atomically swap in new state, returning the old state.
    ///
    /// The caller owns the returned `Box<T>` and is responsible for dropping it.
    /// This should be called from the writer thread (main thread) so the drop
    /// happens off the audio thread.
    pub fn swap(&self, new: Box<T>) -> Box<T> {
        let new_ptr = Box::into_raw(new);
        let old_ptr = self.ptr.swap(new_ptr, Ordering::AcqRel);
        // SAFETY: old_ptr was previously created via Box::into_raw and is no longer
        // accessed by the reader after the atomic swap completes.
        unsafe { Box::from_raw(old_ptr) }
    }

    /// Get a reference to the current state.
    ///
    /// # Safety
    ///
    /// Must only be called from the audio thread (single reader). The caller must
    /// ensure no concurrent call to [`swap`](DoubleBuffer::swap) invalidates the
    /// reference while it is in use. In practice this is safe because the audio
    /// callback is single-threaded and `swap` replaces the pointer atomically —
    /// the old pointer remains valid until the returned `Box` from `swap` is dropped.
    pub unsafe fn get(&self) -> &T {
        let ptr = self.ptr.load(Ordering::Acquire);
        &*ptr
    }
}

// SAFETY: The AtomicPtr provides the necessary synchronization.
// The writer and reader threads coordinate via atomic operations.
unsafe impl<T: Send> Send for DoubleBuffer<T> {}
unsafe impl<T: Send> Sync for DoubleBuffer<T> {}

impl<T> Drop for DoubleBuffer<T> {
    fn drop(&mut self) {
        let ptr = *self.ptr.get_mut();
        if !ptr.is_null() {
            // SAFETY: ptr was created via Box::into_raw and we own it.
            unsafe {
                drop(Box::from_raw(ptr));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_double_buffer_initial_value() {
        let buf = DoubleBuffer::new(42u32);
        let val = unsafe { buf.get() };
        assert_eq!(*val, 42);
    }

    #[test]
    fn test_double_buffer_swap() {
        let buf = DoubleBuffer::new(1u32);
        let _old = buf.swap(Box::new(2));
        let val = unsafe { buf.get() };
        assert_eq!(*val, 2);
    }

    #[test]
    fn test_double_buffer_returns_old() {
        let buf = DoubleBuffer::new(10u32);
        let old = buf.swap(Box::new(20));
        assert_eq!(*old, 10);
    }

    #[test]
    fn test_double_buffer_multiple_swaps() {
        let buf = DoubleBuffer::new(String::from("first"));

        let old = buf.swap(Box::new(String::from("second")));
        assert_eq!(*old, "first");

        let old = buf.swap(Box::new(String::from("third")));
        assert_eq!(*old, "second");

        let val = unsafe { buf.get() };
        assert_eq!(*val, "third");
    }

    #[test]
    fn test_double_buffer_with_vec() {
        let buf = DoubleBuffer::new(vec![1.0f32, 2.0, 3.0]);
        let val = unsafe { buf.get() };
        assert_eq!(*val, vec![1.0, 2.0, 3.0]);

        let old = buf.swap(Box::new(vec![4.0, 5.0, 6.0]));
        assert_eq!(*old, vec![1.0, 2.0, 3.0]);

        let val = unsafe { buf.get() };
        assert_eq!(*val, vec![4.0, 5.0, 6.0]);
    }
}
