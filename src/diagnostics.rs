use crate::model::{Device, Speed};
use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    Neutral,
    Attention,
    Unknown,
}
#[derive(Clone, Debug, Serialize)]
pub struct Assessment {
    pub rule_id: &'static str,
    pub tone: Tone,
    pub title: &'static str,
    pub fact: String,
    pub interpretation: &'static str,
    pub next_step: &'static str,
}
pub fn assess(device: &Device) -> Assessment {
    if device.connection_status.is_some_and(|s| s != 1)
        || device.windows_problem.is_some_and(|c| c != 0)
    {
        return Assessment {
            rule_id: "windows-reported-problem",
            tone: Tone::Attention,
            title: "Windows сообщает о проблеме",
            fact: format!(
                "Статус соединения: {:?}. Код проблемы Windows: {:?}.",
                device.connection_status, device.windows_problem
            ),
            interpretation: "Код описывает состояние, но не устанавливает физическую причину.",
            next_step: "Сохраните снимок и сравните подключение напрямую к другому порту, меняя одно условие за раз.",
        };
    }
    let speed = device.speed.speed();
    if speed == Speed::Unknown {
        return Assessment {
            rule_id: "speed-unavailable",
            tone: Tone::Unknown,
            title: "Недостаточно данных о режиме",
            fact: "Достоверный текущий режим USB не определён.".into(),
            interpretation: "Недоступные или противоречивые поля не доказывают неисправность.",
            next_step: "Откройте подробности: там указаны доступные поля и ошибки запросов. Повторите сбор после переподключения.",
        };
    }
    if speed == Speed::High && device.speed.super_capable() == Some(true) {
        return Assessment {
            rule_id: "superspeed-capable-at-high",
            tone: Tone::Attention,
            title: "Соединение может быть быстрее",
            fact: "Сейчас 480 Мбит/с. Полученные сведения подтверждают поддержку более быстрого режима."
                .into(),
            interpretation: "Путь подключения может ограничивать режим. Причина ограничения не установлена.",
            next_step: "Сохраните снимок, подключите устройство напрямую к подходящему порту с тем же кабелем и сравните результат.",
        };
    }
    Assessment {
        rule_id: "observed-mode",
        tone: Tone::Neutral,
        title: "Режим подключения определён",
        fact: format!("Текущий режим: {}.", device.speed_label()),
        interpretation: "Эта проверка не измеряет скорость копирования и не подтверждает физическую исправность.",
        next_step: "Если есть перебои, сохраните снимок и сравните другой порт. Low/Full-Speed сами по себе не являются неисправностью.",
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ComparisonError {
    IdentityNotConfirmed,
    MixedDemoAndLive,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Comparison {
    pub before: Speed,
    pub after: Speed,
    pub changed: bool,
}
pub fn compare(
    before: &Device,
    after: &Device,
    identity_confirmed: bool,
    same_source: bool,
) -> Result<Comparison, ComparisonError> {
    if !same_source {
        return Err(ComparisonError::MixedDemoAndLive);
    }
    if !identity_confirmed {
        return Err(ComparisonError::IdentityNotConfirmed);
    }
    let a = before.speed.speed();
    let b = after.speed.speed();
    Ok(Comparison {
        changed: a != b || before.speed_label() != after.speed_label(),
        before: a,
        after: b,
    })
}
