//! Canonical writing adapter; no independent prompt or result acceptance contract.
use super::{api_session::RequestTicket, types::ResultProvider};
use crate::{
    codex_client::{parse_rewrite_result, RewriteResult},
    terminology_matcher::TerminologyConstraint,
    translation::RewriteIntent,
};
pub(crate) async fn rewrite(
    ticket: &RequestTicket,
    source: &str,
    intent: RewriteIntent,
    terms: &[TerminologyConstraint],
) -> Result<RewriteResult, String> {
    crate::content_limits::validate_text_limit(
        crate::content_limits::ContentLimitKind::Source,
        source,
    )
    .map_err(|_| "api_input_limit")?;
    let prompt = crate::writing_contract::build_canonical_prompt(source, intent, terms)
        .map_err(|_| "api_prompt_invalid")?;
    let text = ticket
        .complete(&prompt)
        .await
        .map_err(|error| format!("api_{error:?}"))?;
    parse(&text, intent)
}
fn parse(text: &str, intent: RewriteIntent) -> Result<RewriteResult, String> {
    let mut result = parse_rewrite_result(text, intent.mode()).map_err(|_| "api_result_invalid")?;
    result.provider_used = ResultProvider::Api;
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_result_has_api_provenance() {
        let text = r#"{"replacement":"Corrected text.","changed":true,"summary":"Correction","edits":[],"confidence":0.9,"usedTerminologyIds":[],"terminologySuggestions":[]}"#;
        let result = parse(text, RewriteIntent::grammar()).unwrap();
        assert_eq!(result.provider_used, ResultProvider::Api);
        assert_eq!(serde_json::to_value(result).unwrap()["providerUsed"], "api");
    }
    #[test]
    fn model_supplied_errors_cannot_leak_into_diagnostics() {
        assert_eq!(
            parse("sensitive source echo", RewriteIntent::grammar()).unwrap_err(),
            "api_result_invalid"
        );
    }
}
