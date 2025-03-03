# Flare

Flare 是一个高性能的即时通讯框架，基于 Rust 开发，支持多协议接入、分布式部署和全球化服务。

[![Crates.io](https://img.shields.io/crates/v/flare.svg)](https://crates.io/crates/flare)
[![Documentation](https://docs.rs/flare/badge.svg)](https://docs.rs/flare)
[![License](https://img.shields.io/crates/l/flare.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.85%2B-blue.svg)](https://www.rust-lang.org)

## 技术栈

- **运行时**: tokio 1.0+ (异步运行时)
- **网络协议**:
  - WebSocket (tokio-tungstenite 0.26)
  - QUIC (quinn 0.11.5)
  - gRPC (tonic 0.12)
- **安全性**:
  - TLS (rustls 0.23.5)
  - 证书管理 (rustls-pemfile 2.0)
- **数据处理**:
  - Protocol Buffers (prost 0.13.5)
  - JSON (serde_json 1.0)
- **工具库**:
  - 日志 (log 0.4, env_logger 0.11)
  - 错误处理 (anyhow 1.0, thiserror 2.0)
  - 时间处理 (chrono 0.4)
  - 并发原语 (async-broadcast 0.7)
  - 并发集合 (dashmap 6.1)

## 特性

- 🚀 **高性能设计**
  - 基于 Rust 1.85+ 开发，零成本抽象
  - 异步 I/O，基于 tokio 1.0+ 运行时
  - 支持多协议并发处理

- 🌐 **多协议支持**
  - WebSocket：基于 tokio-tungstenite 0.26
  - QUIC：基于 quinn 0.11.5，支持 0-RTT
  - gRPC：基于 tonic 0.12，支持服务发现

- 🔐 **安全性**
  - TLS 1.3 加密传输 (rustls 0.23.5)
  - 支持自定义认证插件
  - 数据加密存储

- 🌍 **全球化部署**
  - 多数据中心支持
  - 就近接入策略
  - 跨区域消息同步

- 🎯 **可扩展性**
  - 插件化架构
  - 支持自定义协议
  - 灵活的消息处理机制

## 架构

```
                                    ┌─────────────────┐
                                    │   DNS Router    │
                                    └────────┬────────┘
                                             │
                     ┌─────────────────────┬─┴──┬─────────────────────┐
                     │                     │    │                     │
              ┌──────┴──────┐       ┌──────┴────┴─┐           ┌──────┴──────┐
              │  DC-A       │       │    DC-B     │           │    DC-C     │
              │(Asia)       │       │  (America)  │           │  (Europe)   │
              └──────┬──────┘       └──────┬──────┘           └──────┬──────┘
                     │                     │                          │
        ┌────────────┼────────────────────┼────────────────────┐     │
        │            │                     │                    │     │
┌───────┴────────┐   │             ┌──────┴──────┐     ┌──────┴─────┴──┐
│ Load Balancer  │   │             │ Service     │     │ Storage       │
│ - WebSocket    │   │             │ Discovery   │     │ - Redis       │
│ - QUIC         │   │             └──────┬──────┘     │ - ScyllaDB    │
└───────┬────────┘   │                    │            └───────────────┘
        │            │                    │
┌───────┴────────┐   │             ┌─────┴─────┐
│ Signal Service │◄──┴─────────────┤ Monitor   │
└───────┬────────┘                 └───────────┘
        │
┌───────┴────────┐
│  Push Service  │
└────────────────┘
```

## 项目结构

```
flare/
├── flare-core/         # 核心库
├── flare-im-core/      # 即时通讯核心实现
└── flare-rpc-core/     # RPC 框架实现
```

## 快速开始

### 环境要求

- Rust 1.85.0 或更高版本
- OS: Linux, macOS
- 内存: 8GB+
- CPU: 4核+

### 安装

```toml
[dependencies]
flare-core = "0.1.0"      # 核心库
flare-im-core = "0.1.0"   # IM 功能
flare-rpc-core = "0.1.0"  # RPC 功能

# 可选依赖
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
```

### 示例

#### WebSocket 服务端

```rust
use flare_im_core::server::websocket::WsServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = WsServer::new()
        .with_addr("127.0.0.1:8080")
        .with_tls("cert.pem", "key.pem")
        .build()?;
        
    server.run().await?;
    Ok(())
}
```

#### QUIC 客户端

```rust
use flare_im_core::client::quic::QuicClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = QuicClient::new("example.com:8443")
        .with_token("your-auth-token")
        .build()?;
        
    client.connect().await?;
    client.send_message("Hello").await?;
    Ok(())
}
```

## 性能指标

- 单机并发连接：100w+
- 消息延迟：<50ms
- CPU 使用率：<30%
- 内存占用：<4GB

## 部署要求

- OS: Linux, macOS
- Rust: 1.85+
- 内存: 8GB+
- CPU: 4核+

## 文档

- [用户指南](docs/guide/README.md)
- [API 文档](https://docs.rs/flare)
- [部署指南](docs/deploy/README.md)
- [开发指南](docs/development/README.md)
- [发布指南](PUBLISHING.md)

## 子项目

### flare-core

基础库，提供核心功能和工具：
- 配置管理 (serde 1.0)
- 日志系统 (log 0.4, env_logger 0.11)
- 错误处理 (anyhow 1.0, thiserror 2.0)
- 通用工具 (chrono 0.4, uuid 1.0)

### flare-im-core

即时通讯核心实现：
- WebSocket 支持 (tokio-tungstenite 0.26)
- QUIC 支持 (quinn 0.11.5)
- 消息处理 (async-broadcast 0.7)
- 会话管理 (dashmap 6.1)

### flare-rpc-core

RPC 框架实现：
- 服务发现 (consul/etcd)
- 负载均衡 (tower 0.5)
- 服务注册 (tonic 0.12)
- 拦截器 (tower 0.5)
- 协议生成 (prost 0.13.5, prost-build 0.13.5)

## 版本要求

- Rust: 1.85.0+
- 主要依赖版本:
  - tokio: 1.0+
  - tonic: 0.12
  - quinn: 0.11.5
  - tokio-tungstenite: 0.26
  - rustls: 0.23.5

## 贡献指南

1. Fork 项目
2. 创建特性分支
3. 提交变更
4. 推送到分支
5. 创建 Pull Request

## 开源协议

本项目采用 MIT 协议，详见 [LICENSE](LICENSE) 文件。

## 联系方式

- Issues: [GitHub Issues](https://github.com/yourusername/flare/issues)
- 邮箱: flare1522@163.com
- 讨论组: [GitHub Discussions](https://github.com/yourusername/flare/discussions)

## 致谢

感谢以下开源项目：
- [tokio](https://github.com/tokio-rs/tokio)
- [tonic](https://github.com/hyperium/tonic)
- [quinn](https://github.com/quinn-rs/quinn)
- [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) 