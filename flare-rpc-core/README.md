# Flare RPC Core

Flare RPC Core 是 Flare 框架的 RPC 核心实现，基于 tonic (gRPC) 构建，提供高性能的服务注册、发现和负载均衡功能。

[![Crates.io](https://img.shields.io/crates/v/flare-rpc-core.svg)](https://crates.io/crates/flare-rpc-core)
[![Documentation](https://docs.rs/flare-rpc-core/badge.svg)](https://docs.rs/flare-rpc-core)
[![License](https://img.shields.io/crates/l/flare-rpc-core.svg)](LICENSE)

## 功能特性

### 1. 服务治理
- **服务注册与发现**
  - 支持 Consul 和 Etcd 两种注册中心
  - 自动服务注册和注销
  - 服务健康检查和故障转移
  - 支持服务元数据管理

- **负载均衡**
  - 多种负载均衡策略
  - 支持服务权重配置
  - 动态服务列表更新
  - 自动故障节点剔除

### 2. 拦截器机制
- **上下文传递**
  - 请求级别上下文
  - 分布式追踪支持
  - 用户认证信息传递
  - 自定义元数据传输

- **中间件支持**
  - 请求/响应拦截
  - 错误处理中间件
  - 日志记录中间件
  - 监控统计中间件

### 3. 应用管理
- **生命周期管理**
  - 优雅启动和关闭
  - 资源自动清理
  - 连接池管理
  - 超时控制

- **配置管理**
  - 服务配置中心
  - 动态配置更新
  - 环境隔离
  - 配置版本管理

## Features 配置

该库提供以下 feature flags:

- `client`: 客户端功能，包含服务发现和拦截器
- `server`: 服务端功能，包含服务注册和应用上下文
- `consul`: 使用 consul 作为服务注册中心
- `etcd`: 使用 etcd 作为服务注册中心
- `full`: 启用所有功能

默认配置：`default = ["client", "server", "consul"]`

### 依赖关系

```
client
  ├── tonic (transport)
  └── tower (中间件支持)

server
  ├── tonic (gRPC 实现)
  └── tower (服务组件)

consul
  └── reqwest (HTTP 客户端，支持 json)

etcd
  └── etcd-client (官方客户端)
```

## 快速开始

### 1. 客户端示例

```toml
[dependencies]
flare-rpc-core = { version = "0.1.0", default-features = false, features = ["client", "consul"] }
```

```rust
use flare_rpc_core::discover::{ConsulConfig, ConsulDiscover, LoadBalanceStrategy};
use flare_core::context::AppContextBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 配置 Consul
    let config = ConsulConfig {
        addr: "127.0.0.1:8500".to_string(),
        timeout: Duration::from_secs(10),
        protocol: "http".to_string(),
        token: None,
    };
    
    // 创建服务发现客户端
    let discovery = ConsulDiscover::new(config, LoadBalanceStrategy::RoundRobin);
    discovery.start_watch().await;
    
    // 构建请求上下文
    let ctx = AppContextBuilder::new()
        .user_id("user-001")
        .client_id("client-001")
        .with_language(Some("zh-CN"))
        .with_trace_id("trace-001")
        .build()?;
        
    // 发送 RPC 请求
    let response = call_rpc(ctx, client, request, |client, req| {
        client.your_method(req)
    }).await?;
}
```

### 2. 服务端示例

```toml
[dependencies]
flare-rpc-core = { version = "0.1.0", default-features = false, features = ["server", "consul"] }
```

```rust
use flare_rpc_core::app::{App, AppBuilder};
use flare_rpc_core::discover::{ConsulConfig, ConsulRegistry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 配置 Consul
    let consul_config = ConsulConfig {
        addr: "127.0.0.1:8500".to_string(),
        timeout: Duration::from_secs(3),
        protocol: "http".to_string(),
        token: None,
    };

    // 创建服务注册器
    let registry = ConsulRegistry::new(consul_config, Duration::from_secs(15)).await?;

    // 构建应用
    let app = AppBuilder::new("your-service")
        .version("1.0.0")
        .tag("rpc")
        .tag("production")
        .meta("protocol", "grpc")
        .meta("region", "us-west")
        .weight(10)
        .register(registry)
        .build();

    // 运行服务
    app.run("127.0.0.1", 50051, |server| {
        server.add_service(your_service)
            .serve(addr)
    }).await?;
}
```

## 高级特性

### 1. 服务发现扩展
- 支持自定义服务发现实现
- 服务筛选和过滤机制
- 自定义负载均衡策略
- 服务缓存和预热

### 2. 拦截器链
- 可组合的拦截器链
- 顺序控制和优先级
- 条件拦截器
- 异常处理机制

### 3. 监控和统计
- 请求量统计
- 延迟监控
- 错误率统计
- 服务健康度量

## 性能优化

1. **连接池管理**
   - 自动连接复用
   - 空闲连接清理
   - 连接预热
   - 断线重连

2. **负载均衡优化**
   - 支持预热权重
   - 动态权重调整
   - 故障快速切换
   - 并发请求控制

3. **内存管理**
   - 零拷贝设计
   - 内存池复用
   - 对象缓存
   - GC 优化

## 最佳实践

1. **服务设计**
   - 合理划分服务边界
   - 统一错误处理
   - 规范接口定义
   - 版本管理策略

2. **部署配置**
   - 环境隔离
   - 容灾备份
   - 监控告警
   - 日志收集

3. **性能调优**
   - 合理的超时设置
   - 重试策略配置
   - 并发度控制
   - 资源限制

## 常见问题

1. **服务注册失败**
   - 检查注册中心连接
   - 验证服务配置
   - 确认网络可达性
   - 查看错误日志

2. **服务发现异常**
   - 检查服务健康状态
   - 验证服务元数据
   - 确认负载均衡配置
   - 检查缓存状态

3. **性能问题**
   - 分析监控指标
   - 检查资源使用
   - 优化配置参数
   - 排查瓶颈

## 贡献指南

欢迎提交 Issue 和 Pull Request！在提交 PR 之前，请确保：

1. 所有测试通过
2. 代码经过 fmt 和 clippy 检查
3. 更新相关文档
4. 遵循项目编码规范

## 开源协议

MIT License 