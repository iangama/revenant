.PHONY: version format lint test build inspector audit release smoke check

version:
	bash scripts/check-version.sh

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
	bash scripts/release.sh

smoke:
	bash tests/smoke.sh

check: version format lint test build inspector audit smoke
