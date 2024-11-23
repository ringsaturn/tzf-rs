
THIRDPARTY.yml: cargo.lock Cargo.toml
	cargo-bundle-licenses --format yaml --output THIRDPARTY.yml
