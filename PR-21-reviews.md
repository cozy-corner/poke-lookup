# PR #21 Reviews

feat: スプライト確認画面にポケモン情報を表示する

---

## PR Reviews

### Review by @coderabbitai[bot] (COMMENTED) - 2026-08-09T02:28:27Z
> Actionable comments posted: 2
>
> 🧹 Nitpick comments (2)
>
> **src/info.rs 454-489 — Add a test for the `ja` / `ja-Hrkt` description fallback.**
> モックに `species` が無いため `fetch_description` が一度も実行されず、言語優先順位と `clean_flavor` が未検証。species パスに2つ目のモックを足せばテスト可能。
>
> **src/info.rs 173-195 — Consider reusing the already-loaded dictionary instead of re-reading `names.json`.**
> `PokemonInfoService::new` が `DataLoader::load_dictionary()` を再読込。`SearchService` が既に保持しており、`interactive.rs:108` で eager 構築されるためスプライト未表示でも読み込みが走る（CryService でも同じ懸念）。既存 id_map/辞書を受け取る別コンストラクタ、または遅延構築を検討。

---

## Review Comments (on specific code lines)

- @coderabbitai[bot] on src/info.rs#L254 (未解決) — 🟡 Minor / Security:
  > **Build the species URL locally instead of following the URL from the response body.**
  > `fetch_description` がリモートの `/pokemon` 応答由来の `species_url` を GET しており、リクエスト先がリモート制御データに依存。ID はローカルで既知なので `base_url + /pokemon-species/{id}` で組み立てれば全リクエストが base_url 内に収まり、`species`/`UrlRef` モデルも削除できる。

- @coderabbitai[bot] on src/info.rs#L359 (未解決) — 🟡 Minor / Maintainability:
  > **Correct the stale comment about clamping.**
  > コメントは「value は既に STAT_MAX にクランプ済み」と述べるが、実際のクランプは次行の `.min(STAT_MAX)` で行われる。将来の読者が `.min` を外すと 367 行の減算がアンダーフローするため、コメントを実際の保証内容に修正すべき。

---

## PR-level Comments

- @coderabbitai[bot] - 2026-08-09T02:26:02Z:
  > 📝 Walkthrough（自動生成のサマリ／シーケンス図）。指摘なし。

---

## Summary

- **PR-level comments**: 1件（Walkthrough、指摘なし）
- **Reviews**: 1件（COMMENTED、Nitpick 2件）
- **Review comments (未解決)**: 2件
  - **Primary comments (in_reply_to_id == null)**: 2件
  - **Reply comments (in_reply_to_id != null)**: 0件
- **Resolved (除外)**: 0件
