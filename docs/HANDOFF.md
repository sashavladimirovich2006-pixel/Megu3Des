# HANDOFF — контекст проекта для следующего агента

> **Новый агент, прочитай этот файл целиком, затем переходи к пункту 11 «Точный следующий шаг» и продолжай с задачи M4.4c (подключение импорта/экспорта геометрии к приложению).**
> Всё, что нужно знать о проекте, есть здесь. Дополняющие документы: `docs/01-prd.md`, `docs/02-architecture.md`, `docs/03-roadmap.md`, `docs/04-ui-architecture.md`, `docs/assumptions.md`, `AGENTS.md`.

Дата передачи: 2026-08-16. Состояние на коммите `20d689f`, ветка `feature/m4-mvp-io`, рабочее дерево чистое, 127 отслеживаемых файлов.

---

## 1. Название проекта и цель

**Megu3D** — профессиональное десктопное 3D-приложение для Windows 10/11 x64: моделирование, UV, материалы, освещение, анимация и рендер в одном окне. Цель — не «клон Blender», а инструмент с честной архитектурой и предсказуемым UI: аккуратный тёмный интерфейс, 15 рабочих пространств, единая история undo/redo, надёжное сохранение и обмен геометрией.

План: сначала P0 (MVP: окно, UI-каркас, вьюпорт, примитивы, трансформации, undo/redo, сохранение, импорт/экспорт), затем реалтайм-PBR-вьюпорт на wgpu, затем моделирование, UV, анимация и путевой трассировщик.

Язык интерфейса: RU/EN (RU по умолчанию при локали ОС `ru-*`). Единицы: метры, ось вверх — Z. Лицензия: MIT.

---

## 2. Текущий статус проекта

| Что | Значение |
| --- | --- |
| Локальный репозиторий | `/data/megu3d` (песочница агента) |
| Текущая ветка | `feature/m4-mvp-io` |
| HEAD | `20d689f docs(assumptions): register the glTF export decisions` |
| Отслеживаемых файлов | 127 |
| Незакоммиченных изменений | нет (`git status --porcelain` пуст) |
| Собственный контроль качества | `python3 tools/gate.py` → `ipc=13 bindings=30 keys=133/133 usedKeys=117`, `PROBLEMS: none` |
| Пройденные этапы | M0 (доки) · M1 (каркас) · M2 (UI-оболочка) · M3 (ядро сцены, вьюпорт, гизмо) · M4.1–M4.3 (контейнер `.megu3d`, автосейв, восстановление, недавние файлы, защита при выходе) · M4.4a (OBJ) · M4.4b (чтение glTF/GLB) · M4.4b-bis (запись glTF/GLB) |
| Следующий этап | **M4.4c** — подключить обмен геометрией к приложению (команды, IPC, палитра, i18n) |
| Ключевое ограничение | **код ни разу не компилировался**: в песочнице нет сети, нет `cargo`, `pnpm`, `go`. Проверялись только структура, балансы скобок и собственный гейт |

---

## 3. Принятые решения

### 3.1 Стек

- **Оболочка**: Tauri v2 (Rust) + WebView2. Окно `main` 1600×1000, минимум 1024×640, тёмный фон, `security.csp = null`, бандлы msi + nsis, идентификатор `app.megu3d.desktop`.
- **UI**: TypeScript 5.6 + React 18.3, Vite 5.4 (target chrome110, порт 5173 strictPort), никаких UI-фреймворков — свои панели и токены в `styles.css`.
- **Ядро**: Rust (edition 2021, rust-version 1.77, resolver 2), крейты `megu3d-core`, `megu3d-mesh`, `megu3d-cmd`, `megu3d-io`, `megu3d-interop`.
- **Планируется**: C++ (MSVC, C++20) через безопасный FFI для тяжёлой геометрии; Go как сайдкар по JSON-RPC/stdio; wgpu (Vulkan/DX12) для рендера.
- **Пакетный менеджер**: pnpm 9.12.3, Node ≥ 20.11. Rust/Go — stable.
- **Зависимости Rust**: serde 1 (derive), serde_json 1, thiserror 1, slotmap 1, uuid 1 (v4 + serde), ts-rs 10, tracing 0.1, tracing-subscriber 0.3 (env-filter). `[profile.release]` lto = thin, codegen-units = 1, strip; `[profile.dev] opt-level = 1`.
- **Инструменты TS**: eslint 9 + typescript-eslint 8, prettier 3.3, vitest 2.1. Строгий TS: `noUncheckedIndexedAccess`, `verbatimModuleSyntax`, запрещены `any`, `!`, пустые `catch {}`; jsx `react-jsx`.

### 3.2 Архитектура

- **Rust — источник истины.** UI не хранит состояние сцены: он отправляет намерения (`CommandRequestDto`) и получает снимки/патчи. Мутирует сцену только ядро.
- **Типизированный IPC.** Каждое обращение — команда Tauri с именем `megu3d_*`; события приходят обратно как `megu3d.event.*`. Список имён — единственный контракт между TS и Rust, его целостность проверяет гейт.
- **Undo/redo — паттерн команд**, глубина `UNDO_LIMIT = 64`. История живёт в `megu3d-cmd`, UI только вызывает `undo`/`redo` и рисует состояние `HistoryStateDto`.
- **Сцена** — `SlotMap<NodeId, Node>` + `SlotMap<MeshId, MeshEntry>` + корни + выделение + активный узел + индекс по UUID (`#[serde(skip)]`, перестраивается при загрузке). Идентификаторы ассетов — UUID.
- **Версионирование данных**: `SCHEMA_VERSION = "0.2.0"`, миграции по шагам в `megu3d-io/src/migrate.rs`, отказ читать схему из будущего (`IO_SCHEMA_TOO_NEW`).
- **Потоки** (план): UI/main, core (единственный мутатор), render (работает по снимку), jobs (rayon).
- **Граф рендера** (план для wgpu): `depth-prepass → shadow (CSM) → opaque-pbr → transparent → outline (jump-flood) → grid → gizmo/overlay → tonemap`.
- **Ошибки** — enum на каждый домен, у каждого `code()` (стабильная строка для UI) и `recoverable()`. UI переводит код в текст через ключ `error.<CODE>`.
- **ADR, ещё не закрытые**: ADR-1 wgpu против нативного DX12 (до M8), ADR-2 бинарная сериализация сцены, ADR-3 виртуальная геометрия/LOD (после v1), ADR-4 язык скриптинга (до P3), ADR-5 Go-сайдкар (до M6).

### 3.3 Структура папок

```
apps/desktop/            Tauri-приложение: index.html, src/main.tsx, vite.config.ts
apps/desktop/src-tauri/  Rust-оболочка: main.rs, ipc.rs, tauri.conf.json, capabilities/
crates/megu3d-core/      сцена, узлы, математика, DTO для UI
crates/megu3d-mesh/      MeshData и примитивы
crates/megu3d-cmd/       сессия, история undo/redo, документ, диспетчер команд
crates/megu3d-io/        контейнер .megu3d: zip, manifest, миграции
crates/megu3d-interop/   OBJ и glTF/GLB: чтение и запись
packages/types/          сгенерированные TS-типы (ts-rs) + ручной индекс
packages/ipc/            обёртки IPC, диалоги, подписки на события
packages/i18n/           каталоги сообщений en/ru и хелпер перевода
packages/ui/             весь интерфейс: оболочка, панели, вьюпорт, палитра, команды
docs/                    PRD, архитектура, роадмап, UI-архитектура, допущения, этот файл
tools/gate.py            собственный статический контроль репозитория
tests/                   зарезервировано под интеграционные тесты (пока пусто)
.github/workflows/ci.yml сборка и проверки в CI
```

### 3.4 UI/UX решения

- **Никакой «каши».** Один кадр: вьюпорт в центре, инструменты слева, Properties справа, Outliner справа сверху, Timeline/Graph/VSE снизу, переключатель рабочих пространств + меню + палитра сверху.
- **15 рабочих пространств** (Layout, Modeling, Sculpting, UV, Texture Paint, Shading, Animation, Rigging, Simulation, Compositing, Geometry Nodes, Rendering, Video, Scripting, Preferences), переключение `Alt+1…9` и через палитру.
- **Панели докируемые**, раскладка сохраняется (`LAYOUT_VERSION = 1`), есть пресеты, минимальный размер узла 0.08.
- **Тёмная тема по умолчанию**, есть светлая; масштаб UI 0.8…2.0; HiDPI; настройки в `localStorage` под ключом `megu3d.preferences.v1`.
- **Клавиатура — первый класс.** 30 сочетаний, палитра команд `Ctrl+Shift+P`, `G/R/S` — перемещение/поворот/масштаб, `F` — вписать выделенное, `Shift+A` — добавить, `Shift+D` — дублировать, `X`/`Delete` — удалить, `Alt+A` — снять выделение, `Ctrl+Z`/`Ctrl+Shift+Z` — undo/redo, `Ctrl+N/O/S/Shift+S` — файл, `Ctrl+Alt+T/L/R` — тема/язык/сброс раскладки.
- **i18n**: 133 ключа в обоих каталогах, порядок ключей одинаковый (проверяет гейт), RU по умолчанию при `ru-*`.
- **Честность интерфейса**: если функция не готова, панель показывает заглушку с текстом, а не пустоту; ошибки ядра всплывают уведомлением с кодом.

### 3.5 Приоритеты MVP (P0)

Окно Tauri · React-оболочка · тёмная тема · докинг · палитра команд · Outliner · Properties · 3D-вьюпорт · навигация камерой · примитивы · выделение · гизмо Move/Rotate/Scale · Object Mode · базовый Edit Mode · undo/redo · сохранение/загрузка · автосейв · OBJ/glTF · базовые материалы и свет.

**Сделано из P0**: всё, кроме базового Edit Mode, материалов/света в Properties и подключения импорта/экспорта к UI. Вьюпорт пока canvas-2D (каркасы), не wgpu.

### 3.6 Соглашения об именах

- **Rust**: модули и функции `snake_case`, типы `PascalCase`, отступ 4 пробела. Ошибки — `enum` с `#[derive(thiserror::Error)]`, у каждой `code()` и `recoverable()`.
- **Коды ошибок**: `SCENE_*`, `HISTORY_*`, `CORE_LOCK_POISONED`, `DOC_NO_PATH`, `IO_*`, `IMPORT_*` — SCREAMING_SNAKE, домен впереди. Каждый код обязан иметь ключ `error.<CODE>` в обоих каталогах i18n.
- **TypeScript**: файлы-компоненты `PascalCase.tsx`, остальное `camelCase.ts`, отступ — табуляция, тесты рядом с кодом как `*.test.ts`.
- **IPC**: обёртка в TS — `camelCase` (`querySceneStats`), команда Tauri — `megu3d_<область>_<действие>` (`megu3d_query_scene_stats`), события — `megu3d.event.<имя>`.
- **Ключи i18n**: точечные пространства — `command.<категория>.<действие>`, `panel.title.*`, `workspace.*`, `error.<CODE>`, `notify.<область>.<событие>`, `close.*`, `recovery.*`.
- **Команды палитры**: `<категория>.<действие>`, категории `file | workspace | panel | view | edit | scene`.
- **Идентификаторы решений** в `docs/assumptions.md`: `D-` решение, `A-` допущение, `P-` цель по производительности, `Q-` открытый вопрос. **Следующий свободный: `D-123`.** Правило: не ссылаться в коде на идентификатор, который ещё не зарегистрирован в документе тем же ходом.
- **Коммиты**: Conventional Commits, тема в нижнем регистре и в императиве, область — подсистема: `feat(interop):`, `fix(tools):`, `docs(assumptions):`, `feat(shell):`, `feat(io):`, `feat(cmd):`.
- **Ветки**: `feature/m<номер>-<слаг>`, например `feature/m4-mvp-io`.

### 3.7 Форматы файлов

- **Проект `.megu3d`** — ZIP (собственная реализация в `megu3d-io/src/zip.rs`, без внешних зависимостей) с двумя записями: `manifest.json` (версия контейнера, единицы, приложение, время) и `scene.json` (сцена целиком, pretty-JSON). Запись атомарная: временный файл → `rename`, прежний файл уходит в `*.megu3d.bak`.
- **Схема**: `SCHEMA_VERSION = "0.2.0"`; миграции — упорядоченные шаги; схема новее текущей — отказ (`IO_SCHEMA_TOO_NEW`).
- **Автосейв**: срезы `{stem}-{revision:06}.megu3d` в `local_data_dir()/Megu3D/autosave`, хранится 5 последних, интервал 5 минут.
- **Обмен геометрией**: OBJ (чтение и запись), glTF 2.0 и GLB (чтение и запись). Файлы считаются Y-up, сцена Z-up; масштаб задаётся как «метров в единице файла», экспорт делит. UV: в файле верх-лево, в сцене низ-лево, координата V переворачивается.
- **Локальные настройки браузера**: `megu3d.preferences.v1` (тема, язык, масштаб), раскладка панелей, `megu3d.recent.v1` (до 8 недавних проектов).

### 3.8 Git workflow

- Работа ведётся ветками `feature/m<N>-<slug>`; `main` держит документы и историю M0.
- Каждый этап — отдельный атомарный коммит; код и регистрация решений в `docs/assumptions.md` идут двумя коммитами подряд (`feat(...)`, затем `docs(assumptions): ...`).
- **Перед каждым коммитом**: `git add -A && python3 tools/gate.py` — гейт перечисляет файлы через `git ls-files`, поэтому новые файлы должны быть проиндексированы, иначе проверки их не увидят.
- Локальная подпись коммитов агента: `git -c user.name='Megu3D Bot' -c user.email='dev@megu3d.local' commit …`.
- Никаких секретов и токенов в коде и в выводе.

---

## 4. Созданные файлы и за что они отвечают

Всего 127 отслеживаемых файлов. Полный список — `git ls-files`. Ниже — назначение каждого важного файла.

### 4.1 Корень

| Файл | Назначение |
| --- | --- |
| `Cargo.toml` | workspace Rust: `members = ["crates/*", "apps/desktop/src-tauri"]`, общие версии зависимостей, профили сборки; строка 17 объявляет `megu3d-interop` |
| `package.json`, `pnpm-workspace.yaml`, `.npmrc` | монорепо pnpm, скрипты `dev`/`build`/`test`/`lint`/`typecheck` |
| `tsconfig.base.json` | строгие настройки TS для всех пакетов |
| `eslint.config.js`, `.prettierrc`, `.editorconfig` | стиль кода |
| `rust-toolchain.toml` | закреплённый stable-тулчейн + `rustfmt`, `clippy` |
| `deny.toml` | проверка лицензий и уязвимостей (`cargo deny`) |
| `README.md` | краткое описание, статус, быстрый старт |
| `LICENSE` | MIT |
| `.github/workflows/ci.yml` | CI: lint, typecheck, vitest, `cargo fmt/clippy/test` |
| `tools/gate.py` | собственный гейт (см. 5.4) |

### 4.2 `apps/desktop`

| Файл | Назначение |
| --- | --- |
| `index.html`, `src/main.tsx` | точка входа React, монтирует `AppShell` |
| `vite.config.ts`, `tsconfig.json`, `package.json` | сборка фронта (chrome110, порт 5173) |
| `src-tauri/Cargo.toml`, `build.rs` | Rust-часть оболочки |
| `src-tauri/src/main.rs` | создание сессии, регистрация 13 команд IPC, `tracing` |
| `src-tauri/src/ipc.rs` | все обработчики IPC, события `megu3d.event.*`, папка автосейва |
| `src-tauri/tauri.conf.json` | окно, бандлы, идентификатор |
| `src-tauri/capabilities/default.json` | разрешения: `core:default`, `dialog:default` для окна `main` |
| `src-tauri/icons/README.md` | пояснение, что иконок пока нет (сборка только `--no-bundle`) |

### 4.3 Крейты Rust

| Файл | Назначение |
| --- | --- |
| `crates/megu3d-core/src/scene.rs` | `Scene`, `Node`, `NodeData`, `Transform`, `CameraData`, `LightData`, `MeshEntry`, индекс по UUID, `stats`, `snapshot`, `SceneError` |
| `crates/megu3d-core/src/dto.rs` | все DTO для UI + генерация TS через ts-rs |
| `crates/megu3d-core/src/math.rs` | векторы, кватернионы, матрицы, AABB |
| `crates/megu3d-core/src/lib.rs` | `SCHEMA_VERSION`, реэкспорты |
| `crates/megu3d-mesh/src/lib.rs` | `MeshData` (positions/normals/uvs/indices), `validate`, `bounds`, примитивы plane/cube/sphere/cylinder/cone/torus, `MeshError` |
| `crates/megu3d-cmd/src/lib.rs` | `Session`, `History`, `UNDO_LIMIT = 64`, `CmdError` и коды |
| `crates/megu3d-cmd/src/dispatch.rs` | разбор `CommandRequestDto` → мутации сцены, превью-трансформации, выделение |
| `crates/megu3d-cmd/src/document.rs` | путь проекта, ревизии, dirty, сохранение/загрузка, автосейв и его ротация |
| `crates/megu3d-io/src/lib.rs` | `Project`, `to_bytes`/`from_bytes`, `save`/`load`, `IoError` и коды, атомарная запись |
| `crates/megu3d-io/src/zip.rs` | ручной ZIP: `crc32`, `write`, `read`, `entry` |
| `crates/megu3d-io/src/manifest.rs` | `manifest.json`, время RFC 3339 |
| `crates/megu3d-io/src/migrate.rs` | цепочка миграций схемы |
| `crates/megu3d-interop/src/lib.rs` | `ImportedMesh`, `UpAxis` (`to_scene`/`from_scene`), `InteropError` (7 вариантов) и коды `IMPORT_*` |
| `crates/megu3d-interop/src/geometry.rs` | общие помощники: `scaled`, `unit`, `subtract`, `cross`, `smooth_normals`, `number` |
| `crates/megu3d-interop/src/obj.rs` | OBJ: `parse`, `write`, `ObjOptions` |
| `crates/megu3d-interop/src/gltf.rs` | чтение glTF/GLB: `parse`, `parse_glb`, `BufferMap`, `GltfOptions`, base64, матрицы, аксессоры; реэкспорт записи |
| `crates/megu3d-interop/src/gltf_write.rs` | запись glTF/GLB: `write_gltf`, `write_glb`, `GENERATOR` |

### 4.4 Пакеты TypeScript

| Файл | Назначение |
| --- | --- |
| `packages/types/src/generated/*.ts` (17 файлов) | типы, отражающие DTO ядра (сейчас ручные копии, ts-rs не запускался) |
| `packages/types/src/index.ts` | реэкспорт + вспомогательные типы |
| `packages/ipc/src/index.ts` | `IPC`-карта 13 имён, `TAURI_COMMAND`, `call`/`subscribe`, обёртки, диалоги выбора файла, транспорты сцены и документа |
| `packages/i18n/src/messages/{en,ru}.ts` | 133 ключа в одинаковом порядке |
| `packages/i18n/src/index.ts` | `translate`, подстановка переменных, выбор каталога |
| `packages/ui/src/AppShell.tsx` | главная оболочка: состояние, контексты команд и панелей, уведомления, заголовок окна |
| `packages/ui/src/commands/registry.ts` | все команды палитры и их `run` |
| `packages/ui/src/commands/keybinding.ts`, `useKeybindings.ts` | разбор сочетаний и глобальный перехват клавиатуры |
| `packages/ui/src/layout/*` | докинг: `types.ts`, `reducer.ts`, `DockView.tsx`, `presets.ts`, `persistence.ts` |
| `packages/ui/src/palette/*` | палитра команд и неточный поиск |
| `packages/ui/src/panels/*` | `Outliner.tsx`, `Properties.tsx`, `registry.tsx` (13 панелей, часть — заглушки) |
| `packages/ui/src/scene/useScene.ts` | `SceneApi`: отправка команд, превью, выделение, undo/redo |
| `packages/ui/src/scene/useDocument.ts` | `DocumentApi`: сохранение, открытие, автосейв каждые 5 минут, восстановление, недавние файлы, коды ошибок |
| `packages/ui/src/scene/CloseGuard.tsx`, `RecoveryBanner.tsx`, `recent.ts`, `selectors.ts` | защита при выходе, баннер восстановления, список недавних, селекторы сцены |
| `packages/ui/src/viewport/*` | `Viewport.tsx` (canvas-2D), `camera.ts`, `math.ts`, `gizmo.ts`, `pick.ts` |
| `packages/ui/src/preferences.ts`, `workspaces.ts`, `styles.css` | настройки, 15 рабочих пространств, все токены дизайна |
| `packages/ui/src/**/*.test.ts` | 14 тестовых файлов на чистые хелперы (без DOM) |

### 4.5 Документация

| Файл | Назначение |
| --- | --- |
| `docs/01-prd.md` | цели, аудитория, объём P0–P3, критерии готовности |
| `docs/02-architecture.md` | крейты, потоки, IPC, граф рендера, ADR |
| `docs/03-roadmap.md` | M0–M10; строка 53 — статус M4 (ещё `☐`, надо закрыть после M4.4d) |
| `docs/04-ui-architecture.md` | рабочие пространства, панели, горячие клавиши, токены |
| `docs/assumptions.md` | **главный журнал решений**: разделы 1–13, идентификаторы `D-`/`A-`/`P-`/`Q-` |
| `docs/HANDOFF.md` | этот файл |

---

## 5. Команды

### 5.1 Установка

```bash
corepack enable && corepack prepare pnpm@9.12.3 --activate
pnpm install                      # локфайла пока нет, первый install его создаст
rustup toolchain install stable   # версия задана rust-toolchain.toml
```
Для Windows дополнительно: MSVC Build Tools (C++), WebView2 Runtime.

### 5.2 Запуск и сборка

```bash
pnpm dev                 # vite на 5173
pnpm tauri dev           # окно приложения (требует MSVC + WebView2)
pnpm build               # tsc + vite build
pnpm tauri build --no-bundle   # без иконок бандлы не соберутся
```

### 5.3 Тесты и проверки

```bash
pnpm test                              # vitest
pnpm lint && pnpm typecheck            # eslint + tsc --noEmit
cargo test --workspace                 # все крейты
cargo test -p megu3d-interop           # 38 тестов обмена геометрией
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

### 5.4 Собственный гейт (работает без сети)

```bash
cd /data/megu3d && git add -A && python3 tools/gate.py
```
Проверяет: завершающий перевод строки во всех текстовых файлах; запрет `any` и `as any` в TS; баланс скобок в `.ts/.tsx/.css/.json`; паритет ключей i18n и их использование; полнота `CommandContext`/`PanelContext` относительно `AppShell`; дубли горячих клавиш; соответствие имён IPC между TS, `ipc.rs` и `main.rs`. Выводит `ipc=… bindings=… keys=… usedKeys=…` и затем `PROBLEMS: none` или список проблем. **Гейт — обязательный шаг перед любым коммитом.**

### 5.5 Архив для выгрузки

```bash
cd /data && rm -f megu3d.zip && zip -qr megu3d.zip megu3d && unzip -l megu3d.zip | tail -2
```

---

## 6. Что уже работает (по коду; сборка не проверялась)

- **Оболочка**: окно Tauri, React-каркас, тёмная/светлая тема, масштаб UI, RU/EN, 15 рабочих пространств, докинг с пресетами и сохранением раскладки, палитра команд с неточным поиском, 30 горячих сочетаний, уведомления.
- **Сцена**: добавление примитивов, выделение (в том числе кликом в вьюпорте), переименование, удаление, дублирование, видимость, репарентинг, трансформации с превью, undo/redo на 64 шага, статистика сцены.
- **Вьюпорт**: орбита/панорама/зум, сетка, каркасы AABB, маркеры начала координат, гизмо перемещения/поворота/масштаба, подбор объекта курсором, «вписать выделенное».
- **Документ**: `.megu3d` сохранение/загрузка, Save As, New, трекинг dirty и ревизий, автосейв каждые 5 минут с ротацией 5 срезов, предложение восстановления после сбоя, список недавних (8) с командами палитры, вопрос при закрытии окна с несохранёнными правками, миграции схемы.
- **Обмен геометрией** (пока только как библиотека): OBJ в обе стороны; glTF 2.0 и GLB в обе стороны; конвертация осей и масштаба, триангуляция, склейка вершин, пересчёт нормалей, запечённые матрицы узлов, base64, детерминированный экспорт.
- **Гейт**: зелёный, ловит рассинхрон ключей, IPC и контекстов.

---

## 7. Что не работает или сломано

1. **Ни одна сборка не запускалась.** Нет проверки `cargo`, `tsc`, `eslint`, `vitest`, `tauri`. Ожидайте первого прогона с правками по мелочам (импорты, лайфтаймы, clippy).
2. **~87 тестов ни разу не выполнялись** (25 io + 10 cmd/document + 14 UI-файлов хелперов + 38 interop).
3. **Нет `pnpm-lock.yaml`** — версии не закреплены до первого `pnpm install`.
4. **Нет иконок** `apps/desktop/src-tauri/icons/*` → `tauri build` без `--no-bundle` упадёт.
5. **Вьюпорт — заглушка на canvas-2D**: сетка, каркасы AABB, гизмо. Шейдинга и мешей нет, wgpu не подключён, цели `P-61`/`P-62` не измерены.
6. **Обмен геометрией не подключён к приложению**: нет команд IPC, нет команд палитры, нет вставки меша в сцену (именно это — задача M4.4c).
7. **Edit Mode отсутствует**; материалы и свет есть в типах ядра, но не редактируются в Properties.
8. **Зеркало GitHub отстаёт на 39 файлов** и в текущем виде **не соберётся** (там нет всего `crates/megu3d-interop/`, `megu3d-cmd` и `megu3d-core` эпохи M3). См. пункт 13.
9. **`.github/workflows/ci.yml` на GitHub отсутствует**: токен агента получает 403 на любой файл в `.github/workflows/`.
10. **`tests/` пуста**: интеграционных тестов нет, e2e нет.
11. **`packages/types/src/generated/*` — ручные копии**, ts-rs не запускался; при первом `cargo test` их надо сверить с генератором.
12. **Диалоги файлов** опираются на `@tauri-apps/plugin-dialog` и разрешение `dialog:default` — не проверено в живом окне; `getCurrentWindow().destroy()` может потребовать `core:window:allow-destroy`.

---

## 8. Выполненные задачи

| Этап | Содержание | Коммиты |
| --- | --- | --- |
| M0 | PRD, архитектура, роадмап, UI-архитектура, журнал допущений, MIT, CI | ветка `main` → `ca653d4` |
| M1 | каркас монорепо, Tauri-окно, первый IPC, i18n, токены темы | `feature/m1-scaffold` → `cfa1ba5` |
| M2 | оболочка: рабочие пространства, докинг, палитра, горячие клавиши, панели | `feature/m2-shell` → `ba5694b` |
| M3 | ядро сцены, примитивы, undo/redo, вьюпорт, гизмо, Outliner, Properties | `feature/m3-scene-core` → `65608ff` |
| M4.1 | контейнер `.megu3d`: zip, manifest, миграции, атомарная запись | `6f373e8`, `39afaa6` |
| M4.2a | `Session` связан с файлом, dirty, ревизии, автосейв | `eac28b8` |
| M4.2b | IPC сохранения/открытия/создания/автосейва | `5bead2f` |
| M4.2c | подключение документа и сцены к оболочке, заголовок окна | `26c39b8` |
| M4.3a | восстановление из срезов автосейва | `629c592` |
| M4.3b | баннер восстановления после сбоя | `42b7015` |
| — | исправление подсчёта горячих клавиш в гейте | `966c0c8` |
| M4.3c | недавние файлы + вопрос при закрытии окна | `1745f4a`, `bc8c464` |
| M4.4b | чтение glTF 2.0 и GLB | `b450a0e`, `7be7d68` |
| M4.4a | чтение и запись OBJ | `7de941c`, `7a085e9` |
| M4.4b-bis | запись glTF/GLB, общие геометрические хелперы | `6db5859`, `20d689f` |

В `docs/assumptions.md` зарегистрированы решения до `A-122` включительно (раздел 13 — обмен геометрией).

---

## 9. Оставшиеся задачи

**Ближайшие (закрывают M4):**
1. **M4.4c** — подключить импорт/экспорт к приложению (см. пункт 11).
2. **M4.4d** — редактирование материалов и света в панели Properties (`LightData`, `LightKind` уже есть в ядре).
3. **Закрытие M4** — в `docs/03-roadmap.md` строка 53 (`## M4 — MVP: persist, autosave, interop, look → P0 закрыт ☐ (3 нед.)`) → `☑`; добавить `megu3d-interop` в `docs/02-architecture.md`; описать защиту при выходе и потоки импорта/экспорта в `docs/04-ui-architecture.md`.
4. **Первая реальная сборка** на машине с сетью: `pnpm install`, `pnpm lint`, `pnpm typecheck`, `pnpm test`, `cargo fmt/clippy/test`, `pnpm tauri dev`. Починить всё, что всплывёт, и закоммитить `pnpm-lock.yaml`.
5. **Синхронизация зеркала GitHub** (пункт 13) и ручное добавление `.github/workflows/ci.yml`.

**Средний горизонт:**
6. **wgpu-вьюпорт** вместо canvas-2D: крейт `megu3d-render`, граф рендера, PBR, тени, обводка выделения, GPU-пикинг.
7. **M5** — моделирование и Edit Mode: полурёберная структура, выделение вершин/рёбер/граней, extrude/bevel/inset/loop cut, честная триангуляция вместо веерной (`D-111`).
8. **M6** — UV, текстуры, нодовый шейдинг; **M7** — анимация и риггинг; **M8** — P2 (симуляции, путевой трассировщик); **M9** — P3 (скриптинг, геометрические ноды); **M10** — стабилизация и релиз.
9. **Техдолг**: сборка мусора мешей (`A-85`), инкрементальные патчи вместо полного снимка (`D-84`), виртуализация Outliner, бинарная сериализация сцены (ADR-2), генерация TS-типов через ts-rs в CI.

---

## 10. Какую задачу продолжить первой

**M4.4c — «последняя миля» обмена геометрией.** Крейт `megu3d-interop` умеет читать и писать OBJ/glTF/GLB, но его никто не вызывает: нет методов сессии, нет IPC, нет команд UI. Пока этого нет, функциональность для пользователя не существует, и пункт P0 «импорт/экспорт OBJ/glTF» не закрыт.

---

## 11. Точный следующий шаг для нового агента

Начни с проверки состояния:
```bash
cd /data/megu3d && git log --oneline -3 && git status --porcelain && python3 tools/gate.py
```
Ожидается HEAD `20d689f` (или коммит с этим файлом поверх), пустой status, `PROBLEMS: none`. Если репозиторий пуст или откатился — см. пункт 14.

Затем выполни M4.4c в таком порядке (один коммит кода + один коммит документации):

1. **`crates/megu3d-cmd/Cargo.toml`** — добавить `megu3d-interop.workspace = true`.
2. **`crates/megu3d-cmd/src/lib.rs`** — два метода `Session`:
   - `pub fn import_mesh(&mut self, path: &Path) -> Result<SceneSnapshotDto, CmdError>` — по расширению выбрать `obj::parse` / `gltf::parse` / `gltf::parse_glb`; чтение файла и сбор `BufferMap` для внешних `.bin` делать здесь (interop с диском не работает, `D-109`); каждый `ImportedMesh` → `scene.insert_mesh` + `scene.add_node`; всё одной записью истории (один undo откатывает всю импортированную группу); ошибки interop отображать в `CmdError::Io { code, message, recoverable }`, сохраняя `IMPORT_*` коды (`CmdError` выводит `PartialEq`, поэтому внутрь нельзя класть `std::io::Error`).
   - `pub fn export_selection(&self, path: &Path, selected_only: bool) -> Result<(), CmdError>` — собрать `Vec<ImportedMesh>` из узлов (выделенные или все), по расширению вызвать `obj::write` / `write_gltf` / `write_glb`, записать через тот же атомарный путь, что и сохранение проекта. Трансформации узлов запечь в вершины (экспортёр пишет один узел на меш без иерархии, `A-122`).
   - Написать тесты: импорт добавляет узлы и пишется одним undo; круговой прогон экспорт → импорт сохраняет число треугольников; неизвестное расширение даёт ошибку без паники.
3. **`apps/desktop/src-tauri/src/ipc.rs`** — два обработчика в стиле существующих: `pub fn megu3d_cmd_import_mesh(path: String, …) -> Ipc<SceneSnapshotDto>` и `pub fn megu3d_cmd_export_mesh(path: String, selected_only: bool, …) -> Ipc<()>`; после импорта разослать `megu3d.event.scenePatch` с `fullReload: true` через `announce`.
4. **`apps/desktop/src-tauri/src/main.rs`** — добавить `ipc::megu3d_cmd_import_mesh,` и `ipc::megu3d_cmd_export_mesh,` в список `invoke_handler`.
5. **`packages/ipc/src/index.ts`** — добавить в `IPC` имена `importMesh`/`exportMesh`, сопоставления в `TAURI_COMMAND`, типизированные обёртки `importMesh(path)` / `exportMesh(path, selectedOnly)`, а также диалоги `pickMeshToImport()` / `pickMeshToExport()` (фильтры `obj`, `gltf`, `glb`) рядом с `pickProjectToOpen`.
6. **`packages/ui/src/scene/useScene.ts`** — добавить в `SceneApi` методы `importMesh` и `exportSelection` (с флагом занятости и передачей кода ошибки в уведомление).
7. **`packages/ui/src/commands/registry.ts`** — команды `file.import` и `file.export` в категории `file`, без горячих клавиш (все удобные `Ctrl+…` заняты; новые сочетания ломают проверку дублей, если не уникальны).
8. **`packages/i18n/src/messages/en.ts` и `ru.ts`** — добавить ключи `command.file.import`, `command.file.export`, `notify.file.imported`, `notify.file.exported`, `error.IMPORT_SYNTAX`, `error.IMPORT_INDEX`, `error.IMPORT_EMPTY`, `error.IMPORT_MESH_INVALID`, `error.IMPORT_STRUCTURE`, `error.IMPORT_CONTAINER`, `error.IMPORT_JSON` — **в оба файла в одном и том же порядке** (гейт сравнивает порядок, а не только набор).
9. **Проверка**: `git add -A && python3 tools/gate.py` — должно стать `ipc=15` (было 13), `keys=144/144`, `PROBLEMS: none`.
10. **Коммиты**: `feat(interop): import and export meshes from the app`, затем регистрация решений `D-123…` в `docs/assumptions.md` коммитом `docs(assumptions): register the M4.4c decisions`.

Решения, которые надо будет зарегистрировать (начиная с `D-123`): где живёт чтение с диска; один undo на импорт; экспорт выделенного против всего; отсутствие горячих клавиш для импорта/экспорта.

---

## 12. Риски, баги, TODO и открытые вопросы

### 12.1 Главные риски

- **Нет компиляции** — главный риск проекта. Первый `cargo test --workspace` обязательно даст правки; `megu3d-interop` (82 899 байт, 38 тестов) не видел компилятора ни разу.
- **Откаты песочницы.** За сессию трижды терялись уже сделанные коммиты (включая reflog). После любого этапа делать архив и отдавать его пользователю.
- **Зеркало GitHub не собирается** (пункт 13): если кто-то клонирует `main`, он получит нерабочее дерево.
- **Цели производительности `P-60`…`P-66` не измерены**: нет рендерера, нет больших сцен.

### 12.2 Известные ограничения кода

- **Импорт glTF**: теряются материалы, скины, анимации, морфы, Draco, sparse-аксессоры; иерархия уплощается; не-треугольные примитивы пропускаются; внешние `.bin` должен подать вызывающий код (`A-118`, `D-116`).
- **Экспорт glTF**: только POSITION/NORMAL/TEXCOORD_0 + индексы `u32`, `mode: 4`, один узел на объект, без материалов/иерархии/камер/анимации/сжатия; буфер целиком в памяти (`A-122`).
- **OBJ**: `g` трактуется как отдельный объект, материалы/текстуры игнорируются, числа округляются до 6 знаков; триангуляция — веером, невыпуклые полигоны могут сломаться (`D-111`, честная триангуляция — в M5).
- **Сцена**: нет сборки мусора мешей (`A-85`); дублирование разделяет геометрию (`D-83`); каждый dispatch отдаёт полный снимок (`D-84`) — на больших сценах это станет узким местом.
- **Файлы**: `SlotMap` сериализуется как последовательность слотов — формат чувствителен к версии крейта; pretty-JSON раздувает файл (угроза `P-64`); собственный ZIP не проверялся в Проводнике Windows; `write_atomically` требует одной файловой системы; фикстура миграции `0.1.0 → 0.2.0` синтетическая.
- **Автосейв**: `*.megu3d.bak` не попадает в ротацию; `revision: u32` может выйти за пределы шести разрядов в имени среза; смена папки автосейва оставляет старые срезы сиротами; баннер восстановления предлагает только свежайший срез; `AutosaveEntryDto.bytes` теряет точность выше 4 ГиБ.
- **Документ**: `open` очищает историю undo; список недавних не проверяет существование файлов; модалка закрытия без Escape и ловушки фокуса.
- **UI**: гизмо поворота/масштаба редактирует локальные эйлеровы углы/масштаб; подбор объектов — на CPU в экранных координатах; Outliner не виртуализован.

### 12.3 Открытые вопросы (`docs/assumptions.md`, раздел 10)

`Q-90` целевое железо и минимальная GPU · `Q-91` глубина совместимости с Blender-хоткеями · `Q-92` формат поставки (MSI против портативной сборки) · `Q-93` нужна ли телеметрия · `Q-94` какая поддержка USD/FBX нужна в v1. Незакрытые ADR — в 3.2.

### 12.4 TODO кода

1. Прогнать ts-rs и сверить `packages/types/src/generated/*`.
2. Заменить полные снимки сцены инкрементальными патчами.
3. Добавить иконки и включить полный `tauri build`.
4. Закрыть доступ палитры к командам, невозможным вне десктопа (сейчас они бросают ошибку в браузере).
5. Виртуализовать Outliner и добавить множественное выделение шифтом.
6. Добавить Escape и ловушку фокуса в модалку закрытия.

---

## 13. GitHub: ветка, коммиты, что надо запушить

**Репозиторий**: `https://github.com/sashavladimirovich2006-pixel/Megu3Des` (владелец `sashavladimirovich2006-pixel`, ветка по умолчанию `main`).

**Локальные ветки** (ремоут не настроен, в песочнице нет сети):
```
main              ca653d4
feature/m1-scaffold   cfa1ba5
feature/m2-shell      ba5694b
feature/m3-scene-core 65608ff
feature/m4-mvp-io     20d689f   <-- HEAD
```

**Последние коммиты текущей ветки** (новые сверху):
```
20d689f docs(assumptions): register the glTF export decisions
6db5859 feat(interop): write glTF documents and glb containers
7a085e9 docs(assumptions): register the OBJ interchange decisions
7de941c feat(interop): read and write Wavefront OBJ
7be7d68 docs(assumptions): register the glTF import decisions
b450a0e feat(interop): read glTF 2.0 documents and glb containers
bc8c464 docs(assumptions): register the M4.3 decisions
1745f4a feat(shell): keep recent projects and ask before the window closes
966c0c8 fix(tools): count keybindings the way the registry declares them
42b7015 feat(shell): offer the autosave back after a crash
629c592 feat(io): recover work from autosave slices
26c39b8 feat(shell): wire the document and the scene into the app shell
5bead2f feat(shell): expose save, open, new and autosave over IPC
eac28b8 feat(cmd): bind the session to a project file with dirty tracking and autosave
39afaa6 docs: record the project io assumptions for M4
6f373e8 feat(io): add the .megu3d container with manifest, migrations and atomic save
```

**Состояние зеркала**: файлы заливались поштучно через GitHub Contents API (агенту доступна только она; Git Trees API и `.github/workflows/*` возвращают 403). Последний залитый коммит зеркала — `c8b6d93`. История там плоская (один коммит на файл), ветки `feature/*` отсутствуют.

**Долг зеркала — 39 файлов**: весь `crates/megu3d-interop/` (6 файлов), новые версии `crates/megu3d-core/src/{dto.rs,scene.rs}`, `crates/megu3d-cmd/src/{lib.rs,dispatch.rs,document.rs}`, `crates/megu3d-io/*`, вся оболочка из `5bead2f`…`1745f4a` (`ipc.rs`, `main.rs`, `AppShell.tsx`, `useDocument.ts`, `CloseGuard.tsx`, `RecoveryBanner.tsx`, `recent.ts`, `registry.ts`, каталоги i18n, `styles.css`, тесты), `docs/assumptions.md`, и этот `docs/HANDOFF.md`, `AGENTS.md`. **В текущем виде `main` на GitHub не соберётся.**

**Команды для пуша с машины с сетью** (единственный надёжный способ выровнять зеркало):
```bash
# распаковать свежий архив проекта и войти в него
unzip megu3d-m4h.zip && cd megu3d
git remote add origin https://github.com/sashavladimirovich2006-pixel/Megu3Des.git
git push -u origin feature/m4-mvp-io
git push origin main feature/m1-scaffold feature/m2-shell feature/m3-scene-core
# если нужно сделать локальное состояние истиной для main:
git checkout main && git merge --ff-only feature/m4-mvp-io || git branch -f main feature/m4-mvp-io
git push origin main --force-with-lease
```
Файл `.github/workflows/ci.yml` придётся добавить вручную или токеном с правами `Contents: RW` **и** `Workflows: RW`.

---

## 14. Незавершённые изменения

**Их нет.** `git status --porcelain` пуст, WIP-коммит не требуется: всё, что сделано, закоммичено в `20d689f` и ранее, плюс коммит с этим документом.

Важно: песочница агента уже три раза откатывалась назад вместе с историей git. Страховка — архивы, выгруженные пользователю (последние: `megu3d-m4g.zip` на `20d689f`, `megu3d-m4h.zip` на коммите с этим документом). Если ты видишь пустой `/data/megu3d` или HEAD старше `20d689f`:

```bash
cd /data && unzip -q megu3d-m4h.zip && cd megu3d
git log --oneline -3 && git status --porcelain && python3 tools/gate.py
```
Если архива нет, попроси его у пользователя — он есть в переписке. Если пропали отдельные файлы, но объекты git целы: `git fsck --lost-found`, затем `git show <sha>:<path> > <path>`.

---

## 15. Инструкция новому агенту

> **Новый агент: прочитай этот файл целиком и `AGENTS.md`, потом продолжай с пункта 11 — задача M4.4c шаг 1.**
>
> Порядок действий: (1) проверь `git log --oneline -3`, `git status --porcelain`, `python3 tools/gate.py`; (2) если состояние не совпадает — восстановись по пункту 14; (3) сделай M4.4c по шагам из пункта 11; (4) перед каждым коммитом — `git add -A && python3 tools/gate.py`, гейт должен быть зелёным; (5) решения записывай в `docs/assumptions.md`, начиная с `D-123`; (6) после этапа сделай архив и отдай его пользователю; (7) всегда говори честно, что код не компилировался, и не выдавай проверки гейта за тесты.
