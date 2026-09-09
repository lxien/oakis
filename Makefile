.PHONY: help run dev build release check clean fmt

help:
	@echo "make run      - 运行"
	@echo "make dev      - 调试"
	@echo "make build    - 构建"
	@echo "make release  - 发行"
	@echo "make check    - 检查"
	@echo "make fmt      - 格式化"
	@echo "make clean    - 清理"

run: release
	./target/release/oakis

dev:
	cargo run

build:
	cargo build

release:
	cargo build --release

check:
	cargo check

fmt:
	cargo fmt

clean:
	cargo clean
