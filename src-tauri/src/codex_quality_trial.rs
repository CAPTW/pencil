//! Explicit owned-Sandbox live trial; never selected by ordinary test runs.
use super::*;

#[tokio::test]
#[ignore = "requires owned Windows Sandbox and explicit live trial invocation"]
async fn owned_sandbox_quality_trial() {
    assert_eq!(
        std::env::var("USERNAME").as_deref(),
        Ok("WDAGUtilityAccount")
    );
    assert_eq!(
        std::env::var("GRAMMAR_QUALITY_TRIAL").as_deref(),
        Ok("owned-synthetic-20261007")
    );
    assert!(std::path::Path::new("C:/GrammarQuality/owned-trial.marker").is_file());
    let receipt = std::path::Path::new("C:/GrammarQualityOutput/quality.json");
    let extended = std::env::var("GRAMMAR_QUALITY_EXTENDED").as_deref() == Ok("1");
    let corpus: Value = serde_json::from_str(if extended {
        include_str!("../../scripts/mission/provider-quality-extended.json")
    } else {
        include_str!("../../scripts/mission/provider-quality-cases.json")
    })
    .unwrap();
    let mut report = json!({"status":"STARTED","scope":"PRODUCTION_CODEX_CLIENT_NOT_WIDGET_ACCEPTANCE",
        "modelRequested":DEFAULT_REWRITE_MODEL,"modelReported":"UNKNOWN",
        "calls":[],"qualityReview":"PENDING","automaticRetry":false});
    let persist =
        |v: &Value| std::fs::write(receipt, serde_json::to_vec_pretty(v).unwrap()).unwrap();
    persist(&report);
    let started = std::time::Instant::now();
    let catalog_new = std::env::var("GRAMMAR_QUALITY_NEWCLI").as_deref() == Ok("1");
    let connection = if catalog_new {
        report["scope"] = json!("NEW_CLI_RUST_TRANSPORT_SYNTHETIC_NOT_INSTALLED_QUALIFICATION");
        CodexClient::connect_version(
            Arc::new(AtomicBool::new(false)),
            "codex-cli 0.162.0-alpha.2",
        )
        .await
    } else {
        CodexClient::connect().await
    };
    let client = match connection {
        Ok(c) => c,
        Err(_) => {
            report["status"] = json!("CONNECTION_FAILED");
            persist(&report);
            panic!("connection failed; no raw protocol data logged");
        }
    };
    report["connectMs"] = json!(started.elapsed().as_millis());
    let session_root = client.runtime_cwd.parent().unwrap().to_path_buf();
    let authenticated = matches!(client.auth_status().await,Ok(a) if a.logged_in);
    if !authenticated || std::env::var("GRAMMAR_QUALITY_LIVE").as_deref() != Ok("1") {
        report["status"] = json!(if authenticated {
            "AUTH_READY_NOT_RUN"
        } else {
            "LOGIN_REQUIRED_NOT_RUN"
        });
        report["shutdownComplete"] = json!(client.shutdown_child().await.is_ok());
        persist(&report);
        assert!(
            authenticated,
            "Owner login required inside guest; no inference attempted"
        );
        return;
    }
    if std::env::var("GRAMMAR_QUALITY_CATALOG").as_deref() == Ok("1") {
        match client
            .request(
                "model/list",
                json!({"limit":100,"includeHidden":true}),
                Duration::from_secs(30),
            )
            .await
        {
            Ok(v) => {
                report["modelCatalog"]=json!(v["data"].as_array().unwrap_or(&vec![]).iter().map(|m|json!({"id":m["id"],"model":m["model"],"supportedReasoningEfforts":m["supportedReasoningEfforts"],"serviceTiers":m["serviceTiers"]})).collect::<Vec<_>>());
                report["catalogHasMore"] = json!(!v["nextCursor"].is_null());
            }
            Err(_) => {
                report["catalogError"] = json!("MODEL_LIST_FAILED");
            }
        }
        report["status"] = json!("CATALOG_ONLY_NO_INFERENCE");
        report["shutdownComplete"] = json!(client.shutdown_child().await.is_ok());
        persist(&report);
        return;
    }
    if std::env::var("GRAMMAR_QUALITY_SAFETY").as_deref() == Ok("1") {
        let source = "This synthetic draft contain errors and need careful revision. ".repeat(30);
        let intent = RewriteIntent::grammar();
        let pending = match client
            .prepare_rewrite_with_terminology(&source, intent, &[])
            .await
        {
            Ok(prepared) => client.start_prepared_rewrite(prepared).await,
            Err(e) => Err(e),
        };
        let pending = match pending {
            Ok(pending) => pending,
            Err(_) => {
                report["status"] = json!("SAFETY_START_FAILED");
                report["shutdownComplete"] = json!(client.shutdown_child().await.is_ok());
                persist(&report);
                panic!("safety start failed; inspect content-free receipt");
            }
        };
        let start = std::time::Instant::now();
        let interrupt = client.interrupt_turn(&pending.active_turn).await;
        report["interruptAcknowledged"] = json!(interrupt.is_ok());
        report["interruptAckMs"] = json!(start.elapsed().as_millis());
        let outcome = client.complete_rewrite(pending).await;
        report["cancelOutcome"] = json!(match outcome {
            Err(ref e) if e == "rewrite_interrupted" => "INTERRUPTED",
            Err(_) => "OTHER_ERROR",
            Ok(_) => "COMPLETED_BEFORE_CANCEL",
        });
        let recovery_start = std::time::Instant::now();
        let recovery = client
            .rewrite_with_terminology("She have two books.", intent, &[])
            .await;
        report["recoveryMs"] = json!(recovery_start.elapsed().as_millis());
        let recovered = recovery.is_ok();
        report["recovery"] = match recovery {
            Ok(r) => serde_json::to_value(r).unwrap(),
            Err(_) => json!({"error":"RECOVERY_FAILED"}),
        };
        report["status"] = json!("SAFETY_REVIEW_REQUIRED");
        report["shutdownComplete"] = json!(client.shutdown_child().await.is_ok());
        report["runtimeRemoved"] = json!(!session_root.exists());
        persist(&report);
        assert!(
            recovered
                && report["interruptAcknowledged"] == true
                && report["cancelOutcome"] == "INTERRUPTED"
                && report["shutdownComplete"] == true,
            "safety trial incomplete; inspect receipt"
        );
        return;
    }
    let cases: Vec<&Value> = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| {
            std::env::var("GRAMMAR_QUALITY_CONTEXT_ONLY").as_deref() != Ok("1")
                || c["id"].as_str().unwrap().starts_with("context-recheck-")
        })
        .collect();
    for case in &cases {
        let mode: RewriteMode = serde_json::from_value(case["mode"].clone()).unwrap();
        let target = if case["targetLanguage"].is_null() {
            None
        } else {
            Some(serde_json::from_value(case["targetLanguage"].clone()).unwrap())
        };
        let reference = case
            .get("autoReference")
            .map(|v| serde_json::from_value(v.clone()).unwrap());
        let intent = RewriteIntent::new_with_auto_reference(mode, target, reference).unwrap();
        let terms: Vec<TerminologyConstraint> = case["terms"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|t| TerminologyConstraint {
                id: t["id"].as_str().unwrap().to_owned(),
                entry_type: serde_json::from_value(t["type"].clone()).unwrap(),
                source_text: t["sourceText"].as_str().unwrap().to_owned(),
                preferred_text: t["preferredText"].as_str().map(str::to_owned),
            })
            .collect();
        let start = std::time::Instant::now();
        let mut phase = "prepare";
        let result = match client
            .prepare_rewrite_with_terminology(case["source"].as_str().unwrap(), intent, &terms)
            .await
        {
            Err(e) => Err(e),
            Ok(mut prepared) => {
                if std::env::var("GRAMMAR_QUALITY_FULL_PROFILE").as_deref() == Ok("1") {
                    prepared.profile = RewriteRequestProfile::Full;
                    prepared.prompt = rewrite_prompt_for_profile(
                        case["source"].as_str().unwrap(),
                        intent,
                        &terms,
                        RewriteRequestProfile::Full,
                    )
                    .unwrap();
                }
                phase = "start";
                match client.start_prepared_rewrite(prepared).await {
                    Err(e) => Err(e),
                    Ok(pending) => {
                        phase = "complete";
                        client.complete_rewrite(pending).await
                    }
                }
            }
        };
        let mut row = json!({"id":case["id"],"clientEndToEndMs":start.elapsed().as_millis(),"draftVisibleMs":null,
            "effort":if mode==RewriteMode::Translate&&terms.is_empty()&&std::env::var("GRAMMAR_QUALITY_FULL_PROFILE").as_deref()!=Ok("1"){FAST_TRANSLATION_REASONING_EFFORT}else{DEFAULT_REWRITE_REASONING_EFFORT}});
        let failed = result.is_err();
        match result {
            Ok(value) => {
                row["status"] = json!("OUTPUT_RECEIVED_NOT_QUALITY_PASS");
                row["result"] = serde_json::to_value(value).unwrap();
            }
            Err(e) => {
                row["status"] = json!("PROVIDER_OR_VALIDATION_ERROR");
                row["phase"] = json!(phase);
                row["errorCode"] = json!(if e.starts_with("codex_")
                    && e.len() < 100
                    && e.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
                {
                    e
                } else {
                    stable_turn_error_code(&json!({"message":e})).to_string()
                });
            }
        }
        report["calls"].as_array_mut().unwrap().push(row);
        persist(&report);
        if failed {
            break;
        } // no fallback, replay, or repeating an unsupported-model request
    }
    report["shutdownComplete"] = json!(client.shutdown_child().await.is_ok());
    report["runtimeRemoved"] = json!(!session_root.exists());
    let all_received = report["calls"].as_array().unwrap().len() == cases.len()
        && report["calls"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["status"] == "OUTPUT_RECEIVED_NOT_QUALITY_PASS");
    report["status"] = json!(if all_received {
        "REVIEW_REQUIRED"
    } else {
        "INCOMPLETE_PROVIDER_ERROR"
    });
    persist(&report);
    assert_eq!(report["shutdownComplete"], true, "cleanup incomplete");
    assert_eq!(report["runtimeRemoved"], true, "runtime still exists");
    assert!(
        all_received,
        "live cases incomplete; inspect content-free receipt"
    );
}
