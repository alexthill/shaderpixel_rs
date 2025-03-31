NAME=$(shell basename $(CURDIR))
BIN_NAME=shaderpixel_rs
CARGO=/root/.cargo/bin/cargo
CONTAINER=rust-rust-1
TARGET=$(HOME)/goinfre/rust_root/target

($NAME): run

c: check
check:
	@docker exec --tty --workdir /src/$(NAME) $(CONTAINER) $(CARGO) check

clippy:
	@docker exec --tty --workdir /src/$(NAME) $(CONTAINER) $(CARGO) clippy

b: build
build:
	@docker exec --tty --workdir /src/$(NAME) $(CONTAINER) $(CARGO) build

br: build_release
build_release:
	@docker exec --tty --workdir /src/$(NAME) $(CONTAINER) $(CARGO) build --release

r: run
run: download build
	@RUST_BACKTRACE=1 RUST_LOG=debug $(TARGET)/debug/$(BIN_NAME)

rr: run_release
run_release: download build_release
	@RUST_LOG=info $(TARGET)/release/$(BIN_NAME)

t: test
test:
	@docker exec --tty --workdir /src/$(NAME) $(CONTAINER) $(CARGO) test

clean:
	@echo "cargo clean"
	@docker exec --tty --workdir /src/$(NAME) $(CONTAINER) $(CARGO) clean
	rm -f assets/downloads/*

re: clean run

download:
	./assets/download.sh
