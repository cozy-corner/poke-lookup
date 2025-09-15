#!/usr/bin/env python3
"""
PokéAPIからポケモンデータを取得してnames.json形式で出力するスクリプト
"""

import json
import sys
import time
import hashlib
from datetime import datetime, timezone
from typing import List, Dict, Optional
import urllib.request

def fetch_json(url: str) -> dict:
    """URLからJSONデータを取得"""
    with urllib.request.urlopen(url) as response:
        return json.loads(response.read())

def get_name_pair(species_data: dict) -> Optional[Dict[str, str]]:
    """種データから日本語名と英名のペアを抽出"""
    names = species_data.get('names', [])

    ja_name = None
    en_name = None

    for name_entry in names:
        lang = name_entry.get('language', {}).get('name')
        if lang == 'ja-Hrkt':
            ja_name = name_entry.get('name')
        elif lang == 'en':
            en_name = name_entry.get('name')

    if ja_name and en_name:
        return {'ja': ja_name, 'en': en_name}
    return None

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
        except Exception as e:
            print(f'Warning: Failed to process {species_ref["name"]}: {e}', file=sys.stderr)
            continue

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

if __name__ == '__main__':
    main()