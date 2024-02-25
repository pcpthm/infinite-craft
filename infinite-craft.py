from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
from typing import Any, Optional
from requests.adapters import HTTPAdapter
import logging
import sys
import time
import requests
import sqlite3


def get_pair_request_raw(first: str, second: str) -> Any:
    res = sess.get(
        "https://neal.fun/api/infinite-craft/pair",
        params=dict(first=first, second=second),
        headers={"Referer": "https://neal.fun/infinite-craft/"})
    res.raise_for_status()
    return res.json()


def get_pair_request(first: str, second: str) -> Optional[str]:
    if first > second:
        second, first = first, second

    retry = 0
    while True:
        try:
            json = get_pair_request_raw(first, second)
            created_at = datetime.now(timezone.utc)
            break
        except Exception as e:
            logging.error("pair(%r, %r) failed: %s", first, second, e)
            time.sleep(2 ** min(retry, 10))
            retry += 1

    assert isinstance(json, dict)
    assert isinstance(json["result"], str)
    assert isinstance(json["emoji"], str)
    assert isinstance(json["isNew"], bool)

    is_nothing = json["result"] == "Nothing" and json["emoji"] == ""
    result = None if is_nothing else json["result"]

    logging.info("pair(%r, %r) = %r", first, second, result)
    if conn.execute("insert or ignore into pair (first, second, result, created_at) values (?, ?, ?, ?)",
                    (first, second, result, created_at)).rowcount < 1:
        logging.warning("ignored insert into pair(%r, %r) = %r", first, second, result)

    if json["isNew"]:
        logging.info("discovery: %r", result)
        conn.execute("insert or ignore into discovery (result, created_at) values (?, ?)",
                     (result, created_at))

    conn.commit()
    return result


def get_pair(first: str, second: str) -> Optional[str]:
    if first > second:
        second, first = first, second

    r = conn.execute("select result from pair where first = ? and second = ?",
                     (first, second)).fetchone()
    if r is not None:
        return r[0]

    result = get_pair_request(first, second)
    return result


DATA_DIR = Path(os.getenv("IC_DATA_DIR") or ".")


def main():
    try:
        for line in sys.stdin:
            cmd, rest = line.strip().split(":", maxsplit=1)
            if cmd == "pair":
                first, second = rest.split("=")
                result = get_pair(first, second)
                print(result or "", flush=True)
            elif cmd == "dump":
                dump_db()
            elif cmd == "tokenize":
                print(len(tokenize_str(rest)), flush=True)
    except BrokenPipeError:
        pass
    except KeyboardInterrupt:
        pass


def dump_db():
    for first, second, result in conn.execute("select first, second, result from pair"):
        print(first, second, result or "", sep="=")

    with (DATA_DIR / "recipes_optimals.json").open("r") as f:
        for key, result in json.load(f).items():
            first, second = key.split("\t")
            if result != "Nothing":
                print(first, second, result, sep="=")

    print(flush=True)


llama_tokenizer = None


def tokenize_str(text: str) -> list[int]:
    from transformers import AutoTokenizer
    global llama_tokenizer
    if llama_tokenizer is None:
        llama_tokenizer = AutoTokenizer.from_pretrained("TheBloke/Llama-2-7B-AWQ")
    return llama_tokenizer(text, add_special_tokens=False).input_ids


def validate_depth():
    ref_depth: dict[str, int] = {}
    with (DATA_DIR / "best_recipes_depth_10.txt").open("r") as f:
        name_re = re.compile(r'^\d+: (.+):$')
        it = iter(f)
        while line := next(it, None):
            line = line.strip()
            if not line:
                continue
            match = name_re.match(line)
            assert match
            name = match[1]
            depth = 0
            while next(it).strip():
                depth += 1
            ref_depth[name] = depth

    with Path("depth.log").open("r") as f:
        count = 0
        it = iter(f)
        remain = dict(ref_depth.items())
        max_depth = 0
        while line := next(it, None):
            name, depth_str, count_str = line.strip().split("=")
            depth = int(depth_str)
            max_depth = max(max_depth, depth)

            if name in remain:
                del remain[name]

            rd = ref_depth.get(name)
            if rd != depth:
                print(f"{name}: {rd} != {depth}")

            for _ in range(int(count_str)):
                next(it)
            next(it)
            count += 1

        for name, rd in remain.items():
            if rd <= max_depth:
                print(f"{name}: {rd} != None")
        print(f"Validated {count} lines")


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)

    sess = requests.Session()
    sess.mount("https://", HTTPAdapter(max_retries=10))
    sess.headers["User-Agent"] = "Mozilla/5.0 (X11; Linux x86_64; rv:122.0) Gecko/20100101 Firefox/122.0"

    conn = sqlite3.Connection("infinite-craft.db")
    conn.executescript("""
        begin;
        create table if not exists pair(first text not null, second text not null, result text null, created_at datetime null, primary key (first, second));
        create table if not exists discovery(result text primary key, created_at datetime not null);
        commit;
    """)

    if len(sys.argv) > 1:
        if sys.argv[1] == "validate-depth":
            validate_depth()
        else:
            print("Unknown command")
    else:
        main()
