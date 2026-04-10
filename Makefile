.PHONY: build run test lint format docker-build docker-run clean

BINARY     := mauns
IMAGE_NAME := ghcr.io/mauns/mauns
IMAGE_TAG  := latest
TASK       ?= "list files in the current directory"

build:
	cargo build --release --bin $(BINARY)

run:
	cargo run --release --bin $(BINARY) -- run $(TASK)

test:
	cargo test --workspace

lint:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

format:
	cargo fmt --all

format-check:
	cargo fmt --all -- --check

docker-build:
	docker build -t $(IMAGE_NAME):$(IMAGE_TAG) .

docker-run:
	docker run --rm -it \
		-e CLAUDE_API_KEY=$(CLAUDE_API_KEY) \
		-e OPENAI_API_KEY=$(OPENAI_API_KEY) \
		-v $(PWD):/workspace \
		$(IMAGE_NAME):$(IMAGE_TAG) \
		run $(TASK)

clean:
	cargo clean
