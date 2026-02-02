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
                "视频轨道: 宽度: {}, 高度: {}, 编解码器: {}, 时长: {}, 帧率: {}, 码率: {}",
                width, height, codec, duration, frame_rate, bit_rate
            );
            break; // 只取第一个视频轨道
        }
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
