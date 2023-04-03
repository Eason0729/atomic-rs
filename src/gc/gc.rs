use std::{cell::Cell, mem};

use super::{
    epoch::{AtomicEpoch, AtomicFlag, Epoch, Flag},
    stack::{AtomicStack},
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
pub struct Global<T, const CAP: usize> {
    epoch: AtomicEpoch,
    bags: [AtomicStack<Bag<T, CAP>>; 3],
    flags: AtomicStack<AtomicFlag>,
}

impl<T, const CAP: usize> Global<T, CAP> {
    pub fn register<'a>(&'a self) -> Local<'a, T, CAP> {
        let flag = self.flags.push(Default::default());
        debug_assert_eq!(flag.load(), Flag::default());
        let local = Local {
            bag: Default::default(),
            flag: &flag,
            global: &self,
        };
        local
    }
    unsafe fn migrate(&self, guard: &PinGuard, bag: Bag<T, CAP>) {
        self.bags[guard.epoch as usize].push(bag);
        if let Some(stack_guard) = self.flags.try_own() {
            for flag in self.flags.into_iter(&stack_guard) {
                if flag.load() == Flag::Unpin {
                    return;
                }
            }
            let grabages = &self.bags[guard.epoch.decrease() as usize];
            while grabages.boxed_pop().is_some() {}
            self.epoch.store(guard.epoch.increase());
        }
    }
}

pub struct PinGuard<'a> {
    epoch: Epoch,
    flag: &'a AtomicFlag,
}

impl<'a> Drop for PinGuard<'a> {
    fn drop(&mut self) {
        self.flag.store(Flag::Unpin);
    }
}

pub struct Local<'a, T, const CAP: usize> {
    bag: Cell<Bag<T, CAP>>,
    flag: &'a AtomicFlag,
    global: &'a Global<T, CAP>,
}

impl<'a, T, const CAP: usize> Local<'a, T, CAP> {
    pub fn pin(&'a self) -> PinGuard<'a> {
        let epoch = self.global.epoch.load();

        self.flag
            .compare_and_swap(Flag::Unpin, Flag::from_epoch(epoch));

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
    use super::Global;

    #[test]
    fn gc_leak() {
        let global: Global<usize, 1> = Global::default();
        let local = global.register();

        let guard = local.pin();
        local.migrate(&guard, Box::new(0_usize));
        drop(guard);
    }
}
