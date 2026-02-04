use std::path::PathBuf;

use super::input::Input;
use crate::config::AppConfig;
use dioxus::prelude::*;

// 2. 提取子组件：输出设置区域
#[component]
pub fn OutputSettings(output_filename: Signal<String>, config: Signal<AppConfig>) -> Element {
    rsx! {
        div { class: "space-y-3",
            div { class: "flex items-center gap-3",
                span { class: "text-gray-400 text-sm", "文件名:" }
                Input {
                    placeholder: "输入输出文件名 (例如: merged.mp4)",
                    value: "{output_filename()}",
                    oninput: move |e: FormEvent| output_filename.set(e.value()),
                }
            }
            div { class: "flex items-center gap-3",
                span { class: "text-gray-400 text-sm", "目录:" }
                span { class: "flex-1 text-gray-300 text-sm break-all",

                    {
                        let path = config().get_transcode_output_directory();
                        let default_path = std::env::current_dir()
                            .unwrap_or_else(|_| PathBuf::from("."));
                        if config().transcode_output_directory.is_none() || path == default_path {
                            "使用默认目录".to_string()
                        } else {
                            path.display().to_string()
                        }
                    }
                }
            }
        }
    }
}
