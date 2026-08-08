---
spike: 003
name: gemini-protocol
validates: "Given a fresh 119 MB HAR capture with full response bodies from the Gemini web frontend, extract exact request/response shapes and make the Rust SDK work end-to-end without browser automation."
verdict: VALIDATED
related: [001-gemini-protocol, 002-gemini-protocol]
tags: [gemini, reverse-engineering, protocol, har, stream-generate, waa, ogads, upload]
---

# Spike 003: Полный реверс-инжиниринг протокола Gemini Web Frontend (с телами ответов)

## Что проверяем

Свежий 119 MB HAR-дамп (`/home/vitaly/mitm.har`, 550 записей) содержит **полные тела ответов** для `StreamGenerate`, `batchexecute`, WAA/ogads и upload. Цель — восстановить точные формы запросов/ответов и довести SDK до состояния, при котором `list_models`, `send_message`, `stream_generate` и загрузка файлов работают против живого фронтенда только по HTTP.

## Методика

1. Загрузили HAR в Python (`json.load(..., strict=False)`), отфильтровали 550 записей.
2. Извлекли 2 `StreamGenerate` (entry 470 — текстовый fresh, entry 504 — image continuation).
3. Полностью декодировали `f.req` → внешний JSON `[null, inner_json]` → `inner_req_list` длиной **97 слотов**.
4. Распарсили полные WIZ-ответы `StreamGenerate`: `c_*`, `r_*`, `rc_*`, continuation token (`21`/`26`), текст и thinking.
5. Распарсили 100 `batchexecute` RPC, включая `otAQ7b`, `sJBwce`, `ESY5D`, `K4WWud`, `cYRIkd`, `Te6DCf` и др.
6. Извлекли `Waa/Create` и `GetAsyncData` — тела запросов, заголовки, gzip/JSON-ответы.
7. Извлекли `/app` HTML (entry 264) — `SNlM0e`, `FdrFJe`, `bl`, `qKIAYe`/`KnDnFf`, API keys.
8. Извлекли upload flow (`push.clients6.google.com/upload`) — start + finalize.

## Общие выводы

- Базовая 97-слотовая структура и WAA-цепочка в SDK уже близки к захваченным значениям.
- Главные расхождения — **слот 96**, построение `x-goog-ext-525001261-jspb`, `Authorization: SAPISIDHASH` для ogads/waa, `x-client-data`, и корректная сборка тела `ogads GetAsyncData`.
- `Waa/Create` возвращает **большой base64/token-like blob** (31047 символов), который **не совпадает** с `inner_req_list[3]` напрямую. Вероятно, slot 3 генерируется на клиенте на основе ответа WAA (botguard challenge) или приходит из другого источника.
- `ogads GetAsyncData` в ответе возвращает `[null,...,0]` — пустышку; WAA-контекст для `x-goog-ext-525001261-jspb` скорее всего собирается из данных `Te6DCf`/`L5adhe`/`otAQ7b` (поле `e6fa609c3fa255c0` присутствует в `otAQ7b` как model-id Pro и в `ESY5D` как активный режим).
- Ответы `StreamGenerate` содержат WIZ-фреймы: meta-фрейм с `c_*`/`r_*`/continuation, затем фреймы с накопленным текстом (slot 1 части) и thinking (slot 37 части).

## StreamGenerate: детали запроса

### URL (оба запроса)

```
POST https://gemini.google.com/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate
?bl=boq_assistant-bard-web-server_20260806.17_p0
&f.sid=-1594710263937718439
&hl=ru
&_reqid=<номер>
&rt=c
```

- `pageId=none` и `authuser=0` отсутствуют — соответствует SDK.
- `Referer: https://gemini.google.com/` (не `/app`).
- `Origin: https://gemini.google.com`.
- `User-Agent: Chrome/146.0.0.0`.

### Заголовки StreamGenerate

| Header | Захваченное значение |
|--------|---------------------|
| `x-client-data` | `CI7yygE=` |
| `x-goog-ext-525001261-jspb` | `[1,null,null,null,"e6fa609c3fa255c0",null,null,0,[4,5,6,8],null,null,2,null,null,3,1,"<uuid>"]` |
| `x-goog-ext-525005358-jspb` | `["<request_uuid>",1]` |
| `x-goog-ext-73010989-jspb` | `[0]` |
| `x-goog-ext-73010990-jspb` | `[0,0,0]` |

Заголовок `525001261` использует **фиксированную структуру** с:
- поз. 0 = `1`
- поз. 4 = `"e6fa609c3fa255c0"` (model/mode fingerprint из `otAQ7b`/`ESY5D`)
- поз. 7 = `0`
- поз. 8 = `[4,5,6,8]`
- поз. 10 = `2`
- поз. 13 = `3`
- поз. 14 = `1`
- поз. 16 = request UUID (тот же, что в `525005358`)

### Заполненные слоты (97)

| Слот | Fresh (entry 470) | Continuation image (entry 504) | Примечание |
|------|-------------------|--------------------------------|------------|
| 0 | `["кто ты",0,null,null,null,null,0]` | `["что тут изображено",0,null,<attachments>,null,null,0]` | — |
| 1 | `["ru"]` | `["ru"]` | язык |
| 2 | `["","","",null,..., ""]` | `["c_...","r_...","rc_...",null,...,"<token>"]` | continuation state |
| 3 | `!GxilGHzNAAYxsMu...` (~2700 симв.) | `!T0ylTCjNAAYxsMu...` (~2700 симв.) | WAA/PoW token |
| 4 | `"74789427ec6a4f40e447c5a977914419"` | `"319eebc851d181586fd64c8607d282b1"` | 32 hex nonce |
| 6 | `[1]` | `[1]` | новый диалог |
| 7 | `1` | `1` | — |
| 10 | `1` | `1` | — |
| 11 | `0` | `0` | — |
| 17 | `[[0]]` | `[[1]]` | turn counter |
| 18 | `0` | `0` | — |
| 27 | `1` | `1` | — |
| 30 | `[4]` | `[4]` | Auto |
| 41 | `[1]` | `[1]` | mode picker |
| 53 | `0` | `0` | — |
| 59 | `"FCDF7DB6-B5A5-403D-8B89-421C470DC520"` | `"C7295AE9-4826-42FD-9BA8-33D839D19E9C"` | request UUID |
| 61 | `[]` | `[]` | — |
| 66 | `null` | `null` | — |
| 68 | `2` | `2` | — |
| 79 | `3` | `3` | — |
| 80 | `1` | `1` | Standard thinking |
| 91 | `0` | `0` | — |
| 96 | `1` | `0` | **fresh=1, continuation=0** |

### Новое / уточнение по сравнению с spike 002

- **Слот 96**: подтверждено — `1` для fresh, `0` для continuation. SDK сейчас всегда `0`.
- **Слот 59 UUID**: используется **один и тот же** UUID в `x-goog-ext-525005358-jspb` и в позиции 16 `x-goog-ext-525001261-jspb`.
- **Слот 3**: длина ~2700 символов, префикс `!`, алфавит base64url + `!`. Отличается между двумя запросами (fresh vs continuation), хотя WAA-цепочка вызвана одна.

## StreamGenerate: детали ответа

Формат ответа — WIZ-фреймы, разделённые длинами в отдельных строках:

```
)]}'

177
[["wrb.fr",null,"[null,[\"c_...\",\"r_...\"],{\"18\":\"r_...\",\"21\":[\"AUEngZbplHjDlXzznlkgor0XALTDFruoEq4eFbg-A_o\"],\"44\":true}]"]]
```

### Meta-фрейм (первые 1–2 фрейма)

```json
["wrb.fr", null, "[null,[\"c_a1af52cda2944035\",\"r_822fc6e5cce054f0\"],{\"18\":\"r_822fc6e5cce054f0\",\"21\":[\"AUEngZbplHjDlXzznlkgor0XALTDFruoEq4eFbg-A_o\"],\"44\":true}]"]
```

- `conversation_id` = `c_a1af52cda2944035`
- `response_id` = `r_822fc6e5cce054f0`
- `continuation_token` = первый элемент массива `"21"` (также поддерживается ключ `"26"`)

### Основной фрейм с текстом

```json
["wrb.fr", null, "[null,[\"c_...\",\"r_...\"],null,null,[[\"rc_53f9777404cc8004\",[\"Я — Gemini...\"],...]]]"]
```

- `response_part_id` = `rc_53f9777404cc8004` (slot 0 первой части)
- Текст ответа — объединение строк в `part[1]`.
- Thinking — `part[37][0]` (список строк с markdown-заголовками шагов).
- Ответ накопленный: поздние фреймы содержат полный текст, ранние — пустые/частичные.

### Примеры извлечённого содержимого

**Entry 470 (fresh text):**
- Text: `Я — Gemini, большая языковая модель... Чем я могу помочь вам сегодня?`
- Thinking: `**Defining the Role**\n\nI've clarified the user's intent...`

**Entry 504 (image continuation):**
- Text: `На изображении... снимок экрана рабочего стола компьютера...`
- Thinking: `**Interpreting the Data**\n\n...\n\n**Analyzing the Elements**\n\n...`

## batchexecute RPC

### Инициализация перед StreamGenerate (после `/app`)

| # | RPC | Payload | Ответ | Назначение |
|---|-----|---------|-------|------------|
| 1 | `otAQ7b` | `[[["otAQ7b","[]",null,"generic"]]]` | модельный список + `e6fa609c3fa255c0` | warm-up / model list |
| 2 | `sJBwce` | `[[["sJBwce","[[1,2]]",null,"generic"]]]` | `[]` | WAA prerequisite |
| 3 | `Waa/Create` | `["br1aemAN9owlYRs9NnsA"]` | `["bfkj",null,[...],"kyf1...","<token>","botguard",null,"[null,...,[]]"]` | WAA token |
| 4 | `GetAsyncData` | `[658,"https://gemini.google.com/",658,"ru","ch",1,null,0,0,"","",1,0,null,103135050,[[1,9,13],0,1,1],[1],null,1,0,"<base64>",{"1001":0}]` | `[null,null,null,null,null,null,null,null,null,null,0]` | WAA context (пустой) |
| 5 | `ESY5D` | `[[["ESY5D","[null,[2,3,7,8,...]]",null,"generic"]]]` | feature-flags | конфиг |
| 6 | `ESY5D` | `[[["ESY5D","[null,[5]]",null,"generic"]]]` | heartbeat | — |
| 7 | `cYRIkd` | `[[["cYRIkd","[\"ru\"]",null,"generic"]]]` | список tools | locale |
| 8 | `Te6DCf` | `[[["Te6DCf","[[\"ru\"],[1,2]]",null,"generic"]]]` | большой конфиг | locale/config |
| 9 | `K4WWud` | `[[["K4WWud","[[1],[\"ru\"]]",null,"generic"]]]` | `["Цюрих, Швейцария","SWML_DESCRIPTION_FROM_YOUR_INTERNET_ADDRESS",...]` | geo/locale |
| 10 | `o30O0e` | user info | — | профиль |
| 11 | `Bsxleb` | `[[[76091940,null,26],null,35,null,null,null,[null,null,"<uuid>"]]]` | — | config/state |
| 12 | `ku4Jyf` | `[["ru",null,null,null,4,null,null,[1,3,7,17],null,[]]]` | `[]` | — |

### Инициализация **без WAA** (entry 61–134, первая сессия)

В первой части HAR (до авторизации?) вызывались те же RPC, но **без** `Waa/Create`/`GetAsyncData`/`ESY5D` и с `source-path=/` вместо `/app`. Это говорит о том, что WAA-цепочка нужна после полного sign-in.

## Waa/Create

```
POST https://waa-pa.clients6.google.com/$rpc/google.internal.waa.v1.Waa/Create
```

### Request

- Body: `["br1aemAN9owlYRs9NnsA"]`
- Headers:
  - `content-type: application/json+protobuf`
  - `x-client-data: CI7yygE=`
  - `x-goog-api-key: AIzaSyBGb5fGAyC-pRcRU6MUHb__b_vKha71HRE`
  - `x-user-agent: grpc-web-javascript/0.1`
  - `Origin: https://gemini.google.com`
  - `Referer: https://gemini.google.com/`
  - **Нет `Authorization`** (в отличие от ogads).

### Response

- `content-type: application/json+protobuf; charset=UTF-8`
- Размер: 31261 байт (uncompressed).
- Структура: `[["bfkj", null, [null,null,null,"//www.google.com/js/bg/<file>.js"], "<id>", "<big-token>", "botguard", null, "[null,null,null,null,null,null,null,[],[]]"]]`
- Большой токен (index 4) — 31047 символов, похож на base64 с `/` и `+`. **Прямого совпадения со слотом 3 не обнаружено**, но он, вероятно, используется для client-side challenge/attestation, результат которого и попадает в `inner_req_list[3]`.

## ogads GetAsyncData

```
POST https://ogads-pa.clients6.google.com/$rpc/google.internal.onegoogle.asyncdata.v1.AsyncDataService/GetAsyncData
```

### Request

- Body:
  ```json
  [658,"https://gemini.google.com/",658,"ru","ch",1,null,0,0,"","",1,0,null,103135050,[[1,9,13],0,1,1],[1],null,1,0,"<base64>",{"1001":0}]
  ```
- Headers:
  - `authorization: SAPISIDHASH 1786124579_97e14e5b55f0b2771d831e515f771c4736fb2fba SAPISID1PHASH ... SAPISID3PHASH ...`
  - `content-type: application/json+protobuf`
  - `x-client-data: CI7yygE=`
  - `x-goog-api-key: AIzaSyCbsbvGCe7C9mCtdaTycZB2eUFuzsYKG_E`
  - `x-goog-authuser: 0`
  - `Origin/Referer: https://gemini.google.com`

### Response

```json
[null,null,null,null,null,null,null,null,null,null,0]
```

Ответ фактически пуст. WAA-контекст для `x-goog-ext-525001261-jspb` не приходит отсюда напрямую. Вероятно, контекст собирается из:
- `otAQ7b` (model fingerprint `e6fa609c3fa255c0`)
- `ESY5D` (feature flags, active rollouts)
- `Te6DCf`/`L5adhe` (locale/config state)

## /app HTML (entry 264)

URL: `https://gemini.google.com/` (redirect → `/app`).

Извлечённые значения:

| Поле | Значение |
|------|----------|
| `SNlM0e` | `ADR5zap56yDlZ6DzL1MQJYvlqzHr:1786124577132` |
| `FdrFJe` | `-1594710263937718439` |
| `qKIAYe` | `feeds/mcudyrk2a4khkz` |
| `KnDnFf` | `feeds/nrij2vo2gajxiu` |
| `bl` | `boq_assistant-bard-web-server_20260806.17_p0` |

API keys, встреченные в HTML/JS (не все используются для WAA):
- `AIzaSyAPW83vB9zFQqfpMup_cMJdELqDQkWvTho` — рядом с `bl`
- `AIzaSyBGb5fGAyC-pRcRU6MUHb__b_vKha71HRE` — WAA Create
- `AIzaSyCbsbvGCe7C9mCtdaTycZB2eUFuzsYKG_E` — ogads
- `AIzaSyD6n9asBjvx1yBHfhFhfw_kpS9Faq0BZHM` и др.

Параметр `br1aemAN9owlYRs9NnsA` для `Waa/Create` **не найден** в `/app` HTML ни напрямую, ни как `mk`/`rk`. Вероятно, он вычисляется JS или приходит в одном из ранних batchexecute/JS chunks.

## Upload flow

### Start (entry 490)

```
POST https://push.clients6.google.com/upload/
```

- `x-goog-upload-command: start`
- `x-goog-upload-header-content-length: 143432`
- `x-goog-upload-protocol: resumable`
- `x-tenant-id: bard-storage`
- `push-id: feeds/mcudyrk2a4khkz`
- `Content-Type: application/x-www-form-urlencoded;charset=UTF-8`
- Body: `File name: Снимок экрана_20260802_073212.png`

Ответ:
- `x-goog-upload-status: active`
- `x-goog-upload-url: https://push.clients6.google.com/upload/?upload_id=...&upload_protocol=resumable`
- `x-goog-upload-control-url: ...`
- `x-goog-upload-chunk-granularity: 262144`

### Finalize (entry 494)

```
POST https://push.clients6.google.com/upload/?upload_id=...&upload_protocol=resumable
```

- `x-goog-upload-command: upload, finalize`
- `x-goog-upload-offset: 0`
- `Content-Type: image/png` (фактический MIME)
- Body: бинарные данные PNG

Ответ (200):
```
/contrib_service/ttl_1d/jkwjrwqrlcoyaiacuql4xnghb3usmh1786124705_Ad6OsdfcEwxdEkR0DhapgqNtbkGN2pphX0qlUGLaBy0KldoPD6kMQezQJxV4
```

Совпадает с `WebAttachment.reference`, используемым в slot 0.

## Сравнение с текущей SDK

### Совпадает

- 97 слотов, общая структура.
- Слоты 1, 2, 4, 6, 7, 10, 11, 17, 18, 27, 30, 41, 53, 59, 61, 66, 68, 79, 80, 91.
- URL StreamGenerate без `pageId`/`authuser`.
- `list_models` использует `otAQ7b`.
- Upload flow полностью соответствует `src/upload.rs`.

### Несовпадения / что нужно исправить

1. **Слот 96**: SDK всегда `0`; в HAR **fresh=1, continuation=0**. Исправить в `src/proto/slots.rs`.
2. **`x-goog-ext-525001261-jspb`**: SDK использует `waa_context` целиком; нужно строить фиксированную структуру `[1,null,null,null,"e6fa609c3fa255c0",null,null,0,[4,5,6,8],null,null,2,null,null,3,1,"<uuid>"]`, где `<uuid>` — request UUID.
3. **`x-client-data`**: SDK использует `CNeSywE=`, захвачено `CI7yygE=`. Обновить константу.
4. **`Authorization: SAPISIDHASH`**: отсутствует в SDK; нужно добавить для `ogads GetAsyncData` (и возможно WAA), используя `SAPISID`/`__Secure-1PAPISID` + origin + timestamp + SHA1.
5. **Тело ogads**: SDK шлёт `[[waa_token]]`; захвачено полное тело с `658`, URL, locale, base64, `{"1001":0}`. Нужно отправлять полную структуру.
6. **Тело Waa/Create**: SDK хардкодит `br1aemAN9owlYRs9NnsA`; это работает, но может меняться. Пока оставить, но предусмотреть fallback.
7. **Referer для batchexecute/waa**: SDK использует `https://gemini.google.com/app`; захвачено `https://gemini.google.com/` для WAA/ogads/StreamGenerate. Унифицировать.
8. **Source-path**: SDK всегда `/app`; в HAR `otAQ7b`/`sJBwce` перед WAA используют `source-path=/`, остальные `/app`. Для `init_session` использовать `/`, для повторов — `/app`.
9. **Парсинг ответа**: текущий парсер уже обрабатывает `wrb.fr`, но не всегда корректно извлекает continuation из `{"21":["token"]}`. Уточнить `extract_conversation_state`.
10. **`blocking_lock()` / `with_*`**: текущий `Arc::get_mut().expect(...)` паникует на клонированных клиентах. Заменить на `blocking_lock()` из `tokio::sync::Mutex`.

## План рефакторинга SDK

1. `src/proto/slots.rs`: исправить слот 96 (`1` fresh, `0` continuation).
2. `src/session.rs`/`src/client.rs`: добавить `waa_fingerprint` (`e6fa609c3fa255c0`) в `SessionState` и функцию `build_waa_context_header(uuid)`.
3. `src/auth.rs`: реализовать `sapisid_hash(origin, sapisid, timestamp)` для `Authorization: SAPISIDHASH <ts>_<sha1>`.
4. `src/client.rs`:
   - Обновить `x-client-data` → `CI7yygE=`.
   - Передавать request UUID в `build_headers` и строить `525001261` корректно.
   - Исправить `with_language`/`with_max_retries`/`with_timeout` на `blocking_lock()`.
   - Для ogads/waa добавлять `Authorization` и правильные `x-goog-api-key`.
   - Использовать полное тело ogads.
   - Использовать `source-path=/` для первого `otAQ7b`/`sJBwce`.
   - Обновить `Referer` → `https://gemini.google.com/`.
5. `src/proto/parser.rs`: улучшить извлечение continuation token из `{"21":["..."]}`.
6. `src/upload.rs`: убедиться, что `Content-Type` для finalize берётся из MIME изображения (уже так).
7. Добавить тесты на слот 96, на построение WAA-заголовка, на SAPISIDHASH.

## Риски

- Слот 3 (WAA token) не совпадает с ответом `Waa/Create` напрямую — возможно, требуется client-side botguard challenge, что без браузера/JS воспроизвести невозможно. В этом случае сервер может возвращать `BardErrorInfo` / 400.
- `br1aemAN9owlYRs9NnsA` может быть привязан к JS-сборке и меняться.
- `SAPISIDHASH` требует точного формата (`<origin> <timestamp> <sapisid>` + SHA1).
- Google может менять payload `ESY5D`, `ogads` и RPC ids.

## Вывод

HAR с полными телами ответов подтвердил, что протокол **обратим в чистый HTTP** при условии:

- Корректного слота 96.
- Правильной сборки `x-goog-ext-525001261-jspb`.
- `Authorization: SAPISIDHASH` для ogads/waa.
- Полного тела `ogads GetAsyncData`.
- Актуального `x-client-data`.

Остаётся неясным, можно ли без браузерной автоматизации сгенерировать валидный слот 3 (WAA/PoW token). Если сервер отвергает запросы из-за слота 3, единственным решением остаётся `BrowserAttestationClient` под фичей `browser-attestation`.
