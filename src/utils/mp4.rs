use crate::{components::mp4_info::Mp4FileInfo, utils::format_duration};
use std::path::PathBuf;
/// 解析单个 MP4 文件信息
pub fn parse_mp4_info(path: PathBuf) -> Result<Mp4FileInfo, Box<dyn std::error::Error>> {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("未知文件")
        .to_string();

    let metadata = std::fs::metadata(&path)?;
    let modified = metadata.modified().ok();
    let size = metadata.len();

    // 使用 mp4 库解析视频信息
    let file = std::fs::File::open(&path)?;
    let size_u64 = file.metadata()?.len();
    let reader = std::io::BufReader::new(file);

    let mp4 = mp4::Mp4Reader::read_header(reader, size_u64)?;

    // 获取视频轨道信息
    let mut width = 0u16;
    let mut height = 0u16;
    let mut codec = "未知".to_string();
    let mut frame_rate = 0.0;
    let mut bit_rate = 0u32;
    // let mut duration = None::<f64>;
    let duration = mp4.duration().as_secs_f64();
    let duration = format_duration(duration);

    for track in mp4.tracks().values() {
        if let mp4::TrackType::Video = track.track_type()? {
            width = track.width();
            height = track.height();
            // 编解码器类型
            codec = match track.media_type() {
                Ok(mp4::MediaType::H264) => "H.264 / AVC".to_string(),
                Ok(mp4::MediaType::H265) => "H.265 / HEVC".to_string(),
                Ok(mp4::MediaType::VP9) => "VP9".to_string(),
                Ok(other) => format!("{:?}", other),
                Err(_) => "未知".to_string(),
            };
            // 帧率
            frame_rate = track.frame_rate();
            // 码率
            bit_rate = track.bitrate();
            println!(
                "视频名称:{},视频轨道: 宽度: {}, 高度: {}, 编解码器: {}, 时长: {}, 帧率: {}, 码率: {}",
                file_name, width, height, codec, duration, frame_rate, bit_rate
            );
            break; // 只取第一个视频轨道
        }
    }
    let h265 = should_transcode_to_h265(
        width.into(),
        height.into(),
        frame_rate,
        bit_rate,
        mp4.duration().as_secs_f64(),
        &codec,
    );
    if h265 {
        println!("该视频{}是H.265: {}", file_name, h265);
    } else {
        // println!("不满足的视频{}", file_name);
    }
    Ok(Mp4FileInfo {
        file_name,
        size,
        modified,
        width,
        height,
        codec,
        duration,
        file_path: path, // 保存完整路径
        frame_rate: frame_rate.to_string(),
        bit_rate: bit_rate.to_string(),
    })
}

/// 判断视频是否需要转码成H.265
fn should_transcode_to_h265(
    width: u32,
    height: u32,
    frame_rate: f64,
    bitrate: u32,
    duration_sec: f64, // 时长（秒）
    codec: &str,
) -> bool {
    // 如果已经是H.265，不需要转码
    if codec.contains("hvc") || codec.contains("hev") || codec.contains("265") {
        println!("已经是H.265，不需要转码");
        return false;
    }
    // 1. 如果是低分辨率视频，可能不需要转码
    let resolution = width * height;
    if resolution < 640 * 480 {
        // 标清以下
        return false;
    }
    // 计算文件大小（MB）
    let current_size_mb = (bitrate as f64 * duration_sec) / (8.0 * 1024.0 * 1024.0);
    let estimated_h265_size_mb = current_size_mb * 0.55; // H.265节省45%
    let saving_mb = current_size_mb - estimated_h265_size_mb;
    println!(
        "当前文件大小: {:.1}MB, 预计H.265大小: {:.1}MB, 节省: {:.1}MB,秒:{}",
        current_size_mb, estimated_h265_size_mb, saving_mb, duration_sec
    );
    let mut score = 0;
    let mut reasons = Vec::new();

    // 1. 分辨率评分（权重：3）
    if resolution >= 3840 * 2160 {
        // 4K
        score += 3;
        reasons.push("4K分辨率适合H.265".to_string());
    } else if resolution >= 1920 * 1080 {
        // 1080p
        score += 2;
        reasons.push("1080p分辨率适合H.265".to_string());
    } else if resolution >= 1280 * 720 {
        // 720p
        score += 1;
        reasons.push("720p分辨率可以考虑H.265".to_string());
    }

    // 2. 帧率评分（权重：1）
    if frame_rate >= 60.0 {
        score += 1;
        reasons.push("高帧率(≥60fps)适合H.265".to_string());
    } else if frame_rate >= 30.0 {
        score += 0; // 标准帧率不加分
    }

    // 3. 码率评估（权重：2）
    let recommended_bitrate = get_recommended_bitrate_improved(width, height, frame_rate);
    let current_bitrate_kbps = bitrate as f64 / 1000.0;
    let recommended_kbps = recommended_bitrate;

    // 更合理的码率比较：超过推荐码率15%就算高
    if current_bitrate_kbps > recommended_kbps * 1.15 {
        // 当前码率比推荐的高15%以上，转码可节省空间
        score += 2;
        reasons.push(format!(
            "当前码率({:.0}kbps)明显高于推荐值({:.0}kbps)",
            current_bitrate_kbps, recommended_kbps
        ));
    } else if current_bitrate_kbps > recommended_kbps * 0.85 {
        // 在推荐范围内
        score += 1;
        reasons.push(format!(
            "当前码率({:.0}kbps)在推荐范围内",
            current_bitrate_kbps
        ));
    } else {
        reasons.push(format!(
            "当前码率({:.0}kbps)低于推荐值",
            current_bitrate_kbps
        ));
    }

    // 4. 时长/文件大小评分（权重：2）
    if current_size_mb > 500.0 {
        // 文件大于500MB，转码节省显著
        score += 2;
        reasons.push(format!("文件较大({:.1}MB)，转码节省明显", current_size_mb));
    } else if current_size_mb > 100.0 {
        score += 1;
        reasons.push(format!("文件大小({:.1}MB)适中", current_size_mb));
    }

    // 5. 绝对节省空间评分（权重：1）
    if saving_mb > 200.0 {
        // 预计节省超过200MB，值得转码
        score += 1;
        reasons.push(format!("预计节省{:.1}MB空间", saving_mb));
    } else if saving_mb > 50.0 {
        reasons.push(format!("预计节省{:.1}MB空间", saving_mb));
    }

    // 决策逻辑
    let should_transcode = score >= 5; // 降低阈值

    println!(
        "分析结果: 分辨率{}x{}, {}fps, {}kbps, {:.1}MB, 评分: {}",
        width, height, frame_rate, current_bitrate_kbps as u32, current_size_mb, score
    );
    should_transcode
}

/// 获取H.264推荐码率（kbps）
fn get_recommended_bitrate_improved(width: u32, height: u32, frame_rate: f64) -> f64 {
    let resolution = (width * height) as f64;

    // 基于实际场景的更合理推荐码率（kbps）
    let base_rate = match resolution {
        r if r >= 3840.0 * 2160.0 => 25000.0, // 4K: 25Mbps
        r if r >= 1920.0 * 1080.0 => 5000.0,  // 1080p: 5Mbps
        r if r >= 1280.0 * 720.0 => 2500.0,   // 720p: 2.5Mbps（更合理）
        _ => 1000.0,                          // 更低分辨率
    };

    // 根据帧率调整
    let frame_rate_factor = if frame_rate >= 60.0 {
        1.5
    } else if frame_rate >= 30.0 {
        1.2
    } else {
        1.0
    };

    base_rate * frame_rate_factor
}
