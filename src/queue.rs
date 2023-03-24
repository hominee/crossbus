//! This module contains some core component
//! of message transition and recepition

use alloc::boxed::Box;
use core::{
    cell::UnsafeCell,
    hint, ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

struct Node<T> {
    next: AtomicPtr<Self>,
    inner: Option<T>,
}

pub(crate) enum NodeData<T> {
    /// no data node left in queue
    Empty,
    /// stored data in the queue
    Data(T),
    /// stored data cannot safely popped for some reason (other thread may be is accessing the queue and
    /// change its nodes but the whole process is not completed)
    ///
    Inconsistent,
}

pub(crate) struct Queue<T> {
    head: AtomicPtr<Node<T>>,
    tail: UnsafeCell<*mut Node<T>>,
}

unsafe impl<T: Send> Send for Queue<T> {}
unsafe impl<T: Sync> Sync for Queue<T> {}

impl<T> Node<T> {
    const fn new_null() -> *mut Self {
        //const NODE: Self = Node {
        //next: AtomicPtr::new(ptr::null_mut()),
        //inner: None,
        //};
        ptr::null_mut::<Self>()
    }

    unsafe fn new(value: Option<T>) -> *mut Self {
        Box::into_raw(Box::new(Self {
            next: AtomicPtr::new(ptr::null_mut()),
            inner: value,
        }))
    }
}

impl<T> Queue<T> {
    pub(crate) const fn new_null() -> Self {
        let node = Node::new_null();
        Queue {
            head: AtomicPtr::new(node),
            tail: UnsafeCell::new(node),
        }
    }

    pub(crate) fn assume_init(&self) -> bool {
        // to ensure that underlying node
        // is not null
        let tail = unsafe { *self.tail.get() };
        if self.head.load(Ordering::Acquire) == tail {
            // if null, replace it with empty node
            if tail.is_null() {
                let node = unsafe { Node::new(None) };
                self.head.store(node, Ordering::Release);
                unsafe {
                    *self.tail.get() = node;
                }
            }
        }
        // the queue is not empty, got node inside
        true
    }

    pub(crate) fn new() -> Self {
        let node = unsafe { Node::new(None) };
        Queue {
            head: AtomicPtr::new(node),
            tail: UnsafeCell::new(node),
        }
    }

    pub(crate) fn push(&self, value: T) {
        unsafe {
            let node = Node::new(Some(value));
            let dest = self.head.swap(node, Ordering::AcqRel);
            (*dest).next.store(node, Ordering::Release);
        }
    }

    pub(crate) unsafe fn try_pop(&self) -> NodeData<T> {
        let tail = *self.tail.get();
        let next = (*tail).next.load(Ordering::Acquire);
        if !next.is_null() {
            *self.tail.get() = next;
            assert!((*tail).inner.is_none());
            assert!((*next).inner.is_some());
            let ret = (*next).inner.take().unwrap();
            drop(Box::from_raw(tail));
            return NodeData::Data(ret);
        }

        if self.head.load(Ordering::Acquire) == tail {
            NodeData::Empty
        } else {
            NodeData::Inconsistent
        }
    }

    pub(crate) unsafe fn pop(&self) -> Option<T> {
        loop {
            match self.try_pop() {
                NodeData::Empty => return None,
                NodeData::Data(node) => return Some(node),
                NodeData::Inconsistent => {
                    hint::spin_loop();
                }
            }
        }
    }
}

impl<T> Drop for Queue<T> {
    fn drop(&mut self) {
        unsafe {
            let mut tail = *self.tail.get();
            while !tail.is_null() {
                let next = (*tail).next.load(Ordering::Relaxed);
                drop(Box::from_raw(tail));
                tail = next;
            }
        }
    }
}

#[test]
fn test_queue() {
    struct Se {
        state: i32,
    }

    let que: Queue<Se> = Queue::new_null();
    let b = que.assume_init();
    assert!(b);
    assert!(unsafe { que.pop() }.is_none());
    let se = Se { state: 0 };
    que.push(se);
    let se = Se { state: 1 };
    que.push(se);
    let data = unsafe { que.pop() };
    assert!(data.is_some());
    assert_eq!(data.unwrap().state, 0);
    let data = unsafe { que.pop() };
    assert!(data.is_some());
    assert_eq!(data.unwrap().state, 1);
}
