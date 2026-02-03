use crate::components::alert_dialog::{
    AlertDialogContent, AlertDialogDescription, AlertDialogRoot, AlertDialogTitle,
};
use crate::components::button::Button;
use crate::components::mp4_info::Mp4FileInfo;
use crate::ffmpeg::transcoding::transcode_file;
use crate::utils::{format_date, format_size};
use crate::utils::{format_duration, parse_duration_to_seconds};
use dioxus::prelude::*;
use dioxus_primitives::toast::{ToastOptions, use_toast};
use std::collections::HashSet;
use std::ops::{AddAssign, SubAssign};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq)]
enum SortBy {
    Duration, // 时长
    Size,     // 大小
}

#[component]
pub fn Mp4InfoTable(
    files: Signal<Vec<Mp4FileInfo>>,
    error_message: Signal<Option<String>>,
    open: Signal<bool>,
    confirmed: Signal<bool>,
    alert_title: Signal<String>,
    alert_description: Signal<String>,
) -> Element {
    // 分页状态
    let mut current_page: Signal<usize> = use_signal(|| 1); // 从1开始
    let mut page_size: Signal<usize> = use_signal(|| 20); // 默认每页20条
    let mut select_all_page: Signal<bool> = use_signal(|| false);
    let mut paginated_files: Signal<Vec<Mp4FileInfo>> = use_signal(Vec::new);
    let mut deleting_files: Signal<HashSet<PathBuf>> = use_signal(Default::default); // 新增：跟踪正在删除的文件
    let sort_by: Signal<SortBy> = use_signal(|| SortBy::Duration);
    let sort_desc: Signal<bool> = use_signal(|| true); // 默认降序（新的在前）
    let mut selected_files: Signal<HashSet<PathBuf>> = use_signal(Default::default);
    let mut show_transcode_dialog = use_signal(|| false);
    // 转码进度对话框状态
    let mut transcode_progress: Signal<f32> = use_signal(|| 0.0); // 单个的
    let mut completed_files: Signal<usize> = use_signal(|| 0); // 完成的数量
    let mut total_files: Signal<usize> = use_signal(|| 0); // 总文件数量
    let mut current_file_path: Signal<String> = use_signal(String::new); // 当前文件路径
    // 新增状态：用于估算
    let mut last_task_end_time = use_signal(Instant::now); // 上一个任务结束时间
    let mut avg_speed = use_signal(|| 0.0); // 平均处理速度
    let mut total_eta_seconds = use_signal(|| 0.0); // 总剩余时间
    let mut current_task_eta = use_signal(|| 0.0); // 当前任务剩余时间
    let toast = use_toast();

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
                    SortBy::Size => sort_desc_clone.set(false),    // 大小默认升序
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

    let mut Sort_by_size = {
        let mut handle_sort_clone = handle_sort;
        move || handle_sort_clone(SortBy::Size)
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
            spawn(async move {
                alert_title.set("删除文件".to_string());
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "未知文件".to_string());
                let description = format!(
                    "确定要永久删除文件 \"{}\" 吗？\n此操作不可撤销。",
                    file_name,
                );
                alert_description.set(description);
                open.set(true);

                // 等待确认
                while !confirmed() {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }

                if confirmed() {
                    if let Err(e) = tokio::fs::remove_file(&path).await {
                        error_message.set(Some(format!("删除文件失败: {}", e)));
                    } else {
                        // 从文件列表中移除
                        let mut files_guard = files.write();
                        if let Some(pos) = files_guard.iter().position(|f| f.file_path == path) {
                            files_guard.remove(pos);
                        }
                        toast.success(
                            "删除成功".to_string(),
                            ToastOptions::new()
                                .duration(Duration::from_secs(5))
                                .permanent(false),
                        );
                    }
                    confirmed.set(false);
                }
                // 添加到删除集合
                deleting_files.write().insert(path.clone());
            });
        }
    };

    let mut batch_delete = {
        move || {
            let selected = selected_files.read().clone();
            if selected.is_empty() {
                error_message.set(Some("请先选择要删除的文件".to_string()));
                return;
            }
            alert_title.set("删除文件".to_string());
            let description = format!(
                "确定要永久删除文件 \"选中的 {} 个文件\" 吗？\n此操作不可撤销。",
                selected.len(),
            );
            alert_description.set(description);
            // 设置确认对话框信息
            open.set(true);

            // 使用 use_effect 来处理确认后的删除操作
            use_effect(move || {
                if confirmed() {
                    let value = selected.clone();
                    spawn(async move {
                        let mut success_count = 0;
                        let mut failed_files = Vec::new();

                        for path in &value {
                            match tokio::fs::remove_file(&path).await {
                                Ok(_) => success_count += 1,
                                Err(e) => {
                                    failed_files.push((path.display().to_string(), e.to_string()))
                                }
                            }
                        }

                        // 更新文件列表
                        let mut files_guard = files.write();
                        files_guard.retain(|f| !value.contains(&f.file_path));

                        // 显示结果
                        if failed_files.is_empty() {
                            toast.success(
                                format!("成功删除 {} 个文件", success_count),
                                ToastOptions::new()
                                    .duration(Duration::from_secs(5))
                                    .permanent(false),
                            );
                        } else {
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
                        }

                        // 清空选择和重置状态
                        selected_files.write().clear();
                        select_all_page.set(false);
                        confirmed.set(false);
                    });
                }
            });
        }
    };
    let mut batch_transcode = {
        move || {
            let selected = selected_files.read().clone();
            if selected.is_empty() {
                error_message.set(Some("请先选择要转码的文件".to_string()));
                return;
            }

            total_files.set(selected.len());
            completed_files.set(0);
            current_file_path.set(String::new());

            let task_overhead_estimated = 2.0_f32; // 估算的任务间隔开销（秒）
            alert_title.set("转码文件".to_string());
            let description = format!(
                "确定要对选中的 {} 个文件进行转码吗？\n\n注意：\n- 转码过程可能需要较长时间\n- 转码后的文件将保存在原文件同目录下，文件名后会添加\"_transcoded\"后缀\n- 此操作不可撤销",
                selected.len(),
            );
            alert_description.set(description);
            open.set(true);

            use_effect(move || {
                if confirmed() {
                    let value = selected.clone();
                    let total = value.len();
                    show_transcode_dialog.set(true);
                    // 初始化开始时间
                    last_task_end_time.set(Instant::now());
                    spawn(async move {
                        let output_dir = Path::new("./").to_path_buf();

                        for (index, path) in value.iter().enumerate() {
                            // 更新当前处理的文件路径
                            current_file_path.set(path.display().to_string());
                            transcode_progress.set(0.0);
                            // 计算任务间隔开销
                            let now = Instant::now();
                            let overhead =
                                now.duration_since(*last_task_end_time.read()).as_secs_f32();
                            // 简单的平滑处理：如果间隔过大（比如第一个任务），使用估算值
                            let effective_overhead = if overhead > 10.0 {
                                task_overhead_estimated
                            } else {
                                overhead
                            };
                            last_task_end_time.set(now);
                            println!("转码文件: {}", path.display());
                            // 目录
                            transcode_file(path.clone(), &output_dir, move |p| {
                                // 在这里处理进度更新
                                // 更新当前任务进度
                                transcode_progress.set(p.percent);
                                current_task_eta.set(p.eta_seconds);

                                // 计算平均速度 (简单的指数移动平均 EMA)
                                let current_avg = *avg_speed.read();
                                let new_avg = if p.speed > 0.0 {
                                    if current_avg == 0.0 {
                                        p.speed
                                    } else {
                                        current_avg * 0.8 + p.speed * 0.2
                                    } // 0.2 是平滑因子
                                } else {
                                    current_avg
                                };
                                avg_speed.set(new_avg);

                                let elapsed = now.elapsed().as_secs_f32(); // 注意：这里闭包捕获的是循环开始时的 now，不准确，应该用 Instant::now()

                                let estimated_single_task_duration = if p.percent > 1.0 {
                                    elapsed / (p.percent / 100.0_f32)
                                } else {
                                    0.0_f32 // 刚开始，无法估算
                                };

                                let future_tasks_estimated = if estimated_single_task_duration > 0.0
                                {
                                    (total - index - 1) as f32
                                        * (estimated_single_task_duration + effective_overhead)
                                } else {
                                    0.0
                                };

                                total_eta_seconds.set(p.eta_seconds + future_tasks_estimated);
                                // 可以在这里更新UI或其他状态
                            })
                            .await;
                            last_task_end_time.set(Instant::now());
                            completed_files.set(index + 1);
                        }

                        // 转码完成
                        completed_files.set(total);
                        // 延迟关闭进度对话框
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        show_transcode_dialog.set(false);

                        // 清空选择和重置状态
                        selected_files.write().clear();
                        select_all_page.set(false);
                        confirmed.set(false);

                        toast.success(
                            format!("成功转码 {} 个文件", total),
                            ToastOptions::default(),
                        );
                    });
                }
            });
        }
    };

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("./style.css") }

        div { class: "grid grid-rows-[auto_1fr_auto] gap-2  overflow-hidden",
            // 顶部统计和分页控制
            // 顶部统计、批量操作和分页控制
            div { class: "flex justify-between items-center",
                // 左侧：批量操作按钮
                div { class: "flex items-center gap-4 h-12",
                    // 批量删除按钮（当有选中文件时显示）
                    if !selected_files.read().is_empty() {
                        Button {
                            class: "px-2 py-1 bg-red-500 text-white rounded-md hover:bg-red-600 transition-colors flex items-center gap-2",
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
                        Button {
                            class: "px-2 py-1 bg-green-500 text-white rounded-md hover:bg-green-600 transition-colors flex items-center gap-2",
                            onclick: move |_| batch_transcode(),
                            svg {
                                class: "w-4 h-4",
                                fill: "currentColor",
                                view_box: "0 0 20 20",
                                path {
                                    fill_rule: "evenodd",
                                    d: "M4 2a1 1 0 011 1v2.101a7.002 7.002 0 0111.601 2.566 1 1 0 11-1.885.666A5.002 5.002 0 005.999 7H9a1 1 0 010 2H4a1 1 0 01-1-1V3a1 1 0 011-1zm.008 9.057a1 1 0 011.276.61A5.002 5.002 0 0014.001 13H11a1 1 0 110-2h5a1 1 0 011 1v5a1 1 0 11-2 0v-2.101a7.002 7.002 0 01-11.601-2.566 1 1 0 01.61-1.276z",
                                    clip_rule: "evenodd",
                                }
                            }
                            "批量转码 ({selected_files.read().len()})"
                        }
                    } else {
                        div { class: "text-sm text-gray-500", "选择文件进行批量操作" }
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
                        option { value: "10", selected: *page_size.read() == 10, "10" }
                        option { value: "20", selected: *page_size.read() == 20, "20" }
                        option { value: "50", selected: *page_size.read() == 50, "50" }
                        option { value: "100", selected: *page_size.read() == 100, "100" }
                    }
                    span { class: "text-sm text-gray-600", "条" }
                }
            }

            div { class: "border border-gray-200 rounded-md overflow-auto h-[380]",
                table { class: "w-full table-fixed divide-y divide-gray-200 min-w-max border-separate border-spacing-0",
                    thead { class: "bg-gray-50 sticky top-0 z-10",
                        tr { class: "",
                            // 全选复选框
                            th { class: " text-left tracking-wider w-10  ",
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
                            th { class: "w-12", "序号" }
                            th { class: "w-60", "文件名" }
                            th { class: "w-28", "分辨率" }
                            th { class: "w-16", "帧率" }
                            th { class: "w-32", "比特率" }
                            th { class: "w-32", "编码格式" }
                            th {
                                class: "w-24",
                                onclick: move |_| sort_by_duration(),
                                span { class: "mr-1.5", "时长" }
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
                            th {
                                class: "w-28",
                                onclick: move |_| Sort_by_size(),
                                span { class: "mr-1.5", "大小" }
                                if *sort_by.read() == SortBy::Size {
                                    if *sort_desc.read() {
                                        span { "↓" }
                                    } else {
                                        span { "↑" }
                                    }
                                } else {
                                    span { class: "text-gray-300", "↕" }
                                }
                            }
                            th { class: "w-44", "修改日期" }
                            th { class: "w-48", "操作" }
                        }
                    }
                    tbody { class: "bg-white divide-y divide-gray-200",
                        for (index , info) in paginated_files.iter().enumerate() {
                            {
                                let info_clone = info.clone();
                                let file_path = info.file_path.clone();
                                let is_selected = selected_files.read().contains(&file_path);
                                rsx! {
                                    tr {
                                        class: if selected_files.read().contains(&info_clone.file_path) { "bg-blue-50 " } else { "" },
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
                                        // 单行复选框
                                        td { class: "px-2 py-4 ",
                                            input {
                                                r#type: "checkbox",
                                                class: "rounded border-gray-300 text-blue-600 focus:ring-blue-500",
                                                checked: is_selected,
                                            }
                                        }
                                        // 序号（计算当前页的序号）
                                        td { class: "px-2 py-4 text-sm text-gray-500 text-center ",
                                            {format!("{}", (current_page() - 1) * page_size() + index + 1)}
                                        }
                                        td {
                                            class: "px-2 py-4 text-sm text-gray-900 truncate text-left",
                                            title: "{info.file_name}",
                                            {info.file_name.clone()}
                                        }
                                        td { class: "px-4 py-4 text-sm text-gray-500 whitespace-nowrap ",
                                            {
                                                if info.width > 0 && info.height > 0 {
                                                    format!("{}x{}", info.width, info.height)
                                                } else {
                                                    "未知".to_string()
                                                }
                                            }
                                        }
                                        td { class: " ", {info.frame_rate.clone()} }
                                        td { class: " ", {info.bit_rate.clone()} }
                                        td { class: " ", {info.codec.clone()} }
                                        td { class: " ", {info.duration.clone()} }
                                        td { class: "px-2 py-4 text-sm text-gray-500 whitespace-nowrap ", {format_size(Some(info.size))} }
                                        td {
                                            class: "px-2 py-4 text-sm text-gray-500 truncate ",
                                            title: "{format_date(info.modified)}",
                                            {format_date(info.modified)}
                                        }
                                        td { class: "flex gap-2 items-center justify-center px-2 py-4 ",

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
        AlertDialogRoot {
            class: "p-0",
            open: show_transcode_dialog(),
            on_open_change: move |v| show_transcode_dialog.set(v),
            AlertDialogContent { class: "max-w-md w-full p-6 overflow-visible", // 设置最大宽度并防止溢出
                AlertDialogTitle { class: "text-lg font-semibold mb-4", "转码进度" }
                AlertDialogDescription { // 转码进度条
                    class: "overflow-visible mb-4", // 设置最大宽度并防止溢出

                    div { class: "w-full bg-gray-200 rounded-full h-2.5 mb-2",
                        div {
                            class: "bg-blue-600 h-2.5 rounded-full transition-all duration-300",
                            style: "width: {transcode_progress()}%",
                        }
                    }

                    // 转码任务信息
                    div { class: "flex justify-between items-center mb-2",
                        div { class: "text-sm text-gray-600",
                            "已处理: {completed_files()} / {total_files()}"
                        }
                        // 新增：显示总剩余时间
                        div { class: "text-sm font-semibold text-blue-600",
                            {
                                let total_sec = *total_eta_seconds.read();
                                if total_sec > 0.0 {
                                    format!("总剩余: {}", format_duration(total_sec.into()))
                                } else {
                                    "计算中...".to_string()
                                }
                            }
                        }
                    }

                    // 当前处理的文件信息
                    if !current_file_path().is_empty() {
                        div { class: "mt-2 p-3 bg-gray-100 rounded",
                            div { class: "text-xs text-gray-500 mb-1", "当前处理文件:" }
                            div { class: "text-sm text-gray-800 truncate", "{current_file_path()}" }
                            div {
                                {
                                    let task_sec = *current_task_eta.read();
                                    // 将格式化逻辑完全包含在变量中
                                    if task_sec > 0.0 {
                                        format!("当前任务剩余: {}", format_duration(task_sec.into()))
                                    } else {
                                        "当前任务剩余: --".to_string()
                                    };
                                }
                            }
                        }
                    }

                    // 转码状态信息
                    div { class: "mt-4 text-sm text-gray-600",
                        if transcode_progress() >= 100.0 {
                            "转码已完成"
                        } else {
                            "转码进行中，请稍候..."
                        }
                    }
                }

            }
        }

    }
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
        SortBy::Size => {
            files.sort_by(|a, b| a.size.cmp(&b.size));
        }
    }

    if desc {
        files.reverse();
    }
}
