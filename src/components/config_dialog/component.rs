use crate::components::alert_dialog::{
    AlertDialogAction, AlertDialogActions, AlertDialogCancel, AlertDialogContent,
    AlertDialogDescription, AlertDialogRoot, AlertDialogTitle,
};
use crate::components::button::Button;
use crate::components::input::Input;
use crate::config::AppConfig;
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn ConfigDialog(
    open: Signal<bool>,
    config: Signal<AppConfig>,
    on_save: EventHandler<AppConfig>,
) -> Element {
    // 本地状态用于存储输入的路径
    let mut merge_input_dir = use_signal(|| {
        config
            .read()
            .last_input_directory
            .clone()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .unwrap_or_default()
    });

    let mut merge_output_dir = use_signal(|| {
        config
            .read()
            .output_directory
            .clone()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .unwrap_or_default()
    });

    let mut transcode_input_dir = use_signal(|| {
        config
            .read()
            .last_input_directory
            .clone()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .unwrap_or_default()
    });

    let mut transcode_output_dir = use_signal(|| {
        config
            .read()
            .compress_output_directory
            .clone()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .unwrap_or_default()
    });

    // 选择合并导入目录
    let select_merge_input_dir = {
        move |_| async move {
            if let Some(result) = rfd::AsyncFileDialog::new()
                .set_title("选择合并导入目录")
                .pick_folder()
                .await
            {
                let path = result.path().to_path_buf();
                if let Some(path_str) = path.to_str() {
                    merge_input_dir.set(path_str.to_string());
                }
            }
        }
    };

    // 选择合并导出目录
    let select_merge_output_dir = {
        move |_| async move {
            if let Some(result) = rfd::AsyncFileDialog::new()
                .set_title("选择合并导出目录")
                .pick_folder()
                .await
            {
                let path = result.path().to_path_buf();
                if let Some(path_str) = path.to_str() {
                    merge_output_dir.set(path_str.to_string());
                }
            }
        }
    };

    // 选择转码导入目录
    let select_transcode_input_dir = {
        move |_| async move {
            if let Some(result) = rfd::AsyncFileDialog::new()
                .set_title("选择转码导入目录")
                .pick_folder()
                .await
            {
                let path = result.path().to_path_buf();
                if let Some(path_str) = path.to_str() {
                    transcode_input_dir.set(path_str.to_string());
                }
            }
        }
    };

    // 选择转码导出目录
    let select_transcode_output_dir = {
        move |_| async move {
            if let Some(result) = rfd::AsyncFileDialog::new()
                .set_title("选择转码导出目录")
                .pick_folder()
                .await
            {
                let path = result.path().to_path_buf();
                if let Some(path_str) = path.to_str() {
                    transcode_output_dir.set(path_str.to_string());
                }
            }
        }
    };

    let handle_save = move |_| {
        let mut new_config = config.read().clone();

        // 更新合并相关配置
        if !merge_input_dir.read().is_empty() {
            new_config.last_input_directory = Some(PathBuf::from(merge_input_dir.read().as_str()));
        }
        if !merge_output_dir.read().is_empty() {
            new_config.output_directory = Some(PathBuf::from(merge_output_dir.read().as_str()));
        }

        // 更新转码相关配置
        if !transcode_input_dir.read().is_empty() {
            new_config.last_input_directory =
                Some(PathBuf::from(transcode_input_dir.read().as_str()));
        }
        if !transcode_output_dir.read().is_empty() {
            new_config.compress_output_directory =
                Some(PathBuf::from(transcode_output_dir.read().as_str()));
        }

        // 保存配置
        if let Err(e) = new_config.save() {
            eprintln!("Failed to save config: {}", e);
        }

        // 触发保存事件
        on_save.call(new_config.clone());

        // 更新全局配置
        config.set(new_config);

        // 关闭弹窗
        open.set(false);
    };

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("./style.css") }

        AlertDialogRoot { open: *open.read(), on_open_change: move |v| open.set(v),
            AlertDialogContent { class: "config-dialog",
                AlertDialogTitle { "配置设置" }
                AlertDialogDescription { "设置合并和转码的默认目录" }

                div { class: "config-form",
                    // 合并配置部分
                    div { class: "config-section",
                        h3 { class: "section-title", "合并设置" }

                        div { class: "form-field",
                            label { "导入目录：" }
                            div { class: "input-group ",
                                Input {
                                    r#type: "text",
                                    value: "{merge_input_dir}",
                                    oninput: move |e: FormEvent| merge_input_dir.set(e.value()),
                                }
                                Button { onclick: select_merge_input_dir, "浏览" }
                            }
                        }

                        div { class: "form-field",
                            label { "导出目录：" }
                            div { class: "input-group",
                                Input {
                                    r#type: "text",
                                    value: "{merge_output_dir}",
                                    oninput: move |e: FormEvent| {
                                        merge_output_dir.set(e.value());
                                    },
                                }
                                Button { onclick: select_merge_output_dir, "浏览" }
                            }
                        }
                    }

                    // 转码配置部分
                    div { class: "config-section",
                        h3 { class: "section-title", "转码设置" }

                        div { class: "form-field",
                            label { "导入目录：" }
                            div { class: "input-group",
                                Input {
                                    r#type: "text",
                                    value: "{transcode_input_dir}",
                                    oninput: move |e: FormEvent| {
                                        transcode_input_dir.set(e.value());
                                    },
                                }
                                Button { onclick: select_transcode_input_dir, "浏览" }
                            }
                        }

                        div { class: "form-field",
                            label { "导出目录：" }
                            div { class: "input-group",
                                Input {
                                    r#type: "text",
                                    value: "{transcode_output_dir}",
                                    oninput: move |e: FormEvent| {
                                        transcode_output_dir.set(e.value());
                                    },
                                }
                                Button { onclick: select_transcode_output_dir, "浏览" }
                            }
                        }
                    }
                }

                AlertDialogActions {
                    AlertDialogCancel { on_click: move |_| open.set(false), "取消" }
                    AlertDialogAction { on_click: handle_save, "保存" }
                }
            }
        }
    }
}
