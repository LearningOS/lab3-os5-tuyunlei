//! Implementation of [`TaskManager`]
//!
//! It is only used to manage processes and schedule process based on ready queue.
//! Other CPU process monitoring functions are in Processor.


use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::{BinaryHeap, VecDeque};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::fmt::{Debug, Formatter};
use core::iter::Map;
use lazy_static::*;

struct StrideComparator(Arc<TaskControlBlock>);

impl Eq for StrideComparator {}

impl PartialEq<Self> for StrideComparator {
    fn eq(&self, other: &Self) -> bool {
        let stride1 = self.0.inner_exclusive_access().stride;
        let stride2 =other.0.inner_exclusive_access().stride;
        stride1 == stride2
    }
}

impl PartialOrd<Self> for StrideComparator {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let stride1 = self.0.inner_exclusive_access().stride;
        let stride2 =other.0.inner_exclusive_access().stride;
        // reverse the order for BinaryHeap
        stride2.partial_cmp(&stride1)
    }
}

impl Ord for StrideComparator {
    fn cmp(&self, other: &Self) -> Ordering {
        let stride1 = self.0.inner_exclusive_access().stride;
        let stride2 = other.0.inner_exclusive_access().stride;
        // reverse the order for BinaryHeap
        stride2.cmp(&stride1)
    }

    fn max(self, other: Self) -> Self where Self: Sized {
        let stride1 = self.0.inner_exclusive_access().stride;
        let stride2 = other.0.inner_exclusive_access().stride;
        // reverse the order for BinaryHeap
        if stride1 < stride2 {
            self
        } else {
            other
        }
    }

    fn min(self, other: Self) -> Self where Self: Sized {
        let stride1 = self.0.inner_exclusive_access().stride;
        let stride2 = other.0.inner_exclusive_access().stride;
        // reverse the order for BinaryHeap
        if stride1 > stride2 {
            self
        } else {
            other
        }
    }

    fn clamp(self, min: Self, max: Self) -> Self where Self: Sized {
        let stride = self.0.inner_exclusive_access().stride;
        let min_stride = min.0.inner_exclusive_access().stride;
        let max_stride = max.0.inner_exclusive_access().stride;
        if stride < min_stride {
            min
        } else if stride > max_stride {
            max
        } else {
            self
        }
    }
}

pub struct TaskManager {
    ready_queue: BinaryHeap<StrideComparator>,
}

// YOUR JOB: FIFO->Stride
/// A simple FIFO scheduler.
impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: BinaryHeap::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        let stride = task.inner_exclusive_access().stride;
        println!("add a task, stride={}", stride);
        self.ready_queue.push(StrideComparator(task));
    }

    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let task = self.ready_queue.pop()?.0;
        let mut inner = task.inner_exclusive_access();
        println!("fetch a task, stride={}", inner.stride);
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
