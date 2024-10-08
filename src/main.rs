#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod patch;
mod main_window;
mod ids;
mod create_tab;
mod apply_tab;

use winsafe::{prelude::*, co, AnyResult, HWND};
use main_window::MainWindow;

fn main() {
    if let Err(e) = run_app() {
        HWND::NULL.MessageBox(
            &e.to_string(), "Uncaught error", co::MB::ICONERROR).unwrap();
    }
}

fn run_app() -> AnyResult<i32> {
    MainWindow::new()
        .run()
}