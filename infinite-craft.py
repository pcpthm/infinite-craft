from datetime import datetime, timezone
import heapq
import os
import random
import re
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
model = None
last_request = 0


def get_pair_request_raw(first: str, second: str) -> Any:
    global sess, last_request
    if not sess:
        sess = requests.Session()
        sess.headers["User-Agent"] = "Mozilla/5.0 (X11; Linux x86_64; rv:123.0) Gecko/20100101 Firefox/123.0"
        sess.headers["Alt-Used"] = "neal.fun"
        sess.headers["Sec-Fetch-Dest"] = "empty"
        sess.headers["Sec-Fetch-Mode"] = "cors"
        sess.headers["Sec-Fetch-Site"] = "same-origin"

    retries = 0
    while True:
        try:
            now = time.monotonic()
            time.sleep(max(0, 0.5 - (now - last_request)))
            last_request = now
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

    if (len(first) > 30 or len(second) > 30) and not force_request:
        return None

    existing = conn.execute("select result from pair where first = ? and second = ?", (first, second)).fetchone()
    if existing is not None and not force_request:
        return existing[0]

    json = get_pair_request_raw(first, second)
    created_at = datetime.now(timezone.utc)

    logging.debug("pair(%r, %r) = %r", first, second, json)

    assert isinstance(json, dict)
    json["emoji"] = json.get("emoji", "")
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
            logging.info("item: pair(%r, %r) = %r, is_new=%r", first, second, result, is_new)

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


example_recipes = []


def enum_reverse_pairs(target_result: str, banned_items: list[str]):
    import torch
    from transformers import AutoModelForCausalLM, BitsAndBytesConfig

    global tokenizer, model

    tokenizer = tokenizer or Tokenizer.from_pretrained("oobabooga/llama-tokenizer")
    model = model or AutoModelForCausalLM.from_pretrained(
        os.environ["REVERESE_MODEL"], device_map="auto",
        quantization_config=BitsAndBytesConfig(load_in_8bit=True))

    banned_lc = set(item.lower() for item in banned_items)

    if not example_recipes:
        one_token_words = ("Water", "Fire", "Wind", "Earth", "Lake", "Plant", "Mountain", "Ocean", "Storm", "Cloud")
        for first in one_token_words:
            assert tokenize(first) == 1
            for second in one_token_words:
                if first <= second:
                    result = get_pair(first, second)
                    if result and tokenize(result) == 1:
                        example_recipes.append((first, second, result))

    # There should be a space before each word.
    prompt_prefix = "".join(f"{r[2]} = {r[0]} + {r[1]}\n " for r in random.sample(example_recipes, 2))
    prompt = f"{prompt_prefix}{target_result} ="
    prompt_enc = tokenizer.encode(prompt)

    pat1 = re.compile(r"^([^=]+) \+")
    pat2 = re.compile(r"^([^=]+) \+ ([^=]+)\n$")

    pq: list[tuple[float, tuple[int, ...]]] = []
    pq.append((0.0, ()))
    while pq:
        batch = []
        while pq and len(batch) < 16:
            score, ids = heapq.heappop(pq)
            text = tokenizer.decode(ids)
            if "=" in text:
                continue

            if "+" in text:
                match = pat1.match(text)
                if not match:
                    continue

                first, = match.groups()
                if first.startswith(" ") or first.lower() in banned_lc or len(first) > 30:
                    continue

            if "\n" in text:
                match = pat2.match(text)
                if not match:
                    continue

                first, second = match.groups()
                if second.startswith(" ") or second.lower() in banned_lc or len(second) > 30:
                    continue

                yield (score, first, second)
                continue

            batch.append((score, ids))

        if not batch:
            continue

        batch_ids = [prompt_enc.ids + list(ids) for _, ids in batch]
        seq_len = max(len(ids) for ids in batch_ids)

        with torch.inference_mode():
            output = model.forward(
                input_ids=torch.tensor([[0] * (seq_len - len(ids)) + ids for ids in batch_ids], device="cuda"),
                attention_mask=torch.tensor([[0] * (seq_len - len(ids)) + [1] * len(ids) for ids in batch_ids], device="cuda"))
        logits: torch.Tensor = output.logits[:, -1, :]
        logits_base = torch.logsumexp(logits, dim=-1).tolist()
        top_logits, top_token_ids = torch.topk(logits, k=1000)
        for i, (score, prefix) in enumerate(batch):
            for logit, token_id in zip(top_logits[i].tolist(), top_token_ids[i].tolist()):
                logit -= logits_base[i]
                if token_id == 1:
                    continue

                next_text = prefix + (token_id,)
                next_score = score - logit
                heapq.heappush(pq, (next_score, next_text))


def reverse_search(target: str, banned_items: list[str], max_pairs: int) -> Optional[tuple[str, str, str]]:
    target_lc = target.lower()
    used = set()

    progress = tqdm(zip(range(max_pairs), enum_reverse_pairs(target, banned_items)))
    for _, (_, first, second) in progress:
        if first > second:
            first, second = second, first
        pair = (first.lower(), second.lower())
        if pair in used:
            continue
        used.add(pair)

        result = get_pair(first, second)
        progress.set_postfix_str(f"{first} + {second} = {result}")
        if result and result.lower() == target_lc:
            return first, second, result

    return None


def main():
    try:
        progress = tqdm(delay=1)
        for line in sys.stdin:
            cmd, rest = line.strip().split(":", maxsplit=1)
            if cmd == "pair":
                first, second = rest.split("=")
                result = get_pair(first, second)
                progress.update(1)
                print(result or "", flush=True)
            elif cmd == "tokenize":
                count = tokenize(rest)
                progress.update(1)
                print(count, flush=True)
            elif cmd == "progress_reset":
                count, rest = rest.split(" ", maxsplit=1)
                progress.disable = True  # Suppress display in `progress.reset()`
                progress.set_description(rest, False)
                progress.reset(int(count))
                progress.disable = False
            elif cmd == "reverse":
                if "=" in rest:
                    target, max_pairs, *banned_items = rest.split("=")
                else:
                    target, max_pairs, banned_items = rest, str(10**9), []

                try:
                    found = reverse_search(target, banned_items + [target], int(max_pairs))
                except KeyboardInterrupt:
                    found = None

                if found:
                    first, second, result = found
                    print(first, second, result, sep="=", flush=True)
                else:
                    print(flush=True)

    except BrokenPipeError:
        pass
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)

    conn = sqlite3.connect("infinite-craft.db")
    conn.executescript("""
begin;
    create table if not exists pair(first text not null, second text not null, result text null, created_at datetime null, primary key (first, second));
    create table if not exists item(name text primary key not null, emoji text not null, is_new integer not null, created_at datetime null);
    create table if not exists tokenize(name text primary key not null, count integer not null);
commit;
    """)

    main()
