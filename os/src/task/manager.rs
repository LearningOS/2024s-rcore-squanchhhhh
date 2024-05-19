//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
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
 /// 取出 stride 最小的进程
 pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
    if self.ready_queue.is_empty() {
        return None;
    }

    // 找到 stride 最小的进程
    let mut min_index = 0;
    let mut min_stride = self.ready_queue[0].inner_exclusive_access().get_stride();

    for (index, task) in self.ready_queue.iter().enumerate() {
        let stride = task.inner_exclusive_access().get_stride();
        if stride < min_stride {
            min_index = index;
            min_stride = stride;
        }
    }

    // 更新选中的进程的 stride 值
    {
        let mut task = self.ready_queue[min_index].inner_exclusive_access();
        let stride = task.get_stride();
        let pass = task.get_pass();
        task.set_stride(stride + pass);
    }
    // 移除并返回 stride 最小的进程
    Some(self.ready_queue.remove(min_index).unwrap())
}
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
