//! Перевод только слоя представления. Идентификаторы/снимки не изменяются.
use serde::{Deserialize, Serialize};
use std::cell::Cell;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    Russian,
    English,
}
thread_local! { static CURRENT: Cell<Language> = const { Cell::new(Language::Russian) }; }
pub fn set_language(language: Language) {
    CURRENT.set(language);
}
pub fn t(text: impl AsRef<str>) -> String {
    CURRENT.with(|l| translate(l.get(), text.as_ref()))
}
pub fn translate(language: Language, text: &str) -> String {
    if language == Language::Russian {
        return text.to_owned();
    }
    english(text, 0)
}
fn english(text: &str, depth: usize) -> String {
    if depth > 8 || !text.chars().any(|c| ('\u{0400}'..='\u{04ff}').contains(&c)) {
        return text.to_owned();
    }
    if let Some(value) = CATALOG.exact.get(text) {
        return value.clone();
    }
    for (ru, en) in &CATALOG.patterns {
        if let Some(values) = captures(text, ru) {
            let mut out = en[0].clone();
            for (i, value) in values.iter().enumerate() {
                out.push_str(&english(value, depth + 1));
                out.push_str(&en[i + 1]);
            }
            return out;
        }
    }
    for separator in [" + ", " / ", " · ", "\n"] {
        if text.contains(separator) {
            let parts: Vec<_> = text
                .split(separator)
                .map(|p| english(p, depth + 1))
                .collect();
            let translated = parts.join(separator);
            if translated != text {
                return translated;
            }
        }
    }
    text.to_owned()
}
fn captures<'a>(text: &'a str, parts: &[String]) -> Option<Vec<&'a str>> {
    let mut rest = text.strip_prefix(&parts[0])?;
    let mut values = Vec::with_capacity(parts.len() - 1);
    for (index, part) in parts.iter().enumerate().skip(1) {
        if index == parts.len() - 1 {
            values.push(rest.strip_suffix(part)?);
            return Some(values);
        }
        let pos = rest.find(part)?;
        values.push(&rest[..pos]);
        rest = &rest[pos + part.len()..];
    }
    None
}

struct Catalogue {
    exact: std::collections::HashMap<String, String>,
    patterns: Vec<(Vec<String>, Vec<String>)>,
}
// Parse once. Translations apply only at display time; collectors never use this catalogue.
static CATALOG: std::sync::LazyLock<Catalogue> = std::sync::LazyLock::new(|| {
    let entries: std::collections::BTreeMap<String, String> =
        serde_json::from_str(include_str!("translations.json"))
            .expect("embedded translation catalogue");
    let mut exact = std::collections::HashMap::new();
    let mut patterns = Vec::new();
    for (ru, en) in entries {
        if ru.contains('{') {
            let ru_parts = parts(&ru);
            let en_parts = parts(&en);
            assert_eq!(
                ru_parts.len(),
                en_parts.len(),
                "translation placeholders: {ru}"
            );
            patterns.push((ru_parts, en_parts));
        } else {
            exact.insert(ru, en);
        }
    }
    // More specific messages must win over generic suffix/prefix patterns.
    patterns.sort_by_key(|(ru, _)| std::cmp::Reverse(ru.iter().map(String::len).sum::<usize>()));
    Catalogue { exact, patterns }
});
fn parts(mut template: &str) -> Vec<String> {
    let mut result = Vec::new();
    while let Some(start) = template.find('{') {
        result.push(template[..start].to_owned());
        template = &template[start..];
        while template.starts_with('{') {
            template = &template[template.find('}').expect("closed translation placeholder") + 1..];
        }
    }
    result.push(template.to_owned());
    result
}
