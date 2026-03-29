use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use xcap::Monitor;
use image::ImageFormat;
use std::io::Cursor;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

pub struct ScreenCapture {
    is_capturing: Arc<Mutex<bool>>,
}

impl ScreenCapture {
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(Mutex::new(false)),
        }
    }

    /// 开始采集屏幕
    /// 在实际的 WebRTC 场景中，这里会将采集到的帧送入 H.264 编码器
    /// 为了方便前端测试，我们这里先提供一个回调，将帧转为 Base64 JPEG 发送给前端
    pub async fn start_capture<F>(&self, mut on_frame: F) -> Result<(), String>
    where
        F: FnMut(String) + Send + 'static,
    {
        let mut is_capturing = self.is_capturing.lock().await;
        if *is_capturing {
            return Err("Capture is already running".to_string());
        }
        *is_capturing = true;
        drop(is_capturing);

        let is_capturing_clone = self.is_capturing.clone();

        // 启动一个后台任务进行循环采集
        std::thread::spawn(move || {
            // 获取主显示器
            let monitors = Monitor::all().unwrap_or_default();
            let primary_monitor = monitors.into_iter().find(|m| m.is_primary().unwrap_or(false));

            if let Some(monitor) = primary_monitor {
                println!("Start capturing monitor: {}", monitor.name().unwrap_or_else(|_| "Unknown".to_string()));
                
                loop {
                    let capturing = futures::executor::block_on(is_capturing_clone.lock());
                    if !*capturing {
                        println!("Stop capturing monitor");
                        break;
                    }

                    // 抓取一帧
                    match monitor.capture_image() {
                        Ok(image) => {
                            // 将 RGBA 图像压缩为 JPEG 格式 (为了测试传输)
                            // 实际项目中，这里应该将原始 RGBA 数据送入硬件编码器 (如 NVENC/AMF 或 WebRTC 内置编码器)
                            let mut buffer = Cursor::new(Vec::new());
                            if let Ok(_) = image.write_to(&mut buffer, ImageFormat::Jpeg) {
                                let base64_str = BASE64.encode(buffer.into_inner());
                                on_frame(base64_str);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to capture screen: {}", e);
                        }
                    }

                    // 控制帧率，例如 30 FPS (约 33ms 一帧)
                    std::thread::sleep(Duration::from_millis(33));
                }
            } else {
                eprintln!("No primary monitor found");
            }
        });

        Ok(())
    }

    /// 停止采集
    pub async fn stop_capture(&self) {
        let mut is_capturing = self.is_capturing.lock().await;
        *is_capturing = false;
    }
}
