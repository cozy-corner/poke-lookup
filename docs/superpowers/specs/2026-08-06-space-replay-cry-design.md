# スプライト確認画面で Space による鳴き声リプレイ

## 背景

`--play-cry` (`-c`) を付けると、選択確定時に鳴き声が1回だけ再生される
(`src/interactive.rs:237`)。スプライト機能付きビルドでは、その直後に
スプライトと `[Enter] 確定 / [ESC] 再選択` の確認画面が出て、キー入力を待つ
(`src/interactive.rs:263-299`)。

この画面に留まっている間、鳴き声をもう一度聞く手段がない。1秒足らずで鳴り終わる
ため聞き逃しやすく、確定するかどうかを判断する材料として聞き直したい。

## 要件

スプライト確認画面で Space を押すと、表示中のポケモンの鳴き声を再生し直す。
画面はそのまま維持し、確定も再選択もしない。

## スコープ外

- **skim の候補リスト上での試聴**。カーソル上の候補を Space で鳴らす案もあるが、
  skim 0.10 の `bind` は Rust のコールバックを呼べず、シェルコマンドを起動する
  `execute-silent` か `refresh-preview` を経由する迂回が必要になる。今回はやらない。
- **`-c` なしでの Space 再生**。`CryService` は辞書を再読み込みするため
  `-c` 指定時のみ初期化している (`src/interactive.rs:115-133`)。Space 押下時の
  遅延初期化は、初回に辞書読み込みと音声デバイスオープンの遅延が可視化される。
  `-c` なしでは Space は無反応のままとする。

## 設計

変更は `src/interactive.rs` の `show_sprite_with_navigation` に閉じる。

### キーループ

`Enter` / `Esc` の分岐に Space を追加する。ループは抜けず raw mode も維持する。

```rust
#[cfg(feature = "cries")]
KeyCode::Char(' ') => self.play_cry_if_enabled(english_name),
```

`cries` 無効ビルドでは既存の `_ => {}` に落ち、Space は無視される。

### ヒント行

現在の `[Enter] 確定  [ESC] 再選択` に `[Space] もう一度鳴らす` を足す。ただし
`cry_service` が `Some` のときだけ。`-c` なしで鳴らないキーを案内しない。

文字列の組み立ては小さい関数に切り出し、そこだけをテストする。キーループ本体は
`crossterm::event::read()` に直結していて自動テストできない（既存コードも同様）。

### 既存の仕組みに乗る点

- `CryService::play_cry_for_pokemon` は先頭で `stop()` してから鳴らし直す
  (`src/cry.rs:136`)。Space を連打しても音が重ならない。
- ESC で再選択した場合、次の確定時の `play_cry_if_enabled` が `stop()` を通るため、
  Space で鳴らした音も止まる。既存フローの変更は不要。
- 確定時は `main.rs` の `selector.wait_for_cry()` が鳴り終わりを待つ。Space で
  始めた再生も同じ `playing` ハンドルに入るので、追加の待ち合わせは要らない。

## テスト

- ヒント文字列の組み立て関数: `cry_service` の有無で Space の案内が出る／出ないこと。
- 手動確認: `cargo run --features sprites,cries -- フシギ -s -c` で候補を選び、
  スプライト画面で Space を数回押して鳴ること、連打で音が重ならないこと、
  Enter / ESC が従来どおり動くことを確認する。

## ドキュメント

README の以下を更新する。

- 鳴き声再生の節 (145-167行): スプライト画面で Space により再生し直せること
- トラブルシューティングの操作方法 (233-239行): Space の追記
