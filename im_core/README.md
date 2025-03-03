# IM Core

IM Core 是一个支持 WebSocket 和 QUIC 协议的即时通讯核心库，提供客户端和服务端功能实现。

## Features 配置

该库提供以下 feature flags:

- `client`: 客户端功能，包含 WebSocket 和 QUIC 客户端实现
- `server`: 服务端功能，包含 WebSocket 和 QUIC 服务端实现
- `full`: 完整功能，包含客户端和服务端所有功能（等同于同时启用 `client` 和 `server`）

默认启用客户端和服务端功能：`default = ["client", "server"]`

### 依赖关系

```
client
  ├── tokio-tungstenite (WebSocket 支持)
  ├── quinn (QUIC 支持)
  ├── rustls (TLS 支持)
  └── rustls-pemfile (证书处理)

server
  ├── tokio-tungstenite (WebSocket 支持)
  ├── quinn (QUIC 支持)
  ├── rustls (TLS 支持)
  └── rustls-pemfile (证书处理)
```

## 使用示例

### 1. 仅客户端场景

```toml
[dependencies]
im_core = { version = "0.1.0", default-features = false, features = ["client"] }
```

### 2. 仅服务端场景

```toml
[dependencies]
im_core = { version = "0.1.0", default-features = false, features = ["server"] }
```

### 3. 完整功能

```toml
[dependencies]
im_core = { version = "0.1.0" }  # 使用默认 features
# 或者
im_core = { version = "0.1.0", features = ["full"] }
```

## 代码示例

### WebSocket 客户端

```rust
use im_core::client::websocket::WsClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = WsClient::new("ws://localhost:8080/chat")
        .with_auth_token("your-token")
        .build()?;
        
    client.connect().await?;
    client.send_message("Hello").await?;
    
    Ok(())
}
```

### QUIC 服务端

```rust
use im_core::server::quic::QuicServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = QuicServer::new()
        .with_cert_path("cert.pem")
        .with_key_path("key.pem")
        .build()?;
        
    server.listen("127.0.0.1:8443").await?;
    
    Ok(())
}
```

## 协议支持

1. **WebSocket**
   - 基于 `tokio-tungstenite` 实现
   - 支持文本和二进制消息
   - 支持心跳保活
   - 支持断线重连

2. **QUIC**
   - 基于 `quinn` 实现
   - 支持多路复用
   - 内置 TLS 1.3
   - 支持 0-RTT 连接
   - 支持连接迁移

## 安全性考虑

1. **TLS 配置**
   - 使用 `rustls` 提供 TLS 支持
   - 支持自定义证书配置
   - 推荐使用 TLS 1.3

2. **认证机制**
   - 支持 token 认证
   - 支持自定义认证插件
   - 支持加密消息传输

## 性能考虑

1. **WebSocket**
   - 适合需要实时性但网络条件较好的场景
   - 支持大规模并发连接
   - 资源占用相对较低

2. **QUIC**
   - 适合移动场景和弱网环境
   - 支持连接迁移，网络切换时保持会话
   - 多路复用减少连接开销
   - 0-RTT 重连提高性能

## 最佳实践

1. 在弱网环境优先使用 QUIC 协议
2. 需要广泛兼容性时使用 WebSocket
3. 生产环境必须启用 TLS
4. 根据实际需求选择合适的重连策略
5. 合理配置心跳间隔，建议 30-60 秒
6. 使用 tokio 运行时的多线程模式提高性能 