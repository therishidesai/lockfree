use core::alloc::Layout;
use core::{mem, ptr};
use std::alloc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::usize;

pub struct Buffer<T> {
    buf: *mut T,
    size: usize,
    write_ptr: AtomicUsize,
    read_ptr: AtomicUsize,
}

unsafe impl<T: Sync> Sync for Buffer<T> {}

pub struct Producer<T> {
    buf: Arc<Buffer<T>>
}

pub struct Consumer<T> {
    buf: Arc<Buffer<T>>
}

unsafe impl<T: Send> Send for Producer<T> {}
unsafe impl<T: Send> Send for Consumer<T> {}

impl<T> Buffer<T> {
    pub fn try_pop(&self) -> Option<T> {
        // Want the most up to date write, hence I need Ordering::Acquire
        let write_pos = self.write_ptr.load(Ordering::Acquire);
        let read_pos = self.read_ptr.load(Ordering::Relaxed);
        if read_pos == write_pos {
            return None
        }

        let val = unsafe {
            ptr::read(&*self.buf.offset((read_pos % self.size) as isize))
        };

        self.read_ptr.store(read_pos+1, Ordering::Release);
        return Some(val)
    }

    pub fn try_push(&self, val: T) -> Option<T> {
        // Want the most up to date read, hence I need Ordering::Acquire
        let read_pos = self.read_ptr.load(Ordering::Acquire);
        let write_pos = self.write_ptr.load(Ordering::Relaxed);

        if (read_pos + self.size) == write_pos {
            return Some(val)
        }

        unsafe {
            let node = self.buf.offset((write_pos % self.size) as isize);
            ptr::write(&mut *node, val);
        }

        self.write_ptr.store(write_pos+1, Ordering::Release);
        return None
    }
}

impl<T> Drop for Buffer<T> {
    fn drop(&mut self){
        // pop off the values
        while let Some(_) = self.try_pop() {}

        unsafe {
            let layout = Layout::from_size_align(
                self.size * mem::size_of::<T>(),
                mem::align_of::<T>(),
            ).unwrap();
            alloc::dealloc(self.buf as *mut u8, layout);
        }
    }
}

impl<T> Producer<T> {
    pub fn try_push(&self, val: T) -> Option<T> {
        self.buf.try_push(val)
    }
}

impl<T> Consumer<T> {
    pub fn try_pop(&self) -> Option<T> {
        self.buf.try_pop()
    }
}

pub fn new_queue<T>(size: usize) -> (Producer<T>, Consumer<T>) {

    let buf = unsafe {
        let layout = Layout::from_size_align(
            size * mem::size_of::<T>(),
            mem::align_of::<T>(),
        ).unwrap();
        let ptr = alloc::alloc(layout);

        if ptr.is_null() {
            alloc::handle_alloc_error(layout)
        } else {
            ptr as *mut T
        }
    };

    let b = Arc::new(Buffer {
        buf,
        size,
        write_ptr: AtomicUsize::new(0),
        read_ptr: AtomicUsize::new(0),
    });

    (
        Producer {
            buf: b.clone(),
        },

        Consumer {
            buf: b.clone(),
        }
    )
}
