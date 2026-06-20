.PHONY: build buildgo buildrust test clean install help

build: buildgo buildrust
	@echo "[+] Build complete"
	@echo "[+] Go collector: ./bin/collector"
	@echo "[+] Rust CLI: ./bin/k8sync"

buildgo:
	@echo "[~] Building Go collector"
	@mkdir -p bin
	@go build -o bin/collector cmd/collector/main.go
	@echo "[+] Go collector built"

buildrust:
	@echo "[~] Building Rust CLI"
	@cargo build --release
	@mkdir -p bin
	@cp target/release/k8sync bin/
	@echo "[+] Rust CLI built"

dev: devgo devrust
	@echo "[+] Dev build complete"

devgo:
	@mkdir -p bin
	@go build -o bin/collector cmd/collector/main.go

devrust:
	@cargo build
	@mkdir -p bin
	@cp target/debug/k8sync bin/

test: testgo testrust
	@echo "[+] All tests passed"

testgo:
	@echo "[~] Running Go tests"
	@go test -v ./...

testrust:
	@echo "[~] Running Rust tests"
	@cargo test

lint: lintgo lintrust

lintgo:
	@echo "[~] Linting Go code"
	@gofmt -l -w .
	@go vet ./...

lintrust:
	@echo "[~] Linting Rust code"
	@cargo fmt
	@cargo clippy

clean:
	@echo "[~] Cleaning build artifacts"
	@rm -rf bin/
	@rm -rf target/
	@go clean
	@echo "[+] Clean complete"

install: build
	@echo "[~] Installing k8sync"
	@cp bin/k8sync /usr/local/bin/
	@cp bin/collector /usr/local/bin/k8sync-collector
	@echo "[+] Installed to /usr/local/bin/"

help:
	@echo "k8sync - Kubernetes multi-cluster drift detector"
	@echo ""
	@echo "Available targets:"
	@echo "  make build      - Build both Go and Rust components (optimized)"
	@echo "  make dev        - Build both components (debug, faster)"
	@echo "  make buildgo    - Build only Go collector"
	@echo "  make buildrust  - Build only Rust CLI"
	@echo "  make test       - Run all tests"
	@echo "  make lint       - Lint and format code"
	@echo "  make clean      - Remove build artifacts"
	@echo "  make install    - Install to /usr/local/bin"
	@echo "  make help       - Show this help"
