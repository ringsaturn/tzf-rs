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

.PHONY: ci
ci: test test-examples
	cargo fmt --check
	cargo bench
