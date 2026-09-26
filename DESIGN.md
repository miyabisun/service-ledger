---
version: alpha
name: service-ledger / Sumi
description: 契約・利用サービスの台帳。Sumi familyの原本 (rust-svelte-template、2026-09-26参照) から取り込んだ製品固有の契約。
colors:
  primary: "#8b3a62"
  accent: "#8b3a62"
  accent-subtle: "rgba(139, 58, 98, 0.10)"
  surface: "#faf6ef"
  surface-raised: "#fffdf8"
  on-surface: "#3a2f28"
  muted: "#6f6257"
  border: "#e3d9c9"
  scrim: "rgba(58, 47, 40, 0.4)"
  link: "#14506e"
  danger: "#9c2b1d"
  danger-subtle: "#f9e9e4"
  wash-base: "#f6efe0"
  wash-raised: "#faf4ea"
  hover-1: "rgba(139, 58, 98, 0.08)"
  hover-2: "rgba(139, 58, 98, 0.14)"
typography:
  title:
    fontFamily: system-ui
    fontSize: 17px
    fontWeight: 600
    lineHeight: 1.3
  body:
    fontFamily: system-ui
    fontSize: 16px
    fontWeight: 400
    lineHeight: 1.6
  body-sm:
    fontFamily: system-ui
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: system-ui
    fontSize: 15px
    fontWeight: 500
    lineHeight: 1.2
  caption:
    fontFamily: system-ui
    fontSize: 12px
    fontWeight: 400
    lineHeight: 1.4
rounded:
  sm: 6px
  md: 8px
  lg: 12px
  full: 9999px
spacing:
  sp-1: 4px
  sp-2: 8px
  sp-3: 12px
  sp-4: 16px
  sp-5: 24px
components:
  app-header:
    backgroundColor: "{colors.wash-base}"
    textColor: "{colors.on-surface}"
    height: 48px
  sub-header:
    backgroundColor: "{colors.wash-raised}"
    textColor: "{colors.on-surface}"
    height: 40px
  hairline:
    backgroundColor: "{colors.border}"
    height: 1px
  card:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.md}"
    padding: 10px
  button:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.on-surface}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: 8px
  button-hover:
    backgroundColor: "{colors.hover-1}"
  button-pressed:
    backgroundColor: "{colors.hover-2}"
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.surface-raised}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: 8px
  button-quiet:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.muted}"
    rounded: "{rounded.sm}"
  icon-button:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.sm}"
    size: 36px
  input:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.on-surface}"
    typography: "{typography.body}"
    rounded: "{rounded.sm}"
    padding: 8px
  modal:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.lg}"
    padding: 16px
  modal-scrim:
    backgroundColor: "{colors.scrim}"
  radio-selected:
    backgroundColor: "{colors.accent-subtle}"
    rounded: "{rounded.sm}"
  link:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.link}"
  error-banner:
    backgroundColor: "{colors.danger-subtle}"
    textColor: "{colors.danger}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.sm}"
    padding: 8px
  spinner:
    textColor: "{colors.accent}"
    size: 18px
  button-danger:
    backgroundColor: "{colors.danger}"
    textColor: "{colors.surface-raised}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: 8px
  filter-chip:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.on-surface}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.sm}"
    padding: 6px
  filter-chip-selected:
    backgroundColor: "{colors.accent-subtle}"
  status-old:
    backgroundColor: "{colors.danger-subtle}"
    textColor: "{colors.danger}"
    typography: "{typography.caption}"
    rounded: "{rounded.full}"
    padding: 4px
  badge:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.muted}"
    typography: "{typography.caption}"
    rounded: "{rounded.full}"
    padding: 4px
---

# service-ledger

## Overview

利用者が契約・利用しているサービスの台帳。主な用途は、引越し後に登録住所が
旧住所のまま残っているサービスを見つけ、住所変更ページを開き、済んだら状態を
変更済みにすることである。登録は AI エージェント・別アプリ・Bitwarden 取り込みが行い、
画面は一覧・検索・状態での絞り込み・編集・削除と、住所変更ページへのリンクを持つ。
画面からの新規登録は置かない。

Sumi family の原本 (rust-svelte-template の DESIGN.md、2026-09-26 参照) を取り込み、
本書を製品の正とする。原本の更新は差分を確認して明示的に採用する。
内容を主役にし、情報を足す前に既存の表示で分かるか、操作時だけ示せば足りるかを判断する。

## Colors

暗色は Sumi、明色は Kinari とする。Sumi から設計し、両方で検証する。Washi は採用しない。
色は `client/src/global.sass` の CSS 変数を使う。frontmatter は Kinari の値を持つ。
製品のアクセントは梅紫 (Kinari #8b3a62 / Sumi #e58fb8) とする。

| 役割 | Kinari | Sumi |
|---|---|---|
| surface | #faf6ef | #191919 |
| surface-raised | #fffdf8 | #232323 |
| on-surface | #3a2f28 | #e6e6e6 |
| muted | #6f6257 | #9a9a9a |
| border | #e3d9c9 | #333333 |
| accent / primary | #8b3a62 | #e58fb8 |
| accent-subtle | rgba(139,58,98,.10) | rgba(229,143,184,.15) |
| link | #14506e | #7fdbff |
| danger | #9c2b1d | #ff6b6b |
| danger-subtle | #f9e9e4 | #3a1a1a |
| scrim | rgba(58,47,40,.4) | rgba(0,0,0,.6) |
| wash-base | #f6efe0 | #232323 |
| wash-raised | #faf4ea | #191919 |
| hover-1 | rgba(139,58,98,.08) | #333333 |
| hover-2 | rgba(139,58,98,.14) | #3d3d3d |

`primary` はlint用に `accent` と同値を持つ。製品の色名はaccentを使う。
背景・補助情報・通常操作は無彩色を基本とする。帯にはwash、ホバーにはhoverの変数を使う。
アクセントは主操作・フォーカス・選択中の絞り込み・処理中のスピナーに使う。
塗りつぶす主操作は画面に最大1つとする。編集画面の「保存」がそれにあたる。

登録住所の状態はデータ色として操作の色と分ける。
「旧住所」は対応が必要な状態なので danger と danger-subtle の小さなバッジで示す。
「不明」「変更済み」は muted の枠付きバッジとし、色を足さない。状態は常に文字で示す。
削除の確定ボタンだけが danger で塗る。通常文字は両テーマでWCAG AAの4.5:1以上を保つ。

### テーマの状態

`:root` をSumiの値と `color-scheme: dark` にする。
`data-theme="light"` でKinari、`data-theme="dark"` でSumiを明示指定する。
自動では属性と保存キー `service-ledger:theme` を削除し、OSの `prefers-color-scheme` に従う。
Kinariの明示指定とOS委任は同じSass mixinから出力し、`color-scheme: light` を設定する。
設定はlocalStorageへ保存し、初回描画前に適用する。

## Typography

書体はsystem-uiとし、Webフォントを追加しない。サイズ・太さ・行高はfrontmatterの5役割を使う。
本文と入力は16px、補助文は14px、ラベルは15px、注記は12pxを使う。
サービス名は折り返して全文を示す。詳細の見出しだけ1行で省略する。

## Layout

縦方向はページの通常スクロールを使い、mainに独立した縦スクロール領域を作らない。
ブレークポイントは768px。本文の列は中央配置で最大720pxとする。
左右の余白は12px、上下は狭幅16px・広幅24pxとする。幅320px以上でページを横にはみ出させない。
余白は4/8/12/16/24pxを使う。

- app headerは全幅・高さ48px・stickyとし、wash-baseと1pxの下境界を使う。
  左に一覧へ戻るアプリ名「サービス台帳」、右に36pxのメニューボタンを置く。
- 編集のsub-headerは高さ40pxで、wash-raisedと1pxの下境界を使う。
  サービス名だけを1行で示し、ボタンやリンクを置かない。header下へ固定する。

### 一覧 (`/`)

最初の画面に検索欄、状態の絞り込み、サービスの一覧を置く。件数や説明の帯は追加しない。

1. ラベル付きの検索欄「名前・URL・メモで検索」。入力中に一覧を絞り込み、Enterは不要。
   大文字小文字を区別しない部分一致で、名前・両URL・根拠・メモを対象にする。
2. 状態の絞り込みは1行のチップ群「すべて・旧住所・不明・変更済み」とする。
   各チップは検索語に一致する件数を持つ。選択中はaccent-subtleの面とaccentの枠で示し、
   `aria-pressed` でも伝える。チップは高さ36px・左右8px・間隔4pxで、375px幅では1行に収め、
   320px幅では折り返してよい。
3. 一覧はカードの1列で、名前順とする。カードは次の2領域からなる。
   - 本体リンク: 1行目に名前、2行目の左端に状態バッジ、続けて注記サイズで変更方法と
     Webサイトのホスト。バッジを左端に固定し、住所変更リンクの有無で位置を変えない。
     2行目は1行に収め、はみ出す補助情報は末尾を省略する (全文は編集画面にある)。
     押すと編集画面へ移る。
   - 住所変更ページがあれば、右端に「住所変更」リンク (外部リンクのアイコン付き) を置く。
     新しいタブで開き、`rel="noopener noreferrer"` を付ける。無い場合は領域を予約しない。

検索語と絞り込みはURLのクエリ (`?q=…&status=old`) に反映し、再読込と編集からの戻りで保つ。
一覧の `data-state` はloading/empty/error/successとする。
空は「サービスはまだ登録されていません」と、API・MCP・Bitwarden 取り込みで登録することを示す。
検索の不一致は「一致するサービスがありません」とし、データが空の場合と区別する。
取得失敗はdanger色の説明と再試行ボタンを示す。タブの再表示で再取得し、検索語を保つ。

### 編集 (`/services/{id}`)

利用者は住所変更ページを開いて手続きし、戻って状態を「変更済み」にして保存する。
その経路が最短になるよう、上から次の順に置く。

1. 登録住所の状態 (旧住所のまま・変更済み・不明) のラジオ。
2. 住所変更ページ URL の入力と、値があるときの「開く」リンク (新しいタブ)。
3. 変更方法 (一括手段・会員ページ・電話・書面・未確認) のラジオ。
4. 名前、Web サイト URL (「開く」リンク付き)、根拠、メモ (複数行)。
5. 「保存」(主ボタン) と「キャンセル」。その下に登録・更新日時を注記で示す。
6. 区切りの下に「削除」ボタン。確認のモーダルで名前を示し、「削除する」(danger) で確定する。

保存・キャンセル・削除の後は、直前の一覧 (検索語と絞り込みを含む) へ戻る。
保存に失敗したら入力を保ったまま、フォームの上にdanger-subtleの帯で理由を示し、帯へフォーカスする。
重複 (同じ名前・同じWebサイト) の場合は、帯に登録済みのサービスを開くリンクを置く。
URLは `type="url"`、名前と根拠は必須とし、ブラウザの入力検証を先に使う。
存在しないIDは「サービスが見つかりません」と一覧へのリンクを示す。

## Elevation & Depth

階層は面の濃淡と1pxの境界で示す。
影はメニューとモーダルの `0 8px 32px rgba(0,0,0,.25)` だけに使う。
フォーカスは共通の `:focus-visible` に2pxのaccent色の輪郭と2pxの間隔を設ける。

## Shapes

角丸は小部品6px、カード8px、モーダルとメニュー12pxとする。
9999pxは状態のバッジだけに使う。円形ボタンを作らない。

## Components

### アイコン

`client/src/lib/Icon.svelte` を辞書の正とする。絵文字や文字記号をアイコンの代わりに使わない。
SVGは24×24、currentColorの2px線、丸い端と角、塗りなしとする。サイズは1.2em。
列挙には `ICON_NAMES` を使う。本製品は `external-link` を追加した (原本への採用候補)。

### メニューとテーマ設定

メニューは右上のボタンに接するドロップダウンとする。
上端をheader下端、右端をボタン右端へそろえる。最小幅180px、1pxの枠と12pxの角丸を使う。
背景はsurface-raised、項目は上下8px・左右12pxの全幅の行とする。scrimは付けない。
外側クリックとEscで閉じ、フォーカスをメニューボタンへ戻す。開閉は `aria-expanded` に反映する。
項目は「テーマ設定」だけとする。
テーマ設定は中央のモーダルで、自動・ライト・ダークの3つのラジオにmonitor/sun/moonを使う。
選択は即時反映し、モーダルを閉じない。閉じるボタン・Esc・scrimで閉じる。

### カードとバッジ

カードはsurface-raised、1pxの枠、8pxの角丸、10pxの内側余白、8pxの間隔を使う。
本体リンクのホバーはhover-1とする。「住所変更」リンクはlink色の文字とし、
左に1pxの境界を置いて本体と分ける。操作領域は高さ44px以上とする。
状態バッジは注記サイズ、上下2px・左右8px、1pxの枠と9999pxの角丸とする。

### 入力と操作部品

- 通常ボタンはsurface-raised、1px枠、6px角丸、上下8px・左右14pxの余白とする。
  ホバーはhover-1、無効時は不透明度50%でポインターを付けない。
- 主ボタンはaccentで塗り、文字はsurface-raisedとする。保存中は無効にする。
- 入力欄はsurface、1px枠、6px角丸、本文サイズとする。
  ラベルは上にmutedの注記で置き、フォーカスはaccent色の枠と共通リングを使う。
- ラジオは枠付きの行として並べ、選択はaccent-subtleの面とaccentの枠で示す。
- モーダルは中央配置、12px角丸、16px余白とscrimを使い、最大80dvhで内部スクロールする。
- 動きは150ms以下の変化とスピナーに限り、`prefers-reduced-motion: reduce` で止める。

## Verification

実装はSassの字下げ構文を使い、normalize.cssを先に読み込む。
変数の接頭辞は色が `--c-*`、余白が `--sp-1..5`、文字が `--fs-xs..xl`、角丸が `--radius-*` とする。
`designmd lint` は形式を検査する。UIへの適用は `client/e2e/` のPlaywrightで実ブラウザを測る。
E2Eは一時DBで起動した本物のサーバーへ架空のサービスを登録して操作する。

- 1440×900・375×812・320×640の明暗で、ページの横はみ出し0px、一覧の内部縦スクロール0px。
- 100件で、1440×900と375×812は最初のカード上端が210px以内・完全に見えるカードが8件以上、
  320×640は260px以内・4件以上。320pxでは絞り込みが2行に折り返し、住所変更リンクの分だけ名前も折り返す。
  基準はheader・検索欄・絞り込みを含む実測に基づく。情報を足して下回る場合は重複を先に見直す。
- Sumiの背景はrgb(25,25,25)、Kinariではrgb(250,246,239)。明示指定がOSより優先される。
- 検索・絞り込みがURLに残り、編集から保存して戻ると同じ条件の一覧に戻る。
- 「住所変更」リンクは `target="_blank"` と `rel="noopener noreferrer"` を持ち、外部URLを指す。
- 長い名前、空、読込、失敗と再試行、不一致、保存失敗と重複、削除の確認をキーボードでも通す。
