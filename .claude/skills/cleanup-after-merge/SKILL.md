---
name: cleanup-after-merge
description: poke-lookup の PR がマージされた後の後片付け（main の更新、作業ワークツリーとブランチの削除、make install での再インストール）を行う。「後片付け」「片付けて」「マージしたので掃除」「ワークツリー消して」「入れ直して」のように、マージ済み PR の残骸を整理したい合図があれば必ず使う。PR をマージした直後で明示的な指示がなくても、この手順を提案する材料として参照してよい。マージ自体はこのスキルの対象外。
---

# poke-lookup: マージ後の後片付け

PR がマージされると、ローカルには 3 つの「遅れ」が残る: main が古いまま、作業ワークツリーとブランチが残る、`~/.cargo/bin/poke-lookup` が古いビルドのまま。この 3 つを揃えるのがこのスキル。

順番には意味がある。main を先に更新しないと、ワークツリーを消した後で「マージ結果を含まないコード」をインストールしてしまう。

## 手順

### 0. 対象ブランチとマージ済みかの確認

片付ける対象がユーザーの発言から自明でなければ、まず現状を見る:

```bash
git gtr list
gh pr list --state merged --limit 5
```

対象 PR が本当にマージ済みかを確認する。未マージのものを消すと作業が消えるので、ここは飛ばさない:

```bash
gh pr view <PR番号> --json state,mergedAt -q '.state, .mergedAt'
```

`MERGED` でなければ止めて、ユーザーに確認する。

### 1. main を最新にする

メインチェックアウト（`/Users/sasakitakashinanji/code/poke-lookup`）で実行する。ワークツリーの中にいると次のステップでそのワークツリーを消せない:

```bash
git checkout main && git pull
```

### 2. ワークツリーとブランチを削除

`git gtr` は git のエイリアスではなく git-worktree-runner。`--delete-branch` はローカルブランチも消す:

```bash
git gtr rm <ブランチ名> --delete-branch --yes
```

`git gtr list` に残る `(detached) .../poke-lookup-worktrees` はワークツリー置き場のディレクトリ自体で、消す対象ではない。毎回出るので気にしなくてよい。

複数のマージ済みブランチが溜まっている場合は 1 本ずつ確認しながら消す。`[gone]` になったローカルブランチをまとめて掃除したいだけなら `commit-commands:clean_gone` の方が向いている。

### 3. 再インストール

`~/.cargo/bin/poke-lookup` はマージ前のビルドのままなので、マージ結果を実際に使うには入れ直す必要がある。メインチェックアウト（main が最新の状態）で:

```bash
make install
```

`Replacing /Users/.../.cargo/bin/poke-lookup` と出れば置き換わっている。ビルドに 30 秒ほどかかるのでタイムアウトは長めに取る。

## 報告

やったことを事実ベースで短く伝える。特に:

- マージ済みコミットが main に入っているか（`git log --oneline -1`）
- 消したワークツリー/ブランチ名
- インストールが置き換わったか

対話画面（skim）の挙動はこちらからは自動確認できないので、動作確認が要る変更なら「起動して確かめてほしい」と添える。確認していないことを確認したように書かない。
