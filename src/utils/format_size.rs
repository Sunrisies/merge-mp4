pub fn format_size(size: Option<u64>) -> String {
    match size {
        Some(s) if s < 1024 => format!("{} B", s),
        Some(s) if s < 1024 * 1024 => format!("{:.2} KB", s as f64 / 1024.0),
        Some(s) if s < 1024 * 1024 * 1024 => format!("{:.2} MB", s as f64 / (1024.0 * 1024.0)),
        Some(s) => format!("{:.2} GB", s as f64 / (1024.0 * 1024.0 * 1024.0)),
        None => "未知".to_string(),
    }
}
