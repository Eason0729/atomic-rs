use std::{
    cell::Cell,
    mem,
    sync::atomic::{fence, Ordering},
};

use super::{
    epoch::{AtomicEpoch, AtomicFlag, Epoch, Flag},
    stack::AtomicStack,
};

#[derive(Debug)]
struct Bag<T, const CAP: usize> {
    data: Vec<Box<T>>,
}

impl<T, const CAP: usize> Default for Bag<T, CAP> {
    fn default() -> Self {
        Self {
            data: Vec::with_capacity(CAP),
        }
    }
}

impl<T, const CAP: usize> Bag<T, CAP> {
    fn is_full(&self) -> bool {
        self.data.len() == CAP
    }
    fn push(&mut self, value: Box<T>) {
        self.data.push(value);
    }
}

#[derive(Debug, Default)]
pub struct Global<T, const CAP: usize=128> {
    epoch: AtomicEpoch,
    bags: [AtomicStack<Bag<T, CAP>>; 3],
    flags: AtomicStack<AtomicFlag>,
}

impl<T, const CAP: usize> Global<T, CAP> {
    pub fn register<'a>(&'a self) -> Local<'a, T, CAP> {
        let flag = self.flags.push(Default::default());
        debug_assert_eq!(flag.load(Ordering::Relaxed), Flag::default());
        let local = Local {
            bag: Default::default(),
            flag: &flag,
            global: &self,
        };
        local
    }
    #[cold]
    unsafe fn migrate(&self, guard: &PinGuard, bag: Bag<T, CAP>) {
        self.bags[guard.epoch as usize].push(bag);

        let epoch = self.epoch.load(Ordering::Relaxed);
        fence(Ordering::SeqCst);

        if let Some(stack_guard) = self.flags.try_own() {
            for flag in self.flags.into_iter(&stack_guard) {
                if flag.load(Ordering::Acquire) == Flag::from_epoch(epoch.decrease()) {
                    return;
                }
            }
            let grabages = &self.bags[epoch.decrease() as usize];
            while grabages.boxed_pop().is_some() {}

            fence(Ordering::Acquire);
            self.epoch.store(epoch.increase(), Ordering::Release);
        }
    }
}

pub struct PinGuard<'a> {
    epoch: Epoch,
    flag: &'a AtomicFlag,
}

impl<'a> Drop for PinGuard<'a> {
    fn drop(&mut self) {
        self.flag.store(Flag::Unpin, Ordering::Relaxed);
    }
}

pub struct Local<'a, T, const CAP: usize> {
    bag: Cell<Bag<T, CAP>>,
    flag: &'a AtomicFlag,
    global: &'a Global<T, CAP>,
}

impl<'a, T, const CAP: usize> Local<'a, T, CAP> {
    #[inline]
    pub fn pin(&'a self) -> PinGuard<'a> {
        debug_assert_eq!(
            self.flag.load(Ordering::Relaxed),
            Flag::Unpin,
            "Local was expected to be unpinned"
        );
        let epoch = self.global.epoch.load(Ordering::Relaxed);

        self.flag.store(Flag::from_epoch(epoch), Ordering::SeqCst);
        fence(Ordering::SeqCst);

        PinGuard {
            epoch,
            flag: &self.flag,
        }
    }
    pub fn migrate(&self, guard: &PinGuard, garbage: Box<T>) {
        let bag = unsafe { &mut *self.bag.as_ptr() };

        bag.push(garbage);
        if bag.is_full() {
            let mut old = Bag::default();
            mem::swap(&mut old, bag);
            unsafe {
                self.global.migrate(guard, old);
            }
        }
    }
}

#[cfg(test)]
pub mod test {
    use std::{sync::Mutex, thread};

    use super::Global;

    #[test]
    fn gc_one() {
        let global: Global<usize, 1> = Global::default();
        let local = global.register();

        let guard = local.pin();
        for _ in 0..100 {
            local.migrate(&guard, Box::new(0_usize));
        }
        drop(guard);
    }
    #[test]
    // #[ignore = "tested, time-consuming"]
    fn gc_multiple() {
        let global: Global<usize, 1> = Global::default();

        let mut handles = Vec::new();
        for _ in 0..30 {
            handles.push(global.register());
        }
        let handles = Mutex::new(handles);

        thread::scope(|s| {
            for _ in 0..30 {
                s.spawn(|| {
                    let mut lock = handles.lock().unwrap();
                    let local = lock.pop().unwrap();
                    drop(lock);
                    for i in 0..500 {
                        let guard = local.pin();
                        local.migrate(&guard, Box::new(i % 3));
                        drop(guard);
                    }
                });
            }
        });
    }
    #[test]
    #[ignore = "datarace"]
    fn gc_onfly_register() {
        let global: Global<usize, 1> = Global::default();

        thread::scope(|s| {
            for _ in 0..10 {
                s.spawn(|| {
                    let local = global.register();
                    for i in 0..1000 {
                        let guard = local.pin();
                        local.migrate(&guard, Box::new(i % 3));
                        drop(guard);
                    }
                });
            }
        });
    }
}
