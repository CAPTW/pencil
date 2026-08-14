use crate::settings::RewriteMode;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum TranslationTargetLanguage {
    #[serde(rename = "ko")]
    Ko,
    #[serde(rename = "en")]
    En,
    #[serde(rename = "ja")]
    Ja,
    #[serde(rename = "zh-Hans")]
    ZhHans,
    #[serde(rename = "zh-Hant")]
    ZhHant,
}

impl TranslationTargetLanguage {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 5] = [Self::Ko, Self::En, Self::Ja, Self::ZhHans, Self::ZhHant];

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Ko => "ko",
            Self::En => "en",
            Self::Ja => "ja",
            Self::ZhHans => "zh-Hans",
            Self::ZhHant => "zh-Hant",
        }
    }

    pub(crate) fn instruction_name(self) -> &'static str {
        match self {
            Self::Ko => "Korean",
            Self::En => "English",
            Self::Ja => "Japanese",
            Self::ZhHans => "Simplified Chinese",
            Self::ZhHant => "Traditional Chinese",
        }
    }
}

impl Default for TranslationTargetLanguage {
    fn default() -> Self {
        Self::En
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RewriteIntent {
    mode: RewriteMode,
    target_language: Option<TranslationTargetLanguage>,
}

impl RewriteIntent {
    pub(crate) fn new(
        mode: RewriteMode,
        target_language: Option<TranslationTargetLanguage>,
    ) -> Result<Self, &'static str> {
        match mode {
            RewriteMode::Translate => target_language
                .map(|target_language| Self {
                    mode,
                    target_language: Some(target_language),
                })
                .ok_or("translation_target_required"),
            _ if target_language.is_some() => Err("translation_target_not_allowed"),
            _ => Ok(Self {
                mode,
                target_language: None,
            }),
        }
    }

    #[cfg(test)]
    pub(crate) const fn grammar() -> Self {
        Self {
            mode: RewriteMode::Grammar,
            target_language: None,
        }
    }

    pub(crate) fn mode(self) -> RewriteMode {
        self.mode
    }

    pub(crate) fn target_language(self) -> Option<TranslationTargetLanguage> {
        self.target_language
    }

    pub(crate) fn is_translation(self) -> bool {
        self.mode == RewriteMode::Translate
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TranslationApplyFormat {
    #[default]
    TranslationOnly,
    SourceWithTranslation,
}

pub(crate) fn format_translation(
    source: &str,
    translated: &str,
    format: TranslationApplyFormat,
) -> Result<String, &'static str> {
    if translated.is_empty() {
        return Err("empty_translation");
    }

    match format {
        TranslationApplyFormat::TranslationOnly => Ok(translated.to_string()),
        TranslationApplyFormat::SourceWithTranslation
            if !source.contains('\r') && !source.contains('\n') =>
        {
            Ok(format!("{source} ({translated})"))
        }
        TranslationApplyFormat::SourceWithTranslation => {
            let separator = line_separator(source);
            let separator_count = if source.ends_with(separator) { 1 } else { 2 };
            let mut output = String::with_capacity(
                source.len() + translated.len() + separator.len() * separator_count + 2,
            );
            output.push_str(source);
            for _ in 0..separator_count {
                output.push_str(separator);
            }
            output.push('(');
            output.push_str(translated);
            output.push(')');
            Ok(output)
        }
    }
}

fn line_separator(source: &str) -> &str {
    if source.ends_with("\r\n") {
        return "\r\n";
    }
    if source.ends_with('\n') {
        return "\n";
    }
    if source.ends_with('\r') {
        return "\r";
    }

    let bytes = source.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\r' {
            return if bytes.get(index + 1) == Some(&b'\n') {
                "\r\n"
            } else {
                "\r"
            };
        }
        if *byte == b'\n' {
            return "\n";
        }
    }
    "\n"
}
