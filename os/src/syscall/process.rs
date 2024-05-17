//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, do_mmap, do_munmap, exit_current_and_run_next, get_sys_call, get_total_time, suspend_current_and_run_next, sys_call_add, TaskStatus
    },
};
use core::slice;
use core::mem;
use crate::timer::get_time_us;
use crate::mm::translated_byte_buffer;
use crate::task::current_user_token;
use core::mem::size_of;
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
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
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

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    sys_call_add(169);
    let us = get_time_us();
    //*_ts的地址是虚拟地址，要根据虚拟地址获取物理地址，然后修改物理地址里面的值 
    let buffers = translated_byte_buffer(current_user_token(), _ts as *const u8, size_of::<TimeVal>());
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let mut time_val_ptr = &time_val as *const _ as *const u8;
    for buffer in buffers {
        unsafe {
            time_val_ptr.copy_to(buffer.as_mut_ptr(), buffer.len());
            time_val_ptr = time_val_ptr.add(buffer.len());
        }
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    if _ti.is_null() {
        return -1;
    }
    trace!("kernel: sys_task_info");
    sys_call_add(410);
    let buffers = translated_byte_buffer(current_user_token(), _ti as *const u8, mem::size_of::<TaskInfo>());
    let task_info = TaskInfo {
        status: TaskStatus::Running,
        syscall_times: get_sys_call(),  
        time: get_total_time(),
    };
    let task_info_bytes: &[u8] = unsafe {
        slice::from_raw_parts(&task_info as *const _ as *const u8, size_of::<TaskInfo>())
    };
    let mut task_info_ptr = task_info_bytes.as_ptr();
    for buffer in buffers {
        unsafe {
            task_info_ptr.copy_to(buffer.as_mut_ptr(), buffer.len());
            task_info_ptr = task_info_ptr.add(buffer.len());
        }
    }

    0
}

/// YOUR JOB: Implement mmap.
///start 需要映射的虚存起始地址，要求按页对齐
///len 映射字节长度，可以为 0
///port：第 0 位表示是否可读，第 1 位表示是否可写，第 2 位表示是否可执行。其他位无效且必须为 
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    sys_call_add(222);
    // 检查 port 的有效性
    if port & !0b111 != 0 || start%4096!=0 || port & 0x7 == 0{
        return -1;
    }
    println!("start = {} and len = {}",start,len);
    do_mmap(start, len, port)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    sys_call_add(215);
    println!("call munmap!");
    if _start%4096!=0 || _len%4096!=0{
        return -1;
    }
    do_munmap(_start, _len)
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    sys_call_add(214);
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
