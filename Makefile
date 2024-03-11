# Copyright (c) 2024 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

build:
	cargo build --package 'gsark' --release --workspace

clean:
	cargo clean

init: install-dependencies

install: init build
	@sudo ./scripts/install_library.sh

install-dependencies:
	@sudo ./scripts/install_dependencies.sh

test:
	cargo test --all --release --workspace

uninstall:
	@./scripts/uninstall_library.sh
