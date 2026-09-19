# drawio-png-cli

`dip` は `.drawio.png` に埋め込まれた draw.io の XML を抽出・検証・更新する Rust 製 CLI です。圧縮ページを展開して AI で編集し、draw.io Desktop で PNG を再描画できます。

## インストール

開発環境の Rust は `mise.toml` で固定しています。

```sh
mise install
mise exec -- cargo install --path . --locked
```

Rust を直接管理している場合は `cargo install --path . --locked` でもインストールできます。実行ファイル名は `dip` です。抽出・検証・メタデータ更新には Node.js や Python は不要です。画像の再描画には別途 draw.io Desktop 27.0.2 以降が必要です。

## 使い方

```sh
# 圧縮ページを展開して、全ページの編集可能な XML を取得
dip extract diagram.drawio.png -o diagram.xml

# XML または PNG 内の図面を検証
dip validate diagram.xml
dip validate diagram.drawio.png

# XML から先頭ページを描画し、全ページの XML を PNG に保存
dip embed -i diagram.xml -o diagram.drawio.png

# XML は stdin でも指定可能
cat diagram.xml | dip embed -o diagram.drawio.png

# ベース画像の見た目を維持し、XML だけを更新（同じパスでも可）
dip embed -i diagram.xml --no-render -b diagram.drawio.png -o diagram.drawio.png

# ベース画像なしなら、透明な 1×1 PNG に XML を保存
dip embed -i diagram.xml --no-render -o new.drawio.png
```

`extract` の `-o` を省略すると XML を stdout に出力します。診断・警告は stderr に出力します。正常終了は `0`、処理・検証エラーは `1`、引数エラーは `2` です。

`--base-image` は `--no-render` と組み合わせます。`--no-render` は画像と XML の見た目が同期されない旨を警告します。ベース画像を指定しない場合、既存の出力 PNG の画像は再利用しません。

## Desktop の準備

draw.io Desktop の検索順は次のとおりです。

1. `DIP_DRAWIO_PATH` に指定された実行ファイル
2. PATH 内の `drawio`／`draw.io`（Windows では `.exe`）
3. OS の標準インストール先（macOS の `/Applications` と `~/Applications`、Windows の `ProgramFiles`／`LOCALAPPDATA`、Linux の `/opt/drawio/drawio`）

```sh
export DIP_DRAWIO_PATH=/Applications/draw.io.app/Contents/MacOS/draw.io
```

明示指定が不正な場合、別の実行ファイルには切り替えません。描画のタイムアウトは 60 秒です。Desktop が見つからない場合や描画に失敗した場合はエラー終了し、既存出力は変更しません。Chrome フォールバックと `DIP_CHROME_PATH`／`CHROME_PATH` は初回実装の対象外です。

Linux のディスプレイがない環境では、Xvfb などを用意し、`DIP_DRAWIO_PATH` にラッパーのパスを指定できます。

```sh
#!/bin/sh
exec xvfb-run -a /opt/drawio/drawio "$@"
```

`dip` は Electron の sandbox を自動で無効化しません。

## 検証・互換性・保存

- 入力 XML は UTF-8（BOM 可）。XML 構文、`mxfile`／`mxGraphModel` ルート、各ページの `root` 直下の基盤セル `id="0"`・`id="1"` を検証します。空の図面や壊れた圧縮ページは拒否します。
- 構造検証は、すべての描画・レイアウト不具合を防ぐ保証ではありません。
- `tEXt`／`zTXt` の `mxfile`／`mxGraphModel` を読み取り、旧 Desktop の raw DEFLATE、URL エンコードの二重化にも対応します。競合する複数の図面メタデータは拒否します。
- 抽出時は全ページを非圧縮 XML にします。ページ順・名前・属性・未知要素を保持しますが、元の XML 文字列との完全一致は保証しません。
- 保存時は既存の図面メタデータを置換し、URL エンコードした XML を一つの `tEXt` チャンクへ格納します。ベース画像の画像データ・無関係なチャンクは保持します。
- 入力、展開データ、出力 PNG、デコード後の画像バッファに 64 MiB の上限があります。DTD と外部エンティティは受け付けません。
- 出力先と同じディレクトリの一時ファイルに保存・同期後、アトミックに置換します。既存ファイルの権限を引き継ぎ、失敗時の一時ファイルは片付けます。出力先ディレクトリは事前に作成してください。
- `--no-validate` はデバッグ用です。XML の構造検証・正規化を省略し、不正な XML も埋め込めます。UTF-8・サイズ制限・PNG 検査・アトミック保存は維持します。

## 開発とテスト

```sh
mise exec -- cargo fmt --check
mise exec -- cargo clippy --locked --all-targets -- -D warnings
mise exec -- cargo test --locked
mise exec -- cargo build --release --locked
```

固定 fixture による互換テスト、CLI の往復編集、保存失敗時の保護、Unix のテスト用レンダラーによる引数・異常終了・タイムアウト検証を含みます。CI は Linux・macOS・Windows で実行します。

Desktop の実機テストは通常のテストから除外しています。Desktop と描画環境を用意して明示的に実行してください。

```sh
DIP_TEST_DRAWIO_PATH=/path/to/drawio-or-wrapper \
  mise exec -- cargo test --test desktop -- --ignored --nocapture
```

実装の参考にしたコードと fixture の説明は [tests/fixtures/README.md](tests/fixtures/README.md)、要件は [docs/prd.md](docs/prd.md) を参照してください。`.tmp` の参考リポジトリはビルドに使用しません。

変更は作業ブランチで行い、Conventional Commits 形式でコミットします。
