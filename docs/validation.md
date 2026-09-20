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

## Chromium 描画対応

2026-09-19、Linux ARM64、Rust 1.98.1、`/usr/bin/chromium`
（Chromium 153.0.8010.47）で検証した。実機テストでは追加引数やXvfbを使用せず、sandboxを有効にした状態で成功した。

- 通常テスト41件が成功。Chromium実機テスト4件も明示実行して成功した。
- 先頭ページの画素一致、全ページXMLの保持、日本語・HTMLラベル、埋め込み画像を確認した。
- 同梱資材とローカル資材の描画、未知の図形・数式・資材欠落・壊れた埋め込み画像・過大画像の拒否を確認した。
- 外部画像は既定で接続せず失敗し、明示許可時は取得して埋め込み画像と同じ画素になることを確認した。
- ブラウザー起動前の設定エラー、追加引数の引用符とシェル非展開、既存出力の保護、起動待機中とJavaScript実行中のタイムアウト、Unix補助プロセス・プロファイルの片付けを検証した。
- 上流Webアプリの `src/main/webapp` を直接指定する手動描画も成功した。
- releaseバイナリをリポジトリ外の一時ディレクトリへコピーし、DesktopからChromiumへの自動フォールバック、同梱資材による描画、2ページの抽出・再検証が成功した。
- format、Linuxのclippy、releaseビルド、資材ハッシュ確認が成功。Windows GNUターゲットの全ターゲットcheck・clippyも成功した。
- `cargo package --allow-dirty --offline` による配布物の検証ビルドが成功し、`.tmp` の参考チェックアウトに依存しないことを確認した。最初のワークスペース内ビルドではオブジェクトファイル作成時にPermission deniedが発生したため、`CARGO_TARGET_DIR=/tmp/dip-package-verification-target` で独立したビルド先を使って検証した。

```sh
DIP_TEST_CHROME_PATH=/usr/bin/chromium \
  mise exec -- cargo test --locked --test chromium -- --ignored --nocapture
python3 scripts/vendor_drawio.py --check
```

Desktopの既存通常テストは成功したが、今回の変更でDesktop実機テストは再実行していない。macOS/Windowsでのブラウザー実機検証とGitHub上のCI実行は、このローカル検証には含まない。

## DesktopとChromiumの出力サイズ計算の統一

2026-09-20、Chromiumの画像サイズをDesktopと同じ `ceil(bounds.size + bounds.offset) + 1` に変更した。追加後の寸法に対して画像バッファの64 MiB制限を適用する。

- 文字なしの100×40の矩形について、Desktop 31.4.5の出力寸法104×44をChromium実機テストに追加した。変更前は103×43で失敗し、変更後は成功した。
- 通常テスト41件、Chromium実機テスト4件、format・clippyが成功した。実機テストは `/usr/bin/chromium`、`DIP_TEST_CHROME_ARGS=--disable-dev-shm-usage`、`--test-threads=1` で実行した。
- 比較用画像を再描画し、文字なし図形104×44とHTMLラベル244×84が、既存のDesktop出力と同じ寸法になることを確認した。4例とも抽出XMLは一致した。
- 日本語と英数字・記号を含むプレーンテキストの例は、Desktop 131×44に対しChromium 149×44となる。実際に選択されるフォントが異なるため、寸法計算の統一だけでは文字を含むすべての図面のサイズ一致を保証しない。
- DPR・縮小方式は変更していないため、寸法が一致しても画素の完全一致は保証しない。

## 既定フォント・代替フォントの指定

2026-09-20、`--default-font` と `--fallback-font` を追加した。仕様は [fonts.md](fonts.md) に記載している。

- 通常テスト48件が成功。複数ページとユーザーオブジェクト、明示フォントの優先、代替候補の順序・重複除去、HTML・Webフォント・未知要素の保持、XMLサイズ制限、保存失敗時の保護を確認した。
- Desktopに渡すXMLとPNGに保存するXMLが一致し、全ページに設定が残ることを確認した。
- Desktop 31.4.5とChromiumを使うフォント実機テストが成功した。共通フォントはNoto Sans CJK JPを指定。各レンダラー内で、既定フォントによる描画、存在しないフォントからの明示的な代替、既存指定を優先した描画が、XMLへフォントを直接指定した基準画像と同じ画素になった。
- 既存のChromium実機テスト4件とDesktop実機テスト1件も成功した。
- format・clippyが成功した。実機検証はLinuxで、Desktopは既存のXvfbラッパーを使用。両レンダラーに `--disable-dev-shm-usage` を追加して順次実行した。
- フォント候補の統一は描画方式の統一ではない。今回の日本語ラベルでは、同じNoto Sans CJK JPを候補に指定してもDesktop 131×44、Chromium 132×44の1px差が残る。macOS/Windowsの実フォント選択は今回検証していない。

```sh
DIP_TEST_DRAWIO_PATH=/absolute/path/to/drawio-wrapper \
  DIP_DRAWIO_ARGS=--disable-dev-shm-usage \
  DIP_TEST_CHROME_PATH=/usr/bin/chromium \
  DIP_TEST_CHROME_ARGS=--disable-dev-shm-usage \
  DIP_TEST_FONT='Noto Sans CJK JP' \
  mise exec -- cargo test --locked --test desktop --test fonts --test chromium \
  -- --ignored --nocapture --test-threads=1
```

## 描画解像度と縮小処理の統一

2026-09-20、ChromiumをDPR 2で描画し、Hamming1方式で縦横それぞれ半分へ縮小するよう変更した。比較対象はDesktop 31.4.5 / Electron 44.2.0 / Chromium 152と、CLI側のChromium 153.0.8010.47。Linux ARM64、1倍表示のXvfbで検証した。方式選定の実測と上流参照は [rendering.md](rendering.md) に記載している。

同じ入力で再比較した結果は次のとおり。「共通フォント」は `--default-font 'Noto Sans CJK JP' --fallback-font 'Noto Sans CJK JP'` を両方に指定した結果。異なる寸法の画像では画素差を集計していない。

| 図面 | フォント指定 | Desktop寸法 | Chromium寸法 | RGBAが異なる画素数 |
| --- | --- | --- | --- | --- |
| 文字なし矩形 | なし／共通 | 104×44 | 104×44 | 0 |
| HTMLラベル | なし | 244×84 | 244×84 | 994 |
| HTMLラベル | 共通 | 244×84 | 244×84 | 761 |
| 日本語・英数字ラベル | なし | 131×44 | 149×44 | — |
| 日本語・英数字ラベル | 共通 | 131×44 | 132×44 | — |
| 複数ページ | なし | 131×44 | 149×44 | — |
| 複数ページ | 共通 | 131×44 | 132×44 | — |

全ケースで抽出XMLはレンダラー間で一致した。文字なし矩形は寸法だけでなくRGBAも一致するようになった。文字部分の差と共通フォント指定時の幅1pxの差は残り、解像度・縮小方式の統一だけでは解消しない。

- Electronで独立生成した半透明・画像端を含む縮小fixtureとRustの出力RGBAが一致した。
- 小さい画像・縦長・横長画像、透明色、取得画像の容量制限、寸法不一致、期限切れを通常テストで確認した。
- 実機テストにDesktopの文字なし矩形とのRGBA比較と、最終画像は64 MiB以内でも2倍取得が上限を超える図面の拒否を追加した。
- 取得前のRGBAバッファを64 MiB以内に制限するため、Chromiumの最終出力上限は4,194,304画素となる。
- 通常テスト51件、Chromium実機4件・Desktop実機1件・共通フォント実機1件が成功した。format・clippy・releaseビルドも成功した。実機テストのコマンドと追加引数は直前のフォント検証と同じ。

比較画像と集計はローカルの `.tmp/resampling/final/` に保存した。回帰テスト用の基準画像は `tests/fixtures/` に保存し、生成条件も記録した。macOS/Windows実機とHiDPI表示は今回の検証に含まない。

## VS Code draw.io拡張のPNG保存との比較

2026-09-20、ローカルの拡張コードと固定されたdraw.ioサブモジュールを使い、拡張のWebview HTMLと `xmlpng` 保存経路をChromiumで再現した。9入力すべてでDesktop／CLI Chromiumと出力寸法が異なり、文字なし矩形も拡張102×42、CLI104×44となった。単純な余白除去後も画素差が残った。拡張経路のDPR 1／2間では全9件のRGBAが一致した。

CLI生成PNGを拡張経路で再読み込み・再保存する18件も成功し、XMLを直接読み込んだ拡張出力とのRGBA一致、再保存PNGからのXML抽出・検証、複数ページの保持を確認した。VS Code本体での実機検証は含まない。詳細と再現条件は [VS Code拡張とのPNG互換性](vscode-rendering.md) を参照。CLIの描画実装はこの調査では変更していない。
