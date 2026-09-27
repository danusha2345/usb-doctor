# USB-доктор

Понятная диагностика для Windows 11 x64.

Статус: проектирование. Исходников приложения и готовых сборок пока нет.

- [План проекта, MVP, архитектура и критерии проверки](PLAN.md).
- [Правила работы с проектом](AGENTS.md).

## Репозитории

Все три репозитория должны оставаться приватными. Основная ветка — `main`.

- [GitHub](https://github.com/danusha2345/usb-doctor)
- [GitLab](https://gitlab.com/pipecpriam/usb-doctor)
- [Forgejo](https://git.danik2files.ru/danik/usb-doctor)

## Навигация и VCS

```sh
jj status
jj diff
codegraph status . --json
codegraph explore --path . "имя символа или задача"
```

Для нового checkout: `jj git init --colocate`, затем `codegraph init . --yes`.
После правок проверять status; `codegraph sync .` — при несинхронизированных изменениях.
На стадии документации граф может не содержать кодовых символов; читать PLAN.md напрямую.
Не создавать фиктивный код ради заполнения индекса.
