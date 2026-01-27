use super::button::Button;
use super::input::Input;
use crate::components::button::ButtonVariant;
use crate::config::AppConfig;
use dioxus::prelude::*;

// 2. 提取子组件：输出设置区域
#[component]
pub fn OutputSettings(
    output_filename: Signal<String>,
    config: Signal<AppConfig>,
    on_select_dir: Callback<MouseEvent>,
    on_clear_dir: Callback<MouseEvent>,
) -> Element {
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
                    if let Some(dir) = config().output_directory.as_ref() {
                        "{dir.display()}"
                    } else {
                        "使用默认目录"
                    }
                }
                Button { variant: ButtonVariant::Secondary, onclick: on_select_dir, "选择目录" }
                Button { variant: ButtonVariant::Secondary, onclick: on_clear_dir, "清除" }
            }
        }
    }
}
