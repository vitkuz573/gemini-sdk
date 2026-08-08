---
spike: 002
name: gemini-protocol
validates: "Given a new 47 MB HAR capture (`/full1.har`) of the Gemini web frontend, compare request/response shapes and the WAA initialization chain to the SDK implementation and identify concrete protocol changes since spike 001."
verdict: VALIDATED
related: [001-gemini-protocol]
tags: [gemini, reverse-engineering, protocol, har, stream-generate, waa]
---

# Spike 002: Реверс-инжиниринг протокола Gemini Web Frontend (обновлённый захват)

## Что проверяем

Новый 47 MB HAR-дамп (`/full1.har`, 262 записи, 200+ запросов) содержит рабочую сессию Gemini с текстовым и image-запросами. Цель — подтвердить/уточнить выводы spike 001, проверить WAA-цепочку и найти новые RPC/заголовки.

## Методика

1. Загрузили HAR в Python, отфильтровали 262 записи.
2. Извлекли 2 `StreamGenerate` (1 image fresh, 1 text continuation).
3. Распарсили `f.req` → `inner_req_list` 97 слотов.
4. Сравнили слоты с spike 001 и SDK (`src/proto/slots.rs`).
5. Проанализировали 62 `batchexecute`, WAA/ogads, upload, `/app`.

## Общие выводы

- **StreamGenerate по-прежнему 97-слотовый**; базовая структура совпадает с spike 001 и SDK.
- **WAA/Create работает в браузере**: `waa-pa.clients6.google.com/$rpc/google.internal.waa.v1.Waa/Create` вернул `200` с `content-type: application/json+protobuf` и `content-length: 23757` (gzip). Тело ответа в HAR не сохранено (только size/mimeType), но статус 200 подтверждает успех.
- **ogads GetAsyncData тоже 200**, возвращает WAA-контекст для заголовка `x-goog-ext-525001261-jspb`.
- **Upload flow работает**: `push.clients6.google.com/upload` start + finalize оба `200`.
- HAR по-прежнему **не сохраняет тела ответов** (ни StreamGenerate, ни batchexecute, ни WAA/ogads), поэтому парсинг ответов невозможен.

## StreamGenerate: сводка

| Параметр | Значение |
|----------|----------|
| Всего захвачено | 2 |
| Вариант 1 (entry 178) | image, fresh (первый ход), статус 200, размер ответа 159076 |
| Вариант 2 (entry 222) | text, continuation (второй ход), статус 200, размер ответа 27449 |
| URL | `POST https://gemini.google.com/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate` |
| Query | `bl`, `f.sid`, `hl=ru`, `_reqid`, `rt=c` (без `pageId`/`authuser`) |

### Заполненные слоты (non-null из 97)

| Слот | Image fresh (178) | Text continuation (222) | Примечание |
|------|-------------------|-------------------------|------------|
| 0 | image attachment list | text only | SDK структура верна |
| 1 | `["ru"]` | `["ru"]` | язык сессии |
| 2 | `["","",...,null,""]` | `[c_id,r_id,rp_id,null,...,token]` | fresh vs continuation |
| 3 | WAA token ~2600 | WAA token ~2645 | из WAA/ogads |
| 4 | 32-hex nonce | 32-hex nonce | SDK генерирует |
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
| 59 | UUID | UUID | request UUID |
| 61 | `[]` | `[]` | — |
| 66 | `null` | `null` | SDK уже null |
| 68 | `2` | `2` | — |
| 79 | `3` | `3` | — |
| 80 | `1` | `1` | Standard thinking |
| 91 | `0` | `0` | — |
| 96 | `1` | `0` | **fresh=1, continuation=0** |

### Заголовки StreamGenerate

- `x-client-data: CNeSywE=`
- `x-goog-ext-525001261-jspb: [1,null,null,null,"e6fa609c3fa255c0",null,null,0,[4,5,6,8],null,null,2,null,null,3,1,"66C38D6F-6D2C-4404-AF55-7ED34A4C707E"]`
- `x-goog-ext-525005358-jspb: ["<uuid>",1]`
- `x-goog-ext-73010989-jspb: [0]`
- `x-goog-ext-73010990-jspb: [0,0,0]`

## batchexecute RPC

Всего 62 вызова, 21 уникальный RPC-id.

### Основные RPC

| RPC | Payload | Назначение | Статус |
|-----|---------|------------|--------|
| `otAQ7b` | `[[["otAQ7b","[]",null,"generic"]]]` | Model list / warm-up | 200 |
| `sJBwce` | `[[["sJBwce","[[1,2]]",null,"generic"]]]` | WAA prerequisite | 200 |
| `ESY5D` | `[[["ESY5D","[null,[5]]",null,"generic"]]]` | Feature-flag heartbeat | 200 |
| `ESY5D` (entry 31) | `[null,[2,3,7,8,9,...,272]]` | Первичный запрос списка фич | 200 |
| `VxUbXb` | `[]` | UI init / session | 200 |
| `aPya6c` | `[]` | Heartbeat | 200 |
| `cYRIkd` | `["ru"]` | Locale | 200 |
| `L5adhe` | `[null...null,"e6fa609c3fa255c0",null,[100]]` / `[null...null,1,null,[265]]` | Telemetry / impressions | 200 |
| `Bsxleb` | `[[76091940,null,26],null,35,null,null,null,[null,null,"F43A8299-..."]]` | Config/state | 200 |
| `GPRiHf` | `[]` | — | 200 |
| `maGuAc` | `[1]` | — | 200 |
| `Te6DCf` | `[["ru"],[1,2]]` | Locale/config | 200 |
| `o30O0e` | `[["me"],[[["person.photo",...]],null,[1,7]]]` | User info | 200 |
| `K4WWud` | `[[1],["ru"]]` | **Новый в spike 002** | 200 |
| `qpEbW` | `[[[1,4],[6,6],[1,15]]]` / `[[[1,11],[2,11],[6,11]]]` | Telemetry | 200 |
| `CNgdBe` | `[1,["ru"],0]` / `[2,["ru"],0,null,[2]]` | Navigation/turn | 200 |
| `whPPme` | `["ru",null,[4]]` | — | 200 |
| `ozz5Z` | длинный массив ID | — | 200 |
| `I4z33b` | `[]` | — | 200 |
| `ku4Jyf` | `["ru",null,null,null,14,null,null,[32],null,[]]` | — | 200 |
| `MyzX6c` | `[]` | — | 200 |
| `PCck7e` | `["r_0d35e86934785889"]` | Rating/feedback | 200 |

### Новый RPC по сравнению с spike 001

- `K4WWud` — payload `[[1],["ru"]]`. Вероятно, связан с locale/model config.

## WAA-инициализация

### Sequence

1. `GET /app` (entry 0) — status 200, но **тело HTML в HAR не сохранено**.
2. `POST batchexecute?rpcids=otAQ7b` (entry 21)
3. `POST batchexecute?rpcids=sJBwce` (entry 22)
4. `POST waa-pa.../Waa/Create` (entry 23)
5. `POST ogads-pa.../GetAsyncData` (entry 17)
6. `POST batchexecute?rpcids=ESY5D` (entry 31/32)

### WAA Create

- URL: `https://waa-pa.clients6.google.com/$rpc/google.internal.waa.v1.Waa/Create`
- Status: **200 OK**
- Request body: `["br1aemAN9owlYRs9NnsA"]`
- Request headers:
  - `content-type: application/json+protobuf`
  - `x-client-data: CNeSywE=`
  - `x-goog-api-key: AIzaSyBGb5fGAyC-pRcRU6MUHb__b_vKha71HRE`
  - `x-user-agent: grpc-web-javascript/0.1`
  - Origin/Referer `https://gemini.google.com`
- Response: `content-type: application/json+protobuf; charset=UTF-8`, `content-length: 23757` (gzip), `size: 31374` uncompressed.
- **Вывод**: WAA Create **успешно отработал в браузере**. SDK имитирует этот запрос с тем же телом (`["br1aemAN9owlYRs9NnsA"]`), но параметр `br1aemAN9owlYRs9NnsA` возможно привязан к JS-сборке и нужно динамически извлекать из `/app` или JS bundle.

### ogads GetAsyncData

- URL: `https://ogads-pa.clients6.google.com/$rpc/google.internal.onegoogle.asyncdata.v1.AsyncDataService/GetAsyncData`
- Status: **200 OK**
- Request body (decoded): `[658,"https://gemini.google.com/app",658,"ru","ch",1,null,0,0,"","",1,0,null,103135050,[[1,9,13],0,1,1],[1],null,1,0,"<base64>",{"1001":0}]`
- Request headers:
  - `authorization: SAPISIDHASH 1786121751_... SAPISID1PHASH ... SAPISID3PHASH ...`
  - `content-type: application/json+protobuf`
  - `x-client-data: CNeSywE=`
  - `x-goog-api-key: AIzaSyCbsbvGCe7C9mCtdaTycZB2eUFuzsYKG_E`
  - `x-goog-authuser: 0`
- Response: `size: 53` — короткий, но статус 200.
- **Вывод**: ogads возвращает короткий WAA-контекст, который затем используется в `x-goog-ext-525001261-jspb`. SDK сейчас передаёт весь JSON ответа как `waa_context`, что избыточно; нужно выделить именно ту часть, что идёт в заголовок.

## /app HTML

- Entry 0: `GET https://gemini.google.com/app`, status 200, size 833989.
- **Тело ответа в HAR отсутствует** (`content.text: null`), поэтому `SNlM0e`, `FdrFJe`, `bl`, `qKIAYe`/`KnDnFf`, API keys извлечь не удалось.
- SDK (`src/session.rs`) уже реализует парсинг этих полей; ограничение HAR, не SDK.

## Upload flow

- `OPTIONS /upload/` → 200
- `POST /upload/` (`x-goog-upload-command: start`) → 200, возвращает `x-goog-upload-url` и `x-goog-upload-control-url`
- `POST /<upload_url>` (`x-goog-upload-command: upload, finalize`, offset 0) → 200 `x-goog-upload-status: final`
- Все шаги status 200 — изображение успешно загружено и использовано в StreamGenerate entry 178.

## Сравнение с spike 001

| Аспект | Spike 001 (`/full.har`) | Spike 002 (`/full1.har`) | Различие |
|--------|------------------------|--------------------------|----------|
| StreamGenerate | 1 (text fresh) | 2 (image fresh + text cont) | Появился image и continuation |
| Слот 96 | `0` (fresh) | `1` (fresh), `0` (cont) | **Зависит от fresh/continuation** |
| WAA token | ~2467 символов | ~2600–2645 символов | Длина варьируется, префикс одинаковый |
| batchexecute RPC | 20 уникальных | 21 уникальный | **+`K4WWud`** |
| WAA/ogads | Присутствовали | Присутствуют, статус 200 | Подтверждено |
| `/app` body | Было частично | **Не сохранено в HAR** | Нельзя извлечь HTML-токены |
| Upload | 1 flow | 1 flow, 200/200 | Подтверждён |

## Сравнение с текущей SDK

### Совпадает

- 97 слотов, общая структура.
- Слот 1 `["ru"]` — SDK использует `session.language`.
- Слот 2 fresh одинарный массив; SDK `ConversationState::to_slot2` корректен.
- Слот 4 — 32 hex nonce, SDK генерирует.
- Слоты 41/68/79/80 — `[1]`, `2`, `3`, `1` соответственно.
- URL StreamGenerate без `pageId`/`authuser`.
- WAA-цепочка `otAQ7b → sJBwce → Waa Create → ogads → ESY5D` реализована в `init_session`.
- `list_models` использует `otAQ7b`.
- Заголовки `x-client-data`, `x-goog-ext-*` добавлены.

### Несовпадения / улучшения SDK

1. **Слот 96**: SDK всегда ставит `0`. В захваченном HAR **fresh=1, continuation=0**. Нужно исправить `build_inner_req_list`.
2. **ogads тело**: SDK шлёт `[[waa_token]]`, но браузер шлёт полную структуру с `658`, URL, locale, base64 и `{"1001":0}`. Возможно, сокращённый вариант тоже работает, но нужно проверить.
3. **WAA Create параметр**: SDK хардкодит `["br1aemAN9owlYRs9NnsA"]`. Параметр, вероятно, нужно извлекать из JS bundle (`play.google.com/log` его логирует как `mk`/`rk`).
4. **`x-goog-ext-525001261-jspb`**: SDK использует ответ ogads целиком. Заголовок имеет фиксированную структуру; поле `"e6fa609c3fa255c0"` в position 4 совпадает с `L5adhe` payload и, вероятно, приходит из ogads/WAA. Нужно правильно извлекать именно эту структуру.
5. **`x-goog-ext-73010989-jspb`**: В HAR у batchexecute entry 21-22 стоит `[]`, а у StreamGenerate `[0]`. SDK всегда шлёт `[0]`; это может быть приемлемо.
6. **`sJBwce`/`ESY5D` payload**: SDK использует `[[1,2]]` и `[null,[5]]` — совпадает с повторяющимися вызовами.
7. **RPC `K4WWud`**: SDK не вызывает. Возможно не критично, но стоит добавить в warm-up для точности.
8. **Слот 6 `[1]`**: SDK ставит `[1]` только когда `browser_payload` is None. В HAR `[1]` и в fresh, и в continuation — SDK поведение совпадает для fallback (None), но стоит убедиться, что не дублируется при browser payload.
9. **Upload body Content-Type**: SDK шлёт `application/x-www-form-urlencoded`; HAR тоже — совпадает.
10. **Authorization для ogads**: SDK не добавляет `SAPISIDHASH` заголовок. Браузер добавляет. Для grpc-web запросов к ogads/waa это может быть обязательным (cookies передаются, но `Authorization` тоже есть).

## Обновлённый план рефакторинга SDK

1. **Исправить слот 96** в `src/proto/slots.rs`: `1` для fresh-запроса, `0` для continuation.
2. **Уточнить `x-goog-ext-525001261-jspb`**: в `src/session.rs`/`src/client.rs` строить строго по захваченной схеме `[1,null,null,null,<token>,null,null,0,[4,5,6,8],null,null,2,null,null,3,1,<uuid>]`, где `<token>` извлекается из ogads/WAA.
3. **Уточнить тело ogads**: попробовать сокращённый `[[waa_token]]`; если сервер отвечает ошибкой, перейти на полную структуру из HAR.
4. **Динамически получать WAA Create параметр**: искать `br1aemAN9owlYRs9NnsA` (или его аналог) в `/app` HTML/JS или логах.
5. **Добавить `Authorization: SAPISIDHASH ...`** для ogads/waa RPC.
6. **Добавить `K4WWud` в warm-up** с payload `[[1],["ru"]]`.
7. **Проверить `x-goog-ext-73010989-jspb`**: оставить `[0]` для StreamGenerate; для batchexecute warm-up можно `[]` или `[0]`.
8. **Реализовать парсинг WAA/ogads ответа** для извлечения `e6fa609c3fa255c0`-подобного токена и UUID для заголовка.
9. **Сохранить conversation state** из ответа StreamGenerate (response_id, rc_, continuation token) — сейчас SDK пытается парсить, но HAR не даёт проверить.
10. **Добавить тесты** на слот 96, на ogads body, на WAA header assembly.

## Риски

- HAR без response bodies не позволяет проверить формат ответа StreamGenerate и WAA/ogads.
- `br1aemAN9owlYRs9NnsA` может меняться между сборками; хардкод ненадёжен.
- `SAPISIDHASH` требует вычисления хэша от cookies (`SAPISID` + origin + timestamp) — нужна реализация в `src/auth.rs`.
- Google может менять payload `ESY5D` и RPC ids.

## Вывод

Протокол в целем **соответствует текущей SDK-реализации**. Главные уточнения:

- Слот 96 зависит от fresh/continuation.
- WAA-цепочка работает в браузере (200) и уже интегрирована в SDK, но требует уточнения тел и заголовков.
- Добавился RPC `K4WWud`.
- Для полной достоверности нужен HAR с включёнными response bodies или live-отладка.
