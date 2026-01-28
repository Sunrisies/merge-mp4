use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AppConfig {
    pub output_directory: Option<PathBuf>,
    pub last_input_directory: Option<PathBuf>,
}

impl AppConfig {
    /// Load configuration from file
    pub fn load() -> Result<Self, io::Error> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)?;
        let config: AppConfig = serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<(), io::Error> {
        let config_path = Self::config_path()?;

        // Ensure directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        fs::write(&config_path, content)?;

        Ok(())
    }

    /// Get the configuration file path
    fn config_path() -> Result<PathBuf, io::Error> {
        let config_dir = dirs::config_dir().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Could not find config directory")
        })?;

        let app_config_dir = config_dir.join("merge-mp4");
        println!("Config dir: {:?}", app_config_dir);
        Ok(app_config_dir.join("config.json"))
    }

    /// Set output directory and save configuration
    pub fn set_output_directory(&mut self, path: PathBuf) -> Result<(), io::Error> {
        self.output_directory = Some(path);
        self.save()
    }

    /// Get output directory, falling back to current directory if not set
    pub fn get_output_directory(&self) -> PathBuf {
        self.output_directory
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
    /// 设置最后一个输入目录并保存配置
    pub fn set_last_input_directory(&mut self, path: PathBuf) -> Result<(), io::Error> {
        self.last_input_directory = Some(path);
        self.save()
    }

    /// 获取最后一个输入目录，如果未设置，则回退到None
    pub fn get_last_input_directory(&self) -> Option<PathBuf> {
        self.last_input_directory.clone()
    }
}
