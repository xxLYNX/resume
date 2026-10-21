# jobdesk dev shortcuts (see README.md / JOBDESK.md)
#
# Optional auto-restart on Rust changes (cargo install cargo-watch):
#   cargo watch -x 'run -p jobdesk -- serve --port $(PORT)'
.PHONY: serve serve-open import test test-render build build-pdf check

PORT ?= 3847

serve:
	cargo run -p jobdesk -- serve --port $(PORT)

serve-open:
	cargo run -p jobdesk -- serve --open --port $(PORT)

import:
	cargo run -p jobdesk -- import

test:
	cargo test --workspace

test-render:
	cargo test -p render_tex

check:
	cargo check --workspace

build:
	cargo run -p jobdesk -- build

build-pdf:
	cargo run -p jobdesk -- build --pdf

build-ats:
	cargo run -p jobdesk -- build --theme ats-classic --pdf
