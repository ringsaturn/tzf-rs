.PHONY: fmt
fmt:
	cargo fmt

THIRDPARTY.yml: Cargo.lock Cargo.toml
	cargo-bundle-licenses --format yaml --output THIRDPARTY.yml

NOTICE: THIRDPARTY.yml scripts/build_notice.py
	python3 scripts/build_notice.py

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
	make benchmark_summary.md
