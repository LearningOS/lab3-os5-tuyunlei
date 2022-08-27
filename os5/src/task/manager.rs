//! Implementation of [`TaskManager`]
//!
//! It is only used to manage processes and schedule process based on ready queue.
//! Other CPU process monitoring functions are in Processor.


use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter};
use core::iter::Map;
use lazy_static::*;

pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

// YOUR JOB: FIFO->Stride
/// A simple FIFO scheduler.
impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }

    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let task = self.ready_queue.pop_front()?;
        let mut inner = task.inner_exclusive_access();
        inner.stride += 6469693230 / inner.priority;
        drop(inner);
        Some(task)
        // let mut target: Option<(isize, usize)> = None;
        // for (index, task) in self.ready_queue.iter_mut().enumerate() {
        //     let inner = task.inner_exclusive_access();
        //     if let Some(t) = &target {
        //         if inner.stride < t.0 {
        //             target = Some((inner.priority, index));
        //         }
        //     } else {
        //         target = Some((inner.priority, index))
        //     }
        // }
        // self.ready_queue.remove(target?.1).map(|task| {
        //     let mut inner = task.inner_exclusive_access();
        //     inner.stride += 6469693230 / inner.priority;
        //     drop(inner);
        //     task
        // })
    }
}

impl Debug for TaskManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "TaskManager")
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}
