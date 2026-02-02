use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::utils::parse_mp4_info;

/// 执行单个文件的转码操作
///
/// # 参数
/// * `path` - 要转码的文件路径
/// * `transcode_progress` - 转码进度状态
/// * `on_progress` - 进度回调函数，接收文件路径和当前进度百分比
pub async fn transcode_file<F>(path: PathBuf, output_dir: &PathBuf, mut on_progress: F)
where
    F: FnMut(f32) + 'static,
{
    // 初始进度回调
    on_progress(0.0);

    // 获取输出路径（在原文件名后添加"_transcoded"）
    let output_path = if let Some(file_name) = path.file_name() {
        let new_name = file_name.to_os_string();
        // new_name.push("_transcoded.mp4");
        output_dir.join(new_name)
    } else {
        return;
    };
    // 确保输出目录存在
    if let Err(e) = tokio::fs::create_dir_all(&output_dir).await {
        println!("Failed to create output directory: {}", e);
        return;
    }
    // 使用mp4获取时长
    let mp4_info = parse_mp4_info(path.clone()).ok();
    let duration_secs = mp4_info.map(|f| crate::utils::parse_duration_to_seconds(&f.duration));
    // 使用FFmpeg进行转码
    let mut child = tokio::process::Command::new("ffmpeg")
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .arg("-i")
        .arg(&path)
        .arg("-c:v")
        .arg("libx264") // 使用H.264编码
        .arg("-c:a")
        .arg("aac") // 使用AAC音频编码
        .arg("-progress")
        .arg("pipe:1") // 将进度输出到标准输出
        .arg("-nostats") // 禁用默认的统计信息输出
        .arg("-y") // 覆盖已存在的文件
        .arg(&output_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn ffmpeg process");

    // 获取FFmpeg的输出
    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    // 读取进度信息
    while let Ok(Some(line)) = lines.next_line().await {
        if line.starts_with("out_time_ms=") {
            let time_ms = line
                .trim_start_matches("out_time_ms=")
                .parse::<f32>()
                .unwrap_or(0.0);

            // 计算进度百分比
            // 用 map_or 一行搞定
            let progress = duration_secs.map_or(0.0, |d| {
                if d > 0 {
                    (time_ms / 1_000_000.0 / d as f32 * 100.0).min(100.0)
                } else {
                    0.0
                }
            });

            // 调用进度回调
            on_progress(progress);
        }
    }

    // 等待FFmpeg进程完成
    let status = child.wait().await;

    match status {
        Ok(status) if status.success() => {
            todo!()
        }
        Ok(_) => {}
        Err(_e) => {
            todo!()
        }
    }

    on_progress(100.0);
}
