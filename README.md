# AutoDesk - 跨平台远程控制软件

AutoDesk 是一款基于 Rust、Tauri 和 WebRTC 开发的轻量级、高性能远程控制软件，类似于 ToDesk 或向日葵。它支持跨平台运行，提供低延迟的屏幕画面传输和实时的键盘鼠标控制。

## 核心特性

*   **高性能屏幕采集**：使用 Rust 底层库 (`xcap`) 直接抓取系统屏幕，性能优异。
*   **P2P 低延迟传输**：基于 WebRTC 协议，实现端到端的音视频流和控制指令传输，无需经过中继服务器（在 NAT 穿透成功的情况下），极大降低延迟。
*   **物理级键鼠注入**：使用 `enigo` 库，在被控端实现真实的鼠标移动、点击、滚轮和键盘按键模拟。
*   **轻量级信令服务器**：使用 Rust `axum` 框架和 WebSocket 协议，负责设备注册、连接码分配和 P2P 握手信息的交换。
*   **现代化 UI**：基于 Tauri、React 和 TailwindCSS 构建，界面美观，打包体积小。
*   **开机自启**：支持配置开机自启，方便无人值守时的远程连接。

## 系统架构

项目分为两个主要部分：

1.  **`signaling-server` (信令服务器)**：
    *   语言：Rust
    *   框架：Axum, Tokio
    *   协议：WebSocket
    *   功能：生成 9 位连接码和 6 位密码，协助客户端交换 WebRTC 的 SDP 和 ICE Candidate 信息。

2.  **`client` (客户端)**：
    *   前端：React, TypeScript, TailwindCSS, SimplePeer (WebRTC)
    *   后端：Rust, Tauri, xcap (屏幕采集), enigo (键鼠控制)
    *   功能：显示本机连接码、发起连接请求、渲染远程画面、捕获并发送本地键鼠事件、接收并执行远程键鼠指令。

## 开发与运行指南

### 环境准备

*   [Node.js](https://nodejs.org/) (推荐 v18+)
*   [Rust](https://www.rust-lang.org/tools/install) (推荐最新稳定版)
*   (Windows) C++ 生成工具 (Visual Studio Build Tools)

### 1. 运行信令服务器

信令服务器可以在本地运行进行测试，也可以部署到具有公网 IP 的云服务器上。

```bash
cd signaling-server
cargo run --release
```
默认监听端口为 `3000`。

### 2. 运行客户端 (开发模式)

在运行客户端之前，请确保 `client/src/App.tsx` 中的 `SIGNALING_SERVER_URL` 指向了您正在运行的信令服务器地址（本地测试默认为 `ws://127.0.0.1:3000/ws`）。

```bash
cd client
npm install
npm run tauri dev
```

### 3. 编译打包 (生成可执行文件)

如果您想生成 Windows 的 `.exe` 或 macOS 的 `.app` / `.dmg` 安装包：

```bash
cd client
npm run tauri build
```
编译完成后，安装包会生成在 `client/src-tauri/target/release/bundle/` 目录下。

## 注意事项

*   **权限问题**：在 Windows 上，为了能够控制 UAC 弹窗或锁屏界面，编译后的程序可能需要以管理员身份运行。
*   **网络穿透**：目前代码中使用了 Google 和 Twilio 的公共 STUN 服务器进行 NAT 打洞。在复杂的网络环境下（如对称型 NAT），P2P 连接可能会失败。在生产环境中，建议搭建自己的 TURN 中继服务器（如 `coturn`）。
*   **性能优化**：目前的屏幕采集为了方便前端测试，使用了 JPEG 压缩并转为 Base64。在实际的生产级 WebRTC 应用中，应将原始 RGBA 数据直接送入硬件编码器（如 NVENC/AMF）以获得最佳性能。

## 许可证

MIT License
