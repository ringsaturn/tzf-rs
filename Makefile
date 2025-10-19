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
	cargo test

.PHONY: test-export-geojson
test-export-geojson:
	cargo build --features export-geojson
	cargo test --features export-geojson

.PHONY: test-all
test-all: test test-export-geojson
	@echo "All test configurations passed!"

.PHONY: test-examples
test-examples:
	cargo build --example geojson_conversion --features export-geojson
	cargo build --example export_tokyo --features export-geojson
	cargo build --example export_specific_timezones --features export-geojson

.PHONY: doc
doc:
	cargo +nightly doc --no-deps --all-features

.PHONY: ci
ci: test-all test-examples
	cargo fmt --check
	cargo bench
