# 发布指南

本指南将帮助您将项目发布到 crates.io。

## 准备工作

### 1. 注册 crates.io 账号

1. 访问 [crates.io](https://crates.io)
2. 使用 GitHub 账号登录
3. 在个人设置页面生成 API Token

### 2. 配置 cargo

```bash
cargo login <your-api-token>
```

### 3. 更新 Cargo.toml

每个要发布的 crate 都需要在 `Cargo.toml` 中添加以下必要信息：

```toml
[package]
name = "flare"        # crate 名称
version = "0.1.0"     # 版本号
edition = "2021"      # Rust 版本
license = "MIT"       # 开源协议
description = "A high performance IM framework"  # 项目描述
homepage = "https://github.com/yourusername/flare"  # 项目主页
repository = "https://github.com/yourusername/flare"  # 代码仓库
documentation = "https://docs.rs/flare"  # 文档地址
readme = "README.md"  # README 文件路径
keywords = ["im", "chat", "websocket", "quic"]  # 关键词，最多 5 个
categories = ["network-programming"]  # 分类，见 crates.io/categories

# 以下是工作空间中各个 crate 的配置
[workspace]
members = [
    "flare",
    "im_core",
    "rpc_core",
    "protobuf-codegen",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
license = "MIT"
rust-version = "1.70"
```

### 4. 完善文档

1. 确保每个公开 API 都有文档注释
2. 更新 README.md，包含：
   - 项目简介
   - 功能特性
   - 安装方法
   - 使用示例
   - API 文档链接
   - 贡献指南
   - 开源协议

## 发布流程

### 1. 发布顺序

由于项目是工作空间结构，需要按照依赖顺序发布：

1. 首先发布基础库 `flare`：
```bash
cd flare
cargo publish --dry-run  # 先进行测试
cargo publish           # 确认无误后正式发布
```

2. 发布 `protobuf-codegen`：
```bash
cd ../protobuf-codegen
cargo publish
```

3. 发布 `rpc_core`：
```bash
cd ../rpc_core
cargo publish
```

4. 发布 `im_core`：
```bash
cd ../im_core
cargo publish
```

### 2. 版本管理

- 遵循 [语义化版本](https://semver.org/lang/zh-CN/) 规范
- 主版本号：不兼容的 API 修改
- 次版本号：向下兼容的功能性新增
- 修订号：向下兼容的问题修正

### 3. 发布检查清单

每次发布前检查：

- [ ] 所有测试通过：`cargo test --all-features`
- [ ] 文档完善：`cargo doc --no-deps`
- [ ] 代码格式化：`cargo fmt --all`
- [ ] Clippy 检查：`cargo clippy --all-features`
- [ ] 更新 CHANGELOG.md
- [ ] 更新版本号
- [ ] 确认 README.md 内容最新
- [ ] 确认所有依赖版本合理

### 4. 持续维护

1. **版本更新**
   - 及时响应用户反馈
   - 定期更新依赖
   - 修复安全问题
   - 添加新功能

2. **文档维护**
   - 保持文档与代码同步
   - 添加更多使用示例
   - 更新 API 变更说明

3. **社区维护**
   - 回应 Issues
   - 处理 Pull Requests
   - 更新 CHANGELOG
   - 发布版本公告

## 常见问题

1. **发布失败**
   - 检查版本号是否已存在
   - 确认所有必填字段已填写
   - 验证依赖版本兼容性
   - 检查 crate 名称是否可用

2. **文档构建失败**
   - 确保所有公开 API 都有文档注释
   - 检查文档示例代码是否能编译
   - 验证文档链接是否有效

3. **依赖冲突**
   - 检查依赖版本兼容性
   - 考虑使用 feature flags 分离功能
   - 更新到最新的兼容版本

## 帮助资源

- [Cargo 手册](https://doc.rust-lang.org/cargo/)
- [发布指南](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [语义化版本](https://semver.org/lang/zh-CN/)
- [crates.io](https://crates.io)
- [Rust API 指南](https://rust-lang.github.io/api-guidelines/) 