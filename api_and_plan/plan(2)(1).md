# План реализации API — `my-infra-otel`

## 1. Общая стратегия

Реализация должна идти от самого рискованного места к стабильному публичному API.

Главный риск проекта:

```txt
trace_id/span_id могут не появиться в JSON logs автоматически
```

Причины:

- неправильный порядок `tracing-subscriber` layers;
- event записан вне active span;
- OpenTelemetry context не связан с текущим `tracing` span;
- async context потерялся.

Поэтому первый этап — technical spike, а не полноценный API.

---

## 2. Milestone 0. Technical spike: trace-log correlation

### Цель

Проверить самый рискованный кусок:

> Один request должен дать span в Jaeger и JSON log с тем же `trace_id`.

### Задачи

1. Создать минимальный Axum service.
2. Поднять OpenTelemetry Collector.
3. Поднять Jaeger.
4. Настроить `tracing-subscriber`.
5. Настроить `tracing-opentelemetry`.
6. Настроить OTLP/HTTP exporter.
7. Создать request span.
8. Внутри handler вызвать `tracing::info!(...)`.
9. Получить JSON log с `trace_id`.
10. Сравнить `trace_id` в log и Jaeger.

### Критерий готовности

```txt
one request
  -> one server span in Jaeger
  -> one JSON stdout log
  -> log.trace_id == jaeger.trace_id
```

### Результат

- spike branch или example;
- короткий README с выводом;
- решение по реализации log enrichment.

---

## 3. Milestone 1. Workspace и базовая структура

### Цель

Создать основу проекта.

### Задачи

1. Создать Cargo workspace.
2. Создать crate `my-infra-otel`.
3. Создать examples:
   - `service-a`;
   - `service-b`.
4. Создать модули:
   - `config`;
   - `error`;
   - `init`;
   - `guard`;
   - `layer`;
   - `client`;
   - `request_builder`;
   - `labels`;
   - `events`;
   - `logging`;
   - `propagation`.
5. Добавить `README.md`.
6. Добавить `api.md`.
7. Добавить `plan.md`.
8. Добавить `infra/docker-compose.yml`.
9. Добавить `infra/otel-collector-config.yml`.

### Критерий готовности

```bash
cargo check --workspace
```

проходит успешно.

---

## 4. Milestone 2. Cargo dependencies и feature flags

### Цель

Заложить зависимости без перегруза.

### Базовые зависимости

```toml
[dependencies]
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
tracing-opentelemetry = "0.27"
opentelemetry = "0.26"
opentelemetry-otlp = { version = "0.26", features = ["http-proto", "reqwest-client"] }
tokio = { version = "1", features = ["rt", "macros", "signal"] }
tower = "0.5"
http = "1"
http-body = "1"
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### Feature flags

```toml
[features]
default = ["axum", "reqwest-client", "otlp", "json-logs"]
axum = []
reqwest-client = []
otlp = []
json-logs = []
env-config = []
file-config = []
test-utils = []
```

### Критерий готовности

```bash
cargo check --workspace --all-features
```

проходит успешно.

---

## 5. Milestone 3. Error model

### Цель

Реализовать typed errors библиотеки.

### Задачи

1. Создать `src/error.rs`.
2. Добавить `MyOtelError`.
3. Добавить `ConfigError`.
4. Добавить public alias:

```rust
pub type Result<T> = std::result::Result<T, MyOtelError>;
```

5. Не использовать `anyhow` в публичном API.
6. Подключить `thiserror`.
7. Покрыть форматирование ошибок тестами.

### Критерий готовности

- ошибки компилируются;
- `?` работает для `ConfigError`;
- `reqwest::Error` конвертируется в `MyOtelError::HttpClient`.

---

## 6. Milestone 4. Config builder

### Цель

Реализовать безопасную конфигурацию.

### Задачи

1. Создать `src/config.rs`.
2. Реализовать:
   - `TracingConfig`;
   - `TracingConfigBuilder`;
   - `SamplingMode`;
   - `LogFormat`.
3. Добавить defaults:
   - `environment = "local"`;
   - `otlp_endpoint = "http://localhost:4318/v1/traces"`;
   - `log_filter = RUST_LOG или "info"`;
   - `sampling = AlwaysOn`;
   - `export_timeout = 5s`;
   - `shutdown_timeout = 5s`;
   - `log_format = Json`.
4. Добавить validation:
   - service name not empty;
   - valid OTLP endpoint;
   - non-empty resource attr key;
   - reserved key rejection;
   - timeout > 0.
5. Написать unit tests.

### Критерий готовности

```rust
let config = TracingConfig::builder("service-a").build()?;
```

работает.

Invalid config возвращает typed error, а не panic.

---

## 7. Milestone 5. Attribute model и header labels

### Цель

Реализовать безопасную модель атрибутов.

### Задачи

1. Создать `src/labels.rs`.
2. Реализовать `AttributeKey`.
3. Реализовать reserved fields.
4. Реализовать `HeaderAttr`.
5. Валидировать `http::HeaderName`.
6. Валидировать telemetry attr key.
7. Написать tests:
   - valid header;
   - invalid header;
   - empty attr;
   - reserved attr;
   - dot-separated attr.

### Критерий готовности

```rust
HeaderAttr::new("x-user-id", "user.id")
```

работает.

Reserved/invalid keys возвращают ошибку.

---

## 8. Milestone 6. OpenTelemetry init

### Цель

Реализовать `init_global_tracing`.

### Задачи

1. Создать `src/init.rs`.
2. Создать `src/internal/otel.rs`.
3. Создать `src/internal/subscriber.rs`.
4. Настроить W3C TraceContext propagator.
5. Создать OTLP/HTTP exporter.
6. Создать tracer provider.
7. Добавить resource attributes.
8. Подключить `tracing-opentelemetry`.
9. Подключить `tracing-subscriber`.
10. Обработать ошибку повторной инициализации.
11. Вернуть `TracingGuard`.

### Критерий готовности

- `init_global_tracing(config)` работает;
- повторная инициализация возвращает `AlreadyInitialized`;
- traces отправляются в Collector.

---

## 9. Milestone 7. JSON logging и trace-log correlation

### Цель

Реализовать JSON stdout logs с `trace_id/span_id`.

### Задачи

1. Создать `src/logging.rs`.
2. Реализовать JSON logging layer.
3. Добавить service metadata:
   - `service.name`;
   - `service.version`;
   - `deployment.environment`.
4. Добавить extraction текущего trace/span id.
5. Проверить порядок layers.
6. Обработать event вне active span.
7. Написать integration test на log correlation.

### Критерий готовности

Внутри handler:

```rust
tracing::info!("checkout started");
```

должен дать JSON log с `trace_id/span_id`.

Вне request span:

```rust
tracing::info!("service starting");
```

может быть без `trace_id/span_id`.

---

## 10. Milestone 8. `TracingGuard`

### Цель

Реализовать корректный shutdown/flush.

### Задачи

1. Создать `src/guard.rs`.
2. Реализовать `TracingGuard`.
3. Реализовать `shutdown(self)`.
4. Реализовать best-effort `Drop`.
5. Учесть timeout.
6. Добавить test на явный shutdown.
7. Добавить README example для graceful shutdown.

### Критерий готовности

```rust
let guard = init_global_tracing(config)?;
guard.shutdown()?;
```

flush'ит spans без panic.

---

## 11. Milestone 9. Axum/Tower tracing layer

### Цель

Реализовать middleware для входящих HTTP-запросов.

### Задачи

1. Создать `src/layer.rs`.
2. Реализовать `MyOtelTracingLayer`.
3. Реализовать `MyOtelTracingLayerBuilder`.
4. Реализовать Tower `Layer`.
5. Реализовать Tower `Service`.
6. Extract incoming `traceparent`.
7. Создавать server span.
8. Записывать HTTP attrs:
   - method;
   - path;
   - route, если доступен;
   - status;
   - duration.
9. Записывать header attrs.
10. Не ломать response body.
11. Написать integration tests.

### Критерий готовности

Axum service:

```rust
Router::new()
    .route("/checkout", get(checkout))
    .layer(MyOtelTracingLayer::new());
```

создает server span на каждый request.

---

## 12. Milestone 10. Header labels in spans/log context

### Цель

Добавить custom labels из headers.

### Задачи

1. В layer читать configured headers.
2. Если header есть — добавить attr в span.
3. Если header отсутствует — ничего не делать.
4. Не логировать sensitive headers по умолчанию.
5. Проверить case-insensitive header names.
6. Написать tests:
   - `x-user-id -> user.id`;
   - `x-request-id -> request.id`;
   - absent header;
   - invalid config.

### Критерий готовности

Request:

```bash
curl \
  -H "X-User-ID: 42" \
  -H "X-Request-ID: req-123" \
  http://localhost:3000/checkout
```

добавляет:

```txt
user.id=42
request.id=req-123
```

в request span.

---

## 13. Milestone 11. `record_event`

### Цель

Реализовать business events.

### Задачи

1. Создать `src/events.rs`.
2. Реализовать `EventValue`.
3. Реализовать `EventField`.
4. Реализовать `record_event`.
5. Добавить `event.name`.
6. Записывать fields в structured log.
7. Проверить, что event внутри request имеет `trace_id/span_id`.
8. Добавить tests.

### Критерий готовности

```rust
record_event(
    "order.created",
    [
        EventField::string("order.id", "123"),
        EventField::string("operation.result", "success"),
    ],
);
```

создает JSON log с business fields и trace correlation.

---

## 14. Milestone 12. `TracedHttpClient`

### Цель

Реализовать outbound propagation через reqwest wrapper.

### Задачи

1. Создать `src/client.rs`.
2. Создать `src/request_builder.rs`.
3. Реализовать `TracedHttpClient`.
4. Реализовать `TracedRequestBuilder`.
5. Поддержать:
   - `get`;
   - `post`;
   - `request`;
   - `header`;
   - `json`;
   - `body`;
   - `send`.
6. В `send` создавать client span.
7. Inject'ить `traceparent`.
8. Записывать:
   - method;
   - URL;
   - host;
   - status code;
   - duration;
   - error type.
9. Не реализовывать retries.
10. Написать tests с локальным test server.

### Критерий готовности

Вызов:

```rust
client
    .get("http://service-b:3001/process")
    .send()
    .await?;
```

передает `traceparent` в `service-b`.

---

## 15. Milestone 13. Demo service-b

### Цель

Сделать второй сервис, который продолжает trace.

### Задачи

1. Создать `examples/service-b`.
2. Добавить `TracingConfig`.
3. Добавить `MyOtelTracingLayer`.
4. Добавить route `/process`.
5. Добавить route `/health`.
6. В handler записать log/event.
7. Проверить incoming `traceparent`.

### Критерий готовности

`service-b` принимает request от `service-a` и создает child/server span в том же trace.

---

## 16. Milestone 14. Demo service-a

### Цель

Сделать первый сервис, который вызывает `service-b`.

### Задачи

1. Создать `examples/service-a`.
2. Добавить `TracingConfig`.
3. Добавить `MyOtelTracingLayer`.
4. Добавить header labels:
   - `x-user-id -> user.id`;
   - `x-request-id -> request.id`;
   - `x-tenant-id -> tenant.id`.
5. Добавить `TracedHttpClient`.
6. Добавить route `/checkout`.
7. Добавить route `/health`.
8. В handler:
   - записать `record_event("checkout.started", ...)`;
   - вызвать `service-b`;
   - вернуть JSON response.

### Критерий готовности

Request в `service-a` вызывает `service-b`, и оба сервиса оказываются в одном trace.

---

## 17. Milestone 15. Infra: Collector + Jaeger

### Цель

Собрать локальную demo-инфраструктуру.

### Задачи

1. Написать `docker-compose.yml`.
2. Добавить OpenTelemetry Collector.
3. Добавить Jaeger.
4. Настроить OTLP/HTTP receiver.
5. Настроить exporter в Jaeger.
6. Описать порты:
   - Collector OTLP/HTTP;
   - Jaeger UI.
7. Проверить `docker compose up`.

### Критерий готовности

Jaeger UI показывает traces от `service-a` и `service-b`.

---

## 18. Milestone 16. End-to-end smoke test

### Цель

Проверить полный flow.

### Задачи

1. Поднять Collector + Jaeger.
2. Запустить `service-b`.
3. Запустить `service-a`.
4. Выполнить curl:

```bash
curl \
  -H "X-User-ID: 42" \
  -H "X-Request-ID: req-123" \
  -H "X-Tenant-ID: demo" \
  http://localhost:3000/checkout
```

5. Проверить stdout logs.
6. Проверить Jaeger trace.
7. Проверить совпадение `trace_id`.
8. Проверить header labels.
9. Проверить business event fields.

### Критерий готовности

Один request дает:

```txt
service-a /checkout
  service-a -> service-b client span
    service-b /process
```

и одинаковый `trace_id` в логах обоих сервисов.

---

## 19. Milestone 17. Testing strategy

### Цель

Покрыть API тестами перед заморозкой MVP.

### Unit tests

Покрыть:

- `TracingConfigBuilder`;
- validation service name;
- validation OTLP endpoint;
- validation resource attrs;
- reserved fields;
- `HeaderAttr::new`;
- `AttributeKey::new`;
- `EventField`.

### Integration tests

Покрыть:

1. Входящий request создает server span.
2. Missing `traceparent` создает новый trace.
3. Existing `traceparent` извлекается.
4. Header labels попадают в span.
5. Missing configured header не ломает request.
6. `TracedHttpClient` inject'ит `traceparent`.
7. `record_event` пишет structured event.
8. Logs inside request содержат `trace_id/span_id`.
9. `TracingGuard::shutdown()` flush'ит exporter.

### Критерий готовности

```bash
cargo test --workspace --all-features
```

проходит успешно.

---

## 20. Milestone 18. Documentation

### Цель

Сделать проект понятным для пользователя.

### Задачи

1. Написать README.
2. Объяснить, зачем проект нужен, если есть OpenTelemetry.
3. Добавить quick start.
4. Добавить minimal example.
5. Добавить production-like example.
6. Добавить demo flow.
7. Добавить troubleshooting:
   - нет trace_id в logs;
   - нет traces в Jaeger;
   - wrong OTLP endpoint;
   - detached `tokio::spawn`;
   - double init subscriber.
8. Добавить limitations MVP.
9. Добавить future roadmap.

### Критерий готовности

Новый пользователь может запустить demo по README без чтения исходников.

---

## 21. Milestone 19. Polish и API freeze для MVP

### Цель

Зафиксировать минимальный стабильный публичный API.

### Задачи

1. Проверить public exports.
2. Убрать лишние public-типы.
3. Проверить `cargo doc`.
4. Добавить rustdoc examples.
5. Проверить naming.
6. Проверить feature flags.
7. Проверить `cargo clippy`.
8. Проверить `cargo fmt`.
9. Проверить `cargo test --workspace`.
10. Проставить версию `0.1.0`.

### Критерий готовности

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace --all-features
cargo doc --workspace --no-deps
```

проходят успешно.

---

## 22. Рекомендуемый порядок коммитов

### Commit 1

```txt
chore: initialize workspace
```

Содержит:

- Cargo workspace;
- crate skeleton;
- examples skeleton;
- empty modules.

---

### Commit 2

```txt
feat(error): add public error model
```

Содержит:

- `MyOtelError`;
- `ConfigError`;
- public `Result<T>` alias.

---

### Commit 3

```txt
feat(config): add tracing config builder
```

Содержит:

- `TracingConfig`;
- `TracingConfigBuilder`;
- defaults;
- validation tests.

---

### Commit 4

```txt
feat(labels): add header attributes and attribute keys
```

Содержит:

- `AttributeKey`;
- `HeaderAttr`;
- reserved fields;
- validation tests.

---

### Commit 5

```txt
feat(init): initialize tracing subscriber and otlp exporter
```

Содержит:

- `init_global_tracing`;
- basic OpenTelemetry setup;
- `TracingGuard`.

---

### Commit 6

```txt
feat(logging): add json logs with trace correlation
```

Содержит:

- JSON logging;
- `trace_id/span_id` enrichment;
- integration test.

---

### Commit 7

```txt
feat(layer): add tower tracing layer
```

Содержит:

- `MyOtelTracingLayer`;
- `MyOtelTracingLayerBuilder`;
- server span creation.

---

### Commit 8

```txt
feat(layer): record http attributes and header labels
```

Содержит:

- method/path/status/duration;
- header attr extraction;
- tests.

---

### Commit 9

```txt
feat(events): add business event recording
```

Содержит:

- `EventField`;
- `EventValue`;
- `record_event`;
- tests.

---

### Commit 10

```txt
feat(client): add traced reqwest client
```

Содержит:

- `TracedHttpClient`;
- `TracedRequestBuilder`;
- outbound `traceparent` injection;
- client span attrs.

---

### Commit 11

```txt
feat(examples): add service-a and service-b
```

Содержит:

- Axum demo services;
- `/checkout`;
- `/process`;
- `/health`.

---

### Commit 12

```txt
chore(infra): add collector and jaeger compose stack
```

Содержит:

- `docker-compose.yml`;
- Collector config;
- Jaeger config.

---

### Commit 13

```txt
test: add end-to-end propagation smoke test
```

Содержит:

- propagation test;
- log correlation test;
- shutdown test.

---

### Commit 14

```txt
docs: add quick start and troubleshooting
```

Содержит:

- README;
- limitations;
- demo flow;
- troubleshooting.

---

### Commit 15

```txt
chore: prepare v0.1.0 mvp
```

Содержит:

- API cleanup;
- docs cleanup;
- clippy/fmt/test pass.

---

## 23. MVP Definition of Done

MVP готов, если:

1. `TracingConfig` собирается через builder.
2. `init_global_tracing` настраивает JSON logs и OTLP/HTTP traces.
3. `TracingGuard` flush'ит traces.
4. `MyOtelTracingLayer` создает server span.
5. Incoming `traceparent` извлекается.
6. Missing `traceparent` создает новый trace.
7. Header labels попадают в span.
8. Logs inside request содержат `trace_id/span_id`.
9. `record_event` пишет business fields.
10. `TracedHttpClient` inject'ит `traceparent`.
11. `service-a -> service-b` видны в Jaeger как один trace.
12. README содержит quick start.
13. README явно объясняет ограничения MVP.
14. Tests покрывают config, labels, propagation, logs correlation и shutdown.

---

## 24. After-MVP roadmap

1. Config from env.
2. Config from TOML/YAML.
3. Sampling policies.
4. Baggage.
5. Metrics.
6. Loki logs.
7. Tempo/Grafana.
8. Tonic support.
9. Actix support.
10. `spawn_in_current_span`.
11. Collector auth.
12. Grafana dashboard.
13. Load test demo.
14. GitHub Actions CI.
15. Published crate docs.
