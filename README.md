# USB Глаз

USB-устройства, порты и подключения для Windows 10/11 x64. Runtime проверен на Windows 11.

Статус: 0.6.0 alpha — компактный Windows-интерфейс, независимый выбор полей, классовые декодеры и обновление по PnP-событиям. Сбор проверен на Windows 11 Home-PC и сопоставлен с USBTreeView; границы покрытия описаны ниже.

- [Покрытие декодеров и ограничения API](docs/research/DESCRIPTOR_DECODERS.md).
- [Скорость, темы, шрифт и совместимость 0.6.0](docs/RELEASE_06.md).
- [Обзор подключений, хабы, колонки и подсветка](docs/INVENTORY_05.md).
- [Исправления ревью и UX](docs/REVIEW_FIXES.md).
- [Проверка на Windows](docs/WINDOWS_LAB.md).
- [Что реализовано и что ещё не проверено](docs/IMPLEMENTATION.md).
- [Сборка, запуск и локальные артефакты](docs/BUILD.md).
- [План проекта, MVP, архитектура и критерии проверки](PLAN.md).
- [Матрица возможностей USBTreeView, 59 пунктов меню и границы исследования](docs/research/USBTREEVIEW_FEATURE_MATRIX.md).
- [Windows API, источники данных и ограничения](docs/research/WINDOWS_API_MAP.md).
- [Дизайн и состояния интерфейса](docs/design/DESIGN.md).
- [Проверка браузерного прототипа](docs/design/QA.md) — не проверка Windows-сборщика.
- [Открыть интерактивный прототип в браузере](docs/design/prototype.html) — локальный HTML без сборки и сетевых зависимостей.
- [Правила работы с проектом](AGENTS.md).

## Репозитории

Все три репозитория должны оставаться приватными. Основная ветка — `main`.

- [GitHub](https://github.com/danusha2345/usb-doctor)
- [GitLab](https://gitlab.com/pipecpriam/usb-doctor)
- [Forgejo](https://git.danik2files.ru/danik/usb-doctor)

## Скачать

Релиз `v0.6.0-alpha`: [GitHub](https://github.com/danusha2345/usb-doctor/releases/tag/v0.6.0-alpha),
[GitLab](https://gitlab.com/pipecpriam/usb-doctor/-/releases/v0.6.0-alpha),
[Forgejo](https://git.danik2files.ru/danik/usb-doctor/releases/tag/v0.6.0-alpha).
Скачать `usb-glaz.exe` и запустить без распаковки.
Windows 10/11 x64; фактически проверена Windows 11.

## Навигация и VCS

```sh
jj status
jj diff
codegraph status . --json
codegraph explore --path . "имя символа или задача"
```

Для нового checkout: `jj git init --colocate`, затем `codegraph init . --yes`.
После правок проверять status; `codegraph sync .` — при несинхронизированных изменениях.
CodeGraph индексирует Rust-сборщик, UI, тесты и JavaScript дизайн-прототипа.
Проектные решения и исследования читать в Markdown напрямую.
Не создавать фиктивный код ради заполнения индекса.
