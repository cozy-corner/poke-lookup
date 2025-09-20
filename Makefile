.PHONY: help check check-all lint format build install install-force test setup clean-build test-integration benchmark profile

help:
	@echo "開発用コマンド:"
	@echo "  make check           - 型チェック"
	@echo "  make check-all       - 型チェック（全features）"
	@echo "  make lint            - Lintチェック"
	@echo "  make format          - フォーマットチェック"
	@echo "  make build           - リリースビルド"
	@echo "  make test            - テスト実行"
	@echo "  make install         - ローカルインストール"
	@echo "  make install-force   - 強制再インストール"
	@echo "  make test-integration - 統合テスト実行"
	@echo ""
	@echo "メンテナンス:"
	@echo "  make clean-build     - ビルド成果物削除"
	@echo "  make setup           - 開発環境セットアップ"
	@echo "  make benchmark       - ベンチマーク実行"
	@echo "  make profile         - プロファイリングビルド"
	@echo ""
	@echo "cargo aliasも利用可能: cargo ba, cargo ta, cargo lw など"

check:
	cargo c

check-all:
	cargo ca

lint:
	cargo lw

format:
	cargo fc

build:
	cargo br

test:
	cargo ta

install:
	cargo ia

install-force:
	cargo if

setup:
	@command -v cargo >/dev/null 2>&1 || { echo "❌ Rustをインストールしてください"; exit 1; }
	@echo "✅ Rust環境OK"
	cargo fetch
	@echo "✅ 依存関係取得完了"

clean-build:
	cargo clean
	@echo "✅ ビルド成果物削除完了"

test-integration:
	cargo ta
	cargo br
	./target/release/poke-lookup フシギダネ
	./target/release/poke-lookup ピカチュウ
	@echo "✅ 統合テスト完了"

benchmark:
	cargo bench --all-features

profile:
	cargo build --profile=release-with-debug --all-features
	@echo "✅ プロファイリング用ビルド完了"
