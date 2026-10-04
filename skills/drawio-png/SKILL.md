---
name: drawio-png
description: dip（drawio-png-cli）で `.drawio.png` に埋め込まれた draw.io 図面を読み取り・編集・新規作成する手順。VS Code の draw.io 拡張で保存したのと同じ見た目の PNG を作る。`.drawio.png` の中身（ノード・接続・ページ）を説明する、図形やラベルを追加・変更・削除する、アイコン（Simple Icons などの draw.io カスタムライブラリ）を図に入れる、XML から VS Code で開ける PNG を作る、図面を検証する、といった依頼では必ずこのスキルを使うこと。ユーザーが「dip」と言わなくても、`.drawio.png` や draw.io の PNG 図面を扱う依頼なら該当する。PNG のバイト列を自前のスクリプトで解析・書き換えしないこと。
---

# drawio-png: dip で .drawio.png を扱う（vscode モード）

`.drawio.png` は、見た目の PNG 画像と、編集用の draw.io XML（PNG のテキストチャンク内）を 1 ファイルに持つ。
`dip` はこの XML を取り出し（extract）、検証し（validate）、編集後の XML から画像を描き直して書き戻す（embed）CLI。
PNG を Python などで直接いじると、圧縮形式の違いや画像との同期を取り違えやすい。必ず dip を経由する。

このスキルは、**VS Code の draw.io 拡張（1.9.0）で保存したのと同じ見た目**になる vscode モード
（Chrome / Chromium で描画）を使う。Desktop レンダラー、`raw`・`desktop` モード、フォント指定、
`--no-render` など、ここに無い機能が必要なときは [SKILL_FULL.md](SKILL_FULL.md) を読む。
その場合も、ユーザーが別のレンダラー・モードを明示的に求めない限り、embed の
`--renderer chromium --chromium-mode vscode` は変えない。

## 準備

```sh
command -v dip && dip --version
```

dip が無ければ、[GitHub Releases](https://github.com/szk302/drawio-png-cli/releases) の OS 別アーカイブ
（展開して `dip` を PATH に置く）か、`cargo install --git https://github.com/szk302/drawio-png-cli --locked` で入れる。
描画には Chrome（無ければ Chromium）が必要。どちらもできない場合は、その旨をユーザーに伝えて止まる。

## 基本の流れ

一時ファイルはリポジトリの外（スクラッチ領域や `mktemp -d`）に置く。作業用 XML をリポジトリに残さない。

```sh
dip extract diagram.drawio.png -o "$tmp/diagram.xml"   # 全ページを非圧縮 XML で取得
# … XML を編集 …
dip validate "$tmp/diagram.xml"                         # 構造を検証
dip embed --renderer chromium --chromium-mode vscode \
  -i "$tmp/diagram.xml" -o diagram.drawio.png           # 先頭ページを描画し、全ページの XML を保存
```

- **embed には常に `--renderer chromium --chromium-mode vscode` を付ける。** 省略すると、draw.io Desktop が
  入っている環境では Desktop で描画され、VS Code 拡張と見た目が変わる。
- **読むだけなら extract だけでよい。** `-o` を省けば stdout に出る。図面の説明を求められたら、
  セルの `value`（ラベル）、`source`/`target`（接続）、`parent`（グループやコンテナ）、ページ名を読み取って答える。
- **大きな画像データを読み込まない。** アイコン入りの図面では、セルの `style` に `image=data:image/...` の
  Base64 が数千〜数万文字入ることがある。行単位の `grep` では同じ行の Base64 も出力されるので、
  必要な属性だけを取り出して読む: `grep -oE ' (id|value|label|source|target|parent|vertex|edge)="[^"]*"' "$tmp/diagram.xml"`
- **入力と同じパスへ embed してよい。** 一時ファイルに書いてから置き換えるので、失敗時は元のファイルが残る。
- **embed は保存前に自動で検証する。** 検証エラーなら何も書き換えないので、XML を直して再実行する。
- 終了コード: `0` 成功、`1` 処理・検証エラー、`2` 引数エラー。エラー内容は stderr に出る。

## XML を編集するときの注意

dip は図面の構造（`mxfile`/`mxGraphModel`、各ページの `root` 直下の `<mxCell id="0"/>` と
`<mxCell id="1" parent="0"/>`）を検証するが、見た目の崩れまでは検出しない。

- **必要な箇所だけ変える。** extract 結果はページ順・ページ名・未知の属性や要素を保持している。
  関係ないセル・ページ・属性を整形し直したり削ったりしない。
- **ID はページ内で一意にする。** 新しいセルには既存と重ならない ID を付ける。
  接続線（`edge="1"`）の `source`/`target` は既存セルの ID を正確に参照する。
- **既存図面のスタイルに合わせる。** 追加する図形は、近くの同種セルの `style`（色・フォント・角丸など）を真似ると図面になじむ。
- **新規作成**では次の最小構成から始める。

```xml
<mxfile>
  <diagram name="Page-1" id="page-1">
    <mxGraphModel>
      <root>
        <mxCell id="0"/>
        <mxCell id="1" parent="0"/>
        <mxCell id="a" value="開始" style="rounded=1;whiteSpace=wrap;html=1;" vertex="1" parent="1">
          <mxGeometry x="40" y="40" width="120" height="60" as="geometry"/>
        </mxCell>
        <mxCell id="b" value="終了" style="rounded=1;whiteSpace=wrap;html=1;" vertex="1" parent="1">
          <mxGeometry x="240" y="40" width="120" height="60" as="geometry"/>
        </mxCell>
        <mxCell id="e1" style="edgeStyle=orthogonalEdgeStyle;html=1;" edge="1" parent="1" source="a" target="b">
          <mxGeometry relative="1" as="geometry"/>
        </mxCell>
      </root>
    </mxGraphModel>
  </diagram>
</mxfile>
```

## アイコン・ライブラリの図形を入れる

ロゴやアイコン（Simple Icons など）は、draw.io のカスタムライブラリ（`<mxlibrary>` の XML）から
`dip insert` で入れる。アイコンの `style` には大きな Base64 が入るので、**自分で書き写さない**。
使えるライブラリは環境変数 `DIP_LIBRARY_PATH` で決まる。ユーザーがライブラリのファイルを示した場合は、
各コマンドに `--library-file <ファイル>` を付ければ `DIP_LIBRARY_PATH` は不要。どちらも無い場合だけ、ユーザーに確認する。

```sh
dip library list                           # ライブラリ名・エントリー数・表示名
dip library search postgres                # ライブラリ名・番号・タイトル・サイズ（画像データは出ない）
dip insert --name PostgreSQL -i "$tmp/diagram.xml" -o "$tmp/diagram.xml" \
  --id db --x 40 --y 40 --width 48         # 挿入したセルの ID を出力
```

- 検索語はタイトルの部分一致（大文字小文字は区別しない）。見つからなければ短い語や別名で探し直す。
- `--name` は一意に決まる必要がある。複数のライブラリにあれば `--library <名前>`、同じライブラリに同名が
  あれば `search` の番号を `--index` で指定する。
- 挿入後のラベル付けや接続線は、出力された ID を使って XML を編集する。
  `--label` でラベルも付けられるが、位置・書式はライブラリの `style` のまま（アイコンに重なることがある）。
- アイコン（ブランドロゴ）のライセンス・商標の条件は提供元に従う。

## 描画について

embed は**先頭ページだけ**を画像にする。2 ページ目以降を編集しても PNG の見た目は変わらない
（XML には全ページが保存される）。2 ページ目以降を変えたときは、そのことをユーザーに伝える。

描画に失敗したときは、勝手に描画を省略したり、レンダラーやモードを変えたりしない。原因をユーザーに伝える。

### AWS・Google Cloud（GCP2）などの図形

`shape=mxgraph.aws4.*`・`mxgraph.gcp2.*` などの draw.io 図形集は同梱していないので、
`Unsupported shape: ...` になる。VS Code 拡張 1.9.0 と同じ **draw.io 26.0.2**
（コミット `96a916a337d13fc8bf622c8a67d422bd284eabe5`）の Web 資材を `DIP_DRAWIO_WEB_PATH` で指定する。
他のコミット（例: `f3abfe0f…` の 31.4.5）は使わない。VS Code 拡張と見た目が 1px ずれる。

```sh
git clone https://github.com/jgraph/drawio.git "$tmp/drawio"
git -C "$tmp/drawio" checkout 96a916a337d13fc8bf622c8a67d422bd284eabe5
DIP_DRAWIO_WEB_PATH="$tmp/drawio/src/main/webapp" \
  dip embed --renderer chromium --chromium-mode vscode -i "$tmp/diagram.xml" -o diagram.drawio.png
```

- clone は外部への通信なので、ユーザーの了承を得てから行う。既に用意された 26.0.2 の Web 資材があればそれを使う。
- 「Google Cloud Platform 2026」（`mxgraph.gcp3.*`）は 26.0.2 に無く、VS Code 拡張 1.9.0 でも表示されない。使わない。
- 画像を `data:` URL で埋め込んだ図形（Simple Icons、Google Cloud Icons など）は Web 資材なしで描ける。

### よくあるエラーと対応

| エラー | 対応 |
| --- | --- |
| `Chromium/Chrome not found` | Chrome / Chromium の導入、または `DIP_CHROME_PATH` の設定をユーザーに案内する |
| `Unsupported shape: ...` | 図形集（AWS 等）が必要。上記の 26.0.2 の Web 資材を指定する。アイコン（`resIcon`・`grIcon` 等）が 26.0.2 に無い場合も出る。図形を勝手に別の形へ置き換えない |
| `Math and automatic layout require ...` | 数式・自動レイアウトにも `DIP_DRAWIO_WEB_PATH` が必要 |
| `external resource blocked` / 外部画像・フォントの取得エラー | 既定でネットワーク取得を禁止している。外部 URL への接続が問題ないかユーザーに確認してから `--allow-network` を付ける |
| `draw.io opened a ... dialog` | 指定した Web 資材がエラーを出している。資材の版・内容を確認する |
| `Chromium rendering timed out` | 描画が60秒以内に終わらない。資材や図面の大きさを確認する |
| `interrupted (signal N)` | 描画中に中断された。後片付け済みで出力は変わっていない |
| 検証エラー（`missing essential <mxCell id="0">` など） | XML を直す。`--no-validate` はデバッグ用なので使わない |

`--allow-network` は外部への通信を許可する。既定では Chrome 自身の Google への通信も止めているが、
`--allow-network` 時は一部残り、プロキシ環境ではプロキシ経由で外に出得る。必要なときだけ、ユーザーの了承を得て使う。

### VS Code 拡張との見た目の差

同じフォント環境なら、保存時の表示倍率が 100% の VS Code 拡張 1.9.0 と同じ見た目（寸法・配置・画素）になる。
VS Code（Windows など）と dip（Linux など）で OS やフォントが違うと、文字の幅が変わり、
画素だけでなく画像の寸法や配置も変わり得る。完全な一致は保証しない。

## 仕上げ

- 変更後は `dip validate diagram.drawio.png` で書き戻した結果を確認できる。
- 報告では、何を変えたか（追加・変更したセル、挿入したアイコン）と、描画の有無
  （先頭ページのみ描画、`DIP_DRAWIO_WEB_PATH` を使った場合はその版）を伝える。
