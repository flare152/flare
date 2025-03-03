# RPC Core

RPC Core 是一个基于 tonic (gRPC) 的 RPC 框架，支持服务注册与发现，提供 etcd 和 consul 两种实现方式。

## Features 配置

该库提供以下 feature flags:

- `client`: 客户端功能，包含服务发现和拦截器
- `server`: 服务端功能，包含服务注册和应用上下文
- `etcd`: 使用 etcd 作为服务注册中心
- `consul`: 使用 consul 作为服务注册中心

默认启用所有功能：`default = ["client", "server", "etcd", "consul"]`

### 依赖关系

```
client
  ├── tonic (transport)
  ├── tower
  └── rand (负载均衡)

server
  ├── tonic
  └── tower

etcd
  └── etcd-client

consul
  └── reqwest
```

## 使用示例

### 1. 客户端场景

#### 使用 etcd 的客户端:
```toml
[dependencies]
rpc_core = { version = "0.1.0", default-features = false, features = ["client", "etcd"] }
```

#### 使用 consul 的客户端:
```toml
[dependencies]
rpc_core = { version = "0.1.0", default-features = false, features = ["client", "consul"] }
```

### 2. 服务端场景

#### 使用 etcd 的服务端:
```toml
[dependencies]
rpc_core = { version = "0.1.0", default-features = false, features = ["server", "etcd"] }
```

#### 使用 consul 的服务端:
```toml
[dependencies]
rpc_core = { version = "0.1.0", default-features = false, features = ["server", "consul"] }
```

### 3. 完整功能

#### 同时支持 etcd 和 consul 的完整功能:
```toml
[dependencies]
rpc_core = { version = "0.1.0" }  # 使用默认 features
```

#### 仅使用 etcd 的完整功能:
```toml
[dependencies]
rpc_core = { version = "0.1.0", default-features = false, features = ["client", "server", "etcd"] }
```

#### 仅使用 consul 的完整功能:
```toml
[dependencies]
rpc_core = { version = "0.1.0", default-features = false, features = ["client", "server", "consul"] }
```

## 代码示例

### 客户端 (etcd)

```rust
use rpc_core::{
    client::RpcClient,
    discover::{EtcdDiscover, LoadBalanceStrategy},
};

#[tokio::main]
async fn main() {
    let discover = EtcdDiscover::new("http://localhost:2379").await?;
    let client = RpcClient::new()
        .with_discover(discover)
        .with_load_balance(LoadBalanceStrategy::RoundRobin)
        .build();
}
```

### 服务端 (consul)

```rust
use rpc_core::{
    app::App,
    discover::{ConsulRegistry, Registration},
};

#[tokio::main]
async fn main() {
    let registry = ConsulRegistry::new("http://localhost:8500")?;
    let app = App::new()
        .with_registry(registry)
        .with_service(MyService::new())
        .build();
    
    app.serve("127.0.0.1:50051").await?;
}
```

## 注意事项

1. 不同的注册中心实现（etcd/consul）不能同时使用，请根据实际需求选择其一
2. 客户端和服务端可以使用不同的注册中心实现
3. 建议在生产环境中使用相同的注册中心实现，以保证服务发现的一致性
4. 负载均衡策略仅在客户端生效
5. 拦截器功能在客户端和服务端都可用，但具体实现会根据场景有所不同

## 性能考虑

1. etcd 实现使用官方的 etcd-client，支持 watch 机制，实时性较好
2. consul 实现使用 HTTP API，需要定期轮询，实时性略差但配置简单
3. 默认的负载均衡策略为 RoundRobin，适合大多数场景
4. 建议根据实际需求选择合适的注册中心和负载均衡策略 