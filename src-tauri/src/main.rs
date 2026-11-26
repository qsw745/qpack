// Windows Release 构建隐藏控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    qpack_lib::run()
}
