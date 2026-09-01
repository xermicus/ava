.PHONY: book clippy doc fmt install serve test-book


book: test-book
	mdbook serve book --open

clippy:
	cargo clippy --all-targets -- -D warnings

doc:
	cargo doc --no-deps --workspace

fmt:
	cargo fmt --all

install:
	cargo install --path crates/ava --locked --force

serve: install
	ava image
	ava serve

test-book:
	cargo install mdbook --version 0.5.4 --locked
	mdbook test book
