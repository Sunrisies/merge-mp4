use dioxus::prelude::*;
//  提取子组件：关于页脚
#[component]
pub fn AboutFooter(version: String, author: String) -> Element {
    rsx! {
        div { class: "px-3 py-2 border-t border-gray-700 flex justify-between items-center",
            p { class: "text-gray-500 text-sm text-center flex-1",
                "这是一个使用Rust编写的视频合并工具。"
            }
            p { class: "text-gray-500 text-sm text-center flex-1", "作者: {author}" }
            p { class: "text-gray-500 text-sm text-center flex-1", "版本: {version}" }
        }
    }
}
