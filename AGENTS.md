# AGENTS.md

本ドキュメントは、このリポジトリ（icfpc2025）で開発支援エージェント（AI/自動化ツール/スクリプト）や開発者が作業するときに必要となる情報を、日本語で整理したものです。レポジトリ構成、セットアップ、ビルド/実行、GCP 認証・操作、コーディング規約、変更手順、セキュリティ方針などを網羅的にまとめています。

## 目的と前提

- 目的:
  - Rust 製のツール群（Web、DB、GCP 操作用 CLI 等）を一つのワークスペースで開発・実行・デプロイする。
  - GCP の認証・操作（Compute Engine/Cloud Storage）を安全に・再現可能に行う。
  - 自動化やエージェントが介入しても破壊的変更やリークを避け、最小の差分で保守可能にする。
- 前提:
  - Rust/Cargo が利用可能であること。
  - GCP へのアクセスはプロジェクトの運用ルールに従う（課金や権限に注意）。
  - 環境変数 `UNAGI_PASSWORD` が適切に設定されていること（GCS 上のシークレット参照に使用）。

## リポジトリ構成（主要）

```
src/
  bin/
    gcp/
      main.rs                # 統合 CLI エントリ（`./run gcp ...`）
      common.rs              # CLI 内で使う共通ユーティリティ（表表示など）
      commands/
        instances.rs         # `gcp instances` 実装（GCE インスタンス一覧）
        run.rs               # `gcp run` 実装（GCE インスタンス作成）
        ls.rs                # `gcp ls` 実装（GCS の ls 風表示）
    hello.rs / list_tables.rs / www.rs など
  gcp/
    mod.rs                  # gcp 名前空間（auth/gcs/gce/types）
    auth.rs                 # 共通の GCP 認証（サービスアカウント→アクセストークン）
    types.rs                # ServiceAccount / AccessToken（サービス非依存の型）
    gcs/
      mod.rs
      client.rs             # GCS JSON API 呼び出し（list 等）
      types.rs              # GCS API レスポンス型
    gce/
      mod.rs
      client.rs             # GCE API 呼び出し（インスタンス作成）
      defaults.rs           # GCE インスタンス要求のデフォルト/生成
static/ configs/ docker/ scripts/ secrets/ など
Makefile
run                         # 汎用ランチャ（`./run <bin> [args...]`）
gcs_ls                      # 互換ラッパ（必要に応じて）
```

## ビルド・実行

- 共通コマンド
  - ビルド: `cargo build`
  - テスト: `make test`（UNAGI を要するテストは除外）
  - テスト(UNAGI 依存): `make test/unagi`（`UNAGI_PASSWORD` が必要なテストを実行）
  - Lint: `make lint`（clippy 警告をエラー化＋fmt チェック）
  - フォーマット: `make format`
- 実行（推奨ランチャ）
  - `./run <bin> [args...]`
    - 例: `./run gcp ls gs://icfpc2025/ -l`
    - 例: `./run gcp instances --zone=asia-northeast1-b`
    - 例: `./run gcp run --zone=asia-northeast1-b --machine-type=c2d-standard-4 my-vm 'echo hello'`
  - 仕様: 既存のコンパイル済みバイナリ（target 配下）を優先し、なければビルド→実行。
  - PWD は変更せずに動作。`CARGO_TARGET_DIR` が相対指定でもスクリプトの位置から解決。

## 環境変数・シークレット

- `UNAGI_PASSWORD`: 必須
  - 値に基づき、GCS から以下をダウンロードして使用:
    - `https://storage.googleapis.com/icfpc2025-data/$UNAGI_PASSWORD/service_account.json`
  - `src/gcp/auth.rs` で JWT を作成し、OAuth2 トークンを取得。
- `secrets/` ディレクトリ
  - `configs/*.encrypted` と `bin/encrypt` / `bin/decrypt` により、暗号化済みシークレットを管理。
  - `make secrets` により復号（Docker ツールイメージ経由のルールもあり）。
  - 秘密情報はコミットしないこと（暗号化ファイルのみコミット）。

## GCP モジュール方針

- 共通認証（サービス非依存）
  - `src/gcp/types.rs`: `ServiceAccount`, `AccessToken` を定義。
  - `src/gcp/auth.rs`: これらの型に依存してトークンを取得（GCS から SA JSON を取得→JWT→トークン）。
- GCS（Cloud Storage）
  - `client.rs` に API 呼び出し（list, detailed list, metadata）を集約。
  - `list_dir` と `list_dir_detailed` は内部のジェネリック関数で重複排除（クロージャ受け取り）。
  - Path 例: `gcp::gcs::{parse_gs_url, list_dir, list_dir_detailed, get_object_metadata}`。
- GCE（Compute Engine）
  - `gce::client::create_instance` と `gce::defaults::{create_default_instance_request, create_instance_request}` を提供。
  - `create_instance_request` はゾーン/マシンタイプ/起動スクリプトを受け取り、完全修飾の API パスを組み立て。
  - デフォルトは Spot/自動再起動なし/外部IPあり 等。必要に応じて編集。

## CLI（`src/bin/gcp`）

- `gcp instances`
  - ゾーン内のインスタンスを取得し、表形式（Status/Name/MachineType/Zone/ExternalIP）で出力。
- `gcp run`
  - インスタンス作成（`--zone`, `--project`, `--machine-type`）＋任意の起動コマンド（metadata `startup-script`）。
  - 本当に作成されるため、課金・割り当て・リージョン/ゾーンに注意。
- `gcp ls`
  - `gs://bucket[/prefix]` の1階層表示、`-l` でサイズ・更新時刻、`-R` で再帰表示。
  - `gs://bucket/object` の場合、単一オブジェクトの詳細表示（Content-Type, Storage-Class, Generation 等）。

## コーディング規約・方針

- Rust スタイル
  - `clippy -D warnings` 準拠。`make lint` で自動検査。
  - `cargo fmt` に従う。
  - 不要な import/可変/空 println 等は避ける（clippy に従う）。
- 変更の粒度
  - 原因箇所の修正を優先し、無関係な大規模変更は避ける。
  - 既存コードのスタイルに合わせ、命名やファイル配置の一貫性を重視。
- 設計
  - サービス非依存の型は `gcp::types` へ、認証は `gcp::auth` へ集約。
  - サービス固有コード（GCS/GCE）は `gcp::gcs` / `gcp::gce` 配下に限定。
  - CLI は `src/bin/gcp/` にまとめ、サブコマンドは `commands/` に分割。

## 開発フロー（推奨）

1. 課題の把握と影響範囲の特定（API 互換性、実行時副作用、課金等）。
2. 小さな計画（箇条書き）→ 実装。必要ならドキュメント/ヘルパ更新。
3. `cargo build` → `make lint` で検証。
4. 必要なら最小限の実行確認（GCP へ書き込みが伴う操作は要注意）。
5. レビュー/コミット（コミットポリシーはチーム慣例に従う）。

## セキュリティ・運用上の注意

- シークレットの取り扱い
  - 平文の鍵/トークンはコミットしない。暗号化済みファイルと `secrets/` を使う。
  - `UNAGI_PASSWORD` をログや出力に含めない。
- GCP 操作
  - インスタンス作成/削除は課金・クォータ・SLA へ影響。ゾーン・マシンタイプ・ネットワーク設定を確認。
  - Spot/プリエンプティブ設定や自動再起動の有無を要件に応じて変更。
  - 権限不足/403/404 の場合はサービスアカウント/スコープを確認。
- ネットワーク・レート
  - API の呼びすぎを避ける（ページネーションに対応済み）。
  - 必要に応じて `fields` パラメータで応答を絞る。

## よくあるタスク

- gcp CLI にコマンドを追加する
  1. `src/bin/gcp/commands/` に `<cmd>.rs` を追加。
  2. `src/bin/gcp/commands/mod.rs` に `pub mod <cmd>;` を追加。
  3. `src/bin/gcp/main.rs` の `Commands` enum と `match` に分岐を追加。
  4. `make lint` で検証。

- GCS 表示の列を増やす
  - `src/bin/gcp/commands/ls.rs` の表示部分を拡張。
  - 追加のメタデータが必要なら `gcp::gcs::types` を拡張し、`client.rs` のフィールド選択/マッピングを調整。

- GCE インスタンス作成のデフォルト変更
  - `src/gcp/gce/defaults.rs` の `create_instance_request` を編集。
  - ディスク/ネットワーク/スケジューリング等の既定をプロジェクト要件に合わせる。

## トラブルシューティング

- `UNAGI_PASSWORD not set`
  - 環境変数を設定してください（`export UNAGI_PASSWORD=...`）。
- `Failed to download service_account.json`
  - GCS 上のパス/権限を確認。ネットワーク制限やプロキシも影響し得ます。
- `403 Forbidden` / `404 Not Found`（GCP API）
  - サービスアカウントの権限/ロール、対象プロジェクト/ゾーン/リソースの存在を確認。
- clippy エラー
  - 提示どおりに修正（`get(0)` → `first()` など）。抑制は最終手段。

## ポリシー（エージェント向け）

- 変更は最小差分・関連箇所のみに限定、周辺の無関係な修正は避ける。
- 既存の命名・配置・パターンを尊重し、統一感のあるコードにする。
- セキュア・プライバシー保護を最優先（シークレット露出禁止、破壊的操作の明示）。
- ドキュメント（本ファイル等）は変更に追随させる。
- 可能なら実行前に影響範囲・課金リスクを説明し、明示的な許可を得る。

---

不明点や追加したい開発ルールがあれば、課題（Issue）や PR の説明に追記してください。チーム全体で改善していきましょう。
