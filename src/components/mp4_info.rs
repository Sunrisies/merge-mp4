use crate::components::button::{Button, ButtonVariant};
use crate::config::AppConfig;
use crate::utils::{format_duration, parse_duration_to_seconds};
use chrono::{DateTime, Local};
use dioxus::prelude::*;
use std::collections::HashSet;
use std::ops::{AddAssign, SubAssign};
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
    pub codec: String,      // H.264 / H.265 / HEVC / AV1 等
    pub duration: String,   // 秒
    pub file_path: PathBuf, // 添加文件路径
}
// 进度状态
#[derive(Debug, Clone, Default)]
pub struct ScanProgress {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
}
#[derive(Clone, Copy, PartialEq)]
enum SortBy {
    Duration,
}
#[component]
pub fn Mp4Info(mut config: Signal<AppConfig>) -> Element {
    let mut selected_directory: Signal<Option<PathBuf>> =
        use_signal(|| config.read().get_query_directory());
    let mut files: Signal<Vec<Mp4FileInfo>> = use_signal(Vec::new);
    let mut paginated_files: Signal<Vec<Mp4FileInfo>> = use_signal(Vec::new);

    let mut is_loading: Signal<bool> = use_signal(|| false);
    let mut error_message: Signal<Option<String>> = use_signal(|| None);
    // 3. 添加取消扫描的功能
    let mut should_cancel = use_signal(|| Arc::new(AtomicBool::new(false)));
    // 新增：进度状态
    let mut progress: Signal<ScanProgress> = use_signal(ScanProgress::default);
    let sort_by: Signal<SortBy> = use_signal(|| SortBy::Duration);
    let sort_desc: Signal<bool> = use_signal(|| true); // 默认降序（新的在前）
    let mut deleting_files: Signal<HashSet<PathBuf>> = use_signal(Default::default); // 新增：跟踪正在删除的文件
    // 分页状态
    let mut current_page: Signal<usize> = use_signal(|| 1); // 从1开始
    let mut page_size: Signal<usize> = use_signal(|| 20); // 默认每页20条
    let mut selected_files: Signal<HashSet<PathBuf>> = use_signal(Default::default);
    let mut select_all_page: Signal<bool> = use_signal(|| false);
    let total_pages = {
        let files_len = files.read().len();
        let size = *page_size.read();
        files_len.div_ceil(size)
    };

    // 计算当前页的文件切片
    let mut update_paginated_files = move || {
        let all_files = files.read();
        let page = *current_page.read();
        let size = *page_size.read();
        let start = (page - 1) * size;
        let end = (start + size).min(all_files.len());
        paginated_files.set(all_files[start..end].to_vec());
    };
    // 使用use_effect在相关状态变化时更新
    use_effect(move || {
        update_paginated_files();
    });
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
                    let mut mp4_files = Vec::with_capacity(total);

                    for (idx, path) in mp4_paths.into_iter().enumerate() {
                        // 检查是否取消
                        if cancel_flag_for_blocking.load(Ordering::SeqCst) {
                            break;
                        }

                        let file_name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("未知文件")
                            .to_string();

                        // 创建进度更新
                        let progress_update = ScanProgress {
                            current: idx + 1,
                            total,
                            current_file: file_name.clone(),
                        };
                        let tx_clone = tx_for_task.clone();
                        let _ = futures::executor::block_on(async {
                            tx_clone.send(progress_update).await.ok()
                        });
                        match parse_mp4_info(path) {
                            Ok(info) => {
                                // println!("解析到文件信息: {:?}", info);
                                mp4_files.push(info);
                            }
                            Err(e) => {
                                println!("解析文件信息失败: {} - {}", file_name, e);
                            }
                        }
                    }

                    Ok(mp4_files)
                })
                .await;
                drop(tx);

                match result {
                    Ok(Ok(mp4_files)) => {
                        println!("扫描到 {} 个 MP4 文件", mp4_files.len(),);
                        println!("扫描耗时: {:.2} 秒", start.elapsed().as_secs());
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
    let mut cancel_scan = move || {
        should_cancel.read().store(true, Ordering::SeqCst);
        is_loading.set(false);
    };
    // 计算进度百分比
    let progress_percent = {
        let p = progress.read();
        if p.total > 0 {
            (p.current as f32 / p.total as f32 * 100.0) as u32
        } else {
            0
        }
    };
    // 2. 在组件中使用排序函数
    let handle_sort = {
        // 开始时间
        let start = Instant::now();
        let mut sort_by_clone = sort_by;
        let mut sort_desc_clone = sort_desc;
        let mut files_clone = files;
        let mut current_page_clone = current_page; // 添加
        move |field: SortBy| {
            let current_field = *sort_by.read();
            let current_desc = *sort_desc_clone.read();

            if current_field == field {
                sort_desc_clone.set(!current_desc);
            } else {
                sort_by_clone.set(field);
                // 根据字段设置默认排序方向
                match field {
                    SortBy::Duration => sort_desc_clone.set(true), // 时长默认降序
                }
            }

            // 获取新的排序参数
            let new_field = *sort_by_clone.read();
            let new_desc = *sort_desc_clone.read();
            current_page_clone.set(1);
            // 对文件进行排序
            let mut sorted_files = files_clone.read().clone();
            sort_mp4_files(&mut sorted_files, new_field, new_desc);
            println!("排序耗时: {:.2} 毫秒", start.elapsed().as_millis());
            files_clone.set(sorted_files);
        }
    };
    let mut sort_by_duration = {
        let mut handle_sort_clone = handle_sort;
        move || handle_sort_clone(SortBy::Duration)
    };

    let open_file = {
        // let error_message = error_message.clone();
        move |path: PathBuf| {
            // let mut error_message = error_message.clone();
            spawn(async move {
                // /select 参数：打开资源管理器并选中指定文件
                let result = std::process::Command::new("explorer")
                    .args(["/select,", &path.to_string_lossy()])
                    .spawn();

                if let Err(e) = result {
                    error_message.set(Some(format!("无法打开资源管理器: {}", e)));
                }
            });
        }
    };

    // 删除文件（带确认对话框）
    let delete_file = {
        move |path: PathBuf| {
            let path_for_operations = path.clone();
            let mut files = files;
            let mut error_message = error_message;
            let mut deleting_files = deleting_files;
            let mut current_page = current_page; // 需要添加这个捕获
            spawn(async move {
                // 显示确认对话框
                if deleting_files.read().contains(&path) {
                    return;
                }
                // 添加到删除集合
                deleting_files.write().insert(path.clone());

                // 显示确认对话框
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "未知文件".to_string());

                let result = rfd::AsyncMessageDialog::new()
                    .set_title("确认删除")
                    .set_description(format!(
                        "确定要永久删除文件 \"{}\" 吗？\n此操作不可撤销。",
                        file_name
                    ))
                    .set_buttons(rfd::MessageButtons::OkCancel)
                    .show()
                    .await;

                if result == rfd::MessageDialogResult::Ok {
                    // 开始时间
                    let start = Instant::now();
                    // 使用spawn_blocking执行文件系统操作
                    let delete_result =
                        tokio::task::spawn_blocking(move || std::fs::remove_file(&path)).await;

                    match delete_result {
                        Ok(Ok(_)) => {
                            let remaining_count = {
                                let mut files_guard = files.write();
                                if let Some(pos) = files_guard
                                    .iter()
                                    .position(|f| f.file_path == path_for_operations)
                                {
                                    files_guard.remove(pos);
                                    println!("删除耗时: {:.2} 毫秒", start.elapsed().as_millis());
                                }
                                // 返回剩余数量，这样就不需要在持有锁的时候读取
                                files_guard.len()
                            }; // 这里写锁被释放
                            // 现在可以安全地读取，不需要files_clone
                            let size = *page_size.read();
                            let new_total_pages = if remaining_count == 0 {
                                1
                            } else {
                                remaining_count.div_ceil(size)
                            };

                            let current = *current_page.read();
                            if current > new_total_pages {
                                current_page.set(new_total_pages.max(1));
                            }
                        }
                        Ok(Err(e)) => {
                            error_message.set(Some(format!("删除失败: {}", e)));
                        }
                        Err(e) => {
                            error_message.set(Some(format!("任务失败: {}", e)));
                        }
                    }
                }

                // 无论结果如何，都从删除集合中移除
                deleting_files.write().remove(&path_for_operations);
            });
        }
    };
    // 分页控制函数
    let mut go_to_page = {
        move |page: usize| {
            let page = page.max(1).min(total_pages);
            current_page.set(page);
            // 切换页面时清空选择
            selected_files.write().clear();
            select_all_page.set(false);
        }
    };

    let mut go_prev = {
        move || {
            if *current_page.read() > 1 {
                current_page.write().sub_assign(1);
                // 切换页面时清空选择
                selected_files.write().clear();
                select_all_page.set(false);
            }
        }
    };

    let mut go_next = {
        move || {
            if *current_page.read() < total_pages {
                current_page.write().add_assign(1);
                // 切换页面时清空选择
                selected_files.write().clear();
                select_all_page.set(false);
            }
        }
    };

    let mut set_page_size = {
        let mut current_page = current_page;
        move |new_size: usize| {
            page_size.set(new_size);
            current_page.set(1); // 切换每页数量时回到第一页
        }
    };
    // 批量删除函数
    let mut batch_delete = {
        move || {
            let selected = selected_files.read().clone();
            if selected.is_empty() {
                error_message.set(Some("请先选择要删除的文件".to_string()));
                return;
            }

            spawn(async move {
                // 显示确认对话框
                let result = rfd::AsyncMessageDialog::new()
                    .set_title("确认批量删除")
                    .set_description(format!(
                        "确定要永久删除选中的 {} 个文件吗？\n此操作不可撤销。",
                        selected.len()
                    ))
                    .set_buttons(rfd::MessageButtons::OkCancel)
                    .show()
                    .await;

                if result == rfd::MessageDialogResult::Ok {
                    // 开始时间
                    let start = Instant::now();

                    // 添加到删除集合
                    for path in &selected {
                        deleting_files.write().insert(path.clone());
                    }

                    let mut success_count = 0;
                    let mut failed_files = Vec::new();

                    // 逐个删除文件
                    for path in &selected {
                        let delete_result = tokio::task::spawn_blocking({
                            let path = path.clone();
                            move || std::fs::remove_file(&path)
                        })
                        .await;

                        match delete_result {
                            Ok(Ok(_)) => {
                                success_count += 1;
                            }
                            Ok(Err(e)) => {
                                failed_files.push((path.display().to_string(), e.to_string()));
                            }
                            Err(e) => {
                                failed_files.push((path.display().to_string(), e.to_string()));
                            }
                        }
                    }

                    // 从列表中移除已删除的文件
                    if success_count > 0 {
                        let mut files_guard = files.write();
                        files_guard.retain(|f| !selected.contains(&f.file_path));
                    }

                    // 显示结果
                    if !failed_files.is_empty() {
                        let error_list = failed_files
                            .iter()
                            .map(|(file, err)| format!("{}: {}", file, err))
                            .collect::<Vec<_>>()
                            .join("\n");

                        error_message.set(Some(format!(
                            "成功删除 {} 个文件，失败 {} 个：\n{}",
                            success_count,
                            failed_files.len(),
                            error_list
                        )));
                    } else {
                        error_message.set(Some(format!(
                            "成功删除 {} 个文件，耗时 {:.2} 秒",
                            success_count,
                            start.elapsed().as_secs_f32()
                        )));
                    }

                    // 清空选择
                    selected_files.write().clear();
                    select_all_page.set(false);

                    // 从删除集合中移除
                    for path in &selected {
                        deleting_files.write().remove(path);
                    }
                }
            });
        }
    };
    rsx! {
        div { class: "flex flex-col h-full p-2",
            div { class: "flex flex-col  overflow-hidden",
                // 顶部操作区域
                div {
                    // 错误消息
                    if let Some(error) = error_message.read().as_ref() {
                        div { class: "mb-4 p-4 rounded-xl bg-red-50 border border-red-200 flex items-start gap-3 animate-pulse",
                            div { class: "text-red-500 text-xl", "⚠️" }
                            div { class: "flex-1",
                                p { class: "font-medium text-red-800", "操作失败" }
                                p { class: "text-sm text-red-600 mt-1", {error.to_string()} }
                            }
                        }
                    }
                }
                // 输出目录选择
                div { class: "flex sm:flex-row gap-3",
                    div { class: "flex-1 flex items-center gap-3 p-2 border border-black-300 rounded-xl ",
                        span { class: "text-gray-400 text-lg", "📂" }
                        div { class: "flex-1 min-w-0",
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
                    // 加载状态
                    div { class: "flex-1 flex flex-col items-center justify-center p-8",
                        div { class: "w-full max-w-md",

                            // 进度显示
                            div { class: "bg-white rounded-2xl shadow-lg p-6 border border-gray-200",
                                div { class: "flex justify-between items-center mb-6",
                                    div { class: "flex-1",
                                        h3 { class: "text-lg font-semibold text-gray-800 mb-2 flex items-center gap-2",
                                            span { class: "text-blue-500 animate-spin",
                                                "🔄"
                                            }
                                            "正在扫描文件..."
                                        }
                                        p {
                                            class: "text-sm text-gray-600 truncate w-[300px]",
                                            title: "正在扫描: {progress.read().current_file}",
                                            "正在扫描: {progress.read().current_file}"
                                        }
                                    }
                                    div { class: "text-right",
                                        p { class: "text-2xl font-bold text-blue-600",
                                            "{progress_percent}%"
                                        }
                                        p { class: "text-sm text-gray-500 mt-1",
                                            "{progress.read().current} / {progress.read().total} 文件"
                                        }
                                    }
                                }

                                // 进度条
                                div { class: "relative h-4 bg-gray-200 rounded-full overflow-hidden",
                                    div {
                                        class: "absolute top-0 left-0 h-full bg-gradient-to-r from-blue-500 to-blue-600 rounded-full transition-all duration-500 ease-out shadow-inner",
                                        style: "width: {progress_percent}%",
                                    }
                                }

                                // 文件进度
                                div { class: "mt-6 pt-6 border-t border-gray-200",
                                    div { class: "grid grid-cols-3 gap-2",
                                        div { class: "text-center",
                                            p { class: "text-xs text-gray-500", "已处理文件" }
                                            p { class: "text-lg font-semibold text-gray-800",
                                                "{progress.read().current}"
                                            }
                                        }
                                        // 取消按钮
                                        Button {
                                            onclick: move |_| cancel_scan(),
                                            variant: ButtonVariant::Destructive,
                                            span { "✕" }
                                            "取消扫描"
                                        }
                                        div { class: "text-center",
                                            p { class: "text-xs text-gray-500", "剩余文件" }
                                            p { class: "text-lg font-semibold text-gray-800",
                                                "{progress.read().total.saturating_sub(progress.read().current)}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if !files.read().is_empty() {
                    div { class: "grid grid-rows-[auto_1fr_auto] gap-2  overflow-hidden",
                        // 顶部统计和分页控制
                        // 顶部统计、批量操作和分页控制
                        div { class: "flex justify-between items-center",
                            // 左侧：批量操作按钮
                            div { class: "flex items-center gap-4",
                                // 批量删除按钮（当有选中文件时显示）
                                if !selected_files.read().is_empty() {
                                    Button {
                                        class: "px-4 py-2 bg-red-500 text-white rounded-md hover:bg-red-600 transition-colors flex items-center gap-2",
                                        onclick: move |_| batch_delete(),
                                        svg {
                                            class: "w-4 h-4",
                                            fill: "currentColor",
                                            view_box: "0 0 20 20",
                                            path {
                                                fill_rule: "evenodd",
                                                d: "M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z",
                                                clip_rule: "evenodd",
                                            }
                                        }
                                        "批量删除 ({selected_files.read().len()})"
                                    }
                                } else {
                                    div { class: "text-sm text-gray-500",
                                        "选择文件进行批量操作"
                                    }
                                }
                            }

                            // 中间：统计信息
                            div { class: "text-sm text-gray-600",
                                span { "共 {files.len()} 个文件" }
                                if !selected_files.read().is_empty() {
                                    span { class: "ml-2 text-blue-600",
                                        "已选择 {selected_files.read().len()} 个"
                                    }
                                }
                            }

                            // 右侧：每页数量选择
                            div { class: "flex items-center gap-2",
                                span { class: "text-sm text-gray-600", "每页" }
                                select {
                                    class: "border rounded px-2 py-1 text-sm bg-white",
                                    onchange: move |evt| {
                                        if let Ok(size) = evt.value().parse::<usize>() {
                                            set_page_size(size);
                                            // 重置选择状态
                                            selected_files.write().clear();
                                            select_all_page.set(false);
                                        }
                                    },
                                    option {
                                        value: "10",
                                        selected: *page_size.read() == 10,
                                        "10"
                                    }
                                    option {
                                        value: "20",
                                        selected: *page_size.read() == 20,
                                        "20"
                                    }
                                    option {
                                        value: "50",
                                        selected: *page_size.read() == 50,
                                        "50"
                                    }
                                    option {
                                        value: "100",
                                        selected: *page_size.read() == 100,
                                        "100"
                                    }
                                }
                                span { class: "text-sm text-gray-600", "条" }
                            }
                        }

                        div { class: "border border-gray-200 rounded-md overflow-auto h-[380]",
                            table { class: "w-full table-auto divide-y divide-gray-200 min-w-max",
                                thead { class: "bg-gray-50 sticky top-0 z-10",
                                    tr {
                                        // 全选复选框
                                        th { class: "px-2 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider w-10",
                                            input {
                                                r#type: "checkbox",
                                                class: "rounded border-gray-300 text-blue-600 focus:ring-blue-500",
                                                checked: select_all_page(),
                                                onchange: move |evt| {
                                                    let is_checked = evt.value().parse::<bool>().unwrap_or(false);
                                                    select_all_page.set(is_checked);

                                                    let current_files: Vec<PathBuf> = paginated_files
                                                        .iter()
                                                        .map(|f| f.file_path.clone())
                                                        .collect();
                                                    let mut selected = selected_files.write();
                                                    if is_checked {
                                                        for path in current_files {
                                                            selected.insert(path);
                                                        }
                                                    } else {
                                                        for path in current_files {
                                                            selected.remove(&path);
                                                        }
                                                    }
                                                },
                                            }
                                        }
                                        // 序号列
                                        th { class: "px-2 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider whitespace-nowrap w-12",
                                            "序号"
                                        }
                                        th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider whitespace-nowrap w-32",
                                            "文件名"
                                        }
                                        th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider whitespace-nowrap",
                                            "分辨率"
                                        }
                                        th { class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider whitespace-nowrap",
                                            "编码格式"
                                        }
                                        th {
                                            class: "px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider whitespace-nowrap flex",
                                            onclick: move |_| sort_by_duration(),
                                            span { "时长" }
                                            div { class: "ml-1 w-3 h-3",
                                                if *sort_by.read() == SortBy::Duration {
                                                    if *sort_desc.read() {
                                                        span { "↓" }
                                                    } else {
                                                        span { "↑" }
                                                    }
                                                } else {
                                                    span { class: "text-gray-300", "↕" }
                                                }
                                            }
                                        }
                                        th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider whitespace-nowrap w-1/4",
                                            "大小"
                                        }
                                        th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider whitespace-nowrap w-1/4",
                                            "修改日期"
                                        }
                                        th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider whitespace-nowrap w-64",
                                            "操作"
                                        }
                                    }
                                }
                                tbody { class: "bg-white divide-y divide-gray-200",
                                    for (index , info) in paginated_files.iter().enumerate() {
                                        {
                                            let info_clone = info.clone();
                                            let file_path = info.file_path.clone();
                                            let is_selected = selected_files.read().contains(&file_path);
                                            rsx! {
                                                tr { class: if selected_files.read().contains(&info_clone.file_path) { "bg-blue-50" } else { "" },
                                                    // 单行复选框
                                                    td { class: "px-2 py-4",
                                                        input {
                                                            r#type: "checkbox",
                                                            class: "rounded border-gray-300 text-blue-600 focus:ring-blue-500",
                                                            checked: is_selected,
                                                            onclick: {
                                                                let path = file_path.clone();
                                                                let mut selected = selected_files;
                                                                let mut select_all_page = select_all_page;

                                                                move |_| {
                                                                    let mut selected_guard = selected.write();
                                                                    if selected_guard.contains(&path) {
                                                                        selected_guard.remove(&path);
                                                                        select_all_page.set(false);
                                                                    } else {
                                                                        selected_guard.insert(path.clone());
                                                                    }
                                                                }
                                                            },

                                                        }
                                                    }
                                                    // 序号（计算当前页的序号）
                                                    td { class: "px-2 py-4 text-sm text-gray-500 text-center",
                                                        {format!("{}", (current_page() - 1) * page_size() + index + 1)}
                                                    }
                                                    td {
                                                        class: "px-2 py-4 text-sm text-gray-900 truncate",
                                                        title: "{info.file_name}",
                                                        {info.file_name.clone()}
                                                    }
                                                    td { class: "px-4 py-4 text-sm text-gray-500 whitespace-nowrap",
                                                        {
                                                            if info.width > 0 && info.height > 0 {
                                                                format!("{}x{}", info.width, info.height)
                                                            } else {
                                                                "未知".to_string()
                                                            }
                                                        }
                                                    }
                                                    td { class: "px-4 py-4 text-sm text-gray-500 whitespace-nowrap", {info.codec.clone()} }
                                                    td { class: "px-4 py-4 text-sm text-gray-500 whitespace-nowrap", {info.duration.clone()} }
                                                    td { class: "px-2 py-4 text-sm text-gray-500 whitespace-nowrap", {format_size(Some(info.size))} }
                                                    td {
                                                        class: "px-2 py-4 text-sm text-gray-500 truncate",
                                                        title: "{format_date(info.modified)}",
                                                        {format_date(info.modified)}
                                                    }
                                                    td { class: "flex gap-2",
                                                        Button {
                                                            class: "px-3 py-1 text-xs bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors",
                                                            onclick: {
                                                                let path = info.file_path.clone();
                                                                move |_| open_file(path.clone())
                                                            },
                                                            "打开"
                                                        }

                                                        // 删除按钮
                                                        Button {
                                                            class: "px-3 py-1 text-xs bg-red-500 text-white rounded hover:bg-red-600 transition-colors",
                                                            onclick: {
                                                                let path = info.file_path.clone();
                                                                move |_| delete_file(path.clone())
                                                            },
                                                            "删除"
                                                        }

                                                        // 转码占位（后续实现）
                                                        Button {
                                                            class: "px-3 py-1 text-xs bg-gray-300 text-gray-700 rounded cursor-not-allowed",
                                                            disabled: true,
                                                            "转码"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                }
                            }
                        }
                        // 分页控制器
                        if total_pages > 1 {
                            div { class: "flex justify-center items-center gap-2 mt-2",
                                // 首页
                                Button {
                                    class: "px-3 py-1 text-sm border rounded hover:bg-gray-100 disabled:opacity-50 disabled:cursor-not-allowed",
                                    disabled: *current_page.read() == 1,
                                    onclick: move |_| go_to_page(1),
                                    "⏮ 首页"
                                }

                                // 上一页
                                Button {
                                    class: "px-3 py-1 text-sm border rounded hover:bg-gray-100 disabled:opacity-50",
                                    disabled: *current_page.read() == 1,
                                    onclick: move |_| go_prev(),
                                    "◀ 上一页"
                                }

                                // 页码显示和跳转
                                div { class: "flex items-center gap-2 mx-4",
                                    span { "第" }
                                    input {
                                        r#type: "number",
                                        class: "w-16 px-2 py-1 text-center border rounded text-sm",
                                        min: "1",
                                        max: "{total_pages}",
                                        value: "{current_page}",
                                        onchange: move |evt| {
                                            if let Ok(page) = evt.value().parse::<usize>() {
                                                go_to_page(page);
                                            }
                                        },
                                    }
                                    span { "页 / 共 {total_pages} 页" }
                                }

                                // 下一页
                                Button {
                                    class: "px-3 py-1 text-sm border rounded hover:bg-gray-100 disabled:opacity-50",
                                    disabled: *current_page.read() >= total_pages,
                                    onclick: move |_| go_next(),
                                    "下一页 ▶"
                                }

                                // 末页
                                Button {
                                    class: "px-3 py-1 text-sm border rounded hover:bg-gray-100 disabled:opacity-50",
                                    disabled: *current_page.read() >= total_pages,
                                    onclick: move |_| go_to_page(total_pages),
                                    "末页 ⏭"
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

/// 解析单个 MP4 文件信息
fn parse_mp4_info(path: PathBuf) -> Result<Mp4FileInfo, Box<dyn std::error::Error>> {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("未知文件")
        .to_string();

    let metadata = std::fs::metadata(&path)?;
    let modified = metadata.modified().ok();
    let size = metadata.len();

    // 使用 mp4 库解析视频信息
    let file = std::fs::File::open(&path)?;
    let size_u64 = file.metadata()?.len();
    let reader = std::io::BufReader::new(file);

    let mp4 = mp4::Mp4Reader::read_header(reader, size_u64)?;

    // 获取视频轨道信息
    let mut width = 0u16;
    let mut height = 0u16;
    let mut codec = "未知".to_string();
    // let mut duration = None::<f64>;
    let duration = mp4.duration().as_secs_f64();
    let duration = format_duration(duration);

    for track in mp4.tracks().values() {
        if let mp4::TrackType::Video = track.track_type()? {
            width = track.width();
            height = track.height();
            // 编解码器类型
            codec = match track.media_type() {
                Ok(mp4::MediaType::H264) => "H.264 / AVC".to_string(),
                Ok(mp4::MediaType::H265) => "H.265 / HEVC".to_string(),
                Ok(mp4::MediaType::VP9) => "VP9".to_string(),
                Ok(other) => format!("{:?}", other),
                Err(_) => "未知".to_string(),
            };
            break; // 只取第一个视频轨道
        }
    }

    Ok(Mp4FileInfo {
        file_name,
        size,
        modified,
        width,
        height,
        codec,
        duration,
        file_path: path, // 保存完整路径
    })
}

// 排序函数
// 1. 添加排序函数
fn sort_mp4_files(files: &mut [Mp4FileInfo], field: SortBy, desc: bool) {
    match field {
        SortBy::Duration => {
            files.sort_by(|a, b| {
                // 需要解析时长字符串为秒数进行比较
                let a_secs = parse_duration_to_seconds(&a.duration);
                let b_secs = parse_duration_to_seconds(&b.duration);
                a_secs
                    .partial_cmp(&b_secs)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    if desc {
        files.reverse();
    }
}
