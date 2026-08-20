use std::fmt;

pub(crate) const SOURCE_MAX_SCALARS: usize = 12_000;
pub(crate) const SOURCE_MAX_BYTES: usize = 48 * 1024;
pub(crate) const MODEL_REPLACEMENT_MAX_SCALARS: usize = 24_000;
pub(crate) const MODEL_REPLACEMENT_MAX_BYTES: usize = 96 * 1024;
pub(crate) const FINAL_APPLY_MAX_SCALARS: usize = 36_000;
pub(crate) const FINAL_APPLY_MAX_BYTES: usize = 144 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ContentLimitKind {
    Source,
    ModelReplacement,
    FinalApply,
}

impl ContentLimitKind {
    fn code(self) -> &'static str {
        match self {
            Self::Source => "source_content_limit_exceeded",
            Self::ModelReplacement => "model_replacement_content_limit_exceeded",
            Self::FinalApply => "final_apply_content_limit_exceeded",
        }
    }

    fn maxima(self) -> (usize, usize) {
        match self {
            Self::Source => (SOURCE_MAX_SCALARS, SOURCE_MAX_BYTES),
            Self::ModelReplacement => (MODEL_REPLACEMENT_MAX_SCALARS, MODEL_REPLACEMENT_MAX_BYTES),
            Self::FinalApply => (FINAL_APPLY_MAX_SCALARS, FINAL_APPLY_MAX_BYTES),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContentLimitError {
    kind: ContentLimitKind,
    current_scalars: usize,
    max_scalars: usize,
    current_bytes: usize,
    max_bytes: usize,
}

impl ContentLimitError {
    pub(crate) fn code(&self) -> &'static str {
        self.kind.code()
    }

    #[cfg(test)]
    pub(crate) fn counts(&self) -> (usize, usize, usize, usize) {
        (
            self.current_scalars,
            self.max_scalars,
            self.current_bytes,
            self.max_bytes,
        )
    }
}

impl fmt::Display for ContentLimitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:current_scalars={}:max_scalars={}:current_bytes={}:max_bytes={}",
            self.code(),
            self.current_scalars,
            self.max_scalars,
            self.current_bytes,
            self.max_bytes
        )
    }
}

pub(crate) fn validate_text_limit(
    kind: ContentLimitKind,
    text: &str,
) -> Result<(), ContentLimitError> {
    let current_scalars = text.chars().count();
    let current_bytes = text.len();
    let (max_scalars, max_bytes) = kind.maxima();
    if current_scalars <= max_scalars && current_bytes <= max_bytes {
        return Ok(());
    }
    Err(ContentLimitError {
        kind,
        current_scalars,
        max_scalars,
        current_bytes,
        max_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_exact_and_one_over(kind: ContentLimitKind, max_scalars: usize, max_bytes: usize) {
        let exact = "a".repeat(max_scalars);
        assert!(validate_text_limit(kind, &exact).is_ok());

        let over = "a".repeat(max_scalars + 1);
        let error = validate_text_limit(kind, &over).expect_err("max + 1 must fail");
        assert_eq!(
            error.counts(),
            (max_scalars + 1, max_scalars, max_scalars + 1, max_bytes)
        );
    }

    #[test]
    fn every_authoritative_limit_accepts_exact_ascii_and_rejects_one_more_scalar() {
        assert_exact_and_one_over(
            ContentLimitKind::Source,
            SOURCE_MAX_SCALARS,
            SOURCE_MAX_BYTES,
        );
        assert_exact_and_one_over(
            ContentLimitKind::ModelReplacement,
            MODEL_REPLACEMENT_MAX_SCALARS,
            MODEL_REPLACEMENT_MAX_BYTES,
        );
        assert_exact_and_one_over(
            ContentLimitKind::FinalApply,
            FINAL_APPLY_MAX_SCALARS,
            FINAL_APPLY_MAX_BYTES,
        );
    }

    #[test]
    fn source_limit_counts_korean_emoji_and_crlf_as_unicode_scalars_without_truncation() {
        let korean = "가".repeat(SOURCE_MAX_SCALARS);
        assert_eq!(korean.chars().count(), SOURCE_MAX_SCALARS);
        assert_eq!(korean.len(), SOURCE_MAX_SCALARS * 3);
        assert!(validate_text_limit(ContentLimitKind::Source, &korean).is_ok());

        let emoji = "😀".repeat(SOURCE_MAX_SCALARS);
        assert_eq!(emoji.encode_utf16().count(), SOURCE_MAX_SCALARS * 2);
        assert_eq!(emoji.chars().count(), SOURCE_MAX_SCALARS);
        assert_eq!(emoji.len(), SOURCE_MAX_SCALARS * 4);
        assert!(validate_text_limit(ContentLimitKind::Source, &emoji).is_ok());

        let crlf = "\r\n".repeat(SOURCE_MAX_SCALARS / 2);
        assert_eq!(crlf.chars().count(), SOURCE_MAX_SCALARS);
        assert!(validate_text_limit(ContentLimitKind::Source, &crlf).is_ok());

        for over in [
            format!("{korean}가"),
            format!("{emoji}😀"),
            format!("{crlf}x"),
        ] {
            let error = validate_text_limit(ContentLimitKind::Source, &over)
                .expect_err("one additional scalar must fail without coercion");
            assert_eq!(error.code(), "source_content_limit_exceeded");
            assert_eq!(error.counts().0, SOURCE_MAX_SCALARS + 1);
        }
    }

    #[test]
    fn limit_error_is_content_free_and_reports_only_counts() {
        let synthetic = format!("{}PRIVATE_SENTINEL", "x".repeat(SOURCE_MAX_SCALARS));
        let error = validate_text_limit(ContentLimitKind::Source, &synthetic)
            .expect_err("synthetic oversized input must fail")
            .to_string();
        assert!(error.starts_with("source_content_limit_exceeded:"));
        assert!(!error.contains("PRIVATE_SENTINEL"));
    }
}
