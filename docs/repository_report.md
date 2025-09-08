# icfpc2025 リポジトリレポート（運営提出用）

本ドキュメントは、Team Unagi の ICFPC 2025 リポジトリ（icfpc-unagi/icfpc2025）に含まれるソルバ群と周辺ツール・インフラの概要を、運営向けに整理したものです。各バイナリの機能、入出力、関連モジュール、実行補助やインフラ構成を俯瞰します。

## 1. 概要
- 目的: 競技用ソルバ群の実装と実行基盤（GCP、DB、Web、可視化、Docker、CI 等）を一体で運用
- 主言語: Rust（ワークスペース単一クレート）
- ドキュメント/運用ガイド: <ref_file file="/home/ubuntu/repos/icfpc2025/AGENTS.md" />
- ビルド/テスト/リント: <ref_file file="/home/ubuntu/repos/icfpc2025/Makefile" />

主要ディレクトリ
- ソルバ/ユーティリティ（実行バイナリ）: src/bin/*.rs
- GCP クライアント: src/gcp/**
- Web バックエンド: src/www/**
- DB レイヤ: <ref_file file="/home/ubuntu/repos/icfpc2025/src/sql.rs" />
- 実行基盤（Executor）: src/executor/**
- 共有ユーティリティ: <ref_file file="/home/ubuntu/repos/icfpc2025/src/lib.rs" />
- その他: Dockerfile 群（docker/）、スクリプト（scripts/）、Secrets（configs/*.encrypted, secrets/）

## 2. ソルバ・バイナリ一覧（src/bin）
以下は src/bin 配下に存在する主なバイナリの分類と役割です。アルゴリズムの詳細はファイル内ロジックに依存しますが、運営向けには入出力と周辺機能の関連を記載します。

### 2.1 SAT 系
- chokudai 系
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/chokudai_sat.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/chokudai_sat_d3.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/chokudai_sat_d3k2.rs" />
  - 入力: 問題表現（内部モジュール/ファイルから取得）／パラメータはバイナリ内定義
  - 出力: 解または最良解スコア（標準出力/ファイル）
  - 備考: d3, d3k2 は探索/局所改善のバリエーション
- wata 系
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/wata_sat.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/wata_sat2.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/wata_sat3.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/wata_sat4.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/wata_sat4_parallel.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/wata_sat5.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/wata_sat5_parallel.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/wata_sat6.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/wata_sat6a.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/wata_sat6_parallel.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/wata_sat72.rs" />
  - 入力/出力: 上記と同様。parallel 付きは内部で並列実行
- iwiwi 系 SAT
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/iwiwi_sat.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/iwiwi_sat_z3.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/iwiwi_chokudai_sat.rs" />
  - 役割: SAT ベースの解法。z3 は外部 SAT/SMT ソルバ連携のバリアント

### 2.2 ルーティング/グラフ生成・評価
- ルーティング
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/iwiwi_routing.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/iwiwi_routing_v2.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/iwiwi_routing_v3.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/iwiwi_routing_hc.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/iwiwi_routing_6n.rs" />
  - 例: 評価ロジック断片 <ref_snippet file="/home/ubuntu/repos/icfpc2025/src/bin/iwiwi_routing_v2.rs" lines="33-41" />
- グラフ/マップ
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/reduce_graph1.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/generate_map.rs" />

### 2.3 探索ポートフォリオ・バッチ実行
- ノーマーク系一括実行・評価
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/run_solve_no_marks.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/run_solve_no_marks_parallel.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/run_solve_no_marks_portfolio.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/run_solve_no_marks_dimacs.rs" />
  - 特徴: タスク分割・並列実行・複数戦略のポートフォリオ評価
- 進化的/強化的探索
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/iwiwi_evo_gen276.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/iwiwi_evo_gen276_clean.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/iwiwi_evo_gen276_kissat.rs" />
  - 役割: 世代ベースの改良や外部 SAT（kissat）統合

### 2.4 実行・評価・ユーティリティ
- 実行器/評価
  - 単体バイナリ: <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/executor.rs" />
  - モジュール: <ref_file file="/home/ubuntu/repos/icfpc2025/src/executor/run.rs" />、<ref_file file="/home/ubuntu/repos/icfpc2025/src/executor/mod.rs" />
- テスト/検証ツール
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/tester.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/api_test.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/http_test.rs" />
- 運用補助
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/fetch_problems.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/migrate_scores.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/post.rs" />
  - ロック制御: <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/lock.rs" /> / <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/unlock.rs" />
- Web エントリ
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/www.rs" />

### 2.5 実験/派生
- chokudai1.rs, chokudai2.rs, chokudai3.rs, chokudai_full1.rs, tos1.rs, tos4.rs, wata.rs, gacha.rs など
  - 役割: 戦略検証、パラメタ検討、派生アルゴリズムの試作

## 3. 実行とオーケストレーション
- ランチャ: プロジェクトルートの ./run（存在するバイナリを優先実行。なければビルドして起動）
- 並列実行/分割: run_solve_no_marks_parallel.rs でスレッド分割し、複数タスクを同時実行
  - 該当箇所例: <ref_snippet file="/home/ubuntu/repos/icfpc2025/src/bin/run_solve_no_marks_parallel.rs" lines="142-151" />
- Executor モジュール: 複数タスクの一括実行や評価ループを担う（src/executor/**）

## 4. 周辺システム
### 4.1 GCP 統合
- CLI バイナリ: <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/gcp/main.rs" />
  - サブコマンド: <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/gcp/commands/instances.rs" />、<ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/gcp/commands/run.rs" />、<ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/gcp/commands/ls.rs" />、<ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/gcp/commands/cat.rs" />
- 認証/型
  - 認証: <ref_file file="/home/ubuntu/repos/icfpc2025/src/gcp/auth.rs" />
  - 共通型: <ref_file file="/home/ubuntu/repos/icfpc2025/src/gcp/types.rs" />
- GCE
  - クライアント: <ref_file file="/home/ubuntu/repos/icfpc2025/src/gcp/gce/client.rs" />
  - 型/デフォルト: <ref_file file="/home/ubuntu/repos/icfpc2025/src/gcp/gce/types.rs" />、<ref_file file="/home/ubuntu/repos/icfpc2025/src/gcp/gce/defaults.rs" />
  - 重要記号: InstanceRequest（types.rs）、create_instance_request（defaults.rs）
- GCS
  - クライアント: <ref_file file="/home/ubuntu/repos/icfpc2025/src/gcp/gcs/client.rs" />
  - 型: <ref_file file="/home/ubuntu/repos/icfpc2025/src/gcp/gcs/types.rs" />
  - 重要関数: list_dir, list_dir_detailed, get_object_metadata（client.rs）

注意
- 実体作成（gcp run 等）は課金・権限が必要。運営提出用レポート作成にあたっては実行不要

### 4.2 DB レイヤ
- <ref_file file="/home/ubuntu/repos/icfpc2025/src/sql.rs" />
  - MySQL 接続プール CLIENT、Row ラッパ、select/row/cell/exec/insert 等の抽象

### 4.3 Web
- エントリ: <ref_file file="/home/ubuntu/repos/icfpc2025/src/bin/www.rs" />
- ハンドラ: <ref_file file="/home/ubuntu/repos/icfpc2025/src/www/handlers/mod.rs" />
  - 個別: leaderboard.rs、tasks.rs、task.rs、api.rs、cron.rs、unlock.rs、template.rs
  - テンプレートレンダリング: render（template.rs）
- ユーティリティ: <ref_file file="/home/ubuntu/repos/icfpc2025/src/www/utils.rs" />
  - maybe_enrich_datetime_str 等

### 4.4 可視化（WASM）
- vis/ 配下（別クレート構成）。ブラウザで問題/解の可視化
  - 参照: vis/index.html、vis/src/lib.rs、vis/run.sh（本レポにはパスのみ記載）

### 4.5 スクリプト・Docker・Secrets
- スクリプト: scripts/（デプロイ、実行補助）
- Docker: docker/ 下に server/builder/runner/tools の各 Dockerfile と Makefile ラッパ
- Secrets:
  - 暗号化ファイル: configs/*.encrypted
  - 復号出力: secrets/（gitignore 対象）
  - 復号/暗号化ラッパ: bin/decrypt / bin/encrypt、Makefile の secrets ルール

## 5. ビルド/CI/品質
- ビルド: cargo build
- テスト
  - 通常: make test（Rust の test + バイナリビルド）
  - UNAGI 依存（外部接続/長時間）: make test/unagi
- リント/整形
  - make lint（clippy -D warnings ＋ fmt チェック）
  - make format（cargo fmt）
- CI: .github/workflows/ による自動テスト/リント（詳細はワークフロー参照）

## 6. 代表的シンボル/重要ファイル
- 共有ユーティリティ: <ref_file file="/home/ubuntu/repos/icfpc2025/src/lib.rs" />
  - 例: mat!、SetMinMax、get_bearer などのヘルパ
- 問題/判定
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/problems.rs" />
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/judge.rs" />
- ネットワーククライアント（API/HTTP）
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/client.rs" />
- ロック制御
  - <ref_file file="/home/ubuntu/repos/icfpc2025/src/lock_guard.rs" />

## 7. 実行例（安全な範囲）
- ビルド/基本
  - cargo build
  - make lint
  - make test
- GCP CLI（ドライラン参照用。実行時は課金/権限に注意）
  - ./run gcp ls gs://icfpc2025/ -l
  - ./run gcp instances --zone=asia-northeast1-b
  - ./run gcp run --zone=asia-northeast1-b --machine-type=c2d-standard-4 test-vm 'echo hello'（注意: 実体作成）
- ソルバ（例: 並列一括）
  - ./run run_solve_no_marks_parallel …（入力/オプションは各バイナリのヘルプ/コード参照）

## 8. セキュリティ・運用
- 環境変数 UNAGI_PASSWORD を通じて GCS からサービスアカウントを取得し、OAuth2 トークンを用いた GCP 操作を実施
- 平文の鍵/トークンは commit しない（暗号化済みのみ）
- GCP 実体作成/削除は課金/クォータに影響、ゾーン/マシンタイプ/権限を確認
- 参考: <ref_file file="/home/ubuntu/repos/icfpc2025/AGENTS.md" />

## 9. 備考
- 本レポートは運営向けの俯瞰資料であり、各ソルバの詳細アルゴリズムはコード参照
- ソルバ/周辺ツールの新規追加・更新時は本レポートも追随が望ましい
