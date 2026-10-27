// Windows release 版不弹控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|a| a == "--mcp") {
        return reicky_lib::mcp_main();
    }
    reicky_lib::run()
}
