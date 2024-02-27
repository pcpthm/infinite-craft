from datetime import datetime, timezone
import time
from typing import Any, Optional
import logging
import sys
import sqlite3
import requests
from tokenizers import Tokenizer
from tqdm import tqdm


sess = None
tokenizer = None


def get_pair_request_raw(first: str, second: str) -> Any:
    global sess
    if not sess:
        sess = requests.Session()
        sess.headers["User-Agent"] = "Mozilla/5.0 (X11; Linux x86_64; rv:122.0) Gecko/20100101 Firefox/122.0"

    retries = 0
    while True:
        try:
            res = sess.get(
                "https://neal.fun/api/infinite-craft/pair",
                params=dict(first=first, second=second),
                headers={"Referer": "https://neal.fun/infinite-craft/"})
            res.raise_for_status()
            return res.json()
        except Exception as e:
            retry_time = 2 ** min(retries, 10)
            retries += 1
            logging.warning("request error: %r; retry %d in %.1fs", e, retries, retry_time)
            time.sleep(retry_time)
            continue


def get_pair(first: str, second: str, force_request: bool = False) -> Optional[str]:
    if first > second:
        second, first = first, second

    existing = conn.execute("select result from pair where first = ? and second = ?", (first, second)).fetchone()
    if existing is not None and not force_request:
        return existing[0]

    json = get_pair_request_raw(first, second)
    created_at = datetime.now(timezone.utc)

    logging.debug("pair(%r, %r) = %r", first, second, json)

    assert isinstance(json, dict)
    assert isinstance(json["result"], str) and isinstance(json["emoji"], str) and isinstance(json["isNew"], bool)

    result, emoji, is_new = json["result"], json["emoji"], json["isNew"]
    if result == "Nothing" and emoji == "":
        result = None

    conn.execute("insert or replace into pair (first, second, result, created_at) values (?, ?, ?, ?)",
                 (first, second, result, created_at))

    if existing is not None and existing[0] != result:
        logging.info("pair(%r, %r) = %r != %r", first, second, result, existing[0])

    if result is not None:
        if conn.execute("insert or ignore into item (name, emoji, is_new, created_at) values (?, ?, ?, ?)",
                        (result, emoji, is_new, created_at)).rowcount != 0:
            logging.debug("item: pair(%r, %r) = %r, is_new=%r", first, second, result, is_new)

    conn.commit()
    return result


def tokenize(name: str) -> int:
    existing = conn.execute("select count from tokenize where name = ?", (name,)).fetchone()
    if existing is not None:
        return existing[0]

    global tokenizer
    tokenizer = tokenizer or Tokenizer.from_pretrained("oobabooga/llama-tokenizer")

    count = len(tokenizer.encode(name, add_special_tokens=False).ids)
    logging.debug("tokenize(%r) = %d", name, count)

    conn.execute("insert or replace into tokenize (name, count) values (?, ?)", (name, count))
    conn.commit()
    return count


def main():
    try:
        for line in sys.stdin:
            cmd, rest = line.strip().split(":", maxsplit=1)
            if cmd == "pair":
                first, second = rest.split("=")
                result = get_pair(first, second)
                print(result or "", flush=True)
            elif cmd == "tokenize":
                count = tokenize(rest)
                print(count, flush=True)
    except BrokenPipeError:
        pass
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)

    conn = sqlite3.Connection("infinite-craft.db")
    conn.executescript("""
begin;
    create table if not exists pair(first text not null, second text not null, result text null, created_at datetime null, primary key (first, second));
    create table if not exists item(name text primary key not null, emoji text not null, is_new integer not null, created_at datetime null);
    create table if not exists tokenize(name text primary key not null, count integer not null);
commit;
    """)

    item_set = set()
    for name, in conn.execute("select name from item"):
        item_set.add(name)

    to_pair = []
    for first, second, result in conn.execute("select first, second, result from pair"):
        if result is not None and result not in item_set:
            item_set.add(result)
            to_pair.append((first, second))

    for first, second in tqdm(to_pair):
        get_pair(first, second, True)

    # main()
