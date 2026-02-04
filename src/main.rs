use dioxus::prelude::*;
use dioxus_desktop::launch::launch_virtual_dom;
use dioxus_desktop::tao::event_loop::EventLoop;
use dioxus_desktop::{Config, tao::window::WindowBuilder};
use dioxus_desktop::{LogicalPosition, LogicalSize};
mod components;
mod config;
mod ffmpeg;
mod utils;
use crate::components::button::Button;
use crate::components::config_dialog::ConfigDialog;
use crate::components::mp4_merger::Mp4Merger;
use crate::components::tabs::*;
use crate::config::AppConfig;
use components::about_footer::AboutFooter;
use components::mp4_info::Mp4Info;
use components::toast::ToastProvider;
const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[derive(Clone, Debug)]
enum MergeEvent {
    Progress(f64),
    Status(String),
    Error(String),
    Success(String),
}
fn main() {
    let window_width = 900.0;
    let window_height = 700.0;
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

    // println!("当前显示器尺寸: {:?}", size);
    let window_builder = WindowBuilder::new()
        .with_always_on_top(false) // 不放在最顶层
        .with_title("mp4文件合并")
        .with_inner_size(LogicalSize::new(window_width, window_height))
        .with_position(LogicalPosition::new(x, y));
    let virtual_dom = VirtualDom::new(App);
    let platform_config = Config::new().with_window(window_builder);

    launch_virtual_dom(virtual_dom, platform_config)
}

#[derive(Routable, PartialEq, Clone)]
enum Route {
    #[layout(Layout)]
    #[route("/")]
    Index,
}
#[component]
fn Layout() -> Element {
    let version = env!("CARGO_PKG_VERSION");
    let authors = env!("CARGO_PKG_AUTHORS");
    // 如果需要将作者字符串分割成列表
    let mut author = String::from("");

    let author_list: Vec<&str> = authors.split(':').collect();
    for _author in author_list.iter() {
        author = _author.trim().to_string();
    }
    rsx! {
        ToastProvider {
            main { class: "h-screen flex flex-col",
                div { class: "flex-1", Outlet::<Route> {} }
                AboutFooter { author: "{author}", version: "{version}" }
            }
        }
    }
}
#[component]
fn Index() -> Element {
    let mut config: Signal<AppConfig> = use_signal(|| {
        AppConfig::load().unwrap_or_else(|e| {
            eprintln!("Failed to load config: {}", e);
            AppConfig::default()
        })
    });

    let mut show_config = use_signal(|| false);

    println!("config{:?}", config);
    rsx! {
        div { class: " flex h-full justify-between p-4 border-b",

            Tabs {
                default_value: "tab1".to_string(),
                horizontal: true,
                class: "flex-1",
                TabList {
                    TabTrigger { value: "tab1".to_string(), index: 0usize, "合并" }
                    TabTrigger { value: "tab2".to_string(), index: 1usize, "文件库" }
                    TabTrigger { value: "tab3".to_string(), index: 1usize, "转码记录" }

                }
                TabContent {
                    index: 0usize,
                    value: "tab1".to_string(),
                    class: "flex-1 p-0",

                    Mp4Merger { config }

                }
                TabContent {
                    index: 1usize,
                    class: "flex-1",
                    value: "tab2".to_string(),
                    Mp4Info { config }
                }
                TabContent {
                    index: 1usize,
                    class: "flex-1",
                    value: "tab3".to_string(),
                    // Mp4Info { config }
                    "转码记录"
                }

            }
            div { class: "absolute right-5 top-5",
                Button { onclick: move |_| show_config.set(true), "配置" }
            }
        }

        ConfigDialog {
            open: show_config,
            config,
            on_save: move |new_config| {
                config.set(new_config);
            },
        }
    }
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        Router::<Route> {}
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
