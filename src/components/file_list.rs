use super::button::{Button, ButtonVariant};
use dioxus::prelude::*;
use std::path::PathBuf;

// 1. 提取子组件：文件列表区域
#[component]
pub fn FileList(files: Signal<Vec<PathBuf>>, on_remove: Callback<usize>) -> Element {
    rsx! {
        div { class: "mt-2",
            if !files.read().is_empty() {
                div { class: "space-y-2 max-h-52 overflow-y-auto pr-2 custom-scrollbar",
                    for (index , file) in files.read().iter().cloned().enumerate() {
                        div { class: "flex items-center justify-between py-1 px-2 rounded-lg border border-gray-600 hover:border-gray-500 transition-colors",
                            div { class: "flex items-center gap-3 overflow-hidden",
                                span { class: "text-gray-400 text-sm font-mono", "{index + 1}." }
                                span { class: " truncate flex-1 max-w-100",
                                    "{file.file_name().unwrap().to_string_lossy()}"
                                }
                            }
                            Button {
                                variant: ButtonVariant::Destructive,
                                onclick: move |_| on_remove.call(index),
                                "删除"
                            }
                        }
                    }
                }
            } else {
                div { class: "text-center py-8 border-2 border-dashed border-gray-600 rounded-lg",
                    p { class: "text-gray-500 text-lg", "尚未选择任何文件" }
                    p { class: "text-gray-600 text-sm mt-1", "点击上方按钮添加MP4文件" }
                }
            }
        }
    }
}
