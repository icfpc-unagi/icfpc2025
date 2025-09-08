# icfpc2025 リポジトリレポート（運営提出用・簡潔版）

本ドキュメントは、Team Unagi の ICFPC 2025 リポジトリに含まれるソルバ群と周辺基盤の「全体像」を短く整理したものです。実行は不要で、構成理解に特化しています。

## 1. 概要（構成と目的）
- 目的: ソルバ実装と実行基盤（GCP、DB、Web、可視化、Docker、CI）を一体運用
- 主言語: Rust（単一ワークスペース）
- 主要ディレクトリ
  - ソルバ/ユーティリティ: src/bin/*.rs
  - 基盤: src/gcp/**（GCE/GCS クライアント）、src/www/**（Web）、src/sql.rs（DB）、src/executor/**（実行器）
- 参考: AGENTS.md、Makefile

## 2. 本質的な分類（src/bin）
- SAT 系
  - chokudai_* / wata_* / iwiwi_* など。内部表現から SAT/局所探索で解を構築。並列版は *_parallel。
- ルーティング/グラフ
  - iwiwi_routing_*、reduce_graph1.rs、generate_map.rs など。経路選択・縮約・マップ生成。
- ポートフォリオ/バッチ実行
  - run_solve_no_marks*.rs 系。タスク分割・並列実行・複数戦略の評価。
- 実行・評価・運用
  - executor.rs と src/executor/**、tester.rs、api_test.rs、http_test.rs、lock/unlock など。
- Web/その他
  - www.rs（Web エントリ）、運用補助スクリプト類、実験的バイナリ群。

## 3. 代表的ソリューションの例
- iwiwi_evo_gen276.rs
  - 方針: 進化的探索。世代的に候補解を改良し、必要に応じ SAT（例: kissat 連携バリアント有）で局所改善。
  - 役割: 大域探索の強化と多様性維持により強い初期解/改良解を得る。
  - I/O: 内部の問題ローダを介して読み込み、標準出力やファイルで解/スコアを出力。
- wata_sat3.rs
  - 方針: SAT ベースの探索（第三世代設定）。節の学習・分枝戦略調整により堅実な解探索を行う。
  - 役割: 安定したベースライン。後続版比較の土台。
- wata_sat6.rs（および *_parallel）
  - 方針: 改良版 SAT 探索。ヒューリスティクスと並列化で探索幅/深さを強化。
  - 役割: 実運用の主力候補。ポートフォリオ内の重要メンバ。

注: いずれも「問題の内部表現→探索→解の検証/スコア化」の共通パイプラインで動作。細部のパラメータは各バイナリ内定義。

## 4. 実行とオーケストレーション
- ランチャ: ./run（既存ビルド物があれば実行、なければビルド→実行）
- 並列/分割: run_solve_no_marks_parallel.rs でスレッド分割・同時実行
- 実行器: src/executor/** が一括実行や評価ループを担当

## 5. 周辺基盤（要点）
- GCP
  - 認証: src/gcp/auth.rs（UNAGI_PASSWORD 経由で SA を取得→トークン化）
  - GCS/GCE: src/gcp/gcs/**, src/gcp/gce/**（list/get、インスタンス作成等の最小呼び出し）
  - CLI: src/bin/gcp/**（instances/run/ls/cat など）
- DB: src/sql.rs（接続・行ラッパ・クエリ補助）
- Web: src/bin/www.rs、src/www/**（handlers, utils）
- 可視化: vis/（別クレート、ブラウザ可視化）
- Docker/Secrets: docker/、configs/*.encrypted と secrets/、bin/encrypt / bin/decrypt

## 6. 実行例（安全な範囲）
- ビルド/検査: cargo build、make test、make lint
- GCP CLI（参考。実体作成は課金に注意）:
  - ./run gcp ls gs://icfpc2025/ -l
  - ./run gcp instances --zone=asia-northeast1-b
  - ./run gcp run --zone=asia-northeast1-b --machine-type=c2d-standard-4 test-vm 'echo hello'（注意）
- ソルバ（例・並列）: ./run run_solve_no_marks_parallel …

## 7. セキュリティ・運用
- UNAGI_PASSWORD をログや成果物に含めない
- GCP 実体操作は課金/権限/クォータに注意
- 秘密情報は暗号化ファイルのみをリポジトリに保持

（以上）- 本レポートは運営向けの俯瞰資料であり、各ソルバの詳細アルゴリズムはコード参照
- ソルバ/周辺ツールの新規追加・更新時は本レポートも追随が望ましい
