
THIRDPARTY.yml: cargo.lock Cargo.toml
	cargo-bundle-licenses --format yaml --output THIRDPARTY.yml

.PHONY: pb
pb:
	export TZF_RS_BUILD_PB=1 && cargo build
