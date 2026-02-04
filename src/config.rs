use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AppConfig {
    // 合并相关配置
    pub merge_input_directory: Option<PathBuf>,
    pub merge_output_directory: Option<PathBuf>,

    // 转码相关配置
    pub transcode_input_directory: Option<PathBuf>,
    pub transcode_output_directory: Option<PathBuf>,
}

impl AppConfig {
    /// Load configuration from file
    pub fn load() -> Result<Self, io::Error> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path).map_err(|e| {
            eprintln!("Failed to read config file: {}", e);
            io::Error::new(io::ErrorKind::InvalidData, e)
        })?;

        let config: AppConfig = serde_json::from_str(&content).map_err(|e| {
            eprintln!("Failed to parse config JSON: {}", e);
            eprintln!("Config content: {}", content);
            io::Error::new(io::ErrorKind::InvalidData, e)
        })?;

        Ok(config)
    }

    /// 将配置保存到文件
    pub fn save(&self) -> Result<(), io::Error> {
        let config_path = Self::config_path()?;

        // Ensure directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                eprintln!("Failed to create config directory: {}", e);
                io::Error::new(io::ErrorKind::InvalidData, e)
            })?;
        }

        let content = serde_json::to_string_pretty(self).map_err(|e| {
            eprintln!("Failed to serialize config: {}", e);
            io::Error::new(io::ErrorKind::InvalidData, e)
        })?;

        fs::write(&config_path, content).map_err(|e| {
            eprintln!("Failed to write config file: {}", e);
            io::Error::new(io::ErrorKind::InvalidData, e)
        })?;

        Ok(())
    }

    /// 获取配置文件路径
    fn config_path() -> Result<PathBuf, io::Error> {
        let config_dir = dirs::config_dir().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Could not find config directory")
        })?;

        let app_config_dir = config_dir.join("merge-mp4");
        println!("Config dir: {:?}", app_config_dir);
        Ok(app_config_dir.join("config.json"))
    }
    // --- 合并 相关方法 ---

    /// 获取合并输入目录
    pub fn get_merge_input_directory(&self) -> Option<PathBuf> {
        self.merge_input_directory.clone()
    }

    /// 设置合并输入目录并保存
    pub fn set_merge_input_directory(&mut self, path: PathBuf) -> Result<(), io::Error> {
        self.merge_input_directory = Some(path);
        self.save()
    }

    /// 获取合并输出目录，如果未设置，则回退到当前目录
    pub fn get_merge_output_directory(&self) -> PathBuf {
        self.merge_output_directory
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// 设置合并输出目录并保存
    pub fn _set_merge_output_directory(&mut self, path: PathBuf) -> Result<(), io::Error> {
        self.merge_output_directory = Some(path);
        self.save()
    }

    // --- 转码 相关方法 ---

    /// 获取转码输入目录
    pub fn get_transcode_input_directory(&self) -> Option<PathBuf> {
        self.transcode_input_directory.clone()
    }

    /// 设置转码输入目录并保存
    pub fn set_transcode_input_directory(&mut self, path: PathBuf) -> Result<(), io::Error> {
        self.transcode_input_directory = Some(path);
        self.save()
    }

    /// 获取转码输出目录，如果未设置，则回退到当前目录
    pub fn get_transcode_output_directory(&self) -> PathBuf {
        self.transcode_output_directory
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// 设置转码输出目录并保存
    pub fn _set_transcode_output_directory(&mut self, path: PathBuf) -> Result<(), io::Error> {
        self.transcode_output_directory = Some(path);
        self.save()
    }
}
