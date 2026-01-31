use dioxus::prelude::*;

use crate::components::button::{Button, ButtonVariant};
use crate::components::mp4_info::ScanProgress;

#[component]
pub fn Mp4InfoLoading(progress: Signal<ScanProgress>, cancel_scan: Callback) -> Element {
    // 计算进度百分比
    let progress_percent = {
        let p = progress.read();
        if p.total > 0 {
            (p.current as f32 / p.total as f32 * 100.0) as u32
        } else {
            0
        }
    };
    rsx! {
        // 加载状态
        div { class: "flex-1 flex flex-col items-center justify-center p-8",
            div { class: "w-full max-w-md",

                // 进度显示
                div { class: "bg-white rounded-2xl shadow-lg p-6 border border-gray-200",
                    div { class: "flex justify-between items-center mb-6",
                        div { class: "flex-1",
                            h3 { class: "text-lg font-semibold text-gray-800 mb-2 flex items-center gap-2",
                                span { class: "text-blue-500 animate-spin", "🔄" }
                                "正在扫描文件..."
                            }
                            p {
                                class: "text-sm text-gray-600 truncate w-[300px]",
                                title: "正在扫描: {progress.read().current_file}",
                                "正在扫描: {progress.read().current_file}"
                            }
                        }
                        div { class: "text-right",
                            p { class: "text-2xl font-bold text-blue-600", "{progress_percent}%" }
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
                                onclick: move |_| cancel_scan(()),
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

    }
}
