/// 将秒数格式化为时间字符串
pub fn format_duration(seconds: f64) -> String {
    let total_seconds = seconds.round() as u32;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

// 将时长字符串转换为秒数
pub fn parse_duration_to_seconds(duration: &str) -> u32 {
    let parts: Vec<&str> = duration.split(':').collect();
    match parts.len() {
        3 => {
            // HH:MM:SS
            let hours: u32 = parts[0].parse().unwrap_or(0);
            let minutes: u32 = parts[1].parse().unwrap_or(0);
            let seconds: u32 = parts[2].parse().unwrap_or(0);
            hours * 3600 + minutes * 60 + seconds
        }
        2 => {
            // MM:SS
            let minutes: u32 = parts[0].parse().unwrap_or(0);
            let seconds: u32 = parts[1].parse().unwrap_or(0);
            minutes * 60 + seconds
        }
        _ => 0,
    }
}
