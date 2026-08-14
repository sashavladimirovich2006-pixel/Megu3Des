# Megu3D — Архитектура

Версия 0.1 · 2026-08-13 · draft M0
Связанное: [assumptions](./assumptions.md) · [PRD](./01-prd.md) · [roadmap](./03-roadmap.md) · [ui-architecture](./04-ui-architecture.md)

## 1. Принципы

1. Одно место истины: состояние сцены живёт в Rust-ядре. UI хранит только производные view-модели.
2. UI отправляет intents, ядро применяет команды. UI не мутирует данные напрямую.
3. Всё обратимо: любое изменение — Command с обратным patch. Пути мутации в обход истории нет.
4. Типизированный контракт: IPC-типы генерируются из Rust и потребляются TS. Ручных дублей типов нет.
5. Границы, а не слои-«кашица»: C++ только через узкий safe FFI, Go только через JSON-RPC.
6. Версионируемые данные: у проекта есть `schemaVersion` и цепочка миграций с первого коммита.
7. UI-поток свят: ничто не блокирует отрисовку дольше кадра, тяжёлое уходит в пул задач.
8. Тестируемость выше удобства: ядро тестируется headless — команды, IO, импорт/экспорт.

## 2. Топология

```text
apps/desktop (Tauri v2)
├─ WebView: React + TS (packages/ui) ── panels, editors, palette
└─ Native surface: viewport (wgpu), без DOM-оверхеда
        │ typed IPC (packages/ipc)          ▲ frames / events
        ▼                                   │
Tauri shell: commands, events, fs, dialogs, updater, single-instance
        │ intents / deltas (mpsc)
        ▼
Rust core (crates/*)
  megu3d-app      orchestration, session, autosave, jobs
  megu3d-core     scene graph, components, selection, invariants
  megu3d-cmd      Command trait, transactions, undo/redo
  megu3d-io       .megu3d container, migrations, OBJ/glTF, asset resolver
  megu3d-render   render graph, PBR passes, gizmo/overlay, picking
  megu3d-mesh     mesh data, half-edge EditMesh, normals/tangents
  megu3d-math     vectors, quats, transforms, AABB, ray, units
  megu3d-kernels  safe wrappers → C++ kernels (FFI)
  megu3d-sidecar  JSON-RPC client для Go-сервисов
        │ C ABI (bindgen)                    │ JSON-RPC 2.0 / stdio
        ▼                                    ▼
cpp/megu3d_kernels                      go/assetd
 geometry, bvh, subdiv,                  индексация ассетов,
 simulation, baking,                     thumbnails, watcher
 path tracing kernels                    (подключается с P1)
```

## 3. Монорепозиторий

```text
megu3d/
├─ apps/desktop/{src,src-tauri}      Tauri v2 приложение
├─ packages/ui                       React-компоненты, панели, дизайн-система
├─ packages/types                    сгенерированные TS-типы из Rust (не править руками)
├─ packages/ipc                      типизированные обёртки invoke/subscribe
├─ packages/i18n                     ICU-сообщения RU/EN
├─ packages/config                   общие eslint/ts/prettier конфиги
├─ crates/megu3d-{app,core,cmd,io,mesh,math,render,kernels,kernels-sys,sidecar}
├─ cpp/megu3d_kernels                C++20, MSVC, C ABI наружу
├─ go/assetd                         sidecar (P1+)
├─ docs/
├─ tests/                            e2e, фикстуры сцен, бенчмарки
└─ .github/workflows/
```

Правила зависимостей (проверяются линтом архитектуры):
- `megu3d-core` не зависит от `render`, `io`, Tauri и UI.
- `megu3d-cmd` зависит только от `core` и `math`.
- Только `megu3d-app` знает про Tauri.
- UI-пакеты не импортируют ничего из `crates/`, кроме сгенерированных типов.
- `cpp/` не знает о Rust-типах: только POD-структуры и C ABI.

## 4. Роли слоёв

| Слой | Отвечает за | Не отвечает за |
|---|---|---|
| React/TS | панели, инспекторы, редакторы, ввод, презентация | состояние сцены, геометрию |
| Tauri shell | окно, IPC, файловые диалоги, updater, single instance | бизнес-логику |
| Rust core | сцена, команды, undo/redo, IO, ассеты, рендер-граф, валидация | оформление UI |
| C++ kernels | BVH, subdiv, симуляции, baking, трассировка | владение состоянием, IO |
| Go sidecar | индексация ассетов, превью, наблюдение за папками | сцену, undo, рендер |

## 5. Поток данных: от клика до кадра

```text
[UI]   pointer/hotkey → CommandId + payload (intent)
       → ipc.invoke("megu3d.cmd.dispatch", intent)
[Core] validate → build Command → Transaction.begin
       → apply(&mut Scene) → ScenePatch (+ обратный patch)
       → History.push(transaction)
       → emit "megu3d.event.scenePatch" (батч, ≤1 на кадр)
[UI]   применяет patch к view-модели → перерисовка только затронутых панелей
[Render] dirty-ноды → render graph обновляет GPU-ресурсы → кадр
```

Правила: патчи батчатся по кадру; один intent = одна транзакция = один undo-шаг. Интерактивный drag работает в preview-режиме: `begin → many previews → commit/abort`, коммит один на `pointerup`.

## 6. Контракт IPC

Генерация типов: Rust-структуры помечаются `#[derive(Serialize, Deserialize, TS)]` (`ts-rs`/`specta`), CI-шаг генерирует `packages/types/src/generated/*.ts`. Расхождение = падение сборки.

| Вид | Имя | Назначение |
|---|---|---|
| request/response | `megu3d.cmd.dispatch` | выполнить команду, вернуть результат или ошибку |
| request/response | `megu3d.query.*` | производные данные: дерево outliner, свойства выбранного, список материалов |
| event | `megu3d.event.scenePatch` | дельта состояния сцены |
| event | `megu3d.event.selection` | изменение выделения |
| event | `megu3d.event.jobProgress` | прогресс долгих задач (импорт, bake, render) |
| event | `megu3d.event.notification` | уведомления и ошибки для пользователя |

Единая схема ошибки: `{ code, message, details, recoverable }`. UI никогда не показывает Rust-панику как текст; коды мапятся в локализованные сообщения.

## 7. Модель данных сцены

```rust
// crates/megu3d-core (иллюстративно)
pub struct Scene {
    pub schema_version: SchemaVersion,
    pub nodes: SlotMap<NodeId, Node>,
    pub roots: Vec<NodeId>,
    pub selection: Selection,
    pub meshes: Registry<MeshId, MeshData>,
    pub materials: Registry<MaterialId, Material>,
    pub images: Registry<ImageId, ImageRef>,
    pub world: WorldSettings, // units, Z-up, environment
}

pub struct Node {
    pub uuid: Uuid,
    pub name: String,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub transform: Transform, // локальный TRS
    pub visible: bool,
    pub locked: bool,
    pub data: NodeData,
}

pub enum NodeData {
    Empty,
    Mesh { mesh: MeshId, material: Option<MaterialId>, modifiers: Vec<Modifier> },
    Light(LightData),
    Camera(CameraData),
}
```

Меши: для рендера — индексированные треугольники (`positions`, `normals`, `uv0`, `indices`); для Edit Mode строится half-edge `EditMesh` и коммитится назад. Это разделение защищает производительность viewport от топологических структур.

Мировые трансформы: кэш `Vec<Mat4>` пересчитывается по dirty-флагам обходом от корней. Никогда не считается в UI.

## 8. Undo/redo

```rust
pub trait Command: Send {
    fn name(&self) -> &'static str;
    fn apply(&mut self, scene: &mut Scene) -> Result<ScenePatch, CmdError>;
    fn invert(&self) -> Box<dyn Command>;
    fn merge_with(&mut self, other: &dyn Command) -> bool { false } // drag-серии
}

pub struct Transaction { pub label: String, pub commands: Vec<Box<dyn Command>> }
pub struct History { undo: VecDeque<Transaction>, redo: Vec<Transaction>, limit: usize /* 64 */ }
```

Гарантии, закреплённые тестами (`D-72`):
- `apply → undo` возвращает состояние, побитово равное исходному снапшоту.
- `redo` после `undo` даёт то же состояние, что первый `apply`.
- Прерванная (`abort`) транзакция не оставляет следов ни в сцене, ни в истории.
- Ошибка внутри команды откатывает всю транзакцию (all-or-nothing).

## 9. Формат проекта и миграции

```text
project.megu3d          ZIP-контейнер (stored + zstd для крупных блобов)
├─ manifest.json        { schemaVersion, app, created, modified, units, thumbnail }
├─ scene.bin            бинарная сериализация сцены (быстрый путь)
├─ scene.json           опционально: читаемый дамп для диффов и отладки
├─ assets/<uuid>.*      упакованные ассеты
├─ assets/index.json    uuid → тип, исходный путь, хеш, packed|linked
└─ thumbnail.png
```

- Запись: сборка во временный файл → `fsync` → rename; предыдущая версия уходит в `.megu3d.bak`.
- Миграции: `migrate(from) -> Vec<Step>`, каждая ступень покрыта тестом на фикстуре в `tests/fixtures/`.
- Файл новее приложения: отказ с кодом `IO_SCHEMA_TOO_NEW`, без частичной загрузки.
- Autosave: отдельная папка с id сессии; recovery-диалог при старте.

## 10. Рендеринг

Абстракция: `megu3d-render` поверх `wgpu` (DX12 по умолчанию на Windows, Vulkan как альтернатива).

Порядок проходов: `depth-prepass` → `shadow` (CSM для sun) → `opaque-pbr` (IBL от HDRI) → `transparent` → `outline` (jump-flood по маске выделения) → `grid` → `gizmo`/`overlay` → `tonemap` (экспозиция, ACES-approx, гамма).

Shading modes `wireframe | solid | material | rendered` — это конфигурации одного графа, а не разные рендереры.

Picking: GPU id-buffer (u32 node id) плюс чтение региона для box-select; ray-cast по BVH из C++ kernel как fallback и для snapping.

Path tracer: отдельный модуль на той же модели материалов. Сначала CPU (rayon, тайлы, прогрессивно, отменяемо), затем GPU compute. Passes: beauty, albedo, normal, depth, id — для композитинга и денойза.

On-demand redraw: кадр рисуется только при dirty-состоянии, вводе или анимации (`P-65`).

## 11. Потоки и задачи

| Поток | Роль |
|---|---|
| UI/main | event loop Tauri, webview, ввод |
| core | владелец `Scene`, единственный мутатор, обрабатывает intents из очереди |
| render | подготовка и submit кадров, читает immutable-снапшот render-данных |
| jobs (rayon) | импорт/экспорт, bake, симуляции, path tracing — с прогрессом и отменой |

Долгая задача не держит блокировку сцены: работает на копии и применяет результат одной командой.

## 12. Границы FFI и sidecar

C++ (`cpp/megu3d_kernels`): наружу только `extern "C"`, POD-структуры, указатель+длина вместо контейнеров. Исключения не пересекают границу — внутри `try/catch`, наружу код ошибки. Владение памятью явное (`*_alloc`/`*_free`). Сборка через `build.rs` (MSVC, C++20). У каждого kernel есть Rust-референс или эталон для сверки.

Go (`go/assetd`): дочерний процесс, JSON-RPC 2.0 по stdio, без состояния сцены. Падение sidecar не роняет приложение — функции ассет-браузера деградируют мягко.

## 13. Плагины

Манифест `plugin.json` (id, версия, требуемая версия API, разрешения). Точки расширения: команды, панели, импорт/экспорт, ноды, кисти, шаблоны сцен. Плагин работает через публичный API команд и событий, без доступа к внутренним структурам. Semver API; несовместимый плагин отключается с внятным сообщением, а не крашит приложение. Sandbox для скриптов — P3.

## 14. Тестовая стратегия

| Уровень | Что покрываем | Инструмент |
|---|---|---|
| unit (Rust) | математика, команды, инварианты сцены | `cargo test` |
| property-based | инвариант undo/redo, миграции, топология меша | `proptest` |
| IO round-trip | `.megu3d`, OBJ, glTF: save→load→save стабилен | фикстуры в `tests/` |
| golden image | viewport-рендер эталонных сцен, допуск по SSIM | wgpu offscreen |
| unit (TS) | стор, view-модели, палитра команд, keymap | `vitest` |
| e2e | запуск приложения, сценарии P0 | Playwright + Tauri driver |
| bench | бюджеты `P-60…P-66` | `criterion` + сценарные сцены |

## 15. Наблюдаемость, ошибки, безопасность

- Логи: `tracing`, уровни, ротация в `%LOCALAPPDATA%/Megu3D/logs`.
- Паника: перехват, emergency-снапшот сцены, предложение восстановления при следующем запуске.
- «Copy diagnostics»: версия, GPU, драйвер, последние логи — без пользовательских данных.
- Секретов в репозитории нет; минимальный Tauri allowlist по fs-скоупам и диалогам.
- Импорт файлов — недоверенный ввод: лимиты размеров, защита от zip-slip, проверка индексов.
- Обновления только по HTTPS с проверкой подписи; плагины запрашивают разрешения явно.

## 16. Отложенные решения (ADR)

| ID | Вопрос | Когда решаем |
|---|---|---|
| ADR-1 | wgpu vs прямой DX12 для GPU path tracer | перед M8 |
| ADR-2 | формат бинарной сериализации сцены (`bincode`/`postcard`/свой) | перед M4 |
| ADR-3 | виртуальная геометрия/LOD для сцен > 5 млн tri | после v1 |
| ADR-4 | язык scripting API | перед P3 |
| ADR-5 | нужен ли Go-sidecar или индексатор остаётся в Rust | перед M6 |

Решения фиксируются как `docs/adr/NNNN-*.md` по мере принятия.
