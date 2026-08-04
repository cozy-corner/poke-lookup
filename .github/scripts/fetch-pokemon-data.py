#!/usr/bin/env python3
"""
PokéAPIからポケモンデータを取得してnames.json形式で出力するスクリプト
"""

import json
import sys
import time
import hashlib
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from typing import List, Dict, Optional, Tuple
import urllib.request

# 既定の python-urllib UA は PokéAPI に 403 で弾かれる
USER_AGENT = 'poke-lookup-data-fetcher'

# フォルム取得は species の 3 倍近いリクエストになるため並列化する
FORM_WORKERS = 8

def fetch_json(url: str, max_retries: int = 3) -> dict:
    """URLからJSONデータを取得（リトライ機能付き）"""
    last_error = None

    request = urllib.request.Request(url, headers={'User-Agent': USER_AGENT})

    for attempt in range(max_retries):
        try:
            with urllib.request.urlopen(request) as response:
                return json.loads(response.read())
        except Exception as e:
            last_error = e
            if attempt < max_retries - 1:
                print(f'  Retry {attempt + 1}/{max_retries - 1}: {e}', file=sys.stderr)
                time.sleep(2)  # 2秒待機してリトライ
            continue

    # 全てのリトライが失敗した場合
    raise last_error

def find_localized_name(names: List[dict], lang: str) -> Optional[str]:
    """名前リストから指定言語の名前を取り出す

    PokéAPI は言語コードの大文字小文字を変えることがある（ja-Hrkt → ja-hrkt）。
    """
    for name_entry in names:
        if name_entry.get('language', {}).get('name', '').lower() == lang:
            return name_entry.get('name')
    return None

def get_name_pair(species_data: dict) -> Optional[Dict[str, str]]:
    """種データから日本語名と英名のペアを抽出"""
    names = species_data.get('names', [])
    pokemon_id = species_data.get('id')

    ja_name = find_localized_name(names, 'ja-hrkt')
    en_name = find_localized_name(names, 'en')

    if ja_name and en_name and pokemon_id:
        return {'ja': ja_name, 'en': en_name, 'id': pokemon_id}
    return None

def get_variety_refs(species_data: dict, base_ja: str) -> List[Tuple[str, str]]:
    """種データから非デフォルトの個体（フォルム）URLと種の日本語名を返す"""
    return [
        (base_ja, v['pokemon']['url'])
        for v in species_data.get('varieties', [])
        if not v.get('is_default')
    ]

def compose_ja(base_ja: str, form_ja: str) -> str:
    """種名とフォルム名から表示用の日本語名を組み立てる

    PokéAPI の form_names には 2 種類あり、メガフシギバナのように単体で
    完結するものと、アローラのすがたのようにフォルムの呼称だけのものがある。
    後者は種名が含まれないので合成する。
    """
    if base_ja in form_ja:
        return form_ja
    return f'{base_ja}（{form_ja}）'

def slug_to_en(slug: str) -> str:
    """英名を持たないフォルム（コライドン/ミライドンの各ビルド）用のフォールバック"""
    return ' '.join(part.capitalize() for part in slug.split('-'))

def fetch_form_entry(base_ja: str, pokemon_url: str) -> Optional[Dict[str, str]]:
    """個体URLからフォルムのエントリを作る

    pokemon と pokemon-form は id 体系が別なので、pokemon 経由で form を辿る。
    日本語名を持たないフォルム（トーテム個体など）は None を返す。
    """
    pokemon_data = fetch_json(pokemon_url)
    form_data = fetch_json(pokemon_data['forms'][0]['url'])

    form_ja = find_localized_name(form_data.get('form_names', []), 'ja-hrkt')
    if not form_ja:
        return None

    form_en = find_localized_name(form_data.get('names', []), 'en')

    return {
        'ja': compose_ja(base_ja, form_ja),
        'en': form_en or slug_to_en(pokemon_data['name']),
        'id': pokemon_data['id'],
        'slug': pokemon_data['name'],
        'species_slug': pokemon_data['species']['name'],
    }

def dedupe_en(form_entries: List[Dict[str, str]]) -> None:
    """英名が衝突するフォルムをスラッグ由来の名前に置き換える

    英名は検索結果の出力値であり、スプライト取得のキーでもあるため一意である必要がある。
    メガイエッサン♂/♀ のように PokéAPI 側で同じ英名を持つフォルムが存在する。
    """
    duplicates = {
        en for en, count in Counter(e['en'] for e in form_entries).items() if count > 1
    }

    for entry in form_entries:
        if entry['en'] in duplicates:
            entry['en'] = slug_to_en(entry['slug'])

def form_tokens(entry: Dict[str, str]) -> List[str]:
    """個体スラッグから種名を除いた、フォルムを表すトークン列"""
    species_tokens = set(entry['species_slug'].split('-'))
    return [t for t in entry['slug'].split('-') if t not in species_tokens]

def disambiguate_ja(entries: List[Dict[str, str]]) -> None:
    """日本語名が衝突するフォルムに英語の識別子を付けて一意にする

    メテノのりゅうせいのすがた（色違い 7 件）のように、英語では区別される
    フォルムが日本語では同名になることがある。検索キーは日本語名なので
    （search.rs の HashMap）、同名のままでは選び分けられない。
    識別子はグループ内で差分になるトークンだけを使う（Orange / Combat など）。
    """
    groups = defaultdict(list)
    for entry in entries:
        if 'slug' in entry:
            groups[entry['ja']].append(entry)

    for group in groups.values():
        if len(group) < 2:
            continue

        token_lists = [form_tokens(e) for e in group]
        shared = set.intersection(*(set(t) for t in token_lists))

        for entry, tokens in zip(group, token_lists):
            distinct = [t for t in tokens if t not in shared] or tokens
            label = ' '.join(t.capitalize() for t in distinct)
            # 「ロコン（アローラのすがた）」のように既に括弧付きなら中に足す
            if entry['ja'].endswith('）'):
                entry['ja'] = f'{entry["ja"][:-1]}・{label}）'
            else:
                entry['ja'] = f'{entry["ja"]}（{label}）'

def main():
    output_file = sys.argv[1] if len(sys.argv) > 1 else 'names.json'

    api_base = 'https://pokeapi.co/api/v2'

    print('Fetching Pokemon species data from PokéAPI...', file=sys.stderr)

    # まず総数を取得
    initial_data = fetch_json(f'{api_base}/pokemon-species?limit=1')
    total_count = initial_data['count']
    print(f'Total species count: {total_count}', file=sys.stderr)

    # 全件取得
    print(f'Fetching all {total_count} species...', file=sys.stderr)
    species_list = fetch_json(f'{api_base}/pokemon-species?limit={total_count}')

    entries = []
    variety_refs = []
    error_count = 0
    total = len(species_list['results'])

    print(f'Processing {total} species...', file=sys.stderr)

    for i, species_ref in enumerate(species_list['results'], 1):
        # 進捗表示（10件ごと）
        if i % 10 == 0 or i == total:
            print(f'Progress: {i}/{total}', file=sys.stderr)

        # APIレート制限対策（100ms待機）
        time.sleep(0.1)

        try:
            # 種データを取得
            species_data = fetch_json(species_ref['url'])

            # 名前ペアを抽出
            name_pair = get_name_pair(species_data)
            if name_pair:
                entries.append(name_pair)
                variety_refs.extend(get_variety_refs(species_data, name_pair['ja']))
        except Exception as e:
            print(f'Error: Failed to process {species_ref["name"]}: {e}', file=sys.stderr)
            error_count += 1
            continue

    # フォルム（アローラのすがた・メガシンカなど）は species ではなく
    # pokemon-form 側にしかないので、非デフォルト個体を辿って追加する
    print(f'\nProcessing {len(variety_refs)} forms...', file=sys.stderr)

    def process_variety(ref):
        base_ja, pokemon_url = ref
        try:
            return fetch_form_entry(base_ja, pokemon_url)
        except Exception as e:
            print(f'Error: Failed to process form {pokemon_url}: {e}', file=sys.stderr)
            return e

    with ThreadPoolExecutor(FORM_WORKERS) as executor:
        results = list(executor.map(process_variety, variety_refs))

    error_count += sum(1 for r in results if isinstance(r, Exception))
    form_entries = [r for r in results if isinstance(r, dict)]
    skipped = sum(1 for r in results if r is None)

    print(f'Forms with names: {len(form_entries)} (skipped {skipped} without a Japanese name)', file=sys.stderr)

    dedupe_en(form_entries)
    entries.extend(form_entries)

    disambiguate_ja(entries)
    for entry in form_entries:
        del entry['slug']
        del entry['species_slug']

    # 名前の抽出に失敗しても error_count は増えないため、空のまま
    # リリースされないようここで止める
    if not entries:
        print('\n❌ Failed: no entries were extracted', file=sys.stderr)
        sys.exit(1)

    # 日本語名は検索キー、英名は検索結果の出力値かつスプライト取得のキーなので
    # どちらも一意でなければならない（search.rs / sprite.rs の HashMap）
    for field in ('ja', 'en'):
        duplicates = [v for v, count in Counter(e[field] for e in entries).items() if count > 1]
        if duplicates:
            print(f'\n❌ Failed: duplicate {field} names: {duplicates}', file=sys.stderr)
            sys.exit(1)

    # エントリをソート
    entries.sort(key=lambda x: x['ja'])

    # 最終的なJSONを生成
    print('\nGenerating final JSON...', file=sys.stderr)

    output = {
        'schema_version': 1,
        'generated_at': datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),
        'count': len(entries),
        'entries': entries
    }

    # ファイルに出力
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(output, f, ensure_ascii=False, indent=2)

    print(f'\nSuccessfully generated {output_file} with {len(entries)} entries', file=sys.stderr)
    print(f'Generated at: {output["generated_at"]}', file=sys.stderr)

    # SHA256ハッシュを計算
    with open(output_file, 'rb') as f:
        sha256_hash = hashlib.sha256(f.read()).hexdigest()
    print(f'SHA256: {sha256_hash}', file=sys.stderr)

    # エラーがあった場合は失敗として終了
    if error_count > 0:
        print(f'\n❌ Failed: {error_count} errors occurred during processing', file=sys.stderr)
        sys.exit(1)

    print(f'\n✅ All {total} species and {len(variety_refs)} forms processed successfully', file=sys.stderr)

if __name__ == '__main__':
    main()