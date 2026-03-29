# Windows 编译脚本

## 前置要求

在 Windows 电脑上安装：

1. **Node.js** (>= 18)
   - 下载地址: https://nodejs.org/

2. **Rust**
   ```powershell
   winget install Rustlang.Rustup
   ```

3. **Visual Studio Build Tools**
   - 下载地址: https://visualstudio.microsoft.com/downloads/
   - 选择 "C++ Build Tools"

## 编译步骤

1. 克隆代码
```powershell
git clone https://github.com/fisher-list/AutoDesk.git
cd AutoDesk/remote-control-app/client
```

2. 安装依赖
```powershell
npm install
```

3. 编译
```powershell
npm run tauri build
```

4. 编译产物位置
```
src-tauri/target/release/bundle/nsis/
```

## 或者直接使用便携版

如果只需要 exe 文件，可以直接使用：
```
src-tauri/target/release/client.exe
```
