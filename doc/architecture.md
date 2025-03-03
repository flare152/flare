# IM 系统架构设计

## 1. 系统概述

本IM系统采用微服务架构，使用 Rust 语言开发，基于 tonic (gRPC) 框架实现。系统划分为以下核心服务：

### 1.1 核心服务
- 信令服务 (Signal Service)
  - 登录服务 (Login Service)
  - 在线服务 (Online Service)
  - 路由服务 (Route Service)
- 会话服务 (Session Service)
- 消息服务 (Message Service)
- 群组服务 (Group Service)
- 关系链服务 (Relationship Service)
- 在线状态服务 (Presence Service)
- 推送服务 (Push Service)
- 存储服务 (Storage Service)

## 2. 技术栈选型

### 2.1 基础框架
- 语言: Rust
- RPC 框架: tonic (gRPC)
- 服务注册与发现: Consul
- 消息队列: Kafka
- 缓存: Redis
- 存储: 
  - 消息存储: ScyllaDB (Cassandra 兼容)
  - 关系存储: PostgreSQL
  - 对象存储: MinIO

### 2.2 监控与运维
- 链路追踪: OpenTelemetry + Jaeger
- 监控: Prometheus + Grafana
- 日志: ELK Stack
- 容器化: Docker + Kubernetes

## 3. 服务详细说明

### 3.1 信令服务 (Signal Service)

#### 3.1.1 登录服务 (Login Service)
- 职责：
  - 用户身份认证
  - Token 生成与验证
  - 登录状态管理
  - 多端登录控制
- 关键技术：
  - JWT 认证
  - Redis Session 存储
  - 设备管理
- 存储：
  - Redis (登录状态)
  - PostgreSQL (用户认证信息)

#### 3.1.2 在线服务 (Online Service)
- 职责：
  - 连接状态管理
  - 心跳检测
  - 在线状态同步
  - 设备状态管理
- 关键技术：
  - WebSocket 长连接
  - 心跳机制
  - 状态同步协议
- 存储：
  - Redis (在线状态)
  - Kafka (状态变更事件)

#### 3.1.3 路由服务 (Route Service)
- 职责：
  - 消息路由转发
  - 负载均衡
  - 连接分配
  - 服务发现
- 关键技术：
  - Consul 服务发现
  - 一致性哈希
  - 动态路由策略
- 存储：
  - Redis (路由表缓存)
  - Consul (服务注册)

#### 3.1.4 信令服务交互流程

1. 用户登录流程：
   - 客户端发起登录请求到 Login Service
   - Login Service 验证用户身份并生成 Token
   - 返回 Token 和服务地址信息给客户端

2. 建立连接流程：
   - 客户端携带 Token 连接 Online Service
   - Online Service 验证 Token 并建立 WebSocket 连接
   - 更新用户在线状态
   - 通知 Route Service 更新路由表

3. 消息路由流程：
   - 客户端发送消息到 Online Service
   - Online Service 请求 Route Service 获取目标路由
   - Route Service 返回最优的服务节点
   - Online Service 将消息转发到目标节点

#### 3.1.5 高可用设计

1. 多机房部署：
   - 就近接入
   - 跨机房容灾
   - 智能 DNS 解析

2. 连接保护：
   - 连接快速恢复
   - 会话保持
   - 断线重连

3. 负载均衡：
   - 服务器负载监控
   - 动态负载均衡
   - 平滑扩缩容

4. 限流保护：
   - 连接数限制
   - 消息频率控制
   - 黑名单机制

### 3.2 会话服务 (Session Service)
- 职责：
  - 管理用户会话状态
  - 维护用户在线状态
  - 会话同步
- 存储：
  - Redis (会话状态)
  - PostgreSQL (持久化)

### 3.3 消息服务 (Message Service)
- 职责：
  - 消息存储
  - 消息同步
  - 消息分发
  - 离线消息处理
- 存储：
  - Kafka (消息队列)
  - ScyllaDB (消息存储)
  - Redis (最近消息缓存)

### 3.4 群组服务 (Group Service)
- 职责：
  - 群组管理
  - 群成员管理
  - 群消息处理
- 存储：
  - PostgreSQL (群组信息)
  - Redis (群成员缓存)

### 3.5 关系链服务 (Relationship Service)
- 职责：
  - 好友关系管理
  - 黑名单管理
  - 关系链同步
- 存储：
  - PostgreSQL (关系数据)
  - Redis (缓存)

### 3.6 在线状态服务 (Presence Service)
- 职责：
  - 用户在线状态管理
  - 状态订阅与推送
- 存储：
  - Redis (状态存储)
  - Kafka (状态变更事件)

### 3.7 推送服务 (Push Service)

#### 3.7.1 PushProxy
- 职责：
  - 接收推送请求
  - 请求验证和过滤
  - 负载均衡
  - 推送任务分发
- 关键技术：
  - gRPC 接口
  - 限流控制
  - 请求路由
- 存储：
  - Redis (限流计数)

#### 3.7.2 Kafka 消息队列
- 职责：
  - 推送任务队列
  - 消息削峰填谷
  - 任务持久化
  - 失败任务重试
- 配置：
  - 多分区部署
  - 消息持久化
  - 消息压缩
  - 消息保留策略

#### 3.7.3 PushServer
- 职责：
  - 推送策略管理
  - 设备令牌管理
  - 推送通道选择
  - 推送统计和监控
- 关键技术：
  - 多通道支持
  - 智能通道选择
  - 设备令牌更新
- 存储：
  - Redis (设备令牌)
  - PostgreSQL (推送配置)

#### 3.7.4 PushWorker
- 职责：
  - 消息投递执行
  - 通道连接维护
  - 失败重试处理
  - 推送结果收集
- 集成：
  - FCM (Firebase Cloud Messaging)
  - APNS (Apple Push Notification Service)
  - HMS (Huawei Mobile Services)
  - 自建长连接通道
- 存储：
  - Redis (推送状态)
  - PostgreSQL (推送记录)

#### 3.7.5 推送服务交互流程

1. 消息接收流程：
   - 业务服务调用 PushProxy 发起推送请求
   - PushProxy 验证请求并进行限流控制
   - 将推送任务写入 Kafka 队列

2. 消息处理流程：
   - PushServer 从 Kafka 获取推送任务
   - 查询设备令牌和推送策略
   - 根据设备类型选择合适的推送通道
   - 将任务分配给 PushWorker

3. 消息投递流程：
   - PushWorker 建立与推送通道的连接
   - 执行消息推送
   - 收集推送结果
   - 处理失败重试

#### 3.7.6 高可用设计

1. 服务容灾：
   - 多机房部署
   - 服务自动扩缩容
   - 故障自动转移

2. 消息可靠性：
   - 消息持久化
   - 失败重试机制
   - 消息幂等处理

3. 性能优化：
   - 批量推送
   - 连接池复用
   - 异步处理

4. 监控告警：
   - 推送成功率监控
   - 延迟监控
   - 通道状态监控
   - 容量预警

### 3.8 存储服务 (Storage Service)

#### 3.8.1 MsgProxy
- 职责：
  - 消息存储请求代理
  - 负载均衡
  - 请求验证和过滤
  - 存储任务分发
- 关键技术：
  - gRPC 接口
  - 一致性哈希
  - 限流控制
- 存储：
  - Redis (限流和缓存)

#### 3.8.2 MsgWriter
- 职责：
  - 消息写入
  - 数据分片
  - 写入性能优化
  - 数据同步
- 关键技术：
  - 批量写入
  - 异步写入
  - 数据压缩
- 存储：
  - ScyllaDB (消息存储)
  - Redis (写缓存)

#### 3.8.3 MsgReader
- 职责：
  - 消息读取
  - 数据查询优化
  - 缓存管理
  - 读取性能优化
- 关键技术：
  - 多级缓存
  - 预读取
  - 读写分离
- 存储：
  - ScyllaDB (消息存储)
  - Redis (读缓存)

#### 3.8.4 运维工具
- 职责：
  - 数据备份恢复
  - 存储监控
  - 性能分析
  - 数据清理
- 功能：
  - 备份管理
  - 容量规划
  - 性能诊断
  - 数据迁移

#### 3.8.5 存储服务交互流程

1. 消息写入流程：
   - 业务服务调用 MsgProxy 发起存储请求
   - MsgProxy 根据分片策略选择 MsgWriter
   - MsgWriter 执行数据写入
   - 更新缓存

2. 消息读取流程：
   - 业务服务通过 MsgProxy 发起读取请求
   - MsgProxy 路由到合适的 MsgReader
   - MsgReader 优先从缓存读取
   - 缓存未命中则从 ScyllaDB 读取

3. 数据维护流程：
   - 运维工具定期执行数据备份
   - 监控存储容量和性能
   - 执行数据清理和优化
   - 处理数据迁移任务

#### 3.8.6 高可用设计

1. 数据可靠性：
   - 多副本存储
   - 实时同步
   - 定期备份
   - 数据校验

2. 性能优化：
   - 读写分离
   - 多级缓存
   - 数据分片
   - 批量操作

3. 容量管理：
   - 动态扩容
   - 冷热数据分离
   - 数据压缩
   - 自动清理

4. 监控告警：
   - 存储容量监控
   - 性能指标监控
   - IO 延迟监控
   - 错误率监控

## 4. 系统特性

### 4.1 高可用设计
- 服务无状态设计
- 多机房部署
- 故障自动转移
- 消息队列削峰填谷

### 4.2 扩展性设计
- 微服务架构
- 服务水平扩展
- 数据分片
- 多级缓存

### 4.3 安全性设计
- 端到端加密
- 传输层 TLS
- Token 认证
- 消息防重放

### 4.4 可观测性
- 分布式追踪
- 性能监控
- 业务监控
- 告警机制

## 5. 部署架构

### 5.1 开发环境
- Docker Compose 部署
- 本地开发工具链
- 测试环境配置

### 5.2 生产环境
- Kubernetes 集群
- 多区域部署
- 灾备方案
- 扩缩容策略 