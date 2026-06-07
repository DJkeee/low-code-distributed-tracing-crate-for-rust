# API Design Document — `my-infra-otel`

## 1. Контекст проекта

`my-infra-otel` — легковесный Rust SDK для стандартизированного distributed tracing в Axum-сервисах.

Проект не заменяет OpenTelemetry. Он предоставляет opinionated platform/golden-path слой поверх:

- `tracing`;
- `tracing-subscriber`;
- `tracing-opentelemetry`;
- `opentelemetry`;
- `opentelemetry-otlp`;
- `tower`;
- `axum`;
- `reqwest`.

Главная задача API:

> Дать разработчику рабочий distributed tracing, JSON stdout logs с `trace_id/span_id`, inbound/outbound trace propagation и business events без ручной настройки всего OpenTelemetry pipeline.

---

## 2. Цели API

API должен позволять:

1. Инициализировать tracing одной конфигурацией.
2. Автоматически создавать server span на каждый входящий Axum HTTP request.
3. Извлекать входящий `traceparent`.
4. Создавать новый trace, если `traceparent` отсутствует.
5. Добавлять `trace_id` и `span_id` в JSON stdout logs.
6. Экспортировать traces через OTLP/HTTP.
7. Пробрасывать trace context в исходящие HTTP-запросы через `reqwest`.
8. Добавлять custom labels из HTTP headers.
9. Записывать business events внутри активного trace.
10. Корректно flush/shutdown exporter при завершении сервиса.

---

## 3. Не цели MVP

В MVP не реализуются:

- собственный OpenTelemetry;
- собственный OTLP exporter;
- observability backend;
- metrics SDK;
- logs export через OTLP;
- Loki;
- Tempo/Grafana;
- Actix-web;
- Tonic;
- baggage;
- сложные sampling policies;
- auth к Collector;
- hot reload конфигурации;
- полноценный wrapper всего `reqwest` API;
- автоматическая корреляция detached `tokio::spawn`.

---

## 4. Пользователи API

| Пользователь | Что ему нужно |
|---|---|
| Rust backend-разработчик | Быстро подключить tracing к Axum-сервису |
| Platform/SRE-инженер | Стандартизировать observability во внутренних сервисах |
| Автор demo | Показать flow `service-a -> service-b -> Collector -> Jaeger` |

---

## 5. Основные сценарии использования

### 5.1 Минимальное подключение

```rust
use axum::{routing::get, Router};
use my_infra_otel::{init_global_tracing, MyOtelTracingLayer, TracingConfig};

let config = TracingConfig::builder("service-a").build()?;
let guard = init_global_tracing(config)?;

let app = Router::new()
    .route("/checkout", get(checkout))
    .layer(MyOtelTracingLayer::new());
```

---

### 5.2 Production-like конфигурация

```rust
use my_infra_otel::{init_global_tracing, TracingConfig};

let config = TracingConfig::builder("service-a")
    .environment("prod")
    .version("1.2.3")
    .otlp_endpoint("http://otel-collector:4318/v1/traces")
    .resource_attr("team", "platform")
    .resource_attr("datacenter", "dc-1")
    .log_filter("info,service_a=debug,my_infra_otel=debug")
    .build()?;

let guard = init_global_tracing(config)?;
```

---

### 5.3 Header labels

```rust
use my_infra_otel::MyOtelTracingLayer;

let layer = MyOtelTracingLayer::builder()
    .header_attr("x-user-id", "user.id")
    .header_attr("x-request-id", "request.id")
    .header_attr("x-tenant-id", "tenant.id")
    .build()?;
```

---

### 5.4 Outbound HTTP propagation

```rust
use my_infra_otel::TracedHttpClient;

let client = TracedHttpClient::new(reqwest::Client::new());

let response = client
    .get("http://service-b:3001/process")
    .send()
    .await?;
```

---

### 5.5 Business event

```rust
use my_infra_otel::{record_event, EventField};

record_event(
    "checkout.started",
    [
        EventField::string("order.id", "123"),
        EventField::string("payment.provider", "demo"),
    ],
);
```

---

## 6. Типы API

### 6.1 Библиотечный Rust API

Основной API проекта.

Используется для:

- инициализации tracing;
- настройки resource attributes;
- подключения Axum/Tower layer;
- создания traced HTTP client;
- записи business events.

Не должен содержать:

- demo-specific routes;
- Docker Compose logic;
- бизнес-логику сервисов;
- детали Jaeger UI.

---

### 6.2 Middleware API

Представлен типом:

```rust
MyOtelTracingLayer
```

Отвечает за:

- incoming HTTP request instrumentation;
- creation server span;
- extraction `traceparent`;
- HTTP attributes;
- custom header labels;
- status/duration;
- trace-log correlation внутри request lifecycle.

---

### 6.3 SDK API для HTTP client

Представлен типом:

```rust
TracedHttpClient
```

Отвечает за:

- outbound client span;
- injection `traceparent`;
- status/duration/error attributes.

Не должен делать:

- retries;
- circuit breaking;
- service discovery;
- load balancing;
- mirror всего `reqwest`.

---

### 6.4 HTTP API

Публичный HTTP API самому SDK не нужен.

SDK инструментирует пользовательский HTTP API, но не предоставляет собственный HTTP server.

---

### 6.5 CLI API

CLI не входит в MVP.

After-MVP возможны команды:

```txt
my-infra-otel check --endpoint http://localhost:4318/v1/traces
my-infra-otel print-default-config --format toml
my-infra-otel demo up
```

---

## 7. Архитектура проекта

## 7.1 Workspace structure

```txt
my-infra-otel/
├── Cargo.toml
├── README.md
├── api.md
├── plan.md
│
├── crates/
│   └── my-infra-otel/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── config.rs
│           ├── error.rs
│           ├── init.rs
│           ├── guard.rs
│           ├── layer.rs
│           ├── client.rs
│           ├── request_builder.rs
│           ├── labels.rs
│           ├── events.rs
│           ├── logging.rs
│           ├── propagation.rs
│           └── internal/
│               ├── mod.rs
│               ├── otel.rs
│               ├── subscriber.rs
│               └── time.rs
│
├── examples/
│   ├── service-a/
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   │
│   └── service-b/
│       ├── Cargo.toml
│       └── src/main.rs
│
├── infra/
│   ├── docker-compose.yml
│   └── otel-collector-config.yml
│
└── tests/
    ├── config.rs
    ├── propagation.rs
    ├── header_attrs.rs
    ├── log_correlation.rs
    └── shutdown.rs
```

---

## 7.2 Модули

| Модуль | Ответственность |
|---|---|
| `config` | `TracingConfig`, builder, defaults, validation |
| `error` | typed errors библиотеки |
| `init` | global tracing initialization |
| `guard` | shutdown/flush |
| `layer` | Tower/Axum middleware |
| `client` | traced reqwest wrapper |
| `request_builder` | traced request builder |
| `labels` | header labels и attribute keys |
| `events` | business events |
| `logging` | JSON logs и trace-log correlation |
| `propagation` | extract/inject trace context |
| `internal` | детали OpenTelemetry/subscriber setup |

---

## 8. Публичные exports

```rust
pub use crate::client::{TracedHttpClient, TracedRequestBuilder};
pub use crate::config::{LogFormat, SamplingMode, TracingConfig, TracingConfigBuilder};
pub use crate::error::{ConfigError, MyOtelError};
pub use crate::events::{record_event, EventField, EventValue};
pub use crate::guard::TracingGuard;
pub use crate::init::init_global_tracing;
pub use crate::labels::{AttributeKey, HeaderAttr};
pub use crate::layer::{MyOtelTracingLayer, MyOtelTracingLayerBuilder};
```

---

## 9. Конфигурация

## 9.1 `TracingConfig`

```rust
#[derive(Debug, Clone)]
pub struct TracingConfig {
    pub service_name: String,
    pub service_version: Option<String>,
    pub environment: String,
    pub otlp_endpoint: String,
    pub log_filter: String,
    pub resource_attrs: Vec<(String, String)>,
    pub sampling: SamplingMode,
    pub export_timeout: std::time::Duration,
    pub shutdown_timeout: std::time::Duration,
    pub log_format: LogFormat,
}
```

---

## 9.2 `TracingConfigBuilder`

```rust
impl TracingConfig {
    pub fn builder(service_name: impl Into<String>) -> TracingConfigBuilder;
}
```

```rust
impl TracingConfigBuilder {
    pub fn version(self, version: impl Into<String>) -> Self;

    pub fn environment(self, environment: impl Into<String>) -> Self;

    pub fn otlp_endpoint(self, endpoint: impl Into<String>) -> Self;

    pub fn log_filter(self, filter: impl Into<String>) -> Self;

    pub fn resource_attr(
        self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self;

    pub fn sampling(self, sampling: SamplingMode) -> Self;

    pub fn export_timeout(self, timeout: std::time::Duration) -> Self;

    pub fn shutdown_timeout(self, timeout: std::time::Duration) -> Self;

    pub fn log_format(self, format: LogFormat) -> Self;

    pub fn build(self) -> Result<TracingConfig, MyOtelError>;
}
```

---

## 9.3 Defaults

| Поле | Значение по умолчанию |
|---|---|
| `environment` | `local` |
| `otlp_endpoint` | `http://localhost:4318/v1/traces` |
| `log_filter` | `RUST_LOG` или `info` |
| `sampling` | `AlwaysOn` |
| `export_timeout` | `5s` |
| `shutdown_timeout` | `5s` |
| `log_format` | `Json` |

---

## 9.4 `SamplingMode`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingMode {
    AlwaysOn,
}
```

После MVP можно расширить:

```rust
pub enum SamplingMode {
    AlwaysOn,
    AlwaysOff,
    Ratio(f64),
    ParentBasedRatio(f64),
}
```

---

## 9.5 `LogFormat`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Pretty,
}
```

MVP default:

```rust
LogFormat::Json
```

---

## 10. Инициализация tracing

## 10.1 `init_global_tracing`

```rust
pub fn init_global_tracing(config: TracingConfig) -> Result<TracingGuard, MyOtelError>;
```

Функция должна:

1. Валидировать config.
2. Настроить W3C propagator.
3. Создать OpenTelemetry tracer provider.
4. Настроить OTLP/HTTP exporter.
5. Добавить resource attributes.
6. Собрать `tracing-subscriber`.
7. Добавить OpenTelemetry layer.
8. Добавить JSON stdout logging layer.
9. Добавить enrichment `trace_id/span_id`.
10. Вернуть `TracingGuard`.

---

## 10.2 `TracingGuard`

```rust
#[derive(Debug)]
pub struct TracingGuard {
    shutdown_timeout: std::time::Duration,
}
```

```rust
impl TracingGuard {
    pub fn shutdown(self) -> Result<(), MyOtelError>;

    pub fn shutdown_timeout(&self) -> std::time::Duration;
}
```

Поведение:

- `shutdown()` явно flush'ит exporter;
- `Drop` делает best-effort shutdown;
- ошибки shutdown возвращаются только из `shutdown()`.

---

## 11. Axum/Tower layer

## 11.1 `MyOtelTracingLayer`

```rust
#[derive(Debug, Clone)]
pub struct MyOtelTracingLayer {
    header_attrs: Vec<HeaderAttr>,
    capture_route: bool,
    capture_user_agent: bool,
}
```

```rust
impl MyOtelTracingLayer {
    pub fn new() -> Self;

    pub fn builder() -> MyOtelTracingLayerBuilder;
}
```

```rust
impl<S> tower::Layer<S> for MyOtelTracingLayer {
    type Service = MyOtelTracingService<S>;

    fn layer(&self, inner: S) -> Self::Service;
}
```

---

## 11.2 `MyOtelTracingLayerBuilder`

```rust
#[derive(Debug, Clone)]
pub struct MyOtelTracingLayerBuilder {
    header_attrs: Vec<HeaderAttr>,
    capture_route: bool,
    capture_user_agent: bool,
}
```

```rust
impl MyOtelTracingLayerBuilder {
    pub fn header_attr(
        self,
        header_name: impl AsRef<str>,
        attr_key: impl AsRef<str>,
    ) -> Self;

    pub fn capture_route(self, enabled: bool) -> Self;

    pub fn capture_user_agent(self, enabled: bool) -> Self;

    pub fn build(self) -> Result<MyOtelTracingLayer, MyOtelError>;
}
```

---

## 11.3 Поведение layer

На каждый request layer должен:

1. Извлечь incoming `traceparent`, если он есть.
2. Создать server span.
3. Записать HTTP method.
4. Записать path.
5. Записать route, если доступен.
6. Записать configured header attrs.
7. Выполнить inner service.
8. Записать response status.
9. Записать duration.
10. Связать handler logs с request span.

---

## 12. Header labels

## 12.1 `HeaderAttr`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderAttr {
    header_name: http::HeaderName,
    attr_key: AttributeKey,
}
```

```rust
impl HeaderAttr {
    pub fn new(
        header_name: impl AsRef<str>,
        attr_key: impl AsRef<str>,
    ) -> Result<Self, MyOtelError>;

    pub fn header_name(&self) -> &http::HeaderName;

    pub fn attr_key(&self) -> &AttributeKey;
}
```

---

## 12.2 `AttributeKey`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttributeKey(String);
```

```rust
impl AttributeKey {
    pub fn new(value: impl Into<String>) -> Result<Self, MyOtelError>;

    pub fn as_str(&self) -> &str;
}
```

Validation:

- key не должен быть пустым;
- key не должен содержать пробелы;
- key не должен конфликтовать с reserved fields;
- рекомендуемый формат: `user.id`, `request.id`, `tenant.id`.

Reserved fields:

```txt
timestamp
level
target
message
trace_id
span_id
service.name
service.version
deployment.environment
event.name
```

---

## 13. Traced HTTP client

## 13.1 `TracedHttpClient`

```rust
#[derive(Debug, Clone)]
pub struct TracedHttpClient {
    inner: reqwest::Client,
    config: ClientTracingConfig,
}
```

```rust
impl TracedHttpClient {
    pub fn new(inner: reqwest::Client) -> Self;

    pub fn builder(inner: reqwest::Client) -> TracedHttpClientBuilder;

    pub fn request(
        &self,
        method: reqwest::Method,
        url: impl reqwest::IntoUrl,
    ) -> TracedRequestBuilder;

    pub fn get(&self, url: impl reqwest::IntoUrl) -> TracedRequestBuilder;

    pub fn post(&self, url: impl reqwest::IntoUrl) -> TracedRequestBuilder;
}
```

---

## 13.2 `TracedRequestBuilder`

```rust
pub struct TracedRequestBuilder {
    inner: reqwest::RequestBuilder,
    method: reqwest::Method,
    url: String,
}
```

```rust
impl TracedRequestBuilder {
    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        K: reqwest::header::IntoHeaderName,
        V: Into<reqwest::header::HeaderValue>;

    pub fn json<T: serde::Serialize + ?Sized>(self, json: &T) -> Self;

    pub fn body<T: Into<reqwest::Body>>(self, body: T) -> Self;

    pub async fn send(self) -> Result<reqwest::Response, MyOtelError>;
}
```

---

## 13.3 Поведение `send`

`send` должен:

1. Создать client span.
2. Взять текущий trace context.
3. Inject'ить `traceparent` в outgoing headers.
4. Выполнить request.
5. Записать status code.
6. Записать duration.
7. Записать error attrs при ошибке.
8. Вернуть `reqwest::Response` или `MyOtelError`.

---

## 14. Business events

## 14.1 `record_event`

```rust
pub fn record_event<I>(name: impl Into<String>, fields: I)
where
    I: IntoIterator<Item = EventField>;
```

---

## 14.2 `EventField`

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct EventField {
    key: AttributeKey,
    value: EventValue,
}
```

```rust
impl EventField {
    pub fn string(key: impl AsRef<str>, value: impl Into<String>) -> Self;

    pub fn bool(key: impl AsRef<str>, value: bool) -> Self;

    pub fn i64(key: impl AsRef<str>, value: i64) -> Self;

    pub fn f64(key: impl AsRef<str>, value: f64) -> Self;
}
```

---

## 14.3 `EventValue`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum EventValue {
    String(String),
    Bool(bool),
    I64(i64),
    F64(f64),
}
```

---

## 14.4 Семантика `record_event`

`record_event` должен:

1. Записать structured `tracing` event.
2. Добавить `event.name`.
3. Добавить переданные fields.
4. Связать event с текущим `trace_id/span_id`, если active trace span существует.
5. Не менять уже созданный span задним числом.

Важно:

```txt
record_event != set_span_attribute
```

Это event/log внутри текущего trace context, а не mutation span attributes.

---

## 15. Error model

## 15.1 Главный Result alias

```rust
pub type Result<T> = std::result::Result<T, MyOtelError>;
```

---

## 15.2 `MyOtelError`

```rust
#[derive(Debug, thiserror::Error)]
pub enum MyOtelError {
    #[error("invalid configuration: {0}")]
    Config(#[from] ConfigError),

    #[error("global tracing subscriber is already initialized")]
    AlreadyInitialized,

    #[error("failed to initialize OpenTelemetry exporter: {0}")]
    ExporterInit(String),

    #[error("failed to initialize tracing subscriber: {0}")]
    SubscriberInit(String),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("trace shutdown failed: {0}")]
    Shutdown(String),

    #[error("invalid header attribute: {0}")]
    HeaderAttr(String),

    #[error("invalid event field: {0}")]
    EventField(String),
}
```

---

## 15.3 `ConfigError`

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("service name cannot be empty")]
    EmptyServiceName,

    #[error("invalid OTLP endpoint: {0}")]
    InvalidOtlpEndpoint(String),

    #[error("invalid resource attribute key: {0}")]
    InvalidResourceAttributeKey(String),

    #[error("reserved attribute key cannot be used: {0}")]
    ReservedAttributeKey(String),

    #[error("export timeout must be greater than zero")]
    InvalidExportTimeout,

    #[error("shutdown timeout must be greater than zero")]
    InvalidShutdownTimeout,
}
```

---

## 15.4 Классификация ошибок

| Ошибка | Тип | Retry | Возвращать наружу | Логировать |
|---|---|---:|---:|---:|
| Empty service name | user config error | нет | да | нет |
| Invalid OTLP endpoint | user config error | нет | да | нет |
| Already initialized | runtime setup error | нет | да | да |
| Exporter init failed | infra startup error | после исправления infra | да | да |
| reqwest error | transport runtime error | решает caller | да | да |
| invalid header attr | user config error | нет | да | нет |
| shutdown failed | infra shutdown error | нет | да | да |
| missing `traceparent` | normal path | нет | нет | нет |
| log outside span | normal path | нет | нет | нет |

---

## 16. Observability behavior

## 16.1 Logs

MVP logs:

- stdout only;
- JSON by default;
- structured fields;
- `trace_id/span_id` только внутри active span;
- service metadata;
- business event fields.

Example:

```json
{
  "timestamp": "2026-06-07T19:00:00Z",
  "level": "INFO",
  "target": "service_a",
  "message": "checkout started",
  "service.name": "service-a",
  "service.version": "0.1.0",
  "deployment.environment": "dev",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba902b7",
  "event.name": "checkout.started",
  "order.id": "123"
}
```

---

## 16.2 Traces

### Server span

Name:

```txt
HTTP {method} {route}
```

Attributes:

```txt
http.request.method
url.path
http.route
http.response.status_code
user.id
request.id
tenant.id
```

---

### Client span

Name:

```txt
HTTP {method}
```

Attributes:

```txt
http.request.method
url.full
server.address
http.response.status_code
duration.ms
error.type
```

---

## 16.3 Trace propagation

MVP:

- W3C `traceparent`;
- extract incoming context;
- inject outgoing context;
- no baggage.

---

## 16.4 Metrics

Metrics не входят в MVP.

After-MVP возможные метрики:

| Metric | Type |
|---|---|
| `my_infra_otel_requests_total` | counter |
| `my_infra_otel_request_duration_seconds` | histogram |
| `my_infra_otel_client_requests_total` | counter |
| `my_infra_otel_export_errors_total` | counter |
| `my_infra_otel_record_events_total` | counter |

---

## 17. Demo HTTP API

Публичный HTTP API SDK не нужен.

Для demo-сервисов достаточно:

| Method | Path | Service | Purpose |
|---|---|---|---|
| GET | `/checkout` | service-a | Создать server span, записать event, вызвать service-b |
| GET | `/process` | service-b | Продолжить trace и вернуть ответ |
| GET | `/health` | оба | Healthcheck |

---

## 18. Feature flags

Рекомендуемые flags:

| Feature | Default | Назначение |
|---|---:|---|
| `axum` | yes | Tower/Axum layer |
| `reqwest-client` | yes | Traced reqwest client |
| `otlp` | yes | OTLP exporter |
| `json-logs` | yes | JSON stdout logs |
| `env-config` | no | Config from env |
| `file-config` | no | Config from TOML/YAML |
| `test-utils` | no | Test helpers |

---

## 19. MVP public API summary

```rust
let config = TracingConfig::builder("service-a")
    .environment("dev")
    .version("0.1.0")
    .otlp_endpoint("http://otel-collector:4318/v1/traces")
    .resource_attr("datacenter", "dc-1")
    .resource_attr("team", "platform")
    .build()?;

let guard = init_global_tracing(config)?;

let client = TracedHttpClient::new(reqwest::Client::new());

let layer = MyOtelTracingLayer::builder()
    .header_attr("x-user-id", "user.id")
    .header_attr("x-request-id", "request.id")
    .header_attr("x-tenant-id", "tenant.id")
    .build()?;

let app = Router::new()
    .route("/checkout", get(checkout))
    .layer(layer);
```

Handler:

```rust
record_event(
    "checkout.started",
    [
        EventField::string("order.id", "123"),
        EventField::string("payment.provider", "demo"),
    ],
);
```

---

## 20. MVP Definition of Done

MVP считается готовым, если:

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
