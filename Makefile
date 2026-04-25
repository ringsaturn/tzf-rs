.PHONY: fmt
fmt:
	cargo fmt

THIRDPARTY.yml: cargo.lock Cargo.toml
	cargo-bundle-licenses --format yaml --output THIRDPARTY.yml

.PHONY: pb
pb:
	export TZF_RS_BUILD_PB=1 && cargo build

# Test commands
.PHONY: test
test:
	cargo test-all

.PHONY: test-examples
test-examples:
	cargo run --example geojson_conversion --features export-geojson
	cargo run --example export_tokyo --features export-geojson
	cargo run --example export_specific_timezones --features export-geojson
	cargo run --example query_tokyo --features export-geojson

.PHONY: doc
doc:
	cargo +nightly doc --no-deps --all-features

.PHONY: bench
bench:
	cargo bench | tee benchmark_result.txt
	./scripts/bench_memory.sh benchmark_result.txt | tee benchmark_report.md

.PHONY: bench-full
bench-full:
	cargo bench --no-default-features --features full | tee benchmark_full_result.txt

.PHONY: test-full
test-full:
	cargo test --no-default-features --features full --lib --tests

.PHONY: ci
ci: test test-full test-examples
	cargo fmt --check
	make bench
	make bench-full

extract-plot: bench
	cp target/criterion/DefaultFinderIndexModes/0/report/violin.svg assets/violin.svg
	cp target/criterion/DefaultFinderIndexModes/NoIndex/0/report/pdf.svg assets/no_index.pdf.svg
	cp target/criterion/DefaultFinderIndexModes/YStripesOnly/0/report/pdf.svg assets/ystripes_only.pdf.svg
