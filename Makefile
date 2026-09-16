.PHONY: check fmt fmt-check clippy test build wasm clean

WASM_TARGET := wasm32v1-none
WASM_PATH := target/$(WASM_TARGET)/release/pulsar_core_contract.wasm

# fmt -> wasm -> clippy -> test. `src/test.rs`'s self-upgrade fixture pulls
# in $(WASM_PATH) via `include_bytes!`, and that's compiled as part of the
# test target — which `clippy --all-targets` also builds — so the wasm must
# exist before both, not just before `cargo test`. See the comment on
# `SELF_WASM` in src/test.rs.
check: fmt-check wasm clippy test

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

clippy:
	cargo clippy --all-targets -- -D warnings

wasm: $(WASM_PATH)

$(WASM_PATH): $(shell find src -name '*.rs') Cargo.toml
	cargo build --release --target $(WASM_TARGET)

test: wasm
	cargo test

build: wasm

clean:
	cargo clean
