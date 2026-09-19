# プロダクト要件定義書 (PRD): `dip` (Draw.io Image Processor)

## 1. 概要 (Overview)

`dip` (Draw.io Image Processor) は、VS Code の draw.io 拡張機能などで扱われる `.drawio.png` 形式のファイルを、AI エージェントと人間がシームレスに相互編集できるようにするための軽量かつ高速な CLI ツールである。

PNG 画像のメタデータ（`zTXt` / `tEXt` チャンク）内に埋め込まれた draw.io の XML データを抽出し、AI による編集後の XML を安全に再埋め込み・画像化するインターフェースを提供する。

---

## 2. 目的・背景 (Background & Goals)

* **課題**: `.drawio.png` 形式はバイナリ（PNG画像）とテキスト（XMLメタデータ）が混在しているため、プログラムや LLM / AI エージェントが直接読み書きを行うことが困難であった。
* **目的**:
1. AI エージェントが安全・確実に draw.io の構造データ（XML）を取り出し、修正・更新できるようにする。
2. 人間が VS Code の GUI で編集した `.drawio.png` と、AI がプログラム経由で作成・更新した構造データを双方向に同期・補完できるようにする。
3. CLI 実行時に XML の構文・基本構造を検証し、不正な入力による上書きを防ぐ。すべての描画・レイアウト不具合の防止は保証しない。



---

## 3. ユースケース (Use Cases)

```
[人間 (VS Code)] ──(GUIで作成)──> filename.drawio.png
                                         │
                                   [dip extract]
                                         │
                                         ▼
[AI エージェント] <──(XML解析・修正)─── diagram.xml
       │
 [dip embed] ──(自動Validation)──> filename.drawio.png
                                         │
[人間 (VS Code)] <──(GUIで微調整)────────┘

```

1. **AI による既存図面の自動アップデート**: 人間が作成した `.drawio.png` から XML を抽出し、AI が図面構造を追加・修正した上で `.drawio.png` へ書き戻す。
2. **AI による新規図面の生成**: AI が一から生成した XML データを `dip embed` に引き渡し、VS Code で即座に開いて人間が微調整できる `.drawio.png` ファイルを出力する。
3. **CI/CD パイプラインでの図面検証**: PR 提出時などに `dip validate` を通すことで、破壊された draw.io ファイルの混入を防ぐ。

---

## 4. CLI インターフェース仕様 (CLI Specification)

* **コマンド名**: `dip`
* **リポジトリ名**: `drawio-png-cli`

### 4.1. サブコマンド一覧

#### `dip extract`

`.drawio.png` から XML メタデータを抽出し、全ページを非圧縮 XML に展開して出力する。ページ順・名前・属性・未知要素を保持するが、元の文字列表現との完全一致は保証しない。

| オプション | 型 | 説明 |
| --- | --- | --- |
| `input` (位置引数) | Path | 対象の `.drawio.png` ファイルパス |
| `-o`, `--output` | Path | 出力先 XML ファイルパス（省略時は標準出力 `stdout`） |

* **実行例**:
```bash
# ファイルに出力
dip extract input.drawio.png -o diagram.xml

# パイプラインで標準出力受け取り
dip extract input.drawio.png > diagram.xml

```



---

#### `dip embed`

XML データを `.drawio.png` に挿入・更新する。**実行時に自動でバリデーションを呼び出し、パスした場合のみ書き込む。**

| オプション | 型 | 説明 |
| --- | --- | --- |
| `-i`, `--input` | Path | 入力 XML ファイルパス（省略時は標準入力 `stdin`） |
| `-o`, `--output` | Path | (必須) 出力先 `.drawio.png` ファイルパス |
| `-b`, `--base-image` | Path | ベースとなる既存 PNG ファイルパス。`--no-render` と併用する |
| `--no-render` | Flag | 画像の再描画を行わず、メタデータ（XML）のみを更新する(このオプションを未指定かつ外部レンダラーが存在しない場合、警告終了ではなくエラー終了させる) |
| `--no-validate` | Flag | （デバッグ用）自動バリデーションをスキップして強制書き込みする |

`--no-render` でベース画像を省略した場合は透明な 1×1 PNG を生成する。既存の出力先はベースとして再利用しない。画像と XML が同期されない旨を stderr に警告する。複数ページはすべて保持し、通常の描画では先頭ページを画像化する。

`--no-validate` は XML 検証と正規化を省略するが、UTF-8・サイズ制限・PNG 検査・アトミック保存は省略しない。

* **実行例**:
```bash
# XMLファイルから埋め込み
dip embed -i modified.xml -o output.drawio.png

# stdin 経由でパイプライン結合
cat modified.xml | dip embed -o output.drawio.png

```



---

#### `dip validate`

指定した XML または `.drawio.png` ファイルが正当な draw.io 構造を持っているか事前検証する。

* **実行例**:
```bash
dip validate diagram.xml
dip validate output.drawio.png

```

---

## 5. 自動バリデーション仕様 (Validation Rules)

ファイル破損を防ぐため、`dip embed` 実行時および `dip validate` 呼び出し時には以下の **3段階の検証** を直列で実施する。いずれかの段階でエラーが発生した場合、処理を即座に中断し、終了コード `1` とエラー内容を出力する。

```
[入力 XML/stdin]
       │
       ▼
1. XML Syntax Check (Well-formed)
       │ Pass
       ▼
2. Root Tag Check (<mxfile> / <mxGraphModel>)
       │ Pass
       ▼
3. Essential Cell Check (id="0" & id="1")
       │ Pass
       ▼
 [処理継続 (embed / exit 0)]

```

1. **XML 構文チェック (Well-formed Check)**:
* タグの対構造、エスケープ漏れ、文字コードが正常な XML であるか。


2. **ルート要素チェック (Root Tag Check)**:
* ルート要素が `<mxfile>` または `<mxGraphModel>` であるか。


3. **基盤セル存在チェック (Essential Cell Check)**:
* 各ページの `<mxGraphModel><root>` 直下にデフォルトセル（`id="0"` および `id="1"` の `<mxCell>`）が存在するか。圧縮ページは展開して検証し、空の `<mxfile>` や復号不能なページは拒否する。



---

## 6. レンダリング戦略（初回実装）

初回は draw.io Desktop 27.0.2 以降による描画に対応する。先頭ページは 1-based の `--page-index 1` で指定する。Chrome / Chromium フォールバックは次段階に分ける。

* Desktop の検索順: `DIP_DRAWIO_PATH` → PATH の `drawio` / `draw.io` → OS 標準インストール先。
* `DIP_DRAWIO_PATH` の明示指定が不正な場合はエラーとし、別の候補には切り替えない。
* 一時ファイルへ先頭ページを描画し、全ページの XML は `dip` が PNG に埋め込む。
* 描画は 60 秒でタイムアウトする。Desktop がない場合・描画失敗時はエラー終了する。
* `--no-render` では Desktop を検索・起動せず、ベース画像または新規透明 PNG にメタデータを保存する。
* Linux のディスプレイがない環境では利用者が Xvfb 等を用意する。`dip` は sandbox を自動無効化しない。
* `DIP_CHROME_PATH` / `CHROME_PATH` は将来の Chrome 対応時に導入する。

---

## 7. 非機能要件 & 技術スタック (Non-functional Requirements)

### 7.1. 非機能要件

* **高速起動**: AI エージェントのツール呼び出しループを阻害しないため、リリースビルドの起動と小規模入力の処理を数十ミリ秒以下の目標で測定する。外部レンダラーの起動・描画時間は別途評価する。
* **ポータビリティ**: 外部依存（Node.js や Python ランタイム等）を持たない単一バイナリ（Single Binary）として配布可能であること。
* **アトミック書き込み**: 出力先と同じディレクトリの一時ファイルに保存・同期後、アトミックに置換する。検証・描画・保存失敗時は既存出力を維持する。

### 7.2. 技術スタック (Rust)

* **言語**: Rust
* **使用クレート**:
* `clap`: CLI 引数・サブコマンドの構文解析
* `png`: PNG 画像の検査と透明画像の生成。メタデータはチャンク単位で直接置換し、`crc32fast` で CRC を検証・生成する
* `flate2`: draw.io データの Deflate 圧縮・解凍
* `base64`, `urlencoding`: メタデータのエンコード・デコード処理
* `roxmltree`: XML の検証とソース範囲に基づくページ展開。全体の再シリアライズを避け、未知要素や属性を保持する
* `tempfile`: 同一ディレクトリでのアトミック保存と描画用一時ファイル

---

### 7.3. 入出力の制約

* UTF-8 XML（BOM 可）を扱い、DTD・外部エンティティは拒否する。
* 入力・展開データ・出力 PNG・デコード後の画像バッファはそれぞれ 64 MiB を上限とする。
* PNG シグネチャ、チャンク境界、CRC、画像データを検査する。
* `tEXt` / `zTXt` の `mxfile` / `mxGraphModel` を読み取る。旧 raw DEFLATE と二重 URL エンコードにも対応する。
* 競合する図面メタデータはエラー。同一内容の重複は許容する。書き込み時は図面メタデータを一つに統一する。
* 成功 `0`、処理・検証エラー `1`、引数エラー `2`。診断・警告は stderr に出力する。

## 8. 将来の拡張性 (Future Scope)

* **GitHub Actions 対応**: リポジトリ内の `.drawio.png` が常に正しい状態であることを自動チェックする CI アクションの提供。
* **Diff サブコマンド (`dip diff`)**: 2つの `.drawio.png` の XML 構造差分（追加・削除された図形やテキスト）をテキストベースで出力する機能。
