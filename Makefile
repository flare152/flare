.PHONY: all build clean run-server run-client1 run-client2

# 默认参数
WS_PORT ?= 8080

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
	cargo run --bin server

# 运行客户端1
run-client1: build
	cargo run --bin client

# 运行客户端2
run-client2: build
	cargo run --bin client