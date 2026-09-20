# DesktopとChromiumの描画互換性

VS Code draw.io拡張のPNG保存は別の書き出し経路を使い、以下のDesktop互換処理とは出力が一致しない。比較結果と検証範囲は [VS Code拡張とのPNG互換性](vscode-rendering.md) を参照。

Chromiumは描画開始前からDPR（deviceScaleFactor）を2に設定する。出力のCSS寸法は `ceil(bounds.size + bounds.offset) + 1` で求め、その縦横2倍のPNGを取得した後、RustでHamming1方式を使って半分へ縮小する。新しいCLIオプションは不要。

## 比較対象と縮小方式

基準はdraw.io Desktop 31.4.5 / Electron 44.2.0 / Chromium 152.0.7977.76をLinux ARM64の1倍表示のXvfb環境で実行した結果。

Desktopのソースには `offscreen: { deviceScaleFactor: 2 }` と `capturePage()` 後の `img.resize(newBounds)` がある。ただし、後者だけを見て縮小方式を判断できない。

実機のコピーへ計測を追加したところ、100×40の矩形を描画した画像は次の状態だった。

- 描画ページのDPR: 2。
- 物理ディスプレイの倍率: 1。
- `capturePage()` から返った画像: 104×44、scale factor 1。
- `resize()` 後も104×44。呼び出し前後のPNGバイト列は一致。

つまりこの環境では、画面取得時にすでに縮小されている。同じ矩形をChromiumで208×88として取得し、Electronの `nativeImage.resize({width:104,height:44,quality:'good'})` で縮小すると、Desktopの出力画素と完全一致した。`quality:'better'` でも同じ結果になる。これらの方式はHamming1である。

一方、`nativeImage.resize()` の品質省略時はLanczos3を選ぶ。同じ208×88画像にこの処理を適用すると、今回のDesktop出力とは一致しない。実装は観測したDesktopの出力に合わせてHamming1を採用した。

参照元:

- [Electron v44.2.0 NativeImage::Resize](https://github.com/electron/electron/blob/v44.2.0/shell/common/api/electron_api_native_image.cc)
- [Electron v44.2.0 WebContents::CapturePage](https://github.com/electron/electron/blob/v44.2.0/shell/browser/api/electron_api_web_contents.cc)
- [Chromium 152 image_operations.cc](https://github.com/chromium/chromium/blob/152.0.7977.76/skia/ext/image_operations.cc)
- [Chromium 152 convolver.cc](https://github.com/chromium/chromium/blob/152.0.7977.76/skia/ext/convolver.cc)

## 画素の扱い

Chromiumの固定小数点フィルターに合わせ、係数は14ビットの小数部で正規化し、水平方向・垂直方向の順に処理する。各段で8ビット値へ戻す。

アルファを乗算したRGB値を縮小し、PNG保存前に乗算を解除する。透明画素に残っているRGB値が周囲へにじむことを防ぐ。半透明の合成画像を使い、Electronが生成した独立した正解画像と比較している。

フィルターは2:1専用で、左右端と内部の係数だけを保持する。画像が横長でも係数の保存量は増えない。PNG取得後にRustの `png` クレートで処理するため、Node.js・Electron等の追加ランタイムは不要。

## 容量と時間の制限

出力幅W・高さHに対し、取得画像は2W×2H、RGBAバッファは `16 × W × H` bytesとなる。これを64 MiB以内に制限する。したがって最終画像の上限は4,194,304画素（例: 2048×2048）。これは以前の等倍取得より小さい上限になる。

過大な取得画像はスクリーンショット要求前に拒否する。取得PNGの実寸法も検査し、不一致を拒否する。縮小処理では行ごと・横長画像では途中でも期限を確認し、PNG再エンコード後にも確認する。保存は従来どおりアトミックで、失敗時に既存出力を保持する。

## 保証する範囲

文字なし矩形のDesktop基準画像とのRGBA一致をChromium実機テストで確認する。縮小処理単独では、半透明・画像端を含むElectron生成の固定fixtureとのRGBA一致を通常テストで確認する。

Desktopの取得方式は表示倍率等の影響を受ける。HiDPIディスプレイ、別のOS、別バージョンのDesktopの結果まで一致するとは限らない。また、同じ解像度と縮小方式でも、フォント・文字の測定・ブラウザーの描画に差があれば文字の位置・画像寸法・画素の差は残る。

フォント候補の指定については [フォント仕様](fonts.md) を参照する。
