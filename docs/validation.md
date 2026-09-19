# 初回実装の検証記録

2026-09-19、Linux ARM64（Debian 13）、Rust 1.98.1 で検証。

## 自動チェック

以下のコマンドが成功した。

```sh
mise exec -- cargo fmt --check
mise exec -- cargo clippy --locked --all-targets -- -D warnings
mise exec -- cargo test --locked
mise exec -- cargo build --release --locked
```

通常テストは 26 件成功。Desktop 実機テスト 1 件は通常実行では除外し、別途実行して成功した。

- CLI: stdin、BOM、Unicode/空白を含むパス、同一パスへの保存、検証失敗時の出力保護、デバッグ用バイパス、終了コード、一時ファイルの片付け、Unix ファイル権限。
- PNG/XML: 独立生成 fixture、旧 raw DEFLATE、zlib、二重 URL エンコード、複数ページ、未知属性・拡張要素・コメント・CDATA・処理命令の保持、CRC/画像データ/圧縮終端の検査、容量制限、重複メタデータ、画像チャンクの保持。
- Desktop 制御: 引数、PATH と明示指定の優先順位、異常終了、出力欠落、不正 PNG、タイムアウト、Unix の補助プロセス終了。

## Desktop 実機

公式 draw.io Desktop 31.4.5 の ARM64 Debian パッケージを `.tmp` に展開し、Xvfb 経由で実行した。テスト環境のラッパーでは Electron の `--no-sandbox --disable-gpu` を指定した。これらのフラグは `dip` 自体では追加しない。

```sh
DIP_TEST_DRAWIO_PATH=/absolute/path/to/drawio-wrapper \
  mise exec -- cargo test --test desktop -- --ignored --nocapture
```

- 圧縮・非圧縮ページが混在する XML から PNG を生成した。
- 2 ページ目だけ図形幅を変え、生成 PNG の画素が先頭ページ単独の描画と一致することを確認した。
- 生成 PNG を Desktop で開き直して XML にエクスポートし、両ページと 2 ページ目の変更を復元できることを確認した。
- 当初計画の `--page-index 0` は Desktop 27.0.2 以降で無効なため、`--page-index 1` に修正した。

Windows/macOS の実機確認と GitHub 上の CI 実行は、このローカル検証には含まない。CI ワークフローは 3 OS の format・clippy・通常テスト・リリースビルドを設定済み。

## 小規模入力の処理時間

リリースバイナリを子プロセスとして起動し、最初の 5 回を除外して各 50 回を測定した。stdout/stderr は破棄。XML は `mixed-pages.xml`、抽出元 PNG は `zlib.drawio.png` を使用した。メタデータ更新は一時ディレクトリ内の同じ出力先へ保存した。

| 操作 | 中央値 | p95 |
| --- | ---: | ---: |
| `dip --help` | 1.93 ms | 2.21 ms |
| `dip validate` | 2.22 ms | 2.54 ms |
| `dip extract` | 2.18 ms | 2.40 ms |
| `dip embed --no-render` | 3.38 ms | 3.82 ms |

実行ファイルは 1,878,264 bytes。測定値はこの環境の小規模 fixture に対する参考値であり、大規模図面・コールドキャッシュ・Desktop の起動や描画時間を含まない。

## 環境変数による Desktop オプション指定の追加

`DIP_DRAWIO_ARGS` 対応後、通常テスト 33 件と Desktop 実機テスト 1 件が成功した。format・clippy・リリースビルドも成功した。

- 未設定・空文字・空白、引用符・エスケープ、Unicode と空白を含む引数を確認した。
- 環境変数・ワイルドカード・コマンド置換が展開されず、追加引数が描画引数の前にそのまま渡ることを確認した。
- 引用符が不正な場合は Desktop が起動せず、既存出力が変化しないことを確認した。
- `extract`・`validate`・`embed --no-render` は不正な `DIP_DRAWIO_ARGS` も読み取らないことを確認した。
- 実機テストでは Xvfb の下でテストを実行し、`DIP_TEST_DRAWIO_PATH` に Desktop 本体を指定した。`DIP_DRAWIO_ARGS` から `--no-sandbox --disable-gpu --disable-dev-shm-usage` と空白を含む `--user-data-dir` を渡し、PNG の描画と Desktop での再読み込みが成功した。
