# Megu3D — Roadmap

Версия 0.1 · 2026-08-13 · draft M0
Связанное: [assumptions](./assumptions.md) · [PRD](./01-prd.md) · [architecture](./02-architecture.md) · [ui-architecture](./04-ui-architecture.md)

## 0. Как читается план

- Единица планирования — вертикальный срез: UI → IPC → ядро → тест. Этапов «только фронтенд» или «только Rust» не бывает.
- Оценки в неделях работы одного разработчика полного цикла (1 нед. ≈ 30 часов фокуса). Это оценки, а не обещания дат.
- Этап закрыт только когда выполнены exit criteria и зелены CI-гейты.
- Каждый этап — ветка `feature/mN-*` и минимум один атомарный коммит по Conventional Commits.

Состояние: ☐ не начато · ◐ в работе · ☑ готово

## M0 — Discovery и документация ☑ (0.5 нед.)

Пакет документов (`assumptions`, PRD, architecture, roadmap, ui-architecture), `README`, `LICENSE` (MIT), `.gitignore`.

**Exit criteria:** все ключевые defaults зафиксированы; границы модулей и правила зависимостей описаны; приоритеты P0–P3 разложены по модулям.

## M1 — Scaffold монорепозитория и CI ☑ (1 нед.)

- pnpm workspace + Cargo workspace, версии инструментов зафиксированы (`.nvmrc`, `rust-toolchain.toml`).
- `apps/desktop` (Tauri v2, пустое окно), `packages/{ui,types,ipc,i18n,config}`, скелеты `crates/*` с тестом-заглушкой.
- Один сквозной IPC-вызов `megu3d.query.appInfo` на сгенерированных типах.
- GitHub Actions: fmt, clippy, cargo test, tsc, eslint, vitest, сборка Tauri-бандла как артефакт.
- Линт правил зависимостей и `cargo-deny`.

**Exit criteria:** `pnpm install && pnpm dev` открывает окно на чистой Windows-машине; все CI-гейты зелёные; типы IPC генерируются, ручных дублей нет.

## M2 — UI shell: layout, workspaces, command palette ☑ (2 нед.)

- Дизайн-токены, тёмная тема, HiDPI, базовые компоненты (кнопки, поля, списки, меню, tooltip).
- Docking: split-дерево, табы, drag-перекладывание, ресайз, persist layout.
- Workspace switcher и пресеты.
- Реестр команд + палитра (fuzzy-поиск, недавние, показ хоткеев) + слой keymap.
- i18n RU/EN без перезапуска.
- Заглушка Viewport-панели на native surface — чтобы граница UI/рендера была реальной с самого начала.

**Exit criteria:** разложенный layout сохраняется между запусками, «Reset layout» работает; палитра находит и выполняет команду; хоткеи назначаются и сохраняются; hardcoded-строк вне i18n нет (проверяется скриптом).

## M3 — Ядро сцены и первый настоящий вертикальный срез ☑ (3 нед.)

- `megu3d-core`: граф сцены, ноды, трансформы, кэш мировых матриц, выделение.
- `megu3d-cmd`: Command/Transaction/History (64 шага), preview-режим для drag.
- Примитивы: plane, cube, sphere, cylinder, cone, torus, empty, camera, light.
- `megu3d-render`: PBR-viewport, grid, orbit/pan/zoom, frame selected, shading modes, id-buffer picking, outline.
- Outliner (иерархия, переименование, видимость, drag-parent) и Properties (transform, mesh info).
- Move/Rotate/Scale гизмо, числовой ввод, local/global оси.

**Exit criteria:** добавление, выделение и трансформация работают из UI и из headless-теста ядра; property-тест `apply → undo` восстанавливает состояние побитово, redo идемпотентен; 60 FPS на 1 млн треугольников (`P-61`), отклик трансформа < 16 ms (`P-62`).

## M4 — MVP: persist, autosave, interop, look → P0 закрыт ☐ (3 нед.)

- `.megu3d` контейнер, `schemaVersion`, миграции, atomic save, `.bak`.
- Autosave 5 мин + recovery-диалог после краша.
- Импорт/экспорт OBJ и glTF 2.0 (`.gltf`/`.glb`) с материалами и иерархией.
- Базовые материалы metallic-roughness, свет sun/point/spot/area, заготовка HDRI-окружения.
- Базовый Edit Mode: выбор вершин/рёбер/полигонов, move, delete, extrude.
- Preferences: язык, тема, единицы, autosave, хоткеи, GPU.

**Exit criteria (это и есть определение MVP):** сценарий «создать → отредактировать → назначить материал и свет → сохранить → открыть → экспортировать glTF» проходит без потерь данных; round-trip тесты `.megu3d`, OBJ, glTF зелёные; убийство процесса теряет не больше 5 минут работы; открытие проекта 100 MB < 5 s без блокировки UI дольше 100 ms (`P-64`).

## M5 — P1: моделирование, модификаторы, точность ☐ (4 нед.)

loop cut, bevel, inset, knife, bridge, merge, normals tools · стек модификаторов Array/Mirror/Subdivision Surface (порядок, предпросмотр, apply) · snapping (grid/vertex/edge/face/increment), measure, точный числовой ввод · первые C++ kernels за FFI: BVH и subdivision.

**Exit criteria:** стек модификаторов не ломает undo и save/load; snapping точен на масштабах 1 мм – 1 км; kernels сверены с Rust-эталоном.

## M6 — P1: UV, текстуры, node-материалы ☐ (4 нед.)

UV unwrap (angle-based), seams, UV-редактор, packing · texture painting (кисти, слои, маски, pressure) · node material editor (типизированные ноды, превью, библиотека) · HDRI-библиотека (вращение, интенсивность, раздельные фон и освещение).

**Exit criteria:** материалы сохраняются и экспортируются в glTF без потерь поддерживаемого подмножества; редактор нод не допускает циклов и валидирует типы.

## M7 — P1: анимация и базовый риггинг ☐ (4 нед.)

Timeline, keyframes, интерполяция, dope sheet, graph editor · armature, bones, weight paint, IK-констрейнт · FBX и STL (FBX — документированное ограниченное подмножество).

**Exit criteria:** анимация переживает round-trip проекта; воспроизведение 30 FPS на риге из 100 костей; кривые редактируются с корректным undo.

## M8 — P2: процедурность, скульпт, симуляции, path tracer ☐ (10 нед., разбивается на подсрезы)

Geometry Nodes (детерминированная оценка графа, атрибуты, инстансинг) · sculpting (dyntopo/multires, кисти, маски, remesh) · auto-rig гуманоида · симуляции rigid → cloth → soft → hair → прототип fluid/smoke/fire · path tracer (CPU-референс → GPU, passes, опциональный денойз) · texture baking (AO, normal, curvature, lightmap, ID) · compositor · Grease-Pencil-like рисование · аддоны: architecture, trees, node assistant, mesh utilities.

**Exit criteria:** у каждой подсистемы есть golden-image или численный тест; ни одна не мутирует сцену вне команд; отмена долгих задач мгновенная.

## M9 — P3: видео, композитинг, pipeline ☐ (8 нед.)

Video Sequence Editor (дорожки, переходы, аудио, экспорт) · продвинутый композитинг (трекинг, ротоскоп, стабилизация) · scripting sandbox с публичным API и разрешениями · render farm hooks, CLI-рендер, библиотеки ассетов и publish/collect.

**Exit criteria:** скрипты не могут обойти систему команд и разрешений; CLI-рендер повторяет результат GUI-рендера.

## M10 — Стабилизация и релиз v1 ☐ (4 нед.)

Прогон бюджетов производительности на трёх конфигурациях GPU · crash-репортинг и recovery-стресс-тесты · MSI (WiX) + portable ZIP, подпись, updater · документация пользователя и хоткей-шпаргалка RU/EN · финальный проход по доступности и локализации.

**Exit criteria:** все метрики из PRD §7 достигнуты; чистая установка и обновление проверены на свежей Windows; известные проблемы задокументированы.

## Суммарная оценка

| Веха | Оценка | Накопительно |
|---|---|---|
| M0–M1 | 1.5 нед. | 1.5 |
| M2–M4 (MVP / P0) | 8 нед. | 9.5 |
| M5–M7 (P1) | 12 нед. | 21.5 |
| M8 (P2) | 10 нед. | 31.5 |
| M9 (P3) | 8 нед. | 39.5 |
| M10 (релиз) | 4 нед. | 43.5 |

Это оценка объёма работ одного разработчика полного цикла, а не календарный срок. При работе не полный день умножать соответственно; при появлении второго разработчика параллелить можно M5/M6 и M7 (разные подсистемы), но не M3/M4 (общее ядро).

## Порядок ветвления и коммитов

```bash
git switch -c feature/m1-scaffold
# атомарные коммиты: chore(repo), feat(ipc), ci(actions)
git switch main && git merge --no-ff feature/m1-scaffold
```

Типы коммитов: `feat`, `fix`, `refactor`, `perf`, `docs`, `test`, `build`, `ci`, `chore`. Скоупы — по модулям: `core`, `cmd`, `io`, `render`, `mesh`, `ui`, `ipc`, `desktop`, `kernels`, `sidecar`, `docs`, `repo`.

## Реестр рисков графика

| Риск | Влияние | Ответ |
|---|---|---|
| Виртуальный viewport на native surface сложнее ожидаемого | сдвиг M2/M3 | ранняя заглушка native surface уже в M2 |
| Совместимость драйверов GPU | сдвиг M3 | тест на трёх вендорах, fallback на Vulkan |
| Половинчатый Edit Mode тянет за собой M5 | сдвиг P1 | half-edge EditMesh проектируется сразу в M3, реализуется в M4 |
| Симуляции и path tracer съедают M8 | сдвиг P2 | подсрезы с независимыми exit criteria, path tracer отделён от симуляций |
| Рост scope от аддонов | размытие релиза | аддоны только после закрытия P1 |
