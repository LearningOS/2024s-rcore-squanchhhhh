//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{exit_current_and_run_next,get_sys_call,get_total_time, suspend_current_and_run_next, sys_call_add, TaskStatus},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    sys_call_add(93);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    sys_call_add(124);
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    sys_call_add(169);
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

//查询当前正在执行的任务信息，任务信息包括任务控制块相关信息（任务状态）、
//任务使用的系统调用及调用次数、系统调用时刻距离任务第一次被调度时刻的时长（单位ms）。
//实现思路：
//1.任务控制块信息：直接将status设置为running
//2.任务使用的系统调用及次数：
//3.系统调用时刻距离任务第一次被调度时刻的时长：在TaskInfo中记录任务第一次被调度的时刻s,设置时长为当前时刻s1-s
/// YOUR JOB: Finish sys_task_info to pass testcases
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    sys_call_add(410);
    trace!("kernel: sys_task_info");
    unsafe {
        //如果指针是空则返回-1
        if _ti.is_null() {
            return -1; 
        }
        // 设置_ti的状态为Running
        (*_ti).status = TaskStatus::Running;
        // 设置_ti的时长为当前时间减去TCB中的time字段
        (*_ti).time = get_total_time();
        // 设置系统调用号
        (*_ti).syscall_times = get_sys_call();
    }
    0
}
