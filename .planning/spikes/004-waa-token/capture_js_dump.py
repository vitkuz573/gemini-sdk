#!/usr/bin/env python3
"""
Mitmproxy addon that dumps every response body to disk for offline analysis.

Usage:
    mitmproxy -s capture_js_dump.py --set hardump=/tmp/full.har

Then open gemini.google.com, authenticate, upload an image, send a prompt.
All JS/HTML/JSON bodies will be saved to ./js_dump/ keyed by URL path.
"""

import hashlib
import os
import urllib.parse
from pathlib import Path

DUMP_DIR = Path(__file__).parent / "js_dump"
DUMP_DIR.mkdir(exist_ok=True)


def _safe_name(url: str, content_type: str) -> str:
    parsed = urllib.parse.urlparse(url)
    path = parsed.path.strip("/") or "index"
    path = path.replace("/", "_")[:120]
    ext = ".bin"
    if content_type:
        ct = content_type.split(";")[0].strip()
        ext_map = {
            "application/javascript": ".js",
            "text/javascript": ".js",
            "application/json": ".json",
            "text/html": ".html",
            "text/plain": ".txt",
            "application/x-protobuf": ".pb",
            "application/json+protobuf": ".jsonpb",
        }
        ext = ext_map.get(ct, ".bin")
    digest = hashlib.sha256(url.encode()).hexdigest()[:12]
    return f"{path}_{digest}{ext}"


def response(flow):
    url = flow.request.pretty_url
    content_type = flow.response.headers.get("content-type", "")
    body = flow.response.content
    if not body or len(body) < 10:
        return

    # Save everything that looks interesting.
    interesting = any(
        x in url
        for x in [
            ".js",
            ".json",
            "/js/bg/",
            "batchexecute",
            "StreamGenerate",
            "Waa/Create",
            "GetAsyncData",
            "gemini.google.com/app",
        ]
    )
    if not interesting:
        return

    name = _safe_name(url, content_type)
    path = DUMP_DIR / name
    path.write_bytes(body)
    print(f"[DUMP] {len(body):>8} bytes -> {path}")
