use super::button::Button;
use super::file_list::FileList;
use super::progress::{Progress, ProgressIndicator};
use dioxus::prelude::*;
use dioxus_primitives::toast::{ToastOptions, use_toast};
use futures_util::StreamExt;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

use crate::MergeEvent;
use crate::components::output_settings::OutputSettings;
use crate::config::AppConfig;
use crate::ffmpeg::merge_mp4::run_ffmpeg_merge;
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
    println!("config{:?}", config);
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
            let mut dialog = rfd::AsyncFileDialog::new()
                .add_filter("MP4 Files", &["mp4"])
                .set_title("选择MP4文件");

            // 如果有上次选择的目录，设置为初始目录
            if let Some(dir) = config().get_last_input_directory() {
                dialog = dialog.set_directory(dir);
            }

            if let Some(result) = dialog.pick_files().await {
                // 获取第一个文件的父目录作为下次的初始目录
                if let Some(first_file) = result.first() {
                    // 使用 path() 方法获取文件路径，然后再调用 parent()
                    if let Some(parent_dir) = first_file.path().parent() {
                        let dir_path: PathBuf = parent_dir.to_path_buf();
                        if let Err(e) = config.write().set_last_input_directory(dir_path) {
                            error_message.set(Some(format!("无法保存输入目录设置: {}", e)));
                        }
                    }
                }

                files
                    .write()
                    .extend(result.into_iter().map(|f| f.path().to_path_buf()));
            }
        }
    };

    let remove_file = move |index: usize| {
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
        div { class: " flex-1",
            div { class: "max-w-2xl mx-auto pt-2 overflow-y-auto",
                // 标题区域
                div { class: "text-center mb-2",
                    h1 { class: "text-3xl font-bold mb-2 tracking-tight", "MP4文件合并工具" }
                }

                // 主要内容卡片
                div { class: "bg-gray-800/80 backdrop-blur-lg rounded-2xl shadow-2xl border border-gray-700 overflow-hidden" }

                // 文件选择区域
                div { class: "p-6 pt-2 border-b border-gray-700",
                    div { class: "flex items-center justify-between",
                        h2 { class: "text-xl font-semibold flex items-center gap-2",
                            "选择要合并的MP4文件"
                        }
                        Button { onclick: add_files, "添加文件" }
                    }

                    // 文件列表
                    FileList { files, on_remove: remove_file }

                }

                // 输出文件名设置区域
                div { class: "p-6 pt-2 border-b border-gray-700",
                    h2 { class: "text-sm font-semibold mb-2 flex items-center gap-2",
                        "输出文件设置"
                    }
                    OutputSettings {
                        output_filename,
                        config,
                        on_select_dir: select_output_directory,
                        on_clear_dir: clear_output_directory,
                    }

                }

                // 合并按钮和状态区域
                div { class: "p-6 pt-2",
                    div { class: "flex justify-center mb-6",
                        Button { disabled: is_merging(), onclick: merge_files,
                            if is_merging() {
                                "合并中..."
                            } else {
                                "开始合并"
                            }
                        }
                    }

                    // 进度条
                    if is_merging() || progress() > 0.0 {
                        div { class: "space-y-3 w-full",
                            div { class: "flex justify-between items-center",
                                span { class: " font-semibold", "合并进度" }
                                span { class: "text-purple-400 font-mono", "{progress():.1}%" }
                            }
                            Progress {
                                aria_label: "Progressbar Demo",
                                value: progress() as f64,
                                ProgressIndicator {}
                            }
                        }
                    }
                }
            }

        }

    }
}
