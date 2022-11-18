run:
	cargo run

fmt:
	cargo fmt

download:
	wget https://github.com/ringsaturn/tzf-rel/raw/main/combined-with-oceans.reduce.pb -O src/data/combined-with-oceans.reduce.pb
