//! File and filesystem-related syscalls
use crate::fs::{open_file, OpenFlags, Stat};
use crate::mm::{translated_byte_buffer, translated_refmut, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};
use alloc::sync::Arc;
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(_fd: usize, _st: *mut Stat) -> isize {
    trace!(
        "kernel:pid[{}] sys_fstat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    // 获取当前任务和用户令牌
    let task = current_task().unwrap();
    let token = current_user_token();

    // 获取文件描述符对应的节点
    let node = &task.inner_exclusive_access().fd_table[_fd];

    // 翻译指针，得到一个可变引用
    let st: &mut Stat = translated_refmut(token, _st) ;

    // 使用可变引用调用 stat 方法
    if let Some(ref file_node) = node.as_ref() {
            file_node.stat(st);

    } else {
        // 处理 node 为空的情况
        return -1;  // 或者其他适当的错误代码
    }

    0  // 返回 0 表示成功
}

/// YOUR JOB: Implement linkat.
/// 思路：1.获取当前old_name的diskInode
/// 2.创建一个新的diskInode
/// 3.复制old_diskinode的值给new_diskinode 这样保证两个diskinode指向了同一块数据区
/// 4.在目录中添加new_diskinode的信息
pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_linkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let old_name  = translated_str(token, _old_name);
    let old_name_str = old_name.as_str();
    let new_name = translated_str(token,_new_name);
    let new_name_str = new_name.as_str();
    // same name
    if old_name_str == new_name_str{
        return -1;
    }
    let os_inode = open_file(old_name_str, OpenFlags::RDWR);
    let os_inode_ = Arc::clone(&os_inode.unwrap());
    let inode = os_inode_.get_inode();
    inode.create_hard_link(new_name_str, old_name_str);
    0
}

/// YOUR JOB: Implement unlinkat.
/// 在目录中删除这个diskinode，并设置对应的位图为0
pub fn sys_unlinkat(_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_unlinkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}
