//! Loading user applications into memory

/// Get the total number of applications.
pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// get applications data
/// 根据传入参数的值来获取对应的应用程序数据段
pub fn get_app_data(app_id: usize) -> &'static [u8] {
    extern "C" {
        fn _num_app();
    }
    //获取应用程序的基地址
    let num_app_ptr = _num_app as usize as *const usize;
    //  获取应用程序的个数
    let num_app = get_num_app();
    // 获取每个应用程序的开始地址
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    assert!(app_id < num_app);
    unsafe {
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,
            app_start[app_id + 1] - app_start[app_id],
        )
    }
}
