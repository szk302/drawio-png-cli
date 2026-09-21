# drawio-png-cli

`dip` は `.drawio.png` に埋め込まれた draw.io の XML を抽出・検証・更新する Rust 製 CLI です。圧縮ページを展開して AI で編集し、draw.io Desktop または Chromium / Chrome で PNG を再描画できます。

## インストール

開発環境の Rust は `mise.toml` で固定しています。

```sh
mise install
mise exec -- cargo install --path . --locked
```

Rust を直接管理している場合は `cargo install --path . --locked` でもインストールできます。実行ファイル名は `dip` です。抽出・検証・メタデータ更新には Node.js や Python は不要です。画像の再描画には draw.io Desktop 27.0.2 以降、または Chromium / Chrome が必要です。Chromium 用の基本描画資材はバイナリに同梱しています。

## 使い方

```sh
# 圧縮ページを展開して、全ページの編集可能な XML を取得
dip extract diagram.drawio.png -o diagram.xml

# XML または PNG 内の図面を検証
dip validate diagram.xml
dip validate diagram.drawio.png

# XML から先頭ページを描画し、全ページの XML を PNG に保存（Desktop優先、なければChromium）
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

## レンダラーの選択

`embed --renderer auto|desktop|chromium` で選択します。既定の `auto` は Desktop を優先し、見つからない場合だけ Chromium に切り替えます。明示パスが不正な場合や描画に失敗した場合は切り替えず、既存出力を保持してエラー終了します。

Chromiumの出力サイズはDesktopと同じく、描画範囲の右端・下端を切り上げて縦横に1pxを追加します。DPR 2で描画し、取得画像をHamming1方式で縦横それぞれ半分に縮小します。これはDesktop 31.4.5を1倍表示のLinux環境で実測した結果に合わせたものです。フォント・ブラウザーバージョン・Desktop側の表示倍率が異なる場合は、寸法や画素の完全一致を保証しません。詳細は [描画互換性](docs/rendering.md) を参照してください。

```sh
dip embed --renderer chromium -i diagram.xml -o diagram.drawio.png
dip embed --renderer desktop -i diagram.xml -o diagram.drawio.png
```

`--renderer` と `--allow-network` は `--no-render` と併用できません。`--allow-network` は Chromium 専用です。`auto` で Desktop が選ばれる環境では `--renderer chromium` も指定してください。

## フォントの統一

`--default-font` でセルの `fontFamily` が未指定の場合のフォントを選び、`--fallback-font` で代替候補を指定できます。Desktop・Chromiumに同じフォント候補を渡し、設定を全ページのXMLにも保存します。オプションを省略すると従来の指定を使用します。

```sh
dip embed --renderer chromium -i diagram.xml -o diagram.drawio.png \
  --default-font "Noto Sans CJK JP" \
  --fallback-font "Noto Sans CJK JP" \
  --fallback-font "Noto Color Emoji"
```

この例では、未指定セルの第一候補がNoto Sans CJK JPになります。`fontFamily=Helvetica` など既存の指定は第一候補として保持し、そのフォントが使えない文字にはNoto Sans CJK JP、Noto Color Emojiの順で代替候補を渡します。同じ候補は重複追加しません。既存候補に `sans-serif` などの一般名がある場合は、その手前に代替候補を挿入します。

- 特定のフォントを製品の既定値として強制しません。両方の描画環境に同じフォントを用意し、そのファミリー名を指定してください。フォントの同梱・自動取得・インストール確認は行いません。
- `--fallback-font` は複数回指定でき、`--default-font` が必要です。1回につき1ファミリーを指定します。未インストール・未対応文字の場合は次の候補へ進み、すべて使えない場合はブラウザーの通常の代替処理になります。
- 対象は図形・接続線のセルの `fontFamily` です。HTMLラベル内の明示的なフォント指定、`inherit` などの継承指定、`fontSource` によるWebフォント指定は保持します。名前付きスタイルだけを指定したセルでは、今回の既定フォントがセルの `fontFamily` として優先されます。
- `--no-render` でもXMLに設定を保存できます。`--no-validate` とは併用できません。
- 同じ候補リストでも、フォントのバージョン・OSの別名解決・ブラウザーの文字計測・縮小処理が異なると寸法や画素に差が残ります。

仕様の詳細は [フォント指定の仕様](docs/fonts.md) を参照してください。

## Chromium / Chrome の準備

検索順は `DIP_CHROME_PATH` → `CHROME_PATH` → PATH → OS 標準インストール先です。PATH では `chromium`、`chromium-browser`、`google-chrome`、`google-chrome-stable`、`chrome`（Windows では `chromium.exe` / `chrome.exe`）を探します。明示指定のパスが不正な場合は別候補へ切り替えません。

```sh
export DIP_CHROME_PATH=/usr/bin/chromium
dip embed --renderer chromium -i diagram.xml -o diagram.drawio.png
```

ヘッドレスで起動するため Xvfb は不要です。Node.js、Python、ChromeDriver、オンライン版 draw.io への接続も不要です。フォントはOSにインストールされたものを使用するので、日本語には日本語フォントを用意してください。

同梱資材は基本図形・接続線・日本語・HTMLラベル・埋め込み画像に対応します。追加アイコン・ステンシル、数式、Mermaid、自動レイアウトの資材は含みません。未知の図形や必要資材の欠落はエラーになります。同梱資材で対応できない図面には、Desktop または別途用意した draw.io Web資材を指定してください。

```sh
# export3.html を含む Webアプリのルートを指定
export DIP_DRAWIO_WEB_PATH=/path/to/drawio/src/main/webapp
dip embed --renderer chromium -i diagram.xml -o diagram.drawio.png
```

`DIP_DRAWIO_WEB_PATH` を解除すると同梱資材に戻ります。指定が不正な場合はエラーとし、同梱資材には切り替えません。互換性を確認した上流コミットは `744cb5420fdf126efd7a09b1d7082ca3e12c0841` です。別バージョンでは `export3.html` / `render()` / `LoadingComplete` の互換性が必要です。

図面が参照する外部画像・Webフォントの取得は既定で禁止し、必要な外部資材がある場合はエラーにします。明示的に許可するときは次を使います。

```sh
dip embed --renderer chromium --allow-network -i diagram.xml -o diagram.drawio.png
```

`--allow-network` は外部HTTP(S)画像・フォント等の資材取得を許可します。外部スクリプト、フレーム、任意のローカルファイルの読み込みは許可しません。Web資材はループバック限定の一時HTTPサーバーで提供し、処理後にサーバーと専用ブラウザープロファイルを片付けます。

追加オプションは `DIP_CHROME_ARGS` にPOSIX形式で指定します。

```sh
export DIP_CHROME_ARGS='--disable-gpu --disable-dev-shm-usage'
```

シェル展開は行いません。追加オプションは `--name` または `--name=value` 形式とし、値に空白があれば引用符で囲みます。プロファイル、デバッグ接続、ネットワーク制御など dip が管理するオプションは指定できません。sandbox は自動で無効化しません。テスト用コンテナで必要な場合に限り `--no-sandbox` を明示できます。

`extract`・`validate`・`embed --no-render` はChromium用の環境変数を読みません。Desktop選択時もChromium用の環境変数は読みません。

## Desktop の準備

draw.io Desktop の検索順は次のとおりです。

1. `DIP_DRAWIO_PATH` に指定された実行ファイル
2. PATH 内の `drawio`／`draw.io`（Windows では `.exe`）
3. OS の標準インストール先（macOS の `/Applications` と `~/Applications`、Windows の `ProgramFiles`／`LOCALAPPDATA`、Linux の `/opt/drawio/drawio`）

```sh
export DIP_DRAWIO_PATH=/Applications/draw.io.app/Contents/MacOS/draw.io
```

明示指定が不正な場合、別の実行ファイルには切り替えません。描画のタイムアウトは 60 秒です。`--renderer desktop` で Desktop が見つからない場合や描画に失敗した場合はエラー終了し、既存出力は変更しません。

Desktop の追加オプションは `DIP_DRAWIO_ARGS` に指定します。`DIP_DRAWIO_PATH` には実行ファイルのパスだけを指定してください。

```sh
export DIP_DRAWIO_PATH=/opt/drawio/drawio
export DIP_DRAWIO_ARGS='--disable-gpu --disable-dev-shm-usage'

# GUI のないコンテナでは、Xvfb 経由で dip を起動する
xvfb-run -a dip embed -i diagram.xml -o diagram.drawio.png
```

Desktop のほかに `xvfb`・`xauth`、日本語を描画する場合は日本語フォントをコンテナに用意してください。`DIP_DRAWIO_ARGS` の指定だけでは仮想ディスプレイは起動しません。

空白を含む値は引用符で囲みます。

```sh
export DIP_DRAWIO_ARGS='--disable-gpu --user-data-dir="/tmp/drawio profile"'
```

- 全 OS で POSIX シェル形式の引用符・バックスラッシュによる引数分割を使います。Windows のパスも引用符で囲むなど、この形式に合わせて指定してください。
- シェルは起動せず、環境変数・`~`・ワイルドカード・コマンド置換は展開しません。必要な値は明示的に指定します。
- 未設定・空文字・空白のみなら追加引数はありません。閉じていない引用符や Unicode として読めない値は、Desktop 起動前に終了コード `1` のエラーにします。
- 追加引数は `dip` が生成する描画引数の前に渡します。入力ファイル、出力先、形式、ページ選択は `dip` が管理するため、これらを変更するオプションや `--` は指定しないでください。
- `extract`・`validate`・`embed --no-render` は `DIP_DRAWIO_ARGS` を読みません。

既存の Xvfb ラッパーを `DIP_DRAWIO_PATH` に指定する方法も使えます。ラッパーでは `"$@"` を転送してください。

```sh
#!/bin/sh
exec xvfb-run -a /opt/drawio/drawio "$@"
```

`dip` は Electron の sandbox を自動で無効化しません。テスト用コンテナで sandbox を無効化する必要がある場合は、`DIP_DRAWIO_ARGS` に `--no-sandbox` を明示的に追加できます。

## 検証・互換性・保存

- 入力 XML は UTF-8（BOM 可）。XML 構文、`mxfile`／`mxGraphModel` ルート、各ページの `root` 直下の基盤セル `id="0"`・`id="1"` を検証します。空の図面や壊れた圧縮ページは拒否します。
- 構造検証は、すべての描画・レイアウト不具合を防ぐ保証ではありません。
- `tEXt`／`zTXt` の `mxfile`／`mxGraphModel` を読み取り、旧 Desktop の raw DEFLATE、URL エンコードの二重化にも対応します。競合する複数の図面メタデータは拒否します。
- 抽出時は全ページを非圧縮 XML にします。ページ順・名前・属性・未知要素を保持しますが、元の XML 文字列との完全一致は保証しません。
- 保存時は既存の図面メタデータを置換し、URL エンコードした XML を一つの `tEXt` チャンクへ格納します。ベース画像の画像データ・無関係なチャンクは保持します。
- 入力、展開データ、出力 PNG、デコード後の画像バッファに 64 MiB の上限があります。Chromiumでは縮小前の2倍画像にもこの制限を適用するため、最終画像は最大4,194,304画素です。DTD と外部エンティティは受け付けません。
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

Desktop・Chromium の実機テストは通常のテストから除外しています。描画環境を用意して明示的に実行してください。

```sh
DIP_TEST_DRAWIO_PATH=/path/to/drawio-or-wrapper \
  mise exec -- cargo test --test desktop -- --ignored --nocapture
```

```sh
DIP_TEST_CHROME_PATH=/usr/bin/chromium \
  mise exec -- cargo test --test chromium -- --ignored --nocapture
# テスト環境に必要な追加引数は DIP_TEST_CHROME_ARGS で指定
python3 scripts/vendor_drawio.py --check
```

Linux CIではChromium実機テストも実行します。資材ハッシュの確認に使うPythonは開発用で、ビルド・実行時には不要です。

実装の参考にしたコードと fixture の説明は [tests/fixtures/README.md](tests/fixtures/README.md)、要件は [docs/prd.md](docs/prd.md) を参照してください。`.tmp` の参考リポジトリはビルドに使用しません。

変更は作業ブランチで行い、Conventional Commits 形式でコミットします。

## ライセンス

本プロジェクトの独自コードは [MIT License](LICENSE) で公開します。Chromiumを参考に移植した縮小処理はBSD-3-Clauseです。Cargoのライセンス表記は同梱描画資材と縮小処理を含めて `MIT AND Apache-2.0 AND Zlib AND BSD-3-Clause` としています。
これは異なるライセンスの構成物を含むパッケージの表記で、独自コードのMITライセンスを変更するものではありません。再配布する構成物ごとの条件に従ってください。
参考にした drawio-exporter と VS Code Draw.io Integration の参照範囲・ライセンスは
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)、Cargo依存の著作権表示・ライセンス全文は
[依存ライセンス一覧](assets/licenses/cargo-dependencies.txt) に記載しています。

本プロジェクトは非公式の独立したプロジェクトであり、draw.ioの提供元であるJGraph Holdings Ltdおよびdraw.io AGとの提携・承認関係はありません。

ソース配布には `LICENSE` と `THIRD_PARTY_NOTICES.md` を含めます。
ソース・バイナリとも、この2ファイルと `assets/drawio/README.md`・`assets/drawio/licenses/`・`assets/licenses/`（Cargo依存の生成済み一覧を含む）を保持してください。
同梱描画資材は各資材のライセンスに従います。対象と除外範囲は [資材一覧](assets/drawio/README.md) に記録しています。`dip licenses` で本体・Cargo依存・同梱資材の通知とライセンス全文を確認できます。通常のビルドにライセンス生成ツールは不要です。

依存を更新した際は、次の手順で一覧を再生成し、差分を確認して `Cargo.lock` とともにコミットしてください。
一覧は本パッケージの全featuresで有効になる依存をOSを絞らず収集し、開発用・ビルド用の依存も含めています。`Cargo.lock` にあっても無効な任意依存や常に偽の `cfg` にある依存は対象外です。`about.toml` の優先順で許諾されたライセンスを選択し、`AND` の条件はすべて保持します。

```sh
mise exec -- cargo install --locked --version 0.9.2 cargo-about
mise exec -- cargo install --locked --version 0.20.2 cargo-deny
mise exec -- cargo fetch --locked
mise exec -- python3 scripts/cargo_licenses.py
mise exec -- cargo deny --locked check licenses
```

生成にはネットワーク接続が必要です。crateにライセンス原本がない場合は `about.toml` に固定した上流リビジョンとハッシュで補います。原本が見つからず汎用ライセンス文に置換された場合は生成を失敗させます。
CIでは `scripts/cargo_licenses.py --check` で更新漏れを検出し、許可ライセンスを検査します。Linux・macOS・Windowsのビルド後に通知一式を同梱したバイナリアーカイブを作成します。手元での作成例:

```sh
mise exec -- cargo build --release --locked
python3 scripts/package_binary.py target/release/dip target/dip.tar.gz
# Windowsでは入力を target/release/dip.exe に変更
```
