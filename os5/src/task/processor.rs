//! Implementation of [`Processor`] and Intersection of control flow
//!
//! Here, the continuous operation of user apps in CPU is maintained,
//! the current running state of CPU is recorded,
//! and the replacement and transfer of control flow of different applications are executed.


use alloc::sync::Arc;
use core::fmt::{Debug, Formatter};

use lazy_static::*;

use crate::sync::UPSafeCell;
use crate::timer::get_time_ms;
use crate::trap::TrapContext;

use super::{fetch_task, TaskStatus};
use super::{TaskContext, TaskControlBlock};
use super::__switch;

/// Processor management structure
pub struct Processor {
    /// The task currently executing on the current processor
    current: Option<Arc<TaskControlBlock>>,
    /// The basic control flow of each core, helping to select and switch process
    idle_task_cx: TaskContext,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }
    #[inline]
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
    #[inline]
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }
    #[inline]
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(|task| Arc::clone(task))
    }
}

impl Debug for Processor {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "PROCESSOR")
    }
}

lazy_static! {
    /// PROCESSOR instance through lazy_static!
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

/// The main part of process execution and scheduling
///
/// Loop fetch_task to get the process that needs to run,
/// and switch the process through __switch
pub fn run_tasks() {
    loop {
        // println!("[kernel] main loop");
        let mut processor = PROCESSOR.exclusive_access();
        // println!("[kernel] got processor");
        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // access coming task TCB exclusively
            let mut task_inner = task.inner_exclusive_access();
            // println!("[kernel] running task pid={} name={}", task.pid.0, task_inner.name);
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;
            if task_inner.start_time_ms == 0 {
                task_inner.start_time_ms = get_time_ms();
            }
            drop(task_inner);
            // release coming task TCB manually
            processor.current = Some(task);
            // release processor manually
            drop(processor);
            unsafe {
                // println!("start execute");
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
                // println!("switch back");
            }
        }
    }
}

/// Get current task through take, leaving a None in its place
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

/// Get a copy of the current task
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

/// Get token of the address space of current task
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let token = task.inner_exclusive_access().get_user_token();
    token
}

/// Get the mutable reference to trap context of current task
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}

/// Return to idle control flow for new scheduling
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}
