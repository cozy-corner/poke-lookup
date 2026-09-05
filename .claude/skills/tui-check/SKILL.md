---
name: tui-check
description: poke-lookup の対話画面（skim ベースの候補選択）を tmux で実際に起動・操作して動作確認する。ローマ字やタイプ名での絞り込み、Ctrl+D / Ctrl+U の半ページ送り、ESC やキャンセルの挙動など、cargo test では検証されない対話層を触ったときは必ず使う。「動作確認して」「実際に動かして」と言われたときはもちろん、interactive.rs / search.rs / romaji.rs / pokemon_type.rs を変更したあと動作を主張する前にも使うこと。ユーザーに手で試してもらう前に、まず自分で確認する。
---

# poke-lookup の対話画面を tmux で動作確認する

`cargo test` は skim を起動しない（`src/interactive.rs` のテストにも「実際のskimなしで動作確認」とある）。
キーを打ったときの挙動は自動テストの範囲外なので、そこを人間に丸投げしないための手順。

## 起動

```bash
TMUX=$(whence -p tmux 2>/dev/null || command -v tmux)   # tmux は zsh 関数のことがあるのでフルパス
S=pk-check; OUT=$(mktemp)

$TMUX kill-session -t $S 2>/dev/null
$TMUX new-session -d -s $S -x 100 -y 24 "poke-lookup > $OUT"
$TMUX set-option -t $S remain-on-exit on   # 終了コードを読むのに必要
```

選択結果の英名は画面ではなく標準出力に出るのでファイルへ。表示行数が変わると半ページの移動量も変わるのでサイズは固定する。

## 待つ・打つ・読む

```bash
wait_for() {  # 固定 sleep は負荷で落ちるので文字列が出るまで待つ
  local pat="$1" i=0
  while [ $i -lt 50 ]; do
    $TMUX capture-pane -t $S -p | grep -q "$pat" && return 0
    sleep 0.2; i=$((i+1))
  done
  echo "timeout: $pat" >&2; $TMUX capture-pane -t $S -p >&2; return 1
}

wait_for 'ポケモンを選択'
$TMUX send-keys -t $S -l 'ほのお'     # -l = リテラル。日本語・ローマ字はこれ
$TMUX send-keys -t $S C-d             # 制御キーは -l なし
$TMUX capture-pane -t $S -p           # -e を足すと反転などの ANSI が残る
```

画面はこう見える:

```
  ロコン → Vulpix
> ブビィ → Magby
  107/1336                                            0/0
ポケモンを選択: ほのお
```

`107/1336` が絞り込み後/全体の件数、右端 `0/0` のスラッシュ左が**カーソルのインデックス**。
移動の検証はこの数字を見るのが確実。

## 落とし穴: リストは下から積み上がる

`>` が最下行にあるときそれがインデックス 0。そのため**初期状態で Ctrl+D を押しても何も起きない**
（不具合ではなく境界にいるだけ）。Ctrl+U で進めてから Ctrl+D で戻す順で確かめる。

確認済みの正しい挙動（`-y 24`、リスト5行）: `Ctrl+U ×4` で index 0→2→4→6→8、`Ctrl+D ×2` で 8→6→4。
半ページ = 表示行数の半分 = 2件、上下対称なら正常。

## 終了コードと後片付け

```bash
$TMUX list-panes -t $S -F '#{pane_dead_status}'   # remain-on-exit on が前提
$TMUX kill-session -t $S 2>/dev/null              # 失敗しても必ず消す
```

期待値は README の表（0 成功 / 1 エラー / 2 候補なし / 130 キャンセル）。
ただし引数ありのパス（`src/main.rs` の `search_pokemon`）はキャンセルも 2 を返す。

## 注意

- デフォルトビルドは確定時に鳴き声（約1秒ブロック）とスプライト表示が入り、スプライト時は ESC の意味が変わる（再選択に戻る）。対話ロジックだけ見たいなら `--no-default-features` ビルドが素直。スプライトの絵が正しいかは pane からは判定できないので人間に任せる。
- 候補数（`1336` や `107`）は月次更新で変わる。絶対値をアサートせず、件数が減ったか・インデックスが2ずつ動いたかで見る。固定したいなら `--dict <PATH>`。
- 報告は推測でなく読み取った数字で。「動くはず」ではなく「`107/1336` になった」「index が 0→2→4」「終了コードは 2」と書く。
