
.PHONY: all build clean run-server run-client-ws run-client-quic run-client-auto gen-cert

# 默认参数
WS_PORT ?= 8080
QUIC_PORT ?= 8081

# 编译所有目标
all: build

# 编译项目
build:
	cargo build

# 清理编译产物
clean:
	cargo clean

# 运行服务器
run-server: build
	RUST_LOG=debug cargo run --example chatroom_server

# 运行 WebSocket 客户端
run-client-ws: build
	RUST_LOG=debug cargo run --example chatroom_client ws

# 运行 QUIC 客户端
run-client-quic: build
	RUST_LOG=debug cargo run --example chatroom_client quic

# 运行自动选择协议的客户端
run-client-auto: build
	RUST_LOG=debug cargo run --example chatroom_client auto

# 生成测试证书
gen-cert:
	mkdir -p certs
	openssl req -x509 -newkey rsa:4096 -keyout certs/key.pem -out certs/cert.pem \
		-days 365 -nodes -subj "/CN=localhost"
server:
	RUST_LOG=debug cargo run --example chatroom_server

client:
	RUST_LOG=debug cargo run --example chatroom_client

quic_server:
	RUST_LOG=debug cargo run --example quic_chat server

quic_client:
	RUST_LOG=debug cargo run --example quic_chat client