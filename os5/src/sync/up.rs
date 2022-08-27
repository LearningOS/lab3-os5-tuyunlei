//! Uniprocessor interior mutability primitives

use alloc::format;
use alloc::string::String;
use core::cell::{BorrowMutError, RefCell, RefMut};
use core::fmt::Debug;
use core::ops::{Deref, DerefMut};

/// Wrap a static data structure inside it so that we are
/// able to access it without any `unsafe`.
///
/// We should only use it in uniprocessor.
///
/// In order to get mutable reference of inner data, call
/// `exclusive_access`.
pub struct UPSafeCell<T: Debug> {
    /// inner data
    name: String,
    inner: RefCell<T>,
}

unsafe impl<T: Debug> Sync for UPSafeCell<T> {}

impl<T: Debug> UPSafeCell<T> {
    /// User is responsible to guarantee that inner struct is only used in
    /// uniprocessor.
    pub unsafe fn new(value: T) -> Self {
        let name = format!("{:?}", value);
        Self {
            name,
            inner: RefCell::new(value),
        }
    }

    pub fn exclusive_access(&self) -> RefMutWrapper<'_, T> {
        let inner = self.inner.try_borrow_mut();
        if let Ok(inner) = inner {
            if self.name == "TCB_Inner(ch5_usertest)" {
                // println!("borrowed");
            }
            RefMutWrapper(inner, self.name.clone())
        } else {
            panic!("[{}] has been borrowed", self.name);
        }
    }
}

pub struct RefMutWrapper<'a, T: Debug>(
    RefMut<'a, T>,
    String,
);

impl<T: Debug> Deref for RefMutWrapper<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T: Debug> DerefMut for RefMutWrapper<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

impl<T: Debug> Drop for RefMutWrapper<'_, T> {
    fn drop(&mut self) {
        if self.1 == "TCB_Inner(ch5_usertest)" {
            // println!("release");
        }
    }
}
