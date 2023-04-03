// use std::sync::atomic::AtomicPtr;

// use crate::gc::gc::Global;

// #[derive(Debug)]
// struct Node<T> {
//     next: AtomicPtr<Node<T>>,
//     data: T,
// }

// #[derive(Debug,Default)]
// pub struct TreiberStack<T,const CAP:usize=256>{
//     head:AtomicPtr<Node<T>>,
//     global:Global<T,CAP>
// }

// impl<T, const CAP:usize> TreiberStack<T, CAP> {
//     // pub fn push(&self, value: T) {
//     //     let boxed_new = Box::new(Node {
//     //         next: AtomicPtr::default(),
//     //         data: value,
//     //     });
//     //     let new = Box::into_raw(boxed_new);
//     //     unsafe {
//     //         (*new).next = AtomicPtr::new(new);
//     //     }
//     //     self.head.swap(new, Ordering::Relaxed);
//     // }
//     // pub unsafe fn pop(&self) -> Option<T> {
//     //     let head = self.head.load(Ordering::Relaxed);
//     //     if !head.is_null() {
//     //         let node_1 = unsafe { &*head }.next.load(Ordering::Relaxed);

//     //         self.head.store(node_1, Ordering::Relaxed);

//     //         let head = unsafe { Box::from_raw(head) };
//     //         Some(head.data)
//     //     } else {
//     //         None
//     //     }
//     // }
// }
