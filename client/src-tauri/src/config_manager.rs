use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub url: String,
    pub priority: u8,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: String,
    pub signaling_servers: Vec<ServerConfig>,
    pub config_center_url: Option<String>,
    pub last_updated: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            signaling_servers: vec![
                ServerConfig {
                    url: "wss://signal.autodesk.example.com/ws".to_string(),
                    priority: 1,
                    enabled: true,
                },
                ServerConfig {
                    url: "wss://signal-backup.autodesk.example.com/ws".to_string(),
                    priority: 2,
                    enabled: true,
                },
            ],
            config_center_url: Some("https://config.autodesk.example.com/config.json".to_string()),
            last_updated: None,
        }
    }
}

impl AppConfig {
    pub fn get_available_servers(&self) -> Vec<String> {
        let mut servers: Vec<_> = self.signaling_servers
            .iter()
            .filter(|s| s.enabled)
            .collect();
        servers.sort_by_key(|s| s.priority);
        servers.into_iter().map(|s| s.url.clone()).collect()
    }
}

pub struct ConfigManager {
    config: Arc<RwLock<AppConfig>>,
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new(app_handle: &tauri::AppHandle) -> Self {
        let config_dir = app_handle.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
        let config_path = config_dir.join("config.json");
        
        let config = if config_path.exists() {
            match fs::read_to_string(&config_path) {
                Ok(content) => {
                    match serde_json::from_str(&content) {
                        Ok(cfg) => cfg,
                        Err(_) => AppConfig::default(),
                    }
                }
                Err(_) => AppConfig::default(),
            }
        } else {
            let default_config = AppConfig::default();
            let _ = fs::create_dir_all(&config_dir);
            let _ = fs::write(&config_path, serde_json::to_string_pretty(&default_config).unwrap());
            default_config
        };
        
        Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
        }
    }
    
    pub async fn get_config(&self) -> AppConfig {
        self.config.read().await.clone()
    }
    
    pub async fn get_servers(&self) -> Vec<String> {
        let config = self.config.read().await;
        config.get_available_servers()
    }
    
    pub async fn update_config(&self, new_config: AppConfig) -> Result<(), String> {
        let mut config = self.config.write().await;
        *config = new_config.clone();
        
        let content = serde_json::to_string_pretty(&new_config)
            .map_err(|e| format!("序列化配置失败: {}", e))?;
        
        fs::write(&self.config_path, &content)
            .map_err(|e| format!("写入配置文件失败: {}", e))?;
        
        Ok(())
    }
    
    pub async fn fetch_remote_config(&self) -> Result<AppConfig, String> {
        let config = self.config.read().await;
        let config_url = match &config.config_center_url {
            Some(url) => url.clone(),
            None => return Err("未配置远程配置中心地址".to_string()),
        };
        drop(config);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;
        
        let response = client.get(&config_url)
            .send()
            .await
            .map_err(|e| format!("请求远程配置失败: {}", e))?;
        
        let remote_config: AppConfig = response.json()
            .await
            .map_err(|e| format!("解析远程配置失败: {}", e))?;
        
        self.update_config(remote_config.clone()).await?;
        
        Ok(remote_config)
    }
    
    pub async fn try_fetch_remote_config(&self) {
        match self.fetch_remote_config().await {
            Ok(_) => println!("成功从配置中心更新配置"),
            Err(e) => println!("从配置中心获取配置失败，使用本地配置: {}", e),
        }
    }
}

#[tauri::command]
pub async fn get_config(manager: tauri::State<'_, Arc<ConfigManager>>) -> Result<AppConfig, String> {
    Ok(manager.get_config().await)
}

#[tauri::command]
pub async fn get_servers(manager: tauri::State<'_, Arc<ConfigManager>>) -> Result<Vec<String>, String> {
    Ok(manager.get_servers().await)
}

#[tauri::command]
pub async fn update_config(
    manager: tauri::State<'_, Arc<ConfigManager>>,
    config: AppConfig,
) -> Result<(), String> {
    manager.update_config(config).await
}

#[tauri::command]
pub async fn refresh_config(manager: tauri::State<'_, Arc<ConfigManager>>) -> Result<AppConfig, String> {
    manager.fetch_remote_config().await
}
