use crate::components::alert_dialog::{
    AlertDialogAction, AlertDialogActions, AlertDialogCancel, AlertDialogContent,
    AlertDialogDescription, AlertDialogRoot, AlertDialogTitle,
};
use crate::components::button::Button;
use crate::components::mp4_info_loading::Mp4InfoLoading;
use crate::components::mp4_info_table::Mp4InfoTable;
use crate::config::AppConfig;
use crate::utils::parse_mp4_info;
use dioxus_primitives::toast::{ToastOptions, use_toast};
use std::time::Duration;

use dioxus::prelude::*;
use rayon::prelude::*;
use std::time::Instant;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use tokio::sync::mpsc;
// MP4 文件信息结构
#[derive(Debug, Clone)]
pub struct Mp4FileInfo {
    pub file_name: String,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
    pub width: u16,
    pub height: u16,
    pub codec: String,    // H.264 / H.265 / HEVC / AV1 等
    pub duration: String, // 秒
    // 帧率
    pub frame_rate: String, // FPS
    // 码率
    pub bit_rate: String,   // kbps
    pub file_path: PathBuf, // 添加文件路径
}
// 进度状态
#[derive(Debug, Clone, Default)]
pub struct ScanProgress {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
    pub estimated_time_remaining: f64,
}

#[component]
pub fn Mp4Info(mut config: Signal<AppConfig>) -> Element {
    let mut selected_directory: Signal<Option<PathBuf>> =
        use_signal(|| config.read().get_query_directory());
    let mut files: Signal<Vec<Mp4FileInfo>> = use_signal(Vec::new);
    let mut open = use_signal(|| false);
    let mut confirmed = use_signal(|| false);
    let mut is_loading: Signal<bool> = use_signal(|| false);
    let mut error_message: Signal<Option<String>> = use_signal(|| None);
    // 3. 添加取消扫描的功能
    let mut should_cancel = use_signal(|| Arc::new(AtomicBool::new(false)));
    // 新增：进度状态
    let mut progress: Signal<ScanProgress> = use_signal(ScanProgress::default);
    // 弹窗的title
    let alert_title = use_signal(String::new);
    // 弹窗的描述
    let alert_description = use_signal(String::new);
    // 转码弹窗
    let toast = use_toast();

    // 提取核心逻辑为无参闭包，避免重复代码
    let mut perform_scan = move || {
        // 开始时间
        let start = Instant::now();
        let dir = selected_directory.read().clone();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        should_cancel.set(cancel_flag.clone());
        spawn(async move {
            if let Some(directory) = dir {
                is_loading.set(true);
                error_message.set(None); // 清除错误
                progress.set(ScanProgress::default()); // 重置进度
                // 创建通道用于接收进度更新
                let (tx, mut rx) = mpsc::channel::<ScanProgress>(100);
                let tx_for_task = tx.clone();
                let _ = spawn(async move {
                    while let Some(progress_update) = rx.recv().await {
                        progress.set(progress_update);
                    }
                });
                let cancel_flag_for_blocking = cancel_flag.clone();
                let result = tokio::task::spawn_blocking(move || {
                    // 先收集所有 MP4 文件路径
                    let mp4_paths: Vec<PathBuf> = match std::fs::read_dir(&directory) {
                        Ok(entries) => entries
                            .par_bridge()
                            .filter_map(|entry| entry.ok())
                            .map(|entry| entry.path())
                            .filter(|path| {
                                path.is_file()
                                    && path
                                        .extension()
                                        .map(|ext| ext.eq_ignore_ascii_case("mp4"))
                                        .unwrap_or(false)
                            })
                            .collect(),
                        Err(e) => return Err(e),
                    };

                    let total = mp4_paths.len();
                    // let mut mp4_files = Vec::with_capacity(total);
                    let batch_size = 100; // 每批处理100个文件
                    let mut mp4_files = Vec::with_capacity(total);
                    let start_time = std::time::Instant::now();

                    // 分批处理文件
                    for (batch_idx, chunk) in mp4_paths.chunks(batch_size).enumerate() {
                        // 检查是否取消
                        if cancel_flag_for_blocking.load(Ordering::SeqCst) {
                            break;
                        }

                        // 并行处理当前批次
                        let batch_results: Vec<Option<Mp4FileInfo>> = chunk
                            .par_iter()
                            .map(|path| match parse_mp4_info(path.clone()) {
                                Ok(info) => Some(info),
                                Err(e) => {
                                    let file_name = path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("未知文件");
                                    println!("解析文件信息失败: {} - {}", file_name, e);
                                    None
                                }
                            })
                            .collect();

                        // 收集成功的结果
                        for info in batch_results.into_iter().flatten() {
                            mp4_files.push(info);
                        }

                        // 更新进度
                        let current = (batch_idx + 1) * batch_size;
                        let elapsed = start_time.elapsed().as_secs_f64();
                        let estimated_total = elapsed * total as f64 / current as f64;
                        let remaining = estimated_total - elapsed;

                        let progress_update = ScanProgress {
                            current: current.min(total),
                            total,
                            current_file: format!(
                                "处理批次 {}/{}",
                                batch_idx + 1,
                                total.div_ceil(batch_size)
                            ),
                            estimated_time_remaining: (remaining * 100.0).round() / 100.0,
                        };

                        let tx_clone = tx_for_task.clone();
                        let _ = futures::executor::block_on(async {
                            tx_clone.send(progress_update).await.ok()
                        });
                    }

                    Ok(mp4_files)
                })
                .await;
                drop(tx);

                match result {
                    Ok(Ok(mp4_files)) => {
                        println!("扫描到 {} 个 MP4 文件", mp4_files.len(),);
                        println!("扫描耗时: {:.2} 秒", start.elapsed().as_secs_f64());
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
    // 5. 添加取消扫描的函数
    let cancel_scan = move || {
        should_cancel.read().store(true, Ordering::SeqCst);
        is_loading.set(false);
    };
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

    rsx! {
        div { class: "flex flex-col h-full p-2",
            div { class: "flex flex-col  overflow-hidden",
                // 输出目录选择
                div { class: "flex sm:flex-row gap-3",
                    div { class: "flex-1 flex items-center gap-3 px-2 py-1 border border-black-300 rounded-xl ",
                        span { class: "text-gray-400 text-lg", "📂" }
                        div { class: "flex-1 min-w-0 flex justify-between items-center",
                            p { class: "text-sm sm:text-base text-gray-800 truncate",
                                {
                                    selected_directory
                                        .read()
                                        .as_ref()
                                        .map(|p| p.display().to_string())
                                        .unwrap_or_else(|| "未选择目录".to_string())
                                }
                            }
                            p { class: "text-xs text-gray-500 mt-1",
                                if selected_directory.read().is_some() {
                                    "点击右侧按钮可以更改目录"
                                } else {
                                    "请先选择输出目录"
                                }
                            }
                        }
                    }
                    Button {
                        class: "bg-gradient-to-r from-blue-600 px-2 to-blue-700 hover:from-blue-700 hover:to-blue-800 text-white font-medium rounded-xl shadow-md hover:shadow-lg transition-all duration-300 transform hover:-translate-y-0.5 flex items-center justify-center gap-2",
                        onclick: select_output_directory,
                        disabled: is_loading(),
                        "选择目录"
                    }
                    // 扫描按钮
                    Button {
                        class: "bg-gradient-to-r from-green-600 px-2 to-emerald-600 hover:from-green-700 hover:to-emerald-700 text-white font-medium rounded-xl shadow-md hover:shadow-lg transition-all duration-300 transform hover:-translate-y-0.5 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:transform-none flex items-center gap-2",
                        disabled: selected_directory.read().is_none() || is_loading(),
                        onclick: on_scan_click,

                        if is_loading() {
                            "扫描中..."
                        } else {
                            "扫描目录"
                        }
                    }

                }

            }

            // 文件列表
            div { class: "mt-4 h-[calc(100%-60px)]",
                if is_loading() {
                    Mp4InfoLoading { progress, cancel_scan }
                } else if !files.read().is_empty() {
                    Mp4InfoTable {
                        files,
                        error_message,
                        open,
                        confirmed,
                        alert_title,
                        alert_description,
                                        // total_files,
                    // completed_files,
                    // transcode_progress,
                    // current_file_path,
                    }
                } else if selected_directory.read().is_some() && !is_loading() {
                    div { class: "text-center p-8 text-gray-500", "该目录下没有找到MP4文件" }
                }
            }
        }
        AlertDialogRoot { open: open(), on_open_change: move |v| open.set(v),
            AlertDialogContent {
                AlertDialogTitle { {alert_title} }
                AlertDialogDescription { {alert_description} }
                AlertDialogActions {
                    AlertDialogCancel { "取消" }
                    AlertDialogAction {
                        on_click: move |_| {
                            confirmed.set(true);
                            open.set(false);
                        },
                        "确定"
                    }

                }
            }
        }

        if confirmed() {
            p { style: "color: var(--contrast-error-color); margin-top: 16px; font-weight: 600;",
                "Item deleted!"
            }
        }

    }
}
