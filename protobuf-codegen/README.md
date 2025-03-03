# Protobuf Codegen

Protobuf Codegen 是 Flare 框架的协议生成工具，用于生成 Protocol Buffers 消息和 gRPC 服务代码。

[![Crates.io](https://img.shields.io/crates/v/protobuf-codegen.svg)](https://crates.io/crates/protobuf-codegen)
[![Documentation](https://docs.rs/protobuf-codegen/badge.svg)](https://docs.rs/protobuf-codegen)
[![License](https://img.shields.io/crates/l/protobuf-codegen.svg)](LICENSE)

## 功能特性

- 🔄 **协议生成**
  - 支持 Protocol Buffers 消息定义
  - 支持 gRPC 服务定义
  - 自动生成 Rust 代码

- 🛠 **定制化选项**
  - 支持自定义类型映射
  - 支持服务器端和客户端代码生成
  - 支持属性注入

## 安装

```toml
[dependencies]
protobuf-codegen = "0.1.0"
```

## 使用示例

### 基本使用

```rust
use protobuf_codegen::Builder;

fn main() {
    Builder::new()
        .out_dir("src/generated")
        .compile_protos(&["path/to/your.proto"])
        .unwrap();
}
```

### 自定义配置

```rust
use protobuf_codegen::{Builder, Config};

fn main() {
    let config = Config::new()
        .enable_type_mapping()
        .enable_service_generation()
        .build();

    Builder::new()
        .with_config(config)
        .out_dir("src/generated")
        .compile_protos(&["path/to/your.proto"])
        .unwrap();
}
```

## 配置选项

- `out_dir`: 生成代码的输出目录
- `include_dirs`: proto 文件的搜索路径
- `type_mapping`: 自定义类型映射规则
- `service_generation`: 服务代码生成选项

## 最佳实践

1. **目录结构**
   ```
   your_project/
   ├── build.rs
   ├── protos/
   │   └── service.proto
   └── src/
       └── generated/
   ```

2. **构建脚本 (build.rs)**
   ```rust
   fn main() {
       protobuf_codegen::Builder::new()
           .out_dir("src/generated")
           .compile_protos(&["protos/service.proto"])
           .unwrap();
   }
   ```

## 常见问题

1. **编译错误**
   - 检查 proto 文件语法
   - 确认依赖版本兼容性
   - 验证输出目录权限

2. **类型映射问题**
   - 确认类型名称正确
   - 检查导入路径
   - 验证类型兼容性

## API 文档

详细的 API 文档请访问 [docs.rs](https://docs.rs/protobuf-codegen)。

## 贡献

欢迎提交 Issue 和 Pull Request！

## 开源协议

MIT License 