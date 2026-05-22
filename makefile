.PHONY: plugin plugins
.ONESHELL:

# Build an arbitrary plugin: `PLUGIN=plugin_name make plugin`
plugin:
	cargo build --target wasm32-wasip1 -p $(PLUGIN) --profile release-wasm

# Build all plugins (automatically discovers plugins in plugins/ directory)
plugins:
	@for dir in plugins/*/; do
		if [ -f "$$dir/Cargo.toml" ]; then
			plugin_name=$$(basename $$dir);
			echo "Building plugin: $$plugin_name";
			cargo build --target wasm32-wasip1 -p $$plugin_name;
			wasm-tools demangle \
				target/wasm32-wasip1/debug/$$plugin_name.wasm \
				-o target/wasm32-wasip1/debug/$$plugin_name.wasm;
		fi
	done

plugins-release:
	@for dir in plugins/*/; do
		if [ -f "$$dir/Cargo.toml" ]; then
			plugin_name=$$(basename $$dir)
			echo "Building plugin: $$plugin_name"
			cargo build --target wasm32-wasip1 -p $$plugin_name  --profile release-wasm
			wasm-opt -O3 \
				--debuginfo \
				--enable-bulk-memory-opt \
				--enable-nontrapping-float-to-int \
				target/wasm32-wasip1/release-wasm/$$plugin_name.wasm \
				-o target/wasm32-wasip1/release-wasm/$$plugin_name.wasm
			wasm-tools demangle \
				target/wasm32-wasip1/release-wasm/$$plugin_name.wasm \
				-o target/wasm32-wasip1/release-wasm/$$plugin_name.wasm; \
			cp target/wasm32-wasip1/release-wasm/$$plugin_name.wasm frontend/public/plugins/$$plugin_name.wasm
			mkdir -p tui-plugins
			cp target/wasm32-wasip1/release-wasm/$$plugin_name.wasm tui-plugins/$$plugin_name.wasm
		fi
	done

fmt:
	cargo fmt

lint:
	cargo clippy --workspace --all-targets -- -D warnings

wrangler-dev:
	cd frontend
	tailwindcss -i ./input.css -o ./assets/tailwind.css
	dx bundle --web --release
	
	cd ../cf-pages
	wrangler pages dev

wrangler-deploy:
	cd frontend
	tailwindcss -i ./input.css -o ./assets/tailwind.css
	dx bundle --web --release
	
	cd ../cf-pages
	wrangler pages deploy
