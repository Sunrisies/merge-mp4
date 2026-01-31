use crate::utils::{format_date, format_size};
use dioxus::prelude::*;
use std::collections::HashSet;
use std::ops::{AddAssign, SubAssign};
use std::path::PathBuf;
use std::time::Instant;

use crate::components::button::Button;
use crate::components::mp4_info::Mp4FileInfo;
use crate::utils::parse_duration_to_seconds;

#[derive(Clone, Copy, PartialEq)]
enum SortBy {
    Duration,
}

#[component]
pub fn Mp4InfoTable(
    files: Signal<Vec<Mp4FileInfo>>,
    error_message: Signal<Option<String>>,
    open: Signal<bool>,
    file_name: Signal<String>,
    confirmed: Signal<bool>,
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
            // let path_for_operations = path.clone();
            // let mut files = files;
            // let mut error_message = error_message;
            // let mut deleting_files = deleting_files;
            // let mut current_page = current_page; // 需要添加这个捕获
            spawn(async move {
                open.set(true);

                // 显示确认对话框
                if deleting_files.read().contains(&path) {
                    return;
                }
                // 添加到删除集合
                deleting_files.write().insert(path.clone());

                // 显示确认对话框
                let file_name_table = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "未知文件".to_string());
                file_name.set(file_name_table);
                // let result = rfd::AsyncMessageDialog::new()
                //     .set_title("确认删除")
                //     .set_description(format!(
                //         "确定要永久删除文件 \"{}\" 吗？\n此操作不可撤销。",
                //         file_name
                //     ))
                //     .set_buttons(rfd::MessageButtons::OkCancel)
                //     .show()
                //     .await;
                // println!("删除文件: {:?}, {:?}", path, confirmed);
                // if result == rfd::MessageDialogResult::Ok {
                //     // 开始时间
                //     let start = Instant::now();
                //     // 使用spawn_blocking执行文件系统操作
                //     let delete_result =
                //         tokio::task::spawn_blocking(move || std::fs::remove_file(&path)).await;

                //     match delete_result {
                //         Ok(Ok(_)) => {
                //             let remaining_count = {
                //                 let mut files_guard = files.write();
                //                 if let Some(pos) = files_guard
                //                     .iter()
                //                     .position(|f| f.file_path == path_for_operations)
                //                 {
                //                     files_guard.remove(pos);
                //                     println!("删除耗时: {:.2} 毫秒", start.elapsed().as_millis());
                //                 }
                //                 // 返回剩余数量，这样就不需要在持有锁的时候读取
                //                 files_guard.len()
                //             }; // 这里写锁被释放
                //             // 现在可以安全地读取，不需要files_clone
                //             let size = *page_size.read();
                //             let new_total_pages = if remaining_count == 0 {
                //                 1
                //             } else {
                //                 remaining_count.div_ceil(size)
                //             };

                //             let current = *current_page.read();
                //             if current > new_total_pages {
                //                 current_page.set(new_total_pages.max(1));
                //             }
                //         }
                //         Ok(Err(e)) => {
                //             error_message.set(Some(format!("删除失败: {}", e)));
                //         }
                //         Err(e) => {
                //             error_message.set(Some(format!("任务失败: {}", e)));
                //         }
                //     }
                // }

                // // 无论结果如何，都从删除集合中移除
                // deleting_files.write().remove(&path_for_operations);
            });
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
    }

    if desc {
        files.reverse();
    }
}
