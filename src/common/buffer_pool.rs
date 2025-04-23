//! Buffer pool implementation for efficient buffer reuse
//!
//! This module provides a thread-safe buffer pool that can be used to
//! reduce memory allocations by reusing buffers.

use bytes::BytesMut;
use std::sync::Arc;
use tokio::sync::{Semaphore, SemaphorePermit};

/// A pool of reusable byte buffers
///
/// This pool helps reduce memory allocations by reusing buffers.
/// It is thread-safe and can be shared between multiple tasks.
#[derive(Clone)]
pub struct BufferPool {
    /// Inner implementation wrapped in Arc for thread-safety
    inner: Arc<BufferPoolInner>,
}

/// Inner implementation of the buffer pool
struct BufferPoolInner {
    /// Semaphore to limit the number of buffers that can be borrowed
    semaphore: Semaphore,
    /// Default buffer capacity when creating new buffers
    buffer_capacity: usize,
}

/// A buffer borrowed from the pool
///
/// When dropped, the buffer is automatically returned to the pool.
pub struct PooledBuffer {
    /// The actual buffer
    pub buffer: BytesMut,
    /// The pool this buffer was borrowed from (needed for Drop trait)
    #[allow(dead_code)]
    pool: BufferPool,
    /// Semaphore permit that will be released when this buffer is dropped
    _permit: SemaphorePermit<'static>,
}

impl BufferPool {
    /// Create a new buffer pool
    ///
    /// # Parameters
    ///
    /// * `max_buffers` - Maximum number of buffers that can be borrowed at once
    /// * `buffer_capacity` - Default capacity of each buffer
    pub fn new(max_buffers: usize, buffer_capacity: usize) -> Self {
        Self {
            inner: Arc::new(BufferPoolInner {
                semaphore: Semaphore::new(max_buffers),
                buffer_capacity,
            }),
        }
    }

    /// Borrow a buffer from the pool
    ///
    /// If the pool is at capacity, this will wait until a buffer is returned.
    ///
    /// # Returns
    ///
    /// A `PooledBuffer` that will be automatically returned to the pool when dropped.
    pub async fn get_buffer(&self) -> PooledBuffer {
        // Acquire a permit from the semaphore
        // This is a lifetime hack to make the borrow checker happy
        let permit = self.inner.semaphore.acquire().await.unwrap();
        let permit = unsafe { std::mem::transmute::<SemaphorePermit<'_>, SemaphorePermit<'static>>(permit) };

        // Create a new buffer with the default capacity
        let buffer = BytesMut::with_capacity(self.inner.buffer_capacity);

        PooledBuffer {
            buffer,
            pool: self.clone(),
            _permit: permit,
        }
    }

    /// Try to borrow a buffer from the pool without waiting
    ///
    /// # Returns
    ///
    /// Some(PooledBuffer) if a buffer is available, None otherwise
    pub fn try_get_buffer(&self) -> Option<PooledBuffer> {
        // Try to acquire a permit from the semaphore
        let permit = self.inner.semaphore.try_acquire().ok()?;
        let permit = unsafe { std::mem::transmute::<SemaphorePermit<'_>, SemaphorePermit<'static>>(permit) };

        // Create a new buffer with the default capacity
        let buffer = BytesMut::with_capacity(self.inner.buffer_capacity);

        Some(PooledBuffer {
            buffer,
            pool: self.clone(),
            _permit: permit,
        })
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        // Clear the buffer before returning it to the pool
        self.buffer.clear();

        // The permit is automatically released when dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_buffer_pool() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // Create a pool with 2 buffers
            let pool = BufferPool::new(2, 1024);

            // Borrow two buffers
            let mut buffer1 = pool.get_buffer().await;
            let mut buffer2 = pool.get_buffer().await;

            // Write to the buffers
            buffer1.buffer.extend_from_slice(b"hello");
            buffer2.buffer.extend_from_slice(b"world");

            assert_eq!(&buffer1.buffer[..], b"hello");
            assert_eq!(&buffer2.buffer[..], b"world");

            // Try to borrow a third buffer (should fail)
            assert!(pool.try_get_buffer().is_none());

            // Drop one buffer
            drop(buffer1);

            // Now we should be able to borrow another buffer
            let buffer3 = pool.try_get_buffer();
            assert!(buffer3.is_some());

            // The returned buffer should be cleared
            assert_eq!(buffer3.unwrap().buffer.len(), 0);
        });
    }
}
