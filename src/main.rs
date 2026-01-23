use dioxus::prelude::*;
use dioxus_desktop::launch::{launch, launch_virtual_dom};
use dioxus_desktop::tao::event_loop::EventLoop;
use dioxus_desktop::tao::monitor::MonitorHandle;
use dioxus_desktop::tao::monitor::VideoMode;
use dioxus_desktop::tao::window::Window;
use dioxus_desktop::{Config, tao::window::WindowBuilder};
use dioxus_desktop::{LogicalPosition, LogicalSize};
const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const HEADER_SVG: Asset = asset!("/assets/header.svg");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    let window_width = 600.0;
    let window_height = 400.0;

    let event_loop = EventLoop::new();
    let monitor = event_loop.primary_monitor().unwrap();
    let monitor_size = monitor.size();
    let scale_factor = monitor.scale_factor(); // 获取缩放因子

    // 🔥 核心：把显示器物理尺寸转成逻辑尺寸
    let monitor_width_logical = monitor_size.width as f64 / scale_factor;
    let monitor_height_logical = monitor_size.height as f64 / scale_factor;

    // 计算居中（现在都是逻辑像素）
    let x = (monitor_width_logical - window_width) / 2.0;
    let y = (monitor_height_logical - window_height) / 2.0;

    println!(
        "显示器逻辑尺寸: {:.0}x{:.0}",
        monitor_width_logical, monitor_height_logical
    );
    println!("窗口位置: {:.0},{:.0}", x, y);
    // println!("当前显示器尺寸: {:?}", size);
    let window_builder = WindowBuilder::new()
        .with_always_on_top(false)
        .with_title("mp4文件合并")
        .with_inner_size(LogicalSize::new(window_width, window_height))
        .with_position(LogicalPosition::new(x, y));
    let virtual_dom = VirtualDom::new(App);
    let platform_config = Config::new().with_window(window_builder);

    launch_virtual_dom(virtual_dom, platform_config)
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        // Hero {}
    }
}

#[component]
pub fn Hero() -> Element {
    rsx! {
        div { id: "hero",
            img { src: HEADER_SVG, id: "header" }
            div { id: "links",
                a { href: "https://discord.gg/XgGxMSkvUM", "👋 Community Discord" }
            }
        }
    }
}

// pub fn add(a: i32, b: i32) -> i32 {
//     a + b
// }

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
