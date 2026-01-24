use crate::MergeEvent;
use dioxus::prelude::Coroutine;
use regex::Regex;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tempfile::NamedTempFile;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use which::which;

pub async fn run_ffmpeg_merge(
    files: Vec<PathBuf>,
    output_path: PathBuf,
    tx: Coroutine<MergeEvent>,
) {
    // Validate FFmpeg installation
    if which("ffmpeg").is_err() {
        tx.send(MergeEvent::Error(
            "未找到FFmpeg，请确保已安装并添加到系统PATH中".to_string(),
        ));
        return;
    }

    // Validate input files
    for file in &files {
        if !file.exists() {
            tx.send(MergeEvent::Error(format!("文件不存在: {}", file.display())));
            return;
        }
        if !file.is_file() {
            tx.send(MergeEvent::Error(format!("不是文件: {}", file.display())));
            return;
        }
    }

    // Validate output directory
    if let Some(parent) = output_path.parent()
        && !parent.exists()
    {
        tx.send(MergeEvent::Error(format!(
            "输出目录不存在: {}",
            parent.display()
        )));
        return;
    }

    tx.send(MergeEvent::Status("计算视频总时长...".to_string()));
    let mut total_duration = 0.0;
    for (i, file) in files.iter().enumerate() {
        match get_video_duration(file).await {
            Ok(dur) => total_duration += dur,
            Err(e) => {
                tx.send(MergeEvent::Error(format!(
                    "无法读取视频时长 {}: {}",
                    file.display(),
                    e
                )));
                return;
            }
        }
        let progress_pct = (i + 1) as f64 / files.len() as f64 * 10.0;
        tx.send(MergeEvent::Progress(progress_pct));
    }

    let mut temp_file = match NamedTempFile::new() {
        Ok(f) => f,
        Err(e) => {
            tx.send(MergeEvent::Error(format!("创建临时文件失败: {}", e)));
            return;
        }
    };

    for file_path in &files {
        let abs_path = match std::fs::canonicalize(file_path) {
            Ok(path) => path,
            Err(e) => {
                tx.send(MergeEvent::Error(format!(
                    "无法解析文件路径 {}: {}",
                    file_path.display(),
                    e
                )));
                return;
            }
        };
        if let Err(e) = writeln!(temp_file, "file '{}'", abs_path.display()) {
            tx.send(MergeEvent::Error(format!("写入临时文件失败: {}", e)));
            return;
        }
    }
    let temp_path = temp_file.path().to_path_buf();

    tx.send(MergeEvent::Status("启动FFmpeg合并...".to_string()));

    let mut child = match Command::new("ffmpeg")
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .args([
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            temp_path.to_str().unwrap(),
            "-c",
            "copy",
            "-y",
        ])
        .arg(&output_path)
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            tx.send(MergeEvent::Error(format!("启动FFmpeg失败: {}", e)));
            return;
        }
    };

    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stderr);
    let mut lines = reader.lines();
    let time_regex = Regex::new(r"time=(\d{2}):(\d{2}):(\d{2}\.\d{2})").unwrap();

    while let Ok(Some(line)) = lines.next_line().await {
        tx.send(MergeEvent::Status(line.clone()));

        if let Some(caps) = time_regex.captures(&line)
            && let (Ok(hours), Ok(minutes), Ok(seconds)) = (
                caps[1].parse::<f64>(),
                caps[2].parse::<f64>(),
                caps[3].parse::<f64>(),
            )
        {
            let current_time = hours * 3600.0 + minutes * 60.0 + seconds;
            if total_duration > 0.0 {
                let progress_pct = (current_time / total_duration).min(0.99) * 90.0 + 10.0;
                tx.send(MergeEvent::Progress(progress_pct));
            }
        }
    }

    match child.wait().await {
        Ok(status) if status.success() => {
            tx.send(MergeEvent::Success(format!(
                "文件已保存到: {}",
                output_path.display()
            )));
        }
        Ok(status) => {
            tx.send(MergeEvent::Error(format!(
                "FFmpeg进程异常退出，退出码: {}",
                status
            )));
        }
        Err(e) => {
            tx.send(MergeEvent::Error(format!("等待FFmpeg进程失败: {}", e)));
        }
    }
}

async fn get_video_duration(path: &Path) -> Result<f64, String> {
    let output = Command::new("ffmpeg")
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .args(["-i", path.to_str().unwrap()])
        .output()
        .await
        .map_err(|e| format!("执行FFmpeg失败: {}", e))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let re = Regex::new(r"Duration: (\d{2}):(\d{2}):(\d{2}\.\d{2})").unwrap();

    if let Some(caps) = re.captures(&stderr) {
        let hours: f64 = caps[1].parse().unwrap_or(0.0);
        let minutes: f64 = caps[2].parse().unwrap_or(0.0);
        let seconds: f64 = caps[3].parse().unwrap_or(0.0);
        Ok(hours * 3600.0 + minutes * 60.0 + seconds)
    } else {
        Err("无法解析视频时长信息".to_string())
    }
}
