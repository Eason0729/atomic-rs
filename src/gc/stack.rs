use std::{
    mem,
    ops::Deref,
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
};

#[derive(Debug)]
struct Node<T> {
    next: AtomicPtr<Node<T>>,
    data: *mut T,
}

pub struct StackGuard<'a, T>(&'a AtomicStack<T>);

impl<'a, T> Deref for StackGuard<'a, T> {
    type Target = AtomicStack<T>;

    fn deref(&self) -> &Self::Target {
        unsafe { mem::transmute(self.0) }
    }
}

impl<'a, T> Drop for StackGuard<'a, T> {
    fn drop(&mut self) {
        self.0.is_taken.store(false, Ordering::Relaxed);
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct AtomicStack<T> {
    head: AtomicPtr<Node<T>>,
    is_taken: AtomicBool,
}

impl<T> Drop for AtomicStack<T> {
    fn drop(&mut self) {
        while unsafe { self.boxed_pop().is_some() } {}
    }
}

impl<T> Default for AtomicStack<T> {
    fn default() -> Self {
        Self {
            head: Default::default(),
            is_taken: Default::default(),
        }
    }
}

impl<T> AtomicStack<T> {
    pub fn push<'a>(&'a self, value: T) -> &'a T {
        self.boxed_push(Box::new(value))
    }
    pub fn boxed_push<'a>(&'a self, value: Box<T>) -> &'a T {
        let value = Box::into_raw(value);
        let boxed_node = Box::new(Node {
            next: AtomicPtr::default(),
            data: value,
        });
        let node = Box::leak(boxed_node);

        // node.next = AtomicPtr::new(node);
        // self.head.swap(node, Ordering::Relaxed);
        let mut head=self.head.load(Ordering::Relaxed);
        node.next = AtomicPtr::new(head);
        while self.head.compare_exchange(head,node,Ordering::AcqRel, Ordering::Relaxed).is_err(){
            head=self.head.load(Ordering::Relaxed);
            node.next = AtomicPtr::new(head);
        };

        unsafe { &*(*node).data }
    }
    pub unsafe fn boxed_pop(&self) -> Option<Box<T>> {
        let popping_node_raw = self.head.load(Ordering::Relaxed);
        if popping_node_raw.is_null() {
            None
        } else {
            let popping_node = unsafe { &*popping_node_raw };
            let next_node = popping_node.next.load(Ordering::Relaxed);

            if self
                .head
                .compare_exchange(
                    popping_node_raw,
                    next_node,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                )
                .is_err()
            {
                return self.boxed_pop();
            }

            let popping_node = unsafe { Box::from_raw(popping_node_raw) };
            Some(Box::from_raw(popping_node.data))
        }
    }
    pub unsafe fn pop(&self) -> Option<T>
    where
        T: Copy,
    {
        self.boxed_pop().map(|x| x.as_ref().clone())
    }
    pub fn into_iter<'a>(&'a self,_guard:&StackGuard<T>) -> QueueIterator<'a, T> {
        QueueIterator {
            _stack: self,
            next: self.head.load(Ordering::Relaxed),
        }
    }
    pub unsafe fn try_own(&self) -> Option<StackGuard<T>> {
        if self
            .is_taken
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            Some(StackGuard(self))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct QueueIterator<'a, T> {
    _stack: &'a AtomicStack<T>,
    next: *mut Node<T>,
}

impl<'a, T> Iterator for QueueIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_null() {
            None
        } else {
            let node = unsafe { &*self.next };
            self.next = node.next.load(Ordering::Relaxed);
            Some(unsafe { &*node.data })
        }
    }
}

#[cfg(test)]
pub mod test {
    use std::sync::atomic::Ordering;

    use super::AtomicStack;

    #[test]
    fn internal_stack_empty() {
        let stack: AtomicStack<usize> = AtomicStack::default();
        assert_eq!(unsafe { stack.pop() }, None);
    }
    #[test]
    fn internal_stack_one() {
        let stack = AtomicStack::default();
        stack.push(0_usize);
        stack.push(0_usize);
        
        // trigger miri's detection
        unsafe{&*stack.head.load(Ordering::Relaxed)};

        assert_eq!(0, unsafe { stack.pop().unwrap() });
        assert_eq!(0, unsafe { stack.pop().unwrap() });
    }
    #[test]
    #[ignore = "not ready"]
    fn internal_stack_leak() {
        let stack = AtomicStack::default();
        for i in 0_usize..10 {
            stack.push(i);
        }
        let guard = unsafe { stack.try_own().unwrap() };
        let mut iter = stack.into_iter(&guard);
        for i in (0_usize..10).rev() {
            assert_eq!(i, *iter.next().unwrap())
        }
        for i in (0_usize..10).rev() {
            assert_eq!(i, unsafe { stack.pop().unwrap() })
        }
    }
}
