//! Process management syscalls
//!
use alloc::sync::Arc;
use crate::{
    config::{BIGSTRIDE, MAX_SYSCALL_NUM}, fs::{open_file, OpenFlags}, mm::{translated_refmut, translated_str, MapPermission, VirtAddr}, syscall::{ SYSCALL_GET_TIME, SYSCALL_MMAP, SYSCALL_MUNMAP, SYSCALL_SET_PRIORITY, SYSCALL_SPAWN, SYSCALL_TASK_INFO}, task::{
        add_task, current_task, current_user_token, exit_current_and_run_next, get_sys_call, get_total_time, suspend_current_and_run_next, sys_call_add, TaskStatus
    }, timer::get_time_us
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

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
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

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
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
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
// pub fn sys_spawn(path: *const u8) -> isize {
//     trace!(
//         "kernel:pid[{}] sys_spawn called with path {:?}",
//         current_task().unwrap().pid.0,
//         path
//     );

//     // 获取当前用户的 token
//     let token = current_user_token();

//     // 转换路径字符串
//     let str_path = translated_str(token, path);
//     // 打开文件
//     let mut inode = match open_file(&str_path, OpenFlags::RDONLY) {
//         Some(inode) => inode,
//         None => {
//             println!("Failed to open file: {}", str_path);
//             return -1;
//         }
//     };
//     if let Some(inode_mut) = Arc::get_mut(&mut inode) {
//         inode_mut.set_offset();
//     } else {
//         println!("Failed to get mutable reference to OSInode");
//         return -1;
//     }
//     println!("offset1 : {}",inode.get_offset());
//     // 读取文件内容
//     let app_data = inode.read_all();
//     println!("offset2 : {}",inode.get_offset());
//     if let Some(inode_mut) = Arc::get_mut(&mut inode) {
//         inode_mut.set_offset();
//     } else {
//         println!("Failed to get mutable reference to OSInode");
//         return -1;
//     }
//     println!("spawn load app : {} lenght : {}",str_path,app_data.len());
//     // 获取当前任务
//     let current_task = match current_task() {
//         Some(task) => task,
//         None => {
//             println!("Failed to get current task");
//             return -1;
//         }
//     };
//     sys_call_add(SYSCALL_SPAWN, &current_task);
//     // 创建新的任务控制块并包裹在 Arc 里
//     let new_task = Arc::new(TaskControlBlock::new(app_data.as_slice()));

//     {
//         // 设置父任务
//         new_task.inner_exclusive_access().set_parent(Some(Arc::downgrade(&current_task)));

//         // 添加子任务到当前任务
//         current_task.inner_exclusive_access().add_children(Arc::clone(&new_task));
//     }

//     // 将新任务添加到调度器
//     add_task(Arc::clone(&new_task));

//     // 执行新任务
//     new_task.exec(app_data.as_slice());

//     // 返回新任务的 PID
//     new_task.pid.0 as isize
// }

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
    let task = current_task().unwrap();
    sys_call_add(SYSCALL_SET_PRIORITY, &task);
    task.inner_exclusive_access().set_priority(_prio);
    task.inner_exclusive_access().set_pass(BIGSTRIDE/(_prio as usize));
    _prio
}
pub fn sys_spawn(path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn called with path {:?}",
        current_task().unwrap().pid.0,
        path
    );

    let current_task = current_task().unwrap();
    sys_call_add(SYSCALL_SPAWN,&current_task);
    match current_task.spawn(path) {
        Ok(new_task) => {
            let new_pid = new_task.pid.0;
            add_task(new_task);
            debug!("new_task via spawn {:?}", new_pid);
            new_pid as isize
        }
        Err(_) => {
            warn!("spawn failed!");
            -1
        }
    }
}