---
spike: 001
name: gemini-protocol
validates: "Given a 40 MB HAR capture of the Gemini web frontend, when we compare request/response shapes to the SDK implementation, then we can identify concrete mismatches that prevent direct API calls without browser automation."
verdict: VALIDATED
related: []
tags: [gemini, reverse-engineering, protocol, har, stream-generate]
---

# Spike 001: Реверс-инжиниринг протокола Gemini Web Frontend

## Что проверяем

Можно ли по 40 MB HAR-дампу (`/full.har`) восстановить актуальные формы запросов `/app`, `batchexecute`, `StreamGenerate` и `push.clients6.google.com/upload` и понять, почему текущая реализация SDK не проходит без браузерной аттестации.

## Методика

1. Загрузили HAR в Python (`json.loads(..., strict=False)`), отфильтровали 200+ записей.
2. Извлекли единственный `StreamGenerate` (200 entries → 1 POST).
3. Распарсили `f.req` → внешний JSON `[null, inner_json]` → `inner_req_list` длиной **97 слотов**.
4. Сравнили слоты с `src/protocol/slots.rs` и остальной SDK.
5. Проанализировали `batchexecute` RPC-ids, upload-flow, инициализацию сессии и заголовки.

## Общие выводы

- `StreamGenerate` по-прежнему использует **97-слотовый `inner_req_list`** — базовая структура верна.
- Однако **конкретные значения слотов и сопутствующие RPC сильно отличаются** от реализации в SDK.
- В HAR **нет тел ответов** (HAR сохранил только size/mimeType, без `content.text`), поэтому формат ответа проверить нельзя, но форму запроса восстановить можно.
- Без исправления слотов и инициализации сессии запросы SDK будут отвергнуты сервером; браузерная аттестация — это не единственная проблема.

## StreamGenerate: формат inner_req_list

Захваченный URL:

```
POST https://gemini.google.com/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate
?bl=boq_assistant-bard-web-server_20260806.17_p0
&f.sid=-1091091728961866802
&hl=ru
&_reqid=2963144
&rt=c
```

Тело: стандартный `application/x-www-form-urlencoded` с двумя полями:

- `f.req=[null,"<json inner_req_list>"]`
- `at=ADR5zaqJA1ci1DKZ73w01K0wqScU:1786105940803`

### Заполненные слоты (из 97)

| Слот | Захваченное значение | Пояснение |
|------|---------------------|-----------|
| 0 | `["кто ты", 0, null, null, null, null, 0]` | Пользовательский промпт; форма совпадает с SDK |
| 1 | `["ru"]` | **Локаль запроса**, не `["en"]` |
| 2 | `["", "", "", null, null, null, null, null, null, ""]` | **Начальное состояние диалога** — одинарный массив |
| 3 | `"!l5SllPDNAAYxsMu..."` (2467 символов) | Токен аттестации/Proof-of-Origin (WAA) |
| 4 | `"2f0aa6b787d0d8fa1f1d9a00016ff7a6"` | Request nonce / client-generated ID |
| 6 | `[1]` | Флаг «новый диалог» |
| 7 | `1` | — |
| 10 | `1` | — |
| 11 | `0` | — |
| 17 | `[[0]]` | Счётчик хода (turn counter) |
| 18 | `0` | — |
| 27 | `1` | — |
| 30 | `[4]` | Категория модели (`Auto`) |
| 41 | `[1]` | Mode picker option |
| 53 | `0` | — |
| 59 | `"4EC09528-F284-49AD-9592-BB04DDFFFF6A"` | UUID запроса (совпадает с `x-goog-ext-525005358-jspb`) |
| 61 | `[]` | — |
| 68 | `2` | — |
| 79 | `3` | — |
| 80 | `1` | Thinking level (`Standard`) |
| 91 | `0` | — |
| 96 | `0` | — |

Остальные слоты — `null`.

### Отличия от SDK (`src/protocol/slots.rs`)

| Слот | Захвачено | SDK | Оценка |
|------|-----------|-----|--------|
| 1 | `["ru"]` | `["en"]` | SDK игнорирует `ClientConfig::language`; для русскоязычного аккаунта получится `["en"]` |
| 2 (fallback) | `["", "", "", null, …, ""]` | `[["", "", "", null, …, ""]]` | **Двойная вложенность в SDK неверна** для первого хода; `ConversationState::to_slot2` возвращает правильную одинарную вложенность |
| 3 | длинный WAA-токен | не генерируется | Требуется получение WAA `Create` → `x-goog-ext-525001261-jspb` |
| 4 | hex nonce | `null` | Возможно, должен быть request UUID или отдельный nonce |
| 41 | `[1]` | `[2]` | Несовпадение mode picker |
| 68 | `2` | `1` | Несовпадение |
| 79 | `3` | `6` | Несовпадение |
| 80 | `1` | опционально | Захвачено `Standard` thinking; SDK ставит только при `thinking_level=Some(Standard)` |
| 66 | `null` | `[timestamp, 0]` (fallback) | SDK пишет timestamp, захвачено `null` |

**Ключевой инсайт:** слот 3 в HAR — это **не base64** (алфавит 65 символов, включая `!`, `-`, `_`). Это либо бинарный protobuf в base64url с нестандартным префиксом, либо специфический WAA token. Длина ~2467 символов, генерируется через `waa-pa.clients6.google.com/$rpc/google.internal.waa.v1.Waa/Create` и/или `ogads-pa.clients6.google.com`.

## batchexecute RPC-ids

В HAR встречаются следующие RPC (с параметрами — только те, что не пустые `[]`):

| RPC | Payload (пример) | Назначение |
|-----|------------------|------------|
| `otAQ7b` | `[[["otAQ7b","[]",null,"generic"]]]` | Модельный список / UserStatus. **SDK использует `Fd0Qje`**, но парсер (`parse_model_list`) ищет `otAQ7b`. Это противоречие. |
| `ESY5D` | `[null,[5]]` | Вероятно, feature-флаги / config. SDK не вызывает. |
| `PCck7e` | `["r_0145d14ae4cdd9b8"]` | Рейтинг/feedback после ответа. SDK не реализует. |
| `qpEbW` | `[[[1,4],[6,6],[1,15]]]` | Telemetry / impression logs. SDK не реализует. |
| `aPya6c` | `[]` | Периодический heartbeat. SDK не реализует. |
| `MyzX6c`, `VxUbXb`, `L5adhe`, `CNgdBe`, `whPPme`, `K4WWud`, `ozz5Z`, `o30O0e`, `Bsxleb`, `GPRiHf`, `maGuAc`, `Te6DCf`, `ku4Jyf`, `cYRIkd`, `sJBwce`, `I4z33b` | разные | Инициализация UI, история, модельный picker, user info, настройки, sJBwce `[[1,2]]` и т.д. |

### Проблемы SDK

- `list_models` отправляет `rpcids=Fd0Qje`, но парсер `parse_model_list` ищет `otAQ7b`. Нужно либо поменять RPC id, либо парсер.
- Перед `StreamGenerate` в первой сессии звали `ESY5D [null,[5]]`; во второй сессии — полная цепочка `otAQ7b → sJBwce → Waa Create → ogads → ESY5D`. SDK вызывает только `/app` и сразу `StreamGenerate`, пропуская, по-видимому, обязательные шаги WAA/инициализации.

## Upload flow (`push.clients6.google.com/upload`)

Захваченный flow совпадает с `src/upload.rs`:

1. **OPTIONS** на `/upload/`.
2. **POST** `/upload/` с заголовками:
   - `x-goog-upload-command: start`
   - `x-goog-upload-header-content-length: 143432`
   - `x-goog-upload-protocol: resumable`
   - `x-tenant-id: bard-storage`
   - `push-id: feeds/mcudyrk2a4khkz`
   - тело: `File name: Снимок экрана_20260802_073212.png`
3. Ответ `200` с `x-goog-upload-url`.
4. **POST** по `x-goog-upload-url` с `x-goog-upload-command: upload, finalize`, `x-goog-upload-offset: 0`, `Content-Type: image/png` и бинарным телом.

**Нюанс:** в захваченном старте `Content-Type: application/x-www-form-urlencoded;charset=UTF-8`, а SDK тоже шлёт `application/x-www-form-urlencoded` с телом `File name: ...`. Это работает, но выглядит как хак; формально лучше `text/plain`.

## Заголовки StreamGenerate

Захваченные заголовки, которых нет в `GeminiClient::build_headers`:

- `x-client-data: CNeSywE=`
- `x-goog-ext-525001261-jspb: [1,null,null,null,"e6fa609c3fa255c0",null,null,0,[4,5,6,8],null,null,2,null,null,3,1,"2F6705F6-1BC4-4BCA-A3D8-C6420653C22B"]`
- `x-goog-ext-73010989-jspb: [0]`
- `x-goog-ext-73010990-jspb: [0,0,0]`

SDK добавляет только `x-goog-ext-525005358-jspb: ["<uuid>",1]`. Судя по HAR, **необходим `525001261`** (WAA context), иначе сервер не сможет проверить `inner_req_list[3]`.

## План рефакторинга SDK

### 1. Исправить `inner_req_list`

- **Слот 1**: использовать `session.language` вместо хардкода `"en"`.
- **Слот 2**: для первого хода использовать одинарный массив `["", "", "", null, …, ""]`; убрать лишнюю вложенность в `build_fallback_base`.
- **Слот 3**: реализовать получение WAA token через `waa-pa.clients6.google.com/$rpc/google.internal.waa.v1.Waa/Create` и/или `ogads-pa.clients6.google.com`. Возможно, требуется `sJBwce [[1,2]]` как prerequisite.
- **Слот 4**: генерировать hex nonce (32 символа) и сохранять в сессии.
- **Слот 41**: исследовать зависимость от категории/режима; захвачено `[1]` при `Auto`.
- **Слот 68, 79**: скорректировать константы под захваченные `2` и `3` соответственно.
- **Слот 80**: по умолчанию `Standard thinking` (`1`) либо сделать поведение по умолчанию `Some(ThinkingLevel::Standard)`.
- **Слот 66**: либо убрать `[ts,0]` в fallback, либо проверить, что сервер принимает `null`.

### 2. Исправить инициализацию сессии

Добавить в `GeminiClient::init_session` после `fetch_app_page`:

1. `batchexecute?rpcids=otAQ7b` — получить список моделей (и заодно warm-up).
2. `batchexecute?rpcids=sJBwce` с payload `[[1,2]]` — возможно, включение WAA.
3. `POST https://waa-pa.clients6.google.com/$rpc/google.internal.waa.v1.Waa/Create` с телом `["br1aemAN9owlYRs9NnsA"]` (значение, вероятно, из JS).
4. `POST https://ogads-pa.clients6.google.com/$rpc/google.internal.onegoogle.asyncdata.v1.AsyncDataService/GetAsyncData` — передать WAA context.
5. `batchexecute?rpcids=ESY5D` с `[null,[5]]` — feature-flags/config.

Сохранить `x-goog-ext-525001261-jspb` и WAA token в `SessionState`.

### 3. Исправить `list_models`

- Поменять `rpcids` с `Fd0Qje` на `otAQ7b`.
- Payload: `[[["otAQ7b","[]",null,"generic"]]]`.
- Парсер уже ищет `otAQ7b` — совместимо.

### 4. Исправить URL StreamGenerate

- Убрать `pageId=none` и `authuser=0` из query (в захваченном URL их нет).
- Оставить `bl`, `f.sid`, `hl`, `_reqid`, `rt=c`.

### 5. Дополнить заголовки

Добавить в `build_headers`:

- `x-client-data: CNeSywE=` (или вычислять из JS bundle / client hints).
- `x-goog-ext-525001261-jspb` — из `SessionState.waa_context`.
- `x-goog-ext-73010989-jspb: [0]`
- `x-goog-ext-73010990-jspb: [0,0,0]`

### 6. Browser attestation vs WAA

Текущая feature `browser-attestation` (Chrome) захватывает slot 3 и slot 4. В HAR эти значения приходят из WAA/ogads, а не из Chrome. После реализации WAA-цепочки browser automation можно будет **сделать опциональной** для текстовых запросов; для картинок, возможно, всё ещё потребуется PoW.

## Риски

- WAA/ogads payload может быть привязан к конкретной JS-сборке (`bl`) и cookies; нужно динамически извлекать параметры из `/app` или JS.
- Захвачен только один `StreamGenerate` на русском языке; другие категории моделей (`Pro`, `Thinking`) могут менять слоты 30/41/79/80.
- HAR без response bodies не позволяет проверить формат ответа и `continuation_token`.
- Аттестация Google (reCAPTCHA / WAA) может меняться часто.

## Вывод

Протокол **обратим в чистый HTTP**, но требует:

1. Корректной 97-слотовой матрицы (исправить слоты 1, 2, 41, 66, 68, 79, 80 и nonce в слоте 4).
2. WAA-инициализации (`sJBwce → Waa Create → ogads`) для получения токена в слот 3.
3. Правильных RPC ids (`otAQ7b` для моделей, `ESY5D` для конфига).
4. Дополнительных заголовков (`x-goog-ext-525001261-jspb` и др.).

Без этих изменений SDK будет получать `BardErrorInfo` / 400 / пустой ответ, даже при наличии валидных cookies.
