use crate::components::button::Button;
use crate::components::input::Input;
use crate::config::AppConfig;
use chrono::{DateTime, Local};
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn Mp4Info(mut config: Signal<AppConfig>) -> Element {
    let mut selected_directory: Signal<Option<PathBuf>> =
        use_signal(|| config.read().get_query_directory());
    let mut files: Signal<Vec<PathBuf>> = use_signal(Vec::new);
    let mut is_loading: Signal<bool> = use_signal(|| false);
    let mut error_message: Signal<Option<String>> = use_signal(|| None);

    // 提取核心逻辑为无参闭包，避免重复代码
    let perform_scan = move || {
        let dir = selected_directory.read().clone();
        spawn(async move {
            if let Some(directory) = dir {
                is_loading.set(true);
                error_message.set(None); // 清除错误

                // 使用 spawn_blocking 执行阻塞操作
                let result =
                    tokio::task::spawn_blocking(move || match std::fs::read_dir(&directory) {
                        Ok(entries) => {
                            let mp4_files: Vec<PathBuf> = entries
                                .filter_map(|entry| entry.ok())
                                .map(|entry| entry.path())
                                .filter(|path| {
                                    path.is_file()
                                        && path
                                            .extension()
                                            .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
                                })
                                .collect();
                            Ok(mp4_files)
                        }
                        Err(e) => Err(e),
                    })
                    .await;

                match result {
                    Ok(Ok(mp4_files)) => {
                        println!(
                            "扫描到 {} 个 MP4 文件,文件内容:{:?}",
                            mp4_files.len(),
                            mp4_files
                        );
                        files.set(mp4_files);
                    }
                    Ok(Err(e)) => {
                        error_message.set(Some(format!("无法读取目录: {}", e)));
                    }
                    Err(e) => {
                        error_message.set(Some(format!("扫描任务失败: {}", e)));
                    }
                }

                is_loading.set(false);
            }
        });
    };

    // 给按钮用的处理器，接收事件但忽略
    let on_scan_click = move |_evt: Event<MouseData>| {
        perform_scan();
    };

    let select_output_directory = {
        move |_| async move {
            if let Some(result) = rfd::AsyncFileDialog::new()
                .set_title("选择输出目录")
                .pick_folder()
                .await
            {
                let path = result.path().to_path_buf();
                selected_directory.set(Some(path.clone()));

                if let Err(e) = config.write().set_query_directory(path.clone()) {
                    error_message.set(Some(format!("无法保存输出目录设置: {}", e)));
                } else {
                    // 直接调用核心逻辑，不传参数
                    perform_scan();
                }
            }
        }
    };

    rsx! {
        div { class: "flex flex-col space-y-4 p-4 bg-white rounded-lg shadow-md",
            h2 { class: "text-xl font-semibold text-gray-800 mb-2", "MP4 文件信息" }

            // 错误消息显示
            if let Some(error) = error_message.read().as_ref() {
                div { class: "p-3 mb-4 text-sm text-red-700 bg-red-100 rounded-lg",
                    {error.to_string()}
                }
            }

            // 输出目录选择
            div { class: "flex flex-col space-y-2",
                label { class: "text-sm font-medium text-gray-700", "输出目录" }
                div { class: "flex space-x-2",
                    Input {
                        class: "flex-1 px-4 py-2 border border-gray-300 rounded-md bg-gray-50",
                        value: "{selected_directory.read().as_ref().map(|p| p.display().to_string()).unwrap_or_default()}",
                        readonly: true,
                        placeholder: "未选择目录",
                    }
                    Button {
                        class: "px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 transition-colors",
                        onclick: select_output_directory,
                        "选择目录"
                    }
                }
            }

            // 扫描按钮 - 使用正确的事件处理器签名
            div { class: "flex justify-end",
                Button {
                    class: "px-6 py-2 bg-green-500 text-white rounded-md hover:bg-green-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                    disabled: selected_directory.read().is_none() || is_loading(),
                    onclick: on_scan_click, // 修复：使用接收 Event 的闭包
                    "扫描目录"
                }
            }

            // 文件列表
            div { class: "mt-4",
                if is_loading() {
                    div { class: "flex justify-center p-4",
                        div { class: "animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500" }
                    }
                } else if !files.read().is_empty() {
                    div { class: "border border-gray-200 rounded-md  h-[300px] overflow-auto",
                        table { class: "w-full table-fixed divide-y divide-gray-200",
                            thead { class: "bg-gray-50 sticky top-0 z-10",
                                tr {
                                    th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider whitespace-nowrap w-1/2",
                                        "文件名"
                                    }
                                    th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider whitespace-nowrap w-1/4",
                                        "大小"
                                    }
                                    th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider whitespace-nowrap w-1/4",
                                        "修改日期"
                                    }
                                }
                            }
                            tbody { class: "bg-white divide-y divide-gray-200",
                                for file in files.read().iter() {
                                    tr {
                                        td {
                                            class: "px-2 py-4 text-sm text-gray-900 truncate",
                                            title: r#"{file.file_name().and_then(|n| n.to_str()).unwrap_or("未知文件")}"#,
                                            {file.file_name().and_then(|n| n.to_str()).unwrap_or("未知文件")}
                                        }
                                        td {
                                            class: "px-2 py-4 text-sm text-gray-500 whitespace-nowrap",
                                            title: "{format_size(file.metadata().and_then(|m| Ok(m.len())).ok())}",
                                            {format_size(file.metadata().map(|m| m.len()).ok())}
                                        }
                                        td {
                                            class: "px-2 py-4 text-sm text-gray-500 truncate",
                                            title: "{format_date(file.metadata().and_then(|m| m.modified()).ok())}",
                                            {format_date(file.metadata().ok().and_then(|m| m.modified().ok()))}
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if selected_directory.read().is_some() && !is_loading() {
                    div { class: "text-center p-8 text-gray-500", "该目录下没有找到MP4文件" }
                }
            }
        }
    }
}

fn format_size(size: Option<u64>) -> String {
    match size {
        Some(s) if s < 1024 => format!("{} B", s),
        Some(s) if s < 1024 * 1024 => format!("{:.2} KB", s as f64 / 1024.0),
        Some(s) if s < 1024 * 1024 * 1024 => format!("{:.2} MB", s as f64 / (1024.0 * 1024.0)),
        Some(s) => format!("{:.2} GB", s as f64 / (1024.0 * 1024.0 * 1024.0)),
        None => "未知".to_string(),
    }
}

fn format_date(modified: Option<std::time::SystemTime>) -> String {
    match modified {
        Some(time) => {
            let datetime: DateTime<Local> = time.into();
            datetime.format("%Y-%m-%d %H:%M:%S").to_string()
        }
        _ => "未知".to_string(),
    }
}
