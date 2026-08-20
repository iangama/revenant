.PHONY: format lint test build inspector audit release smoke check

format:
	cargo fmt --all --check

lint:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
	cargo test --workspace --all-targets

build:
	cargo build --workspace --all-targets

inspector:
	npm --prefix web/control-panel run check
	npm --prefix web/control-panel run build

audit:
	bash scripts/audit-secrets.sh

release:
	bash scripts/release.sh 0.1.0

smoke:
	bash tests/smoke.sh

check: format lint test build inspector audit smoke
