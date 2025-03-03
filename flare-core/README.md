# Flare Core

Flare Core 是 Flare 即时通讯框架的核心基础库，提供了构建高性能网络应用所需的基础组件和工具。

[![Crates.io](https://img.shields.io/crates/v/flare.svg)](https://crates.io/crates/flare)
[![Documentation](https://docs.rs/flare/badge.svg)](https://docs.rs/flare)
[![License](https://img.shields.io/crates/l/flare.svg)](LICENSE)

## 功能特性

### 🔧 配置管理
- 支持多种配置源
  ```rust
  use flare::config::{Config, ConfigBuilder};
  
  let config = ConfigBuilder::new()
      .with_file("config.toml")
      .with_env()
      .build()?;
  ```

### 📝 日志系统
- 异步日志
- 多级别支持
- 自定义格式化
  ```rust
  use flare::log::{Logger, LogLevel};
  
  let logger = Logger::builder()
      .with_level(LogLevel::Info)
      .with_async(true)
      .build()?;
  ```

### ⚠️ 错误处理
- 统一错误类型
- 错误链追踪
- 自定义错误转换
  ```rust
  use flare_core::error::{Error, Result};
  
  #[derive(Debug, Error)]
  pub enum MyError {
      #[error("配置错误: {0}")]
      Config(String),
      #[error("网络错误: {0}")]
      Network(#[from] std::io::Error),
  }
  ```

### 🛠 工具集

#### 1. 时间工具
```rust
use flare::utils::time::{Duration, Instant};

let timeout = Duration::from_secs(30);
let start = Instant::now();
```

#### 2. 字符串处理
```rust
use flare::utils::string::{StringExt, Base64Ext};

let encoded = "Hello".to_base64();
let hash = "Password".to_sha256();
```

#### 3. 并发工具
```rust
use flare::utils::sync::{AsyncMutex, AsyncRwLock};

let data = AsyncMutex::new(vec![]);
let cache = AsyncRwLock::new(HashMap::new());
```

#### 4. 网络工具
```rust
use flare::utils::net::{IpExt, PortExt};

let ip = "127.0.0.1".parse_ip()?;
let port = 8080.is_available()?;
```

## 安装

```toml
[dependencies]
flare = "0.1.0"
```

## 示例

### 基础配置

```rust
use flare::{config::Config, log::Logger};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化配置
    let config = Config::from_file("config.toml")?;
    
    // 设置日志
    Logger::init_with_config(&config)?;
    
    // 使用工具函数
    let now = flare::utils::time::now();
    let id = flare::utils::id::generate();
    
    Ok(())
}
```

### 错误处理

```rust
use flare_core::error::{Error, Result};
use flare::log::error;

async fn process() -> Result<()> {
    let config = Config::from_file("config.toml")
        .map_err(|e| Error::new("配置加载失败").with_cause(e))?;
        
    if let Err(e) = do_something().await {
        error!("处理失败: {}", e);
        return Err(e.into());
    }
    
    Ok(())
}
```

## API 文档

详细的 API 文档请访问 [docs.rs](https://docs.rs/flare)。

## 模块结构

```
flare/
├── config/     # 配置管理
├── log/        # 日志系统
├── error/      # 错误处理
└── utils/      # 工具集
    ├── time/   # 时间工具
    ├── string/ # 字符串处理
    ├── sync/   # 并发工具
    └── net/    # 网络工具
```

## 性能考虑

- 日志异步处理，避免 I/O 阻塞
- 使用无锁数据结构
- 内存池复用
- 零拷贝操作

## 最佳实践

1. **配置管理**
   - 使用分层配置
   - 环境变量覆盖
   - 配置热重载

2. **日志使用**
   - 合理设置日志级别
   - 使用结构化日志
   - 避免过多日志

3. **错误处理**
   - 使用错误链
   - 提供上下文信息
   - 适当的错误恢复

4. **工具选择**
   - 优先使用库提供的工具
   - 注意性能开销
   - 合理使用异步

## 常见问题

1. **配置加载失败**
   - 检查文件权限
   - 验证配置格式
   - 确认环境变量

2. **日志写入问题**
   - 检查磁盘空间
   - 验证文件权限
   - 调整缓冲大小

3. **性能问题**
   - 使用异步日志
   - 避免频繁配置读取
   - 合理使用锁

## 版本说明

- 遵循语义化版本
- 保持向后兼容
- 及时更新依赖

## 贡献

欢迎提交 Issue 和 Pull Request！

## 开源协议

MIT License 