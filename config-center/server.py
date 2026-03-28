#!/usr/bin/env python3
"""
AutoDesk 配置中心服务器

功能：
- 提供客户端配置文件的 HTTP 服务
- 支持跨域访问 (CORS)
- 可部署到任何静态文件服务器

使用方法：
1. 本地测试: python3 server.py
2. 生产部署: 将 config.json 放到任意 HTTP 服务器

部署选项：
- GitHub Pages: 推送到 gh-pages 分支
- Nginx/Apache: 直接托管静态文件
- CDN: 上传到 CloudFlare/AWS S3
"""

import http.server
import json
import os
from datetime import datetime

PORT = 8080
CONFIG_FILE = "config.json"

class ConfigHandler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/config.json" or self.path == "/":
            self.send_config()
        else:
            self.send_error(404, "Not Found")
    
    def send_config(self):
        try:
            with open(CONFIG_FILE, "r", encoding="utf-8") as f:
                config = json.load(f)
            
            config["last_updated"] = datetime.utcnow().isoformat() + "Z"
            
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.send_header("Access-Control-Allow-Methods", "GET, OPTIONS")
            self.send_header("Access-Control-Allow-Headers", "Content-Type")
            self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
            self.end_headers()
            self.wfile.write(json.dumps(config, indent=2, ensure_ascii=False).encode("utf-8"))
            
            print(f"[{datetime.now().isoformat()}] Served config to {self.client_address[0]}")
            
        except FileNotFoundError:
            self.send_error(404, "Config file not found")
        except Exception as e:
            self.send_error(500, f"Internal error: {str(e)}")
    
    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()
    
    def log_message(self, format, *args):
        pass

if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    
    print(f"""
╔══════════════════════════════════════════════════════════╗
║           AutoDesk 配置中心服务器                         ║
╠══════════════════════════════════════════════════════════╣
║  端口: {PORT:<5}                                          ║
║  配置文件: {CONFIG_FILE:<10}                               ║
║  访问地址: http://localhost:{PORT}/config.json             ║
╠══════════════════════════════════════════════════════════╣
║  按 Ctrl+C 停止服务器                                     ║
╚══════════════════════════════════════════════════════════╝
    """)
    
    with http.server.HTTPServer(("", PORT), ConfigHandler) as httpd:
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\n服务器已停止")
