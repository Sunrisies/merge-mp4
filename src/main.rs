use dioxus::prelude::*;
use dioxus_desktop::launch::launch_virtual_dom;
use dioxus_desktop::tao::event_loop::EventLoop;
use dioxus_desktop::{Config, tao::window::WindowBuilder};
use dioxus_desktop::{LogicalPosition, LogicalSize};
use dioxus_primitives::toast::{ToastOptions, use_toast};
mod components;
mod config;
mod ffmpeg;

use components::toast::ToastProvider;
use config::AppConfig;
use futures_util::StreamExt;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

use crate::ffmpeg::merge_mp4::run_ffmpeg_merge;
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
    let window_width = 600.0;
    let window_height = 600.0;

    let event_loop = EventLoop::new();
    let monitor = event_loop.primary_monitor().unwrap();
    let monitor_size = monitor.size();
    let scale_factor = monitor.scale_factor(); // 获取缩放因子
    println!("缩放因子: {:.2}", scale_factor);
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
        // 错误消息（固定在底部）
        ToastProvider { Mp4Merger {} }
    }
}

#[component]
pub fn Mp4Merger() -> Element {
    let mut files: Signal<Vec<PathBuf>> = use_signal(Vec::new);
    let mut output_filename: Signal<String> = use_signal(String::new);
    let mut progress: Signal<f64> = use_signal(|| 0.0);
    let mut is_merging: Signal<bool> = use_signal(|| false);
    let mut status_message: Signal<String> = use_signal(Default::default);
    let mut error_message: Signal<Option<String>> = use_signal(|| None);
    let mut success_message: Signal<Option<String>> = use_signal(|| None);
    let mut config: Signal<AppConfig> = use_signal(|| {
        AppConfig::load().unwrap_or_else(|e| {
            eprintln!("Failed to load config: {}", e);
            AppConfig::default()
        })
    });
    let toast = use_toast();

    use_effect(move || {
        if let Some(error) = error_message() {
            toast.error(
                "发生错误".to_string(),
                ToastOptions::new()
                    .description(error)
                    .duration(Duration::from_secs(5))
                    .permanent(false),
            );
        }
    });

    use_effect(move || {
        if let Some(success) = success_message() {
            toast.success(
                "合并成功!".to_string(),
                ToastOptions::new()
                    .description(success)
                    .duration(Duration::from_secs(3))
                    .permanent(false),
            );
        }
    });

    // Update output filename when files change
    use_effect(move || {
        let files_value = files();
        if !files_value.is_empty()
            && output_filename().is_empty()
            && let Some(first_file) = files_value.first()
            && let Some(file_name) = first_file.file_name()
        {
            let mut name = file_name.to_string_lossy().to_string();
            // Replace .mp4 with _merged.mp4
            if name.ends_with(".mp4") {
                name.truncate(name.len() - 4);
                name.push_str("_merged.mp4");
            } else {
                name.push_str("_merged.mp4");
            }
            output_filename.set(name);
        }
    });

    let add_files = {
        move |_| async move {
            if let Some(result) = rfd::AsyncFileDialog::new()
                .add_filter("MP4 Files", &["mp4"])
                .set_title("选择MP4文件")
                .pick_files()
                .await
            {
                files.write().extend(result.into_iter().map(PathBuf::from));
            }
        }
    };

    let mut remove_file = move |index: usize| {
        files.write().remove(index);
    };

    let select_output_directory = {
        move |_| async move {
            if let Some(result) = rfd::AsyncFileDialog::new()
                .set_title("选择输出目录")
                .pick_folder()
                .await
            {
                let path = result.path().to_path_buf();
                if let Err(e) = config.write().set_output_directory(path.clone()) {
                    error_message.set(Some(format!("无法保存输出目录设置: {}", e)));
                } else {
                    toast.success(
                        "输出目录已保存".to_string(),
                        ToastOptions::new()
                            .description(format!("目录: {}", path.display()))
                            .duration(Duration::from_secs(3))
                            .permanent(false),
                    );
                }
            }
        }
    };

    let clear_output_directory = {
        move |_| {
            config.write().output_directory = None;
            if let Err(e) = config.write().save() {
                error_message.set(Some(format!("无法清除输出目录设置: {}", e)));
            } else {
                toast.success(
                    "输出目录已清除".to_string(),
                    ToastOptions::new()
                        .description("将使用默认目录")
                        .duration(Duration::from_secs(3))
                        .permanent(false),
                );
            }
        }
    };

    // ✅ 订阅接收端
    use_coroutine(move |mut rx: UnboundedReceiver<MergeEvent>| async move {
        while let Some(event) = rx.next().await {
            match event {
                MergeEvent::Progress(p) => progress.set(p),
                MergeEvent::Status(s) => status_message.set(s),
                MergeEvent::Error(e) => {
                    error_message.set(Some(e));
                    is_merging.set(false);
                }

                MergeEvent::Success(msg) => {
                    progress.set(100.0);
                    status_message.set("合并完成!".to_string());
                    success_message.set(Some(msg));
                    sleep(Duration::from_secs(2)).await;
                    is_merging.set(false);
                }
            }
        }
    });

    let merge_files = {
        move |_| {
            let files_value = files();
            let output_filename_value = output_filename();
            let config_value = config();

            if files_value.is_empty() {
                error_message.set(Some("请先选择要合并的MP4文件".to_string()));
                return;
            }

            if output_filename_value.is_empty() {
                error_message.set(Some("请输入输出文件名".to_string()));
                return;
            }

            // Construct output path
            let output_dir = config_value.get_output_directory();
            let output_path_final = output_dir.join(&output_filename_value);

            is_merging.set(true);
            progress.set(0.0);
            status_message.set("正在检查FFmpeg环境...".to_string());
            error_message.set(None);
            let tx = use_coroutine_handle::<MergeEvent>();
            let tx_for_task = tx;
            let files_value = files();

            let output_path_final_clone = output_path_final.clone();
            spawn(async move {
                run_ffmpeg_merge(files_value, output_path_final_clone, tx_for_task).await;
            });
        }
    };

    rsx! {
        div { class: "h-screen overflow-hidden",
            div { class: "h-full max-w-2xl mx-auto pt-4 overflow-y-auto",
                // 标题区域
                div { class: "text-center mb-2",
                    h1 { class: "text-4xl font-bold mb-2 tracking-tight", "🎬 MP4文件合并工具" }
                }

                // 主要内容卡片
                div { class: "bg-gray-800/80 backdrop-blur-lg rounded-2xl shadow-2xl border border-gray-700 overflow-hidden" }

                // 文件选择区域
                div { class: "p-6 border-b border-gray-700",
                    div { class: "flex items-center justify-between mb-2",
                        h2 { class: "text-xl font-semibold flex items-center gap-2",
                            "📁 "
                            "选择要合并的MP4文件"
                        }
                        button {
                            class: "bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700  font-semibold py-1.5 px-2 rounded-lg transition-all duration-200 transform hover:scale-105 shadow-lg",
                            onclick: add_files,
                            "➕ 添加文件"
                        }
                    }

                    // 文件列表
                    div { class: "mt-4",
                        if !files.read().is_empty() {
                            div { class: "space-y-2 max-h-64 overflow-y-auto pr-2 custom-scrollbar",
                                for (index , file) in files.read().iter().cloned().enumerate() {
                                    div { class: "flex items-center justify-between p-3 bg-gray-700/50 rounded-lg border border-gray-600 hover:border-gray-500 transition-colors",
                                        div { class: "flex items-center gap-3 overflow-hidden",
                                            span { class: "text-gray-400 text-sm font-mono",
                                                "{index + 1}."
                                            }
                                            span { class: " truncate flex-1",
                                                "{file.file_name().unwrap().to_string_lossy()}"
                                            }
                                        }
                                        button {
                                            class: "bg-red-500/20 hover:bg-red-500/40 text-red-400 hover:text-red-300 font-medium py-1.5 px-3 rounded-lg transition-all duration-200 text-sm",
                                            onclick: move |_| remove_file(index),
                                            "🗑️ 删除"
                                        }
                                    }
                                }
                            }
                        } else {
                            div { class: "text-center py-8 border-2 border-dashed border-gray-600 rounded-lg",
                                p { class: "text-gray-500 text-lg", "📭 尚未选择任何文件" }
                                p { class: "text-gray-600 text-sm mt-1",
                                    "点击上方按钮添加MP4文件"
                                }
                            }
                        }
                    }
                }

                // 输出文件名设置区域
                div { class: "p-6 border-b border-gray-700",
                    h2 { class: "text-sm font-semibold mb-4 flex items-center gap-2",
                        "💾 "
                        "输出文件设置"
                    }
                    div { class: "space-y-3",
                        div { class: "flex items-center gap-3",
                            span { class: "text-gray-400 text-sm", "文件名:" }
                            input {
                                class: "flex-1 bg-gray-700/50 border border-gray-600 rounded-lg px-3 py-2 text-gray-300 focus:outline-none focus:border-purple-500 transition-colors",
                                placeholder: "输入输出文件名 (例如: merged.mp4)",
                                value: "{output_filename()}",
                                oninput: move |e| output_filename.set(e.value()),
                            }
                        }
                        div { class: "flex items-center gap-3",
                            span { class: "text-gray-400 text-sm", "目录:" }
                            span { class: "flex-1 text-gray-300 text-sm break-all",
                                if let Some(dir) = config().output_directory.as_ref() {
                                    "{dir.display()}"
                                } else {
                                    "使用默认目录"
                                }
                            }
                            button {
                                class: "bg-blue-500/20 hover:bg-blue-500/40 text-blue-400 hover:text-blue-300 font-medium py-1.5 px-3 rounded-lg transition-all duration-200 text-sm",
                                onclick: select_output_directory,
                                "📁 选择目录"
                            }
                            button {
                                class: "bg-gray-500/20 hover:bg-gray-500/40 text-gray-400 hover:text-gray-300 font-medium py-1.5 px-3 rounded-lg transition-all duration-200 text-sm",
                                onclick: clear_output_directory,
                                "🗑️ 清除"
                            }
                        }
                    }
                }

                // 合并按钮和状态区域
                div { class: "p-6",
                    div { class: "flex justify-center mb-6",
                        button {
                            class: "bg-gradient-to-r from-purple-600 to-purple-700 hover:from-purple-700 hover:to-purple-800 disabled:from-gray-600 disabled:to-gray-700  font-bold py-3 px-8 rounded-xl transition-all duration-200 transform hover:scale-105 disabled:hover:scale-100 shadow-lg disabled:shadow disabled:cursor-not-allowed text-lg",
                            disabled: is_merging(),
                            onclick: merge_files,
                            if is_merging() {
                                "⏳ 合并中..."
                            } else {
                                "🚀 开始合并"
                            }
                        }
                    }

                    // 进度条
                    if is_merging() || progress() > 0.0 {
                        div { class: "space-y-3",
                            div { class: "flex justify-between items-center",
                                span { class: " font-semibold", "合并进度" }
                                span { class: "text-purple-400 font-mono", "{progress():.1}%" }
                            }
                            div { class: "w-full bg-gray-700 rounded-full h-3 overflow-hidden",
                                div {
                                    class: "bg-gradient-to-r from-purple-500 to-pink-500 h-3 rounded-full transition-all duration-300 ease-out",
                                    style: "width: {progress()}%",
                                }
                            }
                            if !status_message().is_empty() {
                                p { class: "text-center text-gray-400 text-sm", "{status_message()}" }
                            }
                        }
                    }
                }
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
