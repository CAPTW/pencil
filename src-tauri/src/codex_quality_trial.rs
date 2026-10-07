//! Explicit owned-Sandbox live trial; never selected by ordinary test runs.
use super::*;

#[tokio::test]
#[ignore = "requires owned Windows Sandbox and explicit live trial invocation"]
async fn owned_sandbox_quality_trial() {
    assert_eq!(std::env::var("USERNAME").as_deref(), Ok("WDAGUtilityAccount"));
    assert_eq!(std::env::var("GRAMMAR_QUALITY_TRIAL").as_deref(), Ok("owned-synthetic-20261007"));
    assert!(std::path::Path::new("C:/GrammarQuality/owned-trial.marker").is_file());
    let receipt = std::path::Path::new("C:/GrammarQualityOutput/quality.json");
    let corpus: Value = serde_json::from_str(include_str!("../../scripts/mission/provider-quality-cases.json")).unwrap();
    let mut report = json!({"status":"STARTED","scope":"PRODUCTION_CODEX_CLIENT_NOT_WIDGET_ACCEPTANCE",
        "modelRequested":DEFAULT_REWRITE_MODEL,"modelReported":"UNKNOWN",
        "calls":[],"qualityReview":"PENDING","automaticRetry":false});
    let persist = |v: &Value| std::fs::write(receipt, serde_json::to_vec_pretty(v).unwrap()).unwrap();
    persist(&report);
    let started=std::time::Instant::now();
    let client=match CodexClient::connect().await {
        Ok(c)=>c,Err(_)=>{report["status"]=json!("CONNECTION_FAILED");persist(&report);panic!("connection failed; no raw protocol data logged");}
    };
    report["connectMs"]=json!(started.elapsed().as_millis());
    let authenticated=matches!(client.auth_status().await,Ok(a) if a.logged_in);
    if !authenticated || std::env::var("GRAMMAR_QUALITY_LIVE").as_deref()!=Ok("1") {
        report["status"]=json!(if authenticated {"AUTH_READY_NOT_RUN"}else{"LOGIN_REQUIRED_NOT_RUN"});
        report["shutdownComplete"]=json!(client.shutdown_child().await.is_ok());persist(&report);
        assert!(authenticated,"Owner login required inside guest; no inference attempted");return;
    }
    for case in corpus["cases"].as_array().unwrap() {
        let mode: RewriteMode=serde_json::from_value(case["mode"].clone()).unwrap();
        let target=if case["targetLanguage"].is_null(){None}else{Some(serde_json::from_value(case["targetLanguage"].clone()).unwrap())};
        let intent=RewriteIntent::new(mode,target).unwrap();
        let start=std::time::Instant::now();
        let result=client.rewrite_with_terminology(case["source"].as_str().unwrap(),intent,&[]).await;
        let mut row=json!({"id":case["id"],"clientEndToEndMs":start.elapsed().as_millis(),"draftVisibleMs":null,
            "effort":if mode==RewriteMode::Translate{FAST_TRANSLATION_REASONING_EFFORT}else{DEFAULT_REWRITE_REASONING_EFFORT}});
        let failed=result.is_err();
        match result {Ok(value)=>{row["status"]=json!("OUTPUT_RECEIVED_NOT_QUALITY_PASS");row["result"]=serde_json::to_value(value).unwrap();},
            Err(_)=>{row["status"]=json!("PROVIDER_OR_VALIDATION_ERROR");}}
        report["calls"].as_array_mut().unwrap().push(row);persist(&report);
        if failed {break;} // no fallback, replay, or repeating an unsupported-model request
    }
    report["shutdownComplete"]=json!(client.shutdown_child().await.is_ok());
    report["status"]=json!("REVIEW_REQUIRED");persist(&report);
    assert_eq!(report["shutdownComplete"],true,"cleanup incomplete");
}
