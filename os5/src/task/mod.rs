//! Implementation of process management mechanism
//!
//! Here is the entry for process scheduling required by other modules
//! (such as syscall or clock interrupt).
//! By suspending or exiting the current process, you can
//! modify the process state, manage the process queue through TASK_MANAGER,
//! and switch the control flow through PROCESSOR.
//!
//! Be careful when you see [`__switch`]. Control flow around this function
//! might not be what you expect.

mod context;
mod manager;
mod pid;
mod processor;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::loader::get_app_data_by_name;
use alloc::sync::Arc;
use lazy_static::*;
use manager::fetch_task;
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;
pub use manager::add_task;
pub use pid::{pid_alloc, KernelStack, PidHandle};
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
};
use crate::config::PAGE_SIZE;
use crate::mm::{MapPermission, VirtAddr};
use crate::syscall::TaskInfo;
use crate::task::processor::PROCESSOR;
use crate::timer::{get_time_ms};

/// Make current task suspended and switch to the next task
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

/// Exit current task, recycle process resources and switch to the next task
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();
    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // println!("[exit_current_and_run_next] inner: {:?}", *inner);
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        // println!("[exit_current_and_run_next] getting initproc inner");
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        // println!("[exit_current_and_run_next] got initproc inner");
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

#[inline]
pub fn get_current_pid() -> Option<usize> {
    PROCESSOR.exclusive_access().current().map(|task| task.pid.0)
}

pub fn get_current_task_info() -> Option<TaskInfo> {
    let task = PROCESSOR.exclusive_access().current()?;
    let inner = task.inner_exclusive_access();
    let current_time_ms = get_time_ms();

    Some(TaskInfo {
        status: inner.task_status,
        syscall_times: inner.syscall_times,
        time: current_time_ms - inner.start_time_ms,
    })
}

pub fn set_current_task_priority(priority: isize) -> Option<()> {
    let task = PROCESSOR.exclusive_access().current()?;
    let mut inner = task.inner_exclusive_access();
    inner.priority = priority;
    Some(())
}

pub fn increase_syscall_times(syscall_id: usize) -> Option<()> {
    let task = PROCESSOR.exclusive_access().current()?;
    let mut inner = task.inner_exclusive_access();
    inner.syscall_times[syscall_id] += 1;
    Some(())
}

pub fn decrease_syscall_times(syscall_id: usize) -> Option<()> {
    let task = PROCESSOR.exclusive_access().current()?;
    let mut inner = task.inner_exclusive_access();
    inner.syscall_times[syscall_id] -= 1;
    Some(())
}

pub fn current_task_mmap(start: usize, len: usize, port: usize) -> Option<()> {
    if start & (PAGE_SIZE - 1) != 0 {
        debug!("[kernel] [pid {}] start not aligned, mmap failed", get_current_pid()?);
        return None;
    }
    if port & !0b111 != 0 || port & 0b111 == 0 {
        debug!("[kernel] [pid {}] port `{:#b}` is illegal, mmap failed", port, get_current_pid()?);
        return None;
    }
    let start_va = VirtAddr::from(start);
    let end_va: VirtAddr = VirtAddr::from(start + len).ceil().into();

    let task = PROCESSOR.exclusive_access().current()?;
    let mut inner = task.inner_exclusive_access();
    let memory_set = &mut inner.memory_set;
    if memory_set.is_conflict(start_va, end_va) {
        debug!("[kernel] [pid {:?}] memory conflicted, mmap failed", task.pid);
        return None;
    }
    let permission = MapPermission::from_bits((port << 1) as u8)? | MapPermission::U;
    memory_set.insert_framed_area(start_va, end_va, permission)?;
    Some(())
}

pub fn current_task_munmap(start: usize, len: usize) -> Option<()> {
    let start_va = VirtAddr::from(start);
    let end_va: VirtAddr = VirtAddr::from(start + len).ceil().into();

    let task = PROCESSOR.exclusive_access().current()?;
    let mut inner = task.inner_exclusive_access();
    let memory_set = &mut inner.memory_set;
    memory_set.unmap_area(start_va, end_va)
}

lazy_static! {
    /// Creation of initial process
    ///
    /// the name "initproc" may be changed to any other app name like "usertests",
    /// but we have user_shell, so we don't need to change it.
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        get_app_data_by_name("ch5b_initproc").unwrap(),
        "ch5b_initproc"
    ));
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}
