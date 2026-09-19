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
3. CLI 実行時の誤った XML 書き込みによるファイル破損を 100% 防ぐ自動バリデーション機構を提供する。



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

`.drawio.png` から XML メタデータを抽出して出力する。

| オプション | 型 | 説明 |
| --- | --- | --- |
| `input` (位置引数) | Path | 対象の `.drawio.png` ファイルパス |
| `-o`, `--output` | Path | 出力先 XML ファイルパス（省略時は標準出力 `stdout`） |

* **実行例**:
```bash
# ファイルに出力
dip extract input.drawio.png -o diagram.xml

# パイプラインで標準出力受け取り
dip extract input.drawio.png | jq .

```



---

#### `dip embed`

XML データを `.drawio.png` に挿入・更新する。**実行時に自動でバリデーションを呼び出し、パスした場合のみ書き込む。**

| オプション | 型 | 説明 |
| --- | --- | --- |
| `-i`, `--input` | Path | 入力 XML ファイルパス（省略時は標準入力 `stdin`） |
| `-o`, `--output` | Path | (必須) 出力先 `.drawio.png` ファイルパス |
| `-b`, `--base-image` | Path | ベースとなる既存 PNG ファイルパス |
| `--no-render` | Flag | 画像の再描画を行わず、メタデータ（XML）のみを更新する(このオプションを未指定かつ外部レンダラーが存在しない場合、警告終了ではなくエラー終了させる) |
| `--no-validate` | Flag | （デバッグ用）自動バリデーションをスキップして強制書き込みする |

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
* draw.io 描画エンジンが最低限必要とするデフォルトセル（`id="0"` および `id="1"` の `<mxCell>`）が存在するか。



---

## 6. レンダリング & フォールバック戦略 (Rendering Strategy)

XML メタデータを挿入する際、視覚的な PNG 画像（ビジュアルデータ）を最新化するためのレンダラー呼び出しは以下の **優先順位による自動フォールバック構造** を取る。

```
[dip embed 実行]
       │
       ├── 1. draw.io Desktop CLI (drawio) を探す
       │    └── 発見 ──> 最高精度で画像・メタデータを一括更新
       │
       ├── 2. Headless Chrome / Chromium を探す
       │    └── 発見 ──> Chrome経由で画像をレンダリングして更新
       │
       └── 3. いずれの外部バイナリも見つからない場合
            └── エラー
              ｌ--no-renderが指定されている場合は ️警告を出力し「メタデータ（XML）のみ」を PNG に挿入して保存

```

### 6.1. 環境変数によるパス指定

自動検索に頼らず明示的にバイナリパスを指定できるよう、以下の環境変数をサポートする。

* **`DIP_DRAWIO_PATH`**: draw.io Desktop 実行ファイルのパス
* **`DIP_CHROME_PATH`** (または `CHROME_PATH`): Chrome / Chromium 実行ファイルのパス

---

## 7. 非機能要件 & 技術スタック (Non-functional Requirements)

### 7.1. 非機能要件

* **高速起動**: AI エージェントのツール呼び出しループを阻害しないため、CLI の起動〜応答速度を数十ミリ秒以下に抑える。
* **ポータビリティ**: 外部依存（Node.js や Python ランタイム等）を持たない単一バイナリ（Single Binary）として配布可能であること。
* **アトミック書き込み**: ファイル更新時は一時ファイル（`.tmp`）を作成した上で置き換え処理（アトミックリネーム）を行い、途中で処理が中断された場合のファイル破損を防止する。

### 7.2. 技術スタック (Rust)

* **言語**: Rust
* **使用クレート (予定)**:
* `clap`: CLI 引数・サブコマンドの構文解析
* `png`: PNG チャンク（`zTXt` / `tEXt`）の低レイヤー操作
* `flate2`: draw.io データの Deflate 圧縮・解凍
* `base64`, `urlencoding`: メタデータのエンコード・デコード処理
* `quick-xml` または `roxmltree`: 高速な XML バリデーションパース

---

## 8. 将来の拡張性 (Future Scope)

* **GitHub Actions 対応**: リポジトリ内の `.drawio.png` が常に正しい状態であることを自動チェックする CI アクションの提供。
* **Diff サブコマンド (`dip diff`)**: 2つの `.drawio.png` の XML 構造差分（追加・削除された図形やテキスト）をテキストベースで出力する機能。
