use std::{
    mem,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(usize)]
pub enum Epoch {
    Epoch0 = 0,
    Epoch1 = 1,
    Epoch2 = 2,
}

impl Default for Epoch {
    fn default() -> Self {
        Epoch::Epoch0
    }
}

impl Epoch {
    pub fn increase(self) -> Self {
        unsafe { mem::transmute((self as usize + 1) % 3) }
    }
    pub fn decrease(self) -> Self {
        unsafe { mem::transmute((self as usize + 2) % 3) }
    }
}

#[cfg(target_pointer_width = "64")]
#[repr(C, align(128))]
#[derive(Debug)]
pub struct AtomicEpoch(AtomicUsize);

#[cfg(not(target_pointer_width = "64"))]
#[derive(Debug, Default)]
pub struct AtomicEpoch(AtomicUsize);

impl AtomicEpoch {
    pub fn store(&self, epoch: Epoch) {
        self.0.store(epoch as usize, Ordering::Relaxed);
    }
    pub fn load(&self) -> Epoch {
        unsafe { mem::transmute(self.0.load(Ordering::Relaxed)) }
    }
    pub fn compare_and_swap(&self, epoch: Epoch) {
        let old = self.load() as usize;
        while self
            .0
            .compare_exchange(old, epoch as usize, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {}
    }
}

impl Default for AtomicEpoch {
    fn default() -> Self {
        Self(AtomicUsize::new(Epoch::default() as usize))
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(usize)]
pub enum Flag {
    Epoch0 = 0,
    Epoch1 = 1,
    Epoch2 = 2,
    Unpin = 3,
}

impl Flag {
    pub fn value(self) -> usize {
        debug_assert_ne!(self, Flag::Unpin);
        self as usize
    }
    pub fn from_epoch(epoch: Epoch) -> Self {
        unsafe { mem::transmute(epoch) }
    }
}

impl Default for Flag {
    fn default() -> Self {
        Flag::Unpin
    }
}

#[cfg(target_pointer_width = "64")]
#[repr(C, align(128))]
#[derive(Debug)]
pub struct AtomicFlag(AtomicUsize);

#[cfg(not(target_pointer_width = "64"))]
#[derive(Debug, Default)]
pub struct AtomicFlag(AtomicUsize);

impl AtomicFlag {
    pub fn store(&self, flag: Flag) {
        self.0.store(flag as usize, Ordering::Relaxed);
    }
    pub fn load(&self) -> Flag {
        unsafe { mem::transmute(self.0.load(Ordering::Relaxed)) }
    }
    pub fn compare_and_swap(&self, old: Flag, new: Flag) {
        while self
            .0
            .compare_exchange(
                old as usize,
                new as usize,
                Ordering::SeqCst,
                Ordering::Acquire,
            )
            .is_err()
        {}
    }
}

impl Default for AtomicFlag {
    fn default() -> Self {
        Self(AtomicUsize::new(Flag::default() as usize))
    }
}

#[cfg(test)]
pub mod test {
    use std::mem;

    use super::Flag;

    #[test]
    fn transmute_enum() {
        let a = 3_usize;
        let flag: Flag = unsafe { mem::transmute(a) };
        assert_eq!(flag, Flag::Unpin);
    }
}
