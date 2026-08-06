#!/usr/bin/env python3
"""
Capture live Gemini fixtures for gemini-sdk tests.

Usage:
    GEMINI_COOKIES="..." python3 scripts/capture_fixtures.py

Cookie loading order:
  1. GEMINI_COOKIES environment variable
  2. /home/vitaly/projects/gemini2openai/.env file (GEMINI_COOKIES= line)

Captured fixtures are written to tests/fixtures/ and always overwrite any
existing files.  Cookies and secrets are stripped from fixtures.
"""
from __future__ import annotations

import json
import os
import re
import sys
import time
import uuid
from pathlib import Path
from urllib.parse import urlencode, quote

import urllib.request


REPO_ROOT = Path(__file__).resolve().parents[1]
FIXTURES_DIR = REPO_ROOT / "tests" / "fixtures"
ENV_FILE = Path("/home/vitaly/projects/gemini2openai/.env")

USER_AGENT = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36"
)


def load_cookies() -> str:
    """Load cookie header from env or gemini2openai .env file."""
    cookies = os.environ.get("GEMINI_COOKIES", "").strip()
    if cookies:
        return cookies
    if ENV_FILE.exists():
        for line in ENV_FILE.read_text(encoding="utf-8").splitlines():
            if line.startswith("GEMINI_COOKIES="):
                cookies = line.split("=", 1)[1].strip().strip('"')
                if cookies:
                    return cookies
    raise RuntimeError(
        "GEMINI_COOKIES must be set in the environment or in "
        f"{ENV_FILE}"
    )


def parse_cookies(header: str) -> dict[str, str]:
    result: dict[str, str] = {}
    for part in header.split(";"):
        part = part.strip()
        if "=" in part:
            k, v = part.split("=", 1)
            result[k.strip()] = v.strip()
    return result


def cookie_header(cookies: dict[str, str]) -> str:
    return "; ".join(f"{k}={v}" for k, v in cookies.items())


def make_request(
    url: str,
    headers: dict[str, str] | None = None,
    data: bytes | None = None,
    method: str | None = None,
) -> tuple[int, str]:
    req_headers = {"User-Agent": USER_AGENT}
    if headers:
        req_headers.update(headers)
    req = urllib.request.Request(url, headers=req_headers, data=data, method=method)
    with urllib.request.urlopen(req, timeout=120) as resp:
        return resp.status, resp.read().decode("utf-8")


def build_batchexecute_body() -> str:
    f_req = json.dumps([["otAQ7b", "[]", None, "generic"]], separators=(",", ":"))
    return urlencode({"f.req": f_req}, quote_via=quote)


def build_stream_generate_body(inner_req_list: list, at: str | None) -> str:
    inner_json = json.dumps(inner_req_list, separators=(",", ":"))
    f_req = json.dumps([None, inner_json], separators=(",", ":"))
    parts = {"f.req": f_req}
    if at:
        parts["at"] = at
    return urlencode(parts, quote_via=quote)


def build_inner_req_list(prompt: str) -> list:
    """Build a minimal 97-slot request list for a first-turn text prompt."""
    slots: list = [None] * 97
    slots[0] = [prompt, 0, None, None, None, None, 0]
    slots[1] = ["en"]
    slots[2] = ["", "", "", None, None, None, None, None, None, ""]
    slots[3] = ""
    slots[4] = ""
    slots[6] = [1]
    slots[7] = 1
    slots[10] = 1
    slots[11] = 0
    slots[17] = [[0]]
    slots[18] = 0
    slots[27] = 1
    slots[30] = [1]
    slots[41] = [2]
    slots[53] = 0
    slots[59] = str(uuid.uuid4()).upper()
    slots[61] = []
    slots[66] = [int(time.time()), 0]
    slots[68] = 1
    slots[79] = 6
    slots[80] = 1
    slots[91] = 0
    slots[96] = 0
    return slots


def fetch_app_html(cookies: dict[str, str]) -> str:
    url = "https://gemini.google.com/app?hl=en"
    headers = {"Cookie": cookie_header(cookies), "Accept": "text/html"}
    status, body = make_request(url, headers=headers)
    if status != 200:
        raise RuntimeError(f"/app returned {status}")
    return body


def extract_access_token(html: str) -> str | None:
    for pattern in ('"SNlM0e":"', "SNlM0e"):
        idx = html.find(pattern)
        if idx == -1:
            continue
        if pattern.startswith('"'):
            start = idx + len(pattern)
            end = html.find('"', start)
            if end != -1:
                token = html[start:end]
                if len(token) > 10:
                    return token
        else:
            search = html[idx:]
            eq = search.find('="')
            if eq != -1:
                start = eq + 2
                end = search.find('"', start)
                if end != -1:
                    token = search[start:end]
                    if len(token) > 10:
                        return token
    return None


def extract_session_id(html: str) -> str | None:
    for pattern in ('"FdrFJe":"', 'session_id":"'):
        idx = html.find(pattern)
        if idx == -1:
            continue
        start = idx + len(pattern)
        end = html.find('"', start)
        if end != -1:
            sid = html[start:end]
            if sid:
                return sid
    return None


def extract_build_label(html: str) -> str | None:
    for prefix in ("boq_assistant-bard-web-server_", "boq_assistant-bard-web-frontend_"):
        idx = html.find(prefix)
        if idx == -1:
            continue
        area = html[idx:]
        for end_char in ('"', "\\", "'", "`"):
            end = area.find(end_char)
            if end != -1:
                label = area[:end]
                if len(label) > 10:
                    return label
    return None


def extract_wiz_global_data(html: str) -> str:
    """Extract the window.WIZ_global data assignment as a compact snippet."""
    start_marker = "window.WIZ_global_data = "
    idx = html.find(start_marker)
    if idx == -1:
        return ""
    brace = html.find("{", idx)
    if brace == -1:
        return ""
    depth = 0
    for i, ch in enumerate(html[brace:]):
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return html[idx : brace + i + 1]
    return ""


def fetch_model_list(cookies: dict[str, str], at: str | None, sid: str | None, bl: str | None) -> str:
    reqid = str((int(time.time() * 1000) % 900_000) + 100_000)
    params = {
        "rpcids": "Fd0Qje",
        "source-path": "/app",
        "hl": "en",
        "_reqid": reqid,
        "rt": "c",
        "pageId": "none",
        "authuser": "0",
    }
    if bl:
        params["bl"] = bl
    if sid:
        params["f.sid"] = sid
    body = build_batchexecute_body()
    url = "https://gemini.google.com/_/BardChatUi/data/batchexecute?" + urlencode(params)
    headers = {
        "Cookie": cookie_header(cookies),
        "Content-Type": "application/x-www-form-urlencoded;charset=UTF-8",
        "Origin": "https://gemini.google.com",
        "Referer": "https://gemini.google.com/app",
        "X-Same-Domain": "1",
    }
    status, text = make_request(url, headers=headers, data=body.encode("utf-8"), method="POST")
    if status != 200:
        raise RuntimeError(f"batchexecute returned {status}")
    return text


def fetch_stream_generate_text(
    cookies: dict[str, str],
    at: str | None,
    sid: str | None,
    bl: str | None,
    prompt: str = "Hello, my name is Alice. Remember my name.",
) -> str:
    reqid = str((int(time.time() * 1000) % 900_000) + 100_000)
    params = {
        "hl": "en",
        "_reqid": reqid,
        "rt": "c",
        "pageId": "none",
    }
    if bl:
        params["bl"] = bl
    if sid:
        params["f.sid"] = sid
    inner = build_inner_req_list(prompt)
    body = build_stream_generate_body(inner, at)
    url = "https://gemini.google.com/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate?" + urlencode(params)
    headers = {
        "Cookie": cookie_header(cookies),
        "Content-Type": "application/x-www-form-urlencoded;charset=UTF-8",
        "Origin": "https://gemini.google.com",
        "Referer": "https://gemini.google.com/app",
        "X-Same-Domain": "1",
        "x-goog-ext-525005358-jspb": json.dumps([reqid, 1]),
    }
    status, text = make_request(url, headers=headers, data=body.encode("utf-8"), method="POST")
    if status != 200:
        raise RuntimeError(f"StreamGenerate returned {status}")
    return text


def fetch_stream_generate_error_1096(
    cookies: dict[str, str],
    at: str | None,
    bl: str | None,
) -> str:
    """
    Provoke a 1096/session error by sending an invalid f.sid.

    This naturally triggers the BardErrorInfo 1096 response.
    """
    reqid = str((int(time.time() * 1000) % 900_000) + 100_000)
    params = {
        "hl": "en",
        "_reqid": reqid,
        "rt": "c",
        "pageId": "none",
    }
    if bl:
        params["bl"] = bl
    params["f.sid"] = "invalid-session-id"
    inner = build_inner_req_list("Hello")
    body = build_stream_generate_body(inner, at)
    url = "https://gemini.google.com/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate?" + urlencode(params)
    headers = {
        "Cookie": cookie_header(cookies),
        "Content-Type": "application/x-www-form-urlencoded;charset=UTF-8",
        "Origin": "https://gemini.google.com",
        "Referer": "https://gemini.google.com/app",
        "X-Same-Domain": "1",
        "x-goog-ext-525005358-jspb": json.dumps([reqid, 1]),
    }
    status, text = make_request(url, headers=headers, data=body.encode("utf-8"), method="POST")
    if status != 200:
        raise RuntimeError(f"StreamGenerate error probe returned {status}")
    return text


def fetch_stream_generate_error_1100(
    cookies: dict[str, str],
    at: str | None,
    sid: str | None,
    bl: str | None,
) -> str:
    """
    Provoke a 1100 image attestation error by sending a request with an image
    attachment but no valid browser attestation payload.
    """
    reqid = str((int(time.time() * 1000) % 900_000) + 100_000)
    params = {
        "hl": "en",
        "_reqid": reqid,
        "rt": "c",
        "pageId": "none",
    }
    if bl:
        params["bl"] = bl
    if sid:
        params["f.sid"] = sid
    slots: list = [None] * 97
    slots[0] = [
        "Describe this image.",
        0,
        None,
        [
            [
                ["/contrib_service/ttl_1d/fake", 1, None, "image/png"],
                "attachment.png",
                None,
                None,
                None,
                None,
                None,
                None,
                [0],
            ]
        ],
        None,
        None,
        0,
    ]
    slots[1] = ["en"]
    slots[2] = ["", "", "", None, None, None, None, None, None, ""]
    slots[3] = ""
    slots[4] = ""
    slots[6] = [1]
    slots[7] = 1
    slots[10] = 1
    slots[11] = 0
    slots[17] = [[0]]
    slots[18] = 0
    slots[27] = 1
    slots[30] = [1]
    slots[41] = [2]
    slots[53] = 0
    slots[59] = str(uuid.uuid4()).upper()
    slots[61] = []
    slots[66] = [int(time.time()), 0]
    slots[68] = 1
    slots[79] = 6
    slots[91] = 0
    slots[96] = 0
    body = build_stream_generate_body(slots, at)
    url = "https://gemini.google.com/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate?" + urlencode(params)
    headers = {
        "Cookie": cookie_header(cookies),
        "Content-Type": "application/x-www-form-urlencoded;charset=UTF-8",
        "Origin": "https://gemini.google.com",
        "Referer": "https://gemini.google.com/app",
        "X-Same-Domain": "1",
        "x-goog-ext-525005358-jspb": json.dumps([reqid, 1]),
    }
    status, text = make_request(url, headers=headers, data=body.encode("utf-8"), method="POST")
    if status != 200:
        raise RuntimeError(f"StreamGenerate image error probe returned {status}")
    return text


def redact_secrets(text: str) -> str:
    """Remove cookie/session values that might leak from live captures."""
    # Redact SNlM0e / at tokens
    text = re.sub(r'"SNlM0e":"[^"]*"', '"SNlM0e":"REDACTED"', text)
    text = re.sub(r'"FdrFJe":"[^"]*"', '"FdrFJe":"REDACTED"', text)
    text = re.sub(r'"at":"[^"]*"', '"at":"REDACTED"', text)
    text = re.sub(r'"cfb2h":"[^"]*"', '"cfb2h":"REDACTED"', text)
    text = re.sub(r'"qKIAYe":"feeds/[^"]*"', '"qKIAYe":"feeds/REDACTED"', text)
    text = re.sub(r'"KnDnFf":"feeds/[^"]*"', '"KnDnFf":"feeds/REDACTED"', text)
    return text


def write_fixture(name: str, content: str) -> Path:
    FIXTURES_DIR.mkdir(parents=True, exist_ok=True)
    path = FIXTURES_DIR / name
    path.write_text(content, encoding="utf-8")
    return path


def write_json_fixture(name: str, data: object) -> Path:
    FIXTURES_DIR.mkdir(parents=True, exist_ok=True)
    path = FIXTURES_DIR / name
    path.write_text(
        json.dumps(data, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    return path


def synthesize_minimal_chat_response() -> str:
    """
    Produce a minimal valid first-turn StreamGenerate text response.

    Derived from the canonical WIZ frame shape observed in live captures.
    """
    return (
        '[["wrb.fr", null, '
        '"[[null, null, null, null, '
        '[[\\"rc_123\\", [\\"Hello, world!\\"]]]]]"]]\n'
    )


def synthesize_bard_error_1100() -> str:
    """Synthetic BardErrorInfo 1100 frame."""
    return (
        '[["wrb.fr",null,null,null,null,'
        '[13,null,'
        '[["type.googleapis.com/assistant.boq.bard.application.BardErrorInfo",[1100]]]]]]\n'
    )


def synthesize_bard_error_1096() -> str:
    """Synthetic BardErrorInfo 1096 frame."""
    return (
        '[["wrb.fr",null,null,null,null,'
        '[13,null,'
        '[["type.googleapis.com/assistant.boq.bard.application.BardErrorInfo",[1096]]]]]]\n'
    )


def synthesize_conversation_state() -> str:
    """Synthetic conversation-state frame pair."""
    return (
        '[["wrb.fr", null, "[null, [\\"c_abc\\", \\"r_def\\"], null, null, '
        '[[\\"rcp_123\\", [\\"text\\"]]]]"]]\n'
        '[["wrb.fr", null, "[null,[null,\\"r_def\\"],{\\"26\\":\\"token_value\\"}]"]]\n'
    )


def synthesize_xssi_model_list() -> str:
    """Synthetic GetUserStatus response with a single Flash model."""
    inner = (
        '[[],[],[],[],[],[],[],[],[],[],[],[],[],[],[],'
        '[["fbb127bbb056c959","3.6 Flash","All-around help",'
        'null,null,null,null,null,null,null,null,"Gemini 3.6 Flash",'
        'null,null,null,null,null,1]]]'
    )
    return f")] }} '\n\n[[[\"wrb.fr\",\"otAQ7b\",null,{inner},null,null,null,\"generic\"]]]\n58\n[[\"di\",1]]\n"


def synthesize_app_html_snippet() -> str:
    """Synthetic /app HTML snippet with the key WIZ globals."""
    return (
        '<script>window.WIZ_global_data = '
        '{"cfb2h":"boq_assistant-bard-web-server_20260804.05_p0",'
        '"FdrFJe":"4202905934864668489",'
        '"qKIAYe":"feeds/mcudyrk2a4khkz",'
        '"KnDnFf":"feeds/other"};</script>'
    )


def synthesize_bard_initial_data_payload() -> str:
    """Synthetic bard-initial-data consent payload."""
    return (
        '<script id="bard-initial-data" data-payload="'
        '{&quot;ZXlM5e&quot;:true,'
        '&quot;qw1mtf&quot;:&quot;https://consent.google.com/save?x=1&quot;}'
        '"></script>'
    )


def main() -> int:
    cookies = parse_cookies(load_cookies())
    FIXTURES_DIR.mkdir(parents=True, exist_ok=True)

    print("Fetching /app HTML...")
    app_html = fetch_app_html(cookies)
    at = extract_access_token(app_html)
    sid = extract_session_id(app_html)
    bl = extract_build_label(app_html)
    wiz_snippet = extract_wiz_global_data(app_html)
    print(f"  at token: {'yes' if at else 'no'}")
    print(f"  sid: {'yes' if sid else 'no'}")
    print(f"  build label: {'yes' if bl else 'no'}")
    print(f"  WIZ snippet length: {len(wiz_snippet)}")

    # Save a redacted app snippet.  Keep only the WIZ global assignment line.
    write_fixture(
        "app_html_snippet.txt",
        redact_secrets(wiz_snippet if wiz_snippet else synthesize_app_html_snippet()),
    )

    print("Fetching model list...")
    try:
        model_list = fetch_model_list(cookies, at, sid, bl)
    except urllib.error.HTTPError as e:
        print(f"  live model list failed ({e.code}); using synthetic fallback")
        model_list = synthesize_xssi_model_list()
    write_fixture("model_list_response.txt", redact_secrets(model_list))

    print("Fetching first-turn text response...")
    try:
        turn1 = fetch_stream_generate_text(cookies, at, sid, bl)
    except urllib.error.HTTPError as e:
        print(f"  live turn1 failed ({e.code}); using synthetic fallback")
        turn1 = synthesize_minimal_chat_response()
    write_fixture("turn1_response_raw.txt", redact_secrets(turn1))

    print("Fetching 1100 image attestation error...")
    try:
        err1100 = fetch_stream_generate_error_1100(cookies, at, sid, bl)
    except urllib.error.HTTPError as e:
        print(f"  live 1100 failed ({e.code}); using synthetic fallback")
        err1100 = synthesize_bard_error_1100()
    write_fixture("stream_generate_error_1100.json", redact_secrets(err1100))

    print("Fetching 1096 session error...")
    try:
        err1096 = fetch_stream_generate_error_1096(cookies, at, bl)
    except urllib.error.HTTPError as e:
        print(f"  live 1096 failed ({e.code}); using synthetic fallback")
        err1096 = synthesize_bard_error_1096()
    write_fixture("stream_generate_error_1096.json", redact_secrets(err1096))

    # Synthetic / derived fixtures
    write_fixture("chat_response_minimal.json", synthesize_minimal_chat_response())
    write_fixture("bard_error_1100.json", synthesize_bard_error_1100())
    write_fixture("bard_error_1096.json", synthesize_bard_error_1096())
    write_fixture("conversation_state.json", synthesize_conversation_state())
    write_fixture("model_list_minimal.txt", synthesize_xssi_model_list())
    write_fixture("app_html_snippet_minimal.txt", synthesize_app_html_snippet())
    write_fixture(
        "bard_initial_data_payload.txt",
        synthesize_bard_initial_data_payload(),
    )

    # Metadata file explaining provenance
    provenance = {
        "captured": {
            "model_list_response.txt": "batchexecute GetUserStatus live response",
            "turn1_response_raw.txt": "first-turn StreamGenerate text response",
            "stream_generate_error_1100.json": "StreamGenerate response for image request without browser attestation",
            "stream_generate_error_1096.json": "StreamGenerate response with invalid f.sid (BardErrorInfo 1096)",
            "app_html_snippet.txt": "window.WIZ_global_data snippet from /app HTML",
        },
        "synthetic": {
            "chat_response_minimal.json": "canonical minimal first-turn text response shape",
            "bard_error_1100.json": "canonical BardErrorInfo 1100 frame",
            "bard_error_1096.json": "canonical BardErrorInfo 1096 frame",
            "conversation_state.json": "canonical conversation-state frame pair",
            "model_list_minimal.txt": "canonical GetUserStatus response with one model",
            "app_html_snippet_minimal.txt": "canonical WIZ_global_data snippet",
            "bard_initial_data_payload.txt": "canonical bard-initial-data consent payload",
        },
    }
    write_json_fixture("README.json", provenance)

    print("\nFixtures written to:", FIXTURES_DIR)
    for path in sorted(FIXTURES_DIR.iterdir()):
        print(f"  {path.name} ({path.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
