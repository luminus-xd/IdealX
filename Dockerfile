# Rustビルド用のステージ（最新バージョンを使用）
FROM rust:1.82 as builder

WORKDIR /app

# 依存関係のキャッシュを利用するために、Cargo.tomlを先にコピー
COPY Cargo.toml ./

# ダミーのmain.rsを作成して依存関係をビルド
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm src/main.rs

# 実際のソースコードをコピー
COPY src ./src

# アプリケーションをビルド
RUN cargo build --release

# 実行用の軽量なイメージ
FROM debian:bookworm-slim

# SSL証明書とランタイム依存関係をインストール
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# ビルドしたバイナリをコピー
COPY --from=builder /app/target/release/ideal-x /usr/local/bin/ideal-x

# 実行権限を付与
RUN chmod +x /usr/local/bin/ideal-x

# アプリケーションを実行
CMD ["ideal-x"]