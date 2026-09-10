.PHONY: fmt
fmt:
	cargo fmt

THIRDPARTY.yml: Cargo.lock Cargo.toml
	cargo-bundle-licenses --format yaml --output THIRDPARTY.yml

NOTICE: THIRDPARTY.yml scripts/build_notice.py
	python3 scripts/build_notice.py

# Lint. `--all-features` is never valid for this crate: `bundled` and `full`
# are mutually exclusive (compile_error!), and with the dev-form path
# dependency cargo additionally refuses "depends on crate tzf-dist multiple
# times with different names". So lint the two data legs separately.
.PHONY: lint
lint:
	cargo clippy --all-targets --features bundled,export-geojson,clap -- -D warnings
	cargo clippy --all-targets --no-default-features --features full,export-geojson,clap -- -D warnings

# Test commands
.PHONY: test
test:
	cargo test-all

.PHONY: test-examples
test-examples:
	cargo run --example demo
	cargo run --example geojson_conversion --features export-geojson
	cargo run --example export_tokyo --features export-geojson
	cargo run --example export_specific_timezones --features export-geojson
	cargo run --example query_tokyo --features export-geojson

.PHONY: doc
doc:
	cargo +nightly doc --no-deps --no-default-features --features bundled,export-geojson

.PHONY: bench
bench:
	cargo bench | tee benchmark_result.txt

.PHONY: bench-full
bench-full:
	cargo bench --no-default-features --features full | tee benchmark_full_result.txt

.PHONY: test-full
test-full:
	cargo test --no-default-features --features full --lib --tests

# Peak RSS of each mechanism (macOS: /usr/bin/time -l; Linux: -v).
.PHONY: memory
memory:
	cargo build --release --example memory_probe
	/usr/bin/time -l ./target/release/examples/memory_probe default || /usr/bin/time -v ./target/release/examples/memory_probe default
	/usr/bin/time -l ./target/release/examples/memory_probe embedded || /usr/bin/time -v ./target/release/examples/memory_probe embedded

benchmark_summary.md: bench bench-full
	@printf '# Benchmark Summary\n\n## Topology-Simplified (bundled)\n\n```\n' > benchmark_summary.md
	@cat benchmark_result.txt >> benchmark_summary.md
	@printf '```\n\n## Full-Precision (full)\n\n```\n' >> benchmark_summary.md
	@cat benchmark_full_result.txt >> benchmark_summary.md
	@printf '```\n' >> benchmark_summary.md

.PHONY: ci
ci: test test-full test-examples
	cargo fmt --check
	$(MAKE) lint
	$(MAKE) benchmark_summary.md

# Everything `cargo publish` would exercise, minus the upload. Fails on the
# dev-form path dependency by design — see Cargo.toml's release-form notes and
# release-runbook-v2.md.
.PHONY: publish-check
publish-check:
	cargo package --list
	cargo publish --dry-run --locked
