import { useState, useEffect, useRef } from "react";
import { Monitor, Key, Link as LinkIcon, ShieldCheck, Settings } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { enable, isEnabled, disable } from '@tauri-apps/plugin-autostart';
import { writeText, readText } from '@tauri-apps/plugin-clipboard-manager';
import SimplePeer from "simple-peer";
import "./App.css";

// 信令服务器地址 (请替换为您的实际服务器地址)
const SIGNALING_SERVER_URL = "ws://127.0.0.1:3000/ws";

function App() {
  // 本机信息 (作为被控端)
  const [localCode, setLocalCode] = useState("正在获取...");
  const [localPassword, setLocalPassword] = useState("正在获取...");
  const [, setClientId] = useState("");
  
  // 远程信息 (作为控制端)
  const [remoteCode, setRemoteCode] = useState("");
  const [remotePassword, setRemotePassword] = useState("");
  
  // 状态
  const [status, setStatus] = useState("未连接");
  const [isCapturing, setIsCapturing] = useState(false);
  const [, setScreenFrame] = useState<string | null>(null);
  const [isRemoteView, setIsRemoteView] = useState(false); // 是否正在查看远程画面
  const [autoStart, setAutoStart] = useState(false);
  const lastClipboardText = useRef<string>("");

  // 检查开机自启状态
  useEffect(() => {
    const checkAutoStart = async () => {
      try {
        const enabled = await isEnabled();
        setAutoStart(enabled);
      } catch (e) {
        console.error("Failed to check autostart status:", e);
      }
    };
    checkAutoStart();
  }, []);

  const toggleAutoStart = async () => {
    try {
      if (autoStart) {
        await disable();
        setAutoStart(false);
      } else {
        await enable();
        setAutoStart(true);
      }
    } catch (e) {
      console.error("Failed to toggle autostart:", e);
      alert("设置开机自启失败，可能需要管理员权限");
    }
  };

  // 引用
  const wsRef = useRef<WebSocket | null>(null);
  const peerRef = useRef<SimplePeer.Instance | null>(null);
  const remoteVideoRef = useRef<HTMLImageElement>(null);

  // 监听本地剪贴板变化并发送给对方
  useEffect(() => {
    if (status !== "P2P 连接已建立") return;

    const interval = setInterval(async () => {
      try {
        const text = await readText();
        if (text && text !== lastClipboardText.current) {
          lastClipboardText.current = text;
          if (peerRef.current && peerRef.current.connected) {
            peerRef.current.send(JSON.stringify({
              type: "clipboard",
              text: text
            }));
            console.log("Sent clipboard text to peer");
          }
        }
      } catch (e) {
        // 忽略读取剪贴板失败的错误 (可能是非文本内容)
      }
    }, 1000); // 每秒检查一次剪贴板

    return () => clearInterval(interval);
  }, [status]);

  // 初始化 WebSocket 连接
  useEffect(() => {
    const ws = new WebSocket(SIGNALING_SERVER_URL);
    wsRef.current = ws;

    ws.onopen = () => {
      console.log("Connected to signaling server");
      setStatus("已连接信令服务器");
      // 注册为被控端
      ws.send(JSON.stringify({ type: "RegisterHost" }));
    };

    ws.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      console.log("Received message:", msg);

      switch (msg.type) {
        case "Registered":
          setLocalCode(msg.code);
          setLocalPassword(msg.password);
          setClientId(msg.client_id);
          setStatus("就绪 (等待连接)");
          break;

        case "ConnectResult":
          if (msg.success) {
            setStatus("连接成功，正在建立 P2P...");
            // 作为控制端，发起 WebRTC Offer
            initWebRTC(true);
          } else {
            setStatus(`连接失败: ${msg.message}`);
            alert(`连接失败: ${msg.message}`);
          }
          break;

        case "PeerConnected":
          setStatus("有设备连入，正在建立 P2P...");
          // 作为被控端，等待 WebRTC Offer
          initWebRTC(false);
          break;

        case "Sdp":
        case "IceCandidate":
          // 将信令转发给 SimplePeer
          if (peerRef.current) {
            peerRef.current.signal(msg);
          }
          break;

        case "Error":
          setStatus(`错误: ${msg.message}`);
          if (msg.message === "Peer disconnected") {
            cleanupConnection();
          }
          break;
      }
    };

    ws.onclose = () => {
      setStatus("信令服务器已断开");
    };

    return () => {
      ws.close();
      cleanupConnection();
    };
  }, []);

  // 监听 Rust 发来的屏幕帧 (作为被控端)
  useEffect(() => {
    const unlisten = listen<string>("screen-frame", (event) => {
      const base64Data = `data:image/jpeg;base64,${event.payload}`;
      setScreenFrame(base64Data);
      
      // 如果 P2P 连接已建立，通过 DataChannel 发送画面给控制端
      if (peerRef.current && peerRef.current.connected) {
        try {
          peerRef.current.send(JSON.stringify({
            type: "frame",
            data: base64Data
          }));
        } catch (e) {
          console.error("Failed to send frame:", e);
        }
      }
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  // 初始化 WebRTC (SimplePeer)
  const initWebRTC = (initiator: boolean) => {
    // 注意：在实际生产环境中，需要配置真实的 STUN/TURN 服务器
    const peer = new SimplePeer({
      initiator,
      trickle: true,
      config: {
        iceServers: [
          { urls: "stun:stun.l.google.com:19302" },
          { urls: "stun:global.stun.twilio.com:3478" }
        ]
      }
    });

    peer.on("signal", (data: any) => {
      // 将本地生成的 SDP/ICE 发送给信令服务器
      if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
        if (data.type === "offer" || data.type === "answer") {
          wsRef.current.send(JSON.stringify({
            type: "Sdp",
            sdp: data.sdp,
            sdp_type: data.type
          }));
        } else if (data.candidate) {
          wsRef.current.send(JSON.stringify({
            type: "IceCandidate",
            candidate: data.candidate.candidate,
            sdp_mid: data.candidate.sdpMid,
            sdp_m_line_index: data.candidate.sdpMLineIndex
          }));
        }
      }
    });

    peer.on("connect", async () => {
      console.log("P2P Connection established!");
      setStatus("P2P 连接已建立");
      
      if (!initiator) {
        // 如果我是被控端，开始采集屏幕
        try {
          await invoke("start_screen_capture");
          setIsCapturing(true);
        } catch (e) {
          console.error("Failed to start capture:", e);
        }
      } else {
        // 如果我是控制端，切换到远程视图
        setIsRemoteView(true);
      }
    });

    peer.on("data", (data) => {
      // 接收数据
      try {
        const msg = JSON.parse(data.toString());
        if (msg.type === "frame" && initiator) {
          // 控制端接收画面
          if (remoteVideoRef.current) {
            remoteVideoRef.current.src = msg.data;
          }
        } else if (msg.type === "input" && !initiator) {
          // 被控端接收输入指令，传给 Rust 执行
          if (msg.action === "mousemove") {
            // 将相对坐标转换为绝对坐标 (假设屏幕分辨率为 1920x1080，实际应从 Rust 获取)
            const x = Math.round(msg.x * 1920);
            const y = Math.round(msg.y * 1080);
            invoke("handle_mouse_move", { x, y });
          } else if (msg.action === "mousedown") {
            invoke("handle_mouse_click", { button: msg.button, isDown: true });
          } else if (msg.action === "mouseup") {
            invoke("handle_mouse_click", { button: msg.button, isDown: false });
          } else if (msg.action === "scroll") {
            invoke("handle_mouse_scroll", { x: msg.x, y: msg.y });
          } else if (msg.action === "keydown") {
            invoke("handle_key_event", { keyCode: msg.key, isDown: true });
          } else if (msg.action === "keyup") {
            invoke("handle_key_event", { keyCode: msg.key, isDown: false });
          }
        } else if (msg.type === "clipboard") {
          // 接收到对方的剪贴板内容，写入本地
          if (msg.text && msg.text !== lastClipboardText.current) {
            lastClipboardText.current = msg.text;
            writeText(msg.text).catch(e => console.error("Failed to write clipboard:", e));
            console.log("Received and wrote clipboard text from peer");
          }
        }
      } catch (e) {
        console.error("Failed to parse peer data:", e);
      }
    });

    peer.on("close", () => {
      console.log("P2P Connection closed");
      cleanupConnection();
    });

    peer.on("error", (err) => {
      console.error("P2P Error:", err);
      setStatus(`P2P 错误: ${err.message}`);
    });

    peerRef.current = peer;
  };

  const cleanupConnection = async () => {
    if (peerRef.current) {
      peerRef.current.destroy();
      peerRef.current = null;
    }
    setIsRemoteView(false);
    
    if (isCapturing) {
      try {
        await invoke("stop_screen_capture");
        setIsCapturing(false);
        setScreenFrame(null);
      } catch (e) {
        console.error("Failed to stop capture:", e);
      }
    }
    
    setStatus("就绪 (等待连接)");
  };

  const handleConnect = () => {
    if (!remoteCode || !remotePassword) {
      alert("请输入连接码和密码");
      return;
    }
    setStatus("正在请求连接...");
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({
        type: "Connect",
        code: remoteCode,
        password: remotePassword
      }));
    }
  };

  // 渲染远程控制视图
  if (isRemoteView) {
    return (
      <div className="w-screen h-screen bg-black flex flex-col">
        <div className="bg-gray-900 text-white p-2 flex justify-between items-center text-sm">
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-green-500"></span>
            正在控制: {remoteCode}
          </div>
          <button 
            onClick={cleanupConnection}
            className="bg-red-600 hover:bg-red-700 px-4 py-1 rounded transition-colors"
          >
            断开连接
          </button>
        </div>
        <div 
          className="flex-1 overflow-hidden flex items-center justify-center relative"
          onMouseMove={(e) => {
            if (!peerRef.current || !peerRef.current.connected) return;
            const rect = e.currentTarget.getBoundingClientRect();
            // 计算相对坐标 (0.0 - 1.0)
            const x = (e.clientX - rect.left) / rect.width;
            const y = (e.clientY - rect.top) / rect.height;
            peerRef.current.send(JSON.stringify({ type: "input", action: "mousemove", x, y }));
          }}
          onMouseDown={(e) => {
            if (!peerRef.current || !peerRef.current.connected) return;
            const button = e.button === 0 ? "left" : e.button === 2 ? "right" : "middle";
            peerRef.current.send(JSON.stringify({ type: "input", action: "mousedown", button }));
          }}
          onMouseUp={(e) => {
            if (!peerRef.current || !peerRef.current.connected) return;
            const button = e.button === 0 ? "left" : e.button === 2 ? "right" : "middle";
            peerRef.current.send(JSON.stringify({ type: "input", action: "mouseup", button }));
          }}
          onWheel={(e) => {
            if (!peerRef.current || !peerRef.current.connected) return;
            // 简单的滚轮方向判断
            const deltaY = e.deltaY > 0 ? -1 : 1;
            peerRef.current.send(JSON.stringify({ type: "input", action: "scroll", x: 0, y: deltaY }));
          }}
          onContextMenu={(e) => e.preventDefault()}
          tabIndex={0} // 使 div 可以接收键盘事件
          onKeyDown={(e) => {
            if (!peerRef.current || !peerRef.current.connected) return;
            e.preventDefault();
            peerRef.current.send(JSON.stringify({ type: "input", action: "keydown", key: e.key }));
          }}
          onKeyUp={(e) => {
            if (!peerRef.current || !peerRef.current.connected) return;
            e.preventDefault();
            peerRef.current.send(JSON.stringify({ type: "input", action: "keyup", key: e.key }));
          }}
        >
          {/* 远程画面渲染区域 */}
          <img 
            ref={remoteVideoRef}
            className="max-w-full max-h-full object-contain pointer-events-none"
            alt="Remote Screen"
          />
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900 text-gray-800 dark:text-gray-100 p-6 flex flex-col items-center justify-center">
      <div className="max-w-4xl w-full grid grid-cols-1 md:grid-cols-2 gap-8">
        
        {/* 左侧：本机信息 */}
        <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-lg p-8 border border-gray-100 dark:border-gray-700">
          <div className="flex items-center gap-3 mb-6">
            <div className="p-3 bg-blue-100 dark:bg-blue-900/30 rounded-lg text-blue-600 dark:text-blue-400">
              <Monitor size={24} />
            </div>
            <h2 className="text-2xl font-bold">允许控制本机</h2>
          </div>
          
          <p className="text-gray-500 dark:text-gray-400 mb-8">
            将以下连接码和密码发送给您信任的人，他们将能够远程控制您的设备。
          </p>

          <div className="space-y-6">
            <div>
              <label className="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-2">
                本机连接码
              </label>
              <div className="text-4xl font-mono font-bold tracking-wider text-blue-600 dark:text-blue-400">
                {localCode}
              </div>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-500 dark:text-gray-400 mb-2">
                本机密码
              </label>
              <div className="flex items-center gap-3">
                <div className="text-2xl font-mono font-semibold tracking-widest">
                  {localPassword}
                </div>
                <button 
                  className="text-sm text-blue-500 hover:text-blue-600"
                  onClick={() => {
                    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
                      wsRef.current.send(JSON.stringify({ type: "RegisterHost" }));
                    }
                  }}
                >
                  刷新
                </button>
              </div>
            </div>
          </div>

          <div className="mt-8 pt-6 border-t border-gray-100 dark:border-gray-700 flex items-center justify-between">
            <div className="flex items-center gap-2 text-sm text-green-600 dark:text-green-400">
              <ShieldCheck size={18} />
              <span>状态: {status}</span>
            </div>
            
            <button
              onClick={toggleAutoStart}
              className={`flex items-center gap-2 px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${
                autoStart 
                  ? "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400" 
                  : "bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-400"
              }`}
            >
              <Settings size={14} />
              {autoStart ? "已开启自启" : "开启自启"}
            </button>
          </div>
        </div>

        {/* 右侧：控制远程设备 */}
        <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-lg p-8 border border-gray-100 dark:border-gray-700">
          <div className="flex items-center gap-3 mb-6">
            <div className="p-3 bg-purple-100 dark:bg-purple-900/30 rounded-lg text-purple-600 dark:text-purple-400">
              <LinkIcon size={24} />
            </div>
            <h2 className="text-2xl font-bold">控制远程设备</h2>
          </div>

          <p className="text-gray-500 dark:text-gray-400 mb-8">
            输入对方的连接码和密码，即可发起远程控制。
          </p>

          <div className="space-y-5">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                伙伴连接码
              </label>
              <input
                type="text"
                value={remoteCode}
                onChange={(e) => setRemoteCode(e.target.value)}
                placeholder="请输入 9 位连接码"
                className="w-full px-4 py-3 rounded-lg border border-gray-300 dark:border-gray-600 bg-gray-50 dark:bg-gray-700 focus:ring-2 focus:ring-purple-500 focus:border-transparent outline-none transition-all font-mono text-lg"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                验证密码
              </label>
              <div className="relative">
                <input
                  type="password"
                  value={remotePassword}
                  onChange={(e) => setRemotePassword(e.target.value)}
                  placeholder="请输入密码"
                  className="w-full px-4 py-3 rounded-lg border border-gray-300 dark:border-gray-600 bg-gray-50 dark:bg-gray-700 focus:ring-2 focus:ring-purple-500 focus:border-transparent outline-none transition-all font-mono text-lg pl-11"
                />
                <Key className="absolute left-4 top-3.5 text-gray-400" size={20} />
              </div>
            </div>

            <button
              onClick={handleConnect}
              disabled={status.includes("正在")}
              className="w-full mt-4 bg-purple-600 hover:bg-purple-700 disabled:bg-purple-400 text-white font-semibold py-3.5 px-6 rounded-lg shadow-md hover:shadow-lg transition-all active:scale-[0.98] flex items-center justify-center gap-2"
            >
              <LinkIcon size={20} />
              {status.includes("正在") ? "连接中..." : "连接"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
