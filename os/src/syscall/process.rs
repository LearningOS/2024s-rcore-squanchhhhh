//! Process management syscalls
use core::ffi::CStr;
use crate::mm::MapPermission;
use crate::task::TaskControlBlock;
use alloc::sync::Arc;
use crate::mm::VirtAddr;
use crate::timer::get_time_us;
use crate::task::{get_sys_call, get_total_time};
use crate::{
    config::MAX_SYSCALL_NUM, loader::get_app_data_by_name, mm::{translated_refmut, translated_str}, syscall::{ SYSCALL_GETPID, SYSCALL_GET_TIME, SYSCALL_MMAP, SYSCALL_MUNMAP, SYSCALL_SBRK, SYSCALL_SPAWN, SYSCALL_TASK_INFO, }, task::{
        add_task, current_task, current_user_token, exit_current_and_run_next, suspend_current_and_run_next, sys_call_add, TaskStatus
    }
};
use crate::syscall::SYSCALL_FORK;
use core::ffi::c_char;
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
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {

    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    sys_call_add(SYSCALL_GETPID,&task);
    task.pid.0 as isize
}

pub fn sys_fork() -> isize {

    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    
    let current_task = current_task().unwrap();
    sys_call_add(SYSCALL_FORK,&current_task);
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}
use crate::syscall::SYSCALL_EXEC;
pub fn sys_exec(path: *const u8) -> isize {
    
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        sys_call_add(SYSCALL_EXEC,&task);
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
   // sys_call_add(SYSCALL_WAITPID);
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {

    trace!(
        "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let task = current_task().unwrap();
    sys_call_add(SYSCALL_GET_TIME,&task);
    let token = task.get_user_token();
    let ts = translated_refmut(token, _ts);
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    *ts = time_val;
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {

    trace!(
        "kernel:pid[{}] sys_task_info NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let task = current_task().unwrap();
    sys_call_add(SYSCALL_TASK_INFO,&task);
    let token = task.get_user_token();
    let taskinfo = translated_refmut(token, _ti);
    (*taskinfo).syscall_times = get_sys_call();
    (*taskinfo).status = TaskStatus::Running;
    (*taskinfo).time = get_total_time();
    0
}

/// YOUR JOB: Implement mmap.
/// 思路：
/// 1.检查参数有效性
/// 2.转换地址
/// 3.获取当前进程的地址空间
/// 4.检查是否有重复空间
/// 5.添加
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {

    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    // 检查 port 的有效性
    if _port & !0b111 != 0 || _start%4096!=0 || _port & 0x7 == 0{
        return -1;
    }
    //转换地址
    let page_floor = VirtAddr(_start).floor();
    let page_ceil = VirtAddr(_start+_len).ceil();
    let mut permission = MapPermission::from_bits((_port as u8) << 1).unwrap();
    permission.set(MapPermission::U, true);
    //获取地址空间
    let task = current_task().unwrap();
    sys_call_add(SYSCALL_MMAP,&task);
    let mut inner = task.inner_exclusive_access();
    let memset = &mut inner.memory_set;
    //检查是否有重复
    if memset.get_areas().iter().any(|area| area.get_start()<page_ceil&&area.get_end()>page_floor){
        return -1;
    }
    memset.insert_framed_area(VirtAddr(_start), VirtAddr(_start+_len), permission);
    0
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {

    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if _start%4096!=0 || _len%4096!=0{
        return -1;
    }
    let task = current_task().unwrap();
    sys_call_add(SYSCALL_MUNMAP,&task);
    let mut inner = task.inner_exclusive_access();
    let memset = &mut inner.memory_set;
    memset.unmap(_start, _len);
    0
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {

    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    sys_call_add(SYSCALL_SBRK,&task);
    if let Some(old_brk) = task.change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
/// 1.加载进程进入内存
/// 2.设置进程控制块
/// 3.添加到tasks
pub fn sys_spawn(_path: *const u8) -> isize {
    // 记录系统调用

    trace!(
        "kernel:pid[{}] sys_spawn called with path {:?}",
        current_task().unwrap().pid.0,
        _path
    );

    // 将路径指针转换为字符串
    let c_str_path = unsafe { CStr::from_ptr(_path as *const c_char) };
    let str_path = c_str_path.to_str().unwrap();

    // 加载进程
    let app_data = get_app_data_by_name(str_path).unwrap();
    let task_block = TaskControlBlock::new(app_data);

    // 设置进程控制块
    let current_task = current_task().unwrap();
    task_block.inner_exclusive_access().set_parent(Some(Arc::downgrade(&current_task)));
    sys_call_add(SYSCALL_SPAWN,&current_task);
    // 将新任务添加到任务管理器
    let task_block_arc = Arc::new(task_block);
    add_task(task_block_arc.clone());

    // 启动新任务
    task_block_arc.exec(&app_data);

    // 返回新任务的 PID
    task_block_arc.pid.0 as isize
}


// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    //sys_call_add(SYSCALL_SET_PRIORITY);
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if _prio<2{
        return -1;
    }
    0
}
