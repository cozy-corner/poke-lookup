#!/usr/bin/env python3
"""指定サイズの pty でコマンドを起動し、端末へ出力された生バイトを記録する。

skim / viuer のようにカーソル制御を直接行うコードは、出力されたエスケープ
シーケンスを見ないと挙動を確認できないため。

usage: pty-capture.py --rows 30 --cols 100 --out capture.raw \
           --send $'\\r' --delay 0.6 -- ./target/debug/poke-lookup -s ピカ
"""

import argparse
import fcntl
import os
import pty
import select
import struct
import sys
import termios
import time


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--rows", type=int, default=30)
    p.add_argument("--cols", type=int, default=100)
    p.add_argument("--out", required=True)
    p.add_argument("--send", action="append", default=[],
                   help="子プロセスへ送るキー入力。複数指定で順に送信")
    p.add_argument("--delay", type=float, default=0.6,
                   help="各送信の前に待つ秒数")
    p.add_argument("cmd", nargs=argparse.REMAINDER)
    args = p.parse_args()

    cmd = args.cmd[1:] if args.cmd and args.cmd[0] == "--" else args.cmd
    if not cmd:
        p.error("コマンドが指定されていません")

    winsz = struct.pack("HHHH", args.rows, args.cols, 0, 0)

    pid, fd = pty.fork()
    if pid == 0:
        # 親が設定するより先に子が走り出すと、skim / viuer が既定のサイズを
        # 見てしまう。exec の前に子自身の端末へ適用する。
        fcntl.ioctl(0, termios.TIOCSWINSZ, winsz)
        os.execvp(cmd[0], cmd)
        os._exit(127)

    chunks: list[bytes] = []
    pending = list(args.send)
    next_send = time.time() + args.delay

    while True:
        now = time.time()
        if pending and now >= next_send:
            os.write(fd, pending.pop(0).encode())
            next_send = now + args.delay
        r, _, _ = select.select([fd], [], [], 0.1)
        if fd in r:
            try:
                data = os.read(fd, 65536)
            except OSError:
                break
            if not data:
                break
            chunks.append(data)
        elif not pending and now > next_send + 2.0:
            break

    os.close(fd)
    os.waitpid(pid, 0)

    raw = b"".join(chunks)
    with open(args.out, "wb") as f:
        f.write(raw)
    print(f"captured {len(raw)} bytes -> {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
