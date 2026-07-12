# mcp-1c-tools — Портирование скилов из cc-1c-skills

## Статус: 35/71 скилов портировано (+5)

## Priority 1 (простые, ~5-10 KB)
- [x] `template-add` — Добавить макет/шаблон в XML
- [x] `template-remove` — Удалить макет из XML
- [x] `form-add` — Создать Form.xml + модуль формы
- [x] `form-remove` — Удалить форму из метаданных
- [x] `help-add` — Добавить справку
- [x] `support-edit` — Снять/поставить с поддержки
- [x] `cf-init` — Создать Configuration.xml
- [x] `epf-validate` — Проверка структуры EPF
- [x] `erf-validate` — Проверка структуры ERF
- [x] `cf-info` — Информация о корне конфигурации
- [x] `cf-edit` — Правка Configuration.xml
- [x] `cf-validate` — Валидация корня конфигурации
- [x] `meta-edit` — Правка метаданных
- [x] `meta-validate` — Валидация XML метаданных
- [x] `meta-remove` — Удаление объекта метаданных
- [ ] `subsystem-info` — Инфо о подсистеме
- [ ] `subsystem-edit` — Правка подсистемы
- [ ] `subsystem-validate` — Валидация подсистемы
- [ ] `interface-edit` — Правка командного интерфейса
- [ ] `role-edit` — Правка роли
- [ ] `role-validate` — Валидация роли
- [ ] `mxl-info` — Инфо о макете
- [ ] `mxl-compile` — MXL JSON DSL → XML
- [ ] `mxl-decompile` — MXL XML → JSON DSL

## Priority 2 (средние, ~15-30 KB)
- [ ] `form-edit` — Правка формы через DSL
- [ ] `form-validate` — Проверка формы
- [ ] `form-patterns` — Паттерны форм (документация)
- [ ] `cfe-init` — Создать расширение
- [ ] `cfe-borrow` — Заимствовать объекты
- [ ] `cfe-diff` — Сравнить расширение
- [ ] `cfe-patch-method` — Пропатчить метод
- [ ] `cfe-validate` — Валидация расширения
- [ ] `img-grid` — Сетка для изображений

## Priority 3 (сложные, 30+ KB или Node.js)
- [ ] `web-test` — E2E-тестирование веб-клиента
- [ ] `skd-edit` — Правка СКД
