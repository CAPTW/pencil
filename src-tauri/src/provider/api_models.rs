//! Explicit catalog lookup only: no inference, prompt, persistence or retries.
use super::http_api::{validate_key, ApiConfig};
use reqwest::{header::HeaderValue, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    time::Duration,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Lookup {
    pub endpoint: String,
    pub key: String,
}
impl Drop for Lookup {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.key.zeroize();
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Model {
    pub id: String,
    pub name: String,
    pub selectable: bool,
}
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Claude,
    Google,
    Router,
    Xai,
}
fn destination(endpoint: &str) -> Result<(Kind, Url), &'static str> {
    let (kind, url) = match endpoint {
        "https://api.anthropic.com/v1/messages" => {
            (Kind::Claude, "https://api.anthropic.com/v1/models")
        }
        "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions" => (
            Kind::Google,
            "https://generativelanguage.googleapis.com/v1beta/models",
        ),
        "https://openrouter.ai/api/v1/chat/completions" => {
            (Kind::Router, "https://openrouter.ai/api/v1/models/user")
        }
        "https://api.x.ai/v1/chat/completions" => (Kind::Xai, "https://api.x.ai/v1/models"),
        _ => return Err("api_catalog_unsupported_endpoint"),
    };
    Ok((
        kind,
        Url::parse(url).map_err(|_| "api_catalog_configuration")?,
    ))
}
static BUSY: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
pub(crate) async fn list(lookup: Lookup) -> Result<Vec<Model>, &'static str> {
    let _permit = BUSY.try_acquire().map_err(|_| "api_catalog_busy")?;
    let (kind, url) = destination(&lookup.endpoint)?;
    validate_key(&lookup.key).map_err(|_| "api_invalid_key")?;
    tokio::time::timeout(
        Duration::from_secs(20),
        fetch(kind, url, &lookup.key, &lookup.endpoint),
    )
    .await
    .map_err(|_| "api_catalog_timeout")?
}
async fn fetch(
    kind: Kind,
    base: Url,
    key: &str,
    endpoint: &str,
) -> Result<Vec<Model>, &'static str> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .no_proxy()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(20))
        .pool_max_idle_per_host(0)
        .build()
        .map_err(|_| "api_catalog_transport")?;
    let credential = zeroize::Zeroizing::new(if kind == Kind::Claude || kind == Kind::Google {
        key.to_owned()
    } else {
        format!("Bearer {key}")
    });
    let mut header = HeaderValue::from_str(&credential).map_err(|_| "api_invalid_key")?;
    header.set_sensitive(true);
    let mut cursor: Option<String> = None;
    let mut seen = HashSet::new();
    let mut models = BTreeMap::new();
    let mut budget = 8 * 1024 * 1024usize;
    for _ in 0..20 {
        let mut url = base.clone();
        if kind == Kind::Claude {
            url.query_pairs_mut().append_pair("limit", "1000");
        }
        if kind == Kind::Google {
            url.query_pairs_mut().append_pair("pageSize", "1000");
        }
        if kind == Kind::Router {
            url.query_pairs_mut()
                .append_pair("output_modalities", "all");
        }
        if let Some(ref token) = cursor {
            url.query_pairs_mut().append_pair(
                if kind == Kind::Claude {
                    "after_id"
                } else {
                    "pageToken"
                },
                token,
            );
        }
        let mut request = client.get(url).header(
            match kind {
                Kind::Claude => "x-api-key",
                Kind::Google => "x-goog-api-key",
                _ => "authorization",
            },
            header.clone(),
        );
        if kind == Kind::Claude {
            request = request.header("anthropic-version", "2023-06-01");
        }
        let mut response = request.send().await.map_err(|_| "api_catalog_transport")?;
        match response.status().as_u16() {
            200 => (),
            401 | 403 => return Err("api_catalog_auth_denied"),
            429 => return Err("api_catalog_rate_limited"),
            _ => return Err("api_catalog_http_error"),
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "api_catalog_transport")?
        {
            if chunk.len() > budget {
                return Err("api_catalog_limit");
            }
            budget -= chunk.len();
            bytes.extend_from_slice(&chunk);
        }
        let (rows, next) = parse(kind, &bytes, endpoint)?;
        for row in rows {
            models.insert(row.id.clone(), row);
        }
        if models.len() > 10000 {
            return Err("api_catalog_limit");
        }
        match next {
            None => return Ok(models.into_values().collect()),
            Some(token) => {
                if !seen.insert(token.clone()) {
                    return Err("api_catalog_invalid_response");
                }
                cursor = Some(token);
            }
        }
    }
    Err("api_catalog_limit")
}
fn parse(
    kind: Kind,
    bytes: &[u8],
    endpoint: &str,
) -> Result<(Vec<Model>, Option<String>), &'static str> {
    let v: Value = serde_json::from_slice(bytes).map_err(|_| "api_catalog_invalid_response")?;
    let rows = v[if kind == Kind::Google {
        "models"
    } else {
        "data"
    }]
    .as_array()
    .ok_or("api_catalog_invalid_response")?;
    let mut models = Vec::new();
    for row in rows {
        let raw = row[if kind == Kind::Google { "name" } else { "id" }]
            .as_str()
            .ok_or("api_catalog_invalid_response")?;
        let id = if kind == Kind::Google {
            raw.strip_prefix("models/")
                .ok_or("api_catalog_invalid_response")?
        } else {
            raw
        };
        if id.is_empty() || id.len() > 256 || id.chars().any(char::is_control) {
            return Err("api_catalog_invalid_response");
        }
        let name = row[match kind {
            Kind::Claude => "display_name",
            Kind::Google => "displayName",
            _ => "name",
        }]
        .as_str()
        .unwrap_or(id);
        if name.len() > 512 || name.chars().any(char::is_control) {
            return Err("api_catalog_invalid_response");
        }
        let supported = if kind == Kind::Google {
            row["supportedGenerationMethods"]
                .as_array()
                .is_some_and(|a| a.iter().any(|m| m == "generateContent"))
        } else if kind == Kind::Router {
            row["architecture"]["output_modalities"]
                .as_array()
                .map_or(true, |a| a.iter().any(|m| m == "text"))
        } else {
            true
        };
        models.push(Model {
            id: id.into(),
            name: name.into(),
            selectable: supported && ApiConfig::new(endpoint, id).is_ok(),
        });
    }
    if kind == Kind::Claude
        && v["has_more"] == true
        && v["last_id"].as_str().map_or(true, str::is_empty)
    {
        return Err("api_catalog_invalid_response");
    }
    let next = match kind {
        Kind::Claude => match v["has_more"].as_bool() {
            Some(true) => Some(
                v["last_id"]
                    .as_str()
                    .ok_or("api_catalog_invalid_response")?
                    .to_owned(),
            ),
            Some(false) => None,
            None => return Err("api_catalog_invalid_response"),
        },
        Kind::Google => match v.get("nextPageToken") {
            Some(t) => Some(t.as_str().ok_or("api_catalog_invalid_response")?.to_owned()),
            None => None,
        },
        _ => None,
    }
    .filter(|s| !s.is_empty());
    if next
        .as_ref()
        .is_some_and(|s| s.len() > 2048 || s.chars().any(char::is_control))
    {
        return Err("api_catalog_invalid_response");
    }
    Ok((models, next))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    const CLAUDE: &str = "https://api.anthropic.com/v1/messages";
    const GOOGLE: &str = "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions";
    const ROUTER: &str = "https://openrouter.ai/api/v1/chat/completions";
    const XAI: &str = "https://api.x.ai/v1/chat/completions";
    async fn server(
        replies: Vec<(&'static str, String)>,
    ) -> (Url, tokio::task::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!("http://{}/models", listener.local_addr().unwrap())).unwrap();
        let handle = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (status, body) in replies {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut data = Vec::new();
                let mut b = [0];
                while !data.ends_with(b"\r\n\r\n") {
                    socket.read_exact(&mut b).await.unwrap();
                    data.push(b[0]);
                    assert!(data.len() < 8192);
                }
                requests.push(String::from_utf8(data).unwrap());
                let response=format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\nLocation: http://127.0.0.1:1/leak\r\n\r\n{body}",body.len());
                let _ = socket.write_all(response.as_bytes()).await;
            }
            requests
        });
        (url, handle)
    }
    #[tokio::test]
    async fn all_provider_headers_and_get_only() {
        for (endpoint, body, header) in [
            (
                CLAUDE,
                json!({"data":[{"id":"claude-test"}],"has_more":false}),
                "x-api-key: synthetic-key",
            ),
            (
                GOOGLE,
                json!({"models":[{"name":"models/gemini-test","supportedGenerationMethods":["generateContent"]}]}),
                "x-goog-api-key: synthetic-key",
            ),
            (
                ROUTER,
                json!({"data":[{"id":"vendor/model","architecture":{"output_modalities":["text"]}}]}),
                "authorization: bearer synthetic-key",
            ),
            (
                XAI,
                json!({"data":[{"id":"grok-test"}]}),
                "authorization: bearer synthetic-key",
            ),
        ] {
            let (kind, official) = destination(endpoint).unwrap();
            assert_eq!(official.scheme(), "https");
            let (url, task) = server(vec![("200 OK", body.to_string())]).await;
            let models = fetch(kind, url, "synthetic-key", endpoint).await.unwrap();
            assert_eq!(models.len(), 1);
            assert!(models[0].selectable);
            let requests = task.await.unwrap();
            assert_eq!(requests.len(), 1);
            let wire = requests[0].to_lowercase();
            assert!(wire.starts_with("get /models"));
            assert!(wire.contains(header));
            assert!(!wire.lines().next().unwrap().contains("synthetic-key"));
            assert!(!wire.contains("content-length:"));
            assert!(!wire.contains("chat/completions"));
            if kind == Kind::Claude {
                assert!(wire.contains("anthropic-version: 2023-06-01"));
            }
        }
    }
    #[tokio::test]
    async fn pagination_is_complete_deduplicated_and_encoded() {
        for endpoint in [CLAUDE, GOOGLE] {
            let (kind, _) = destination(endpoint).unwrap();
            let replies = if kind == Kind::Claude {
                vec![
                    json!({"data":[{"id":"a"}],"has_more":true,"last_id":"a&secret=no"}),
                    json!({"data":[{"id":"a"},{"id":"b"}],"has_more":false}),
                ]
            } else {
                vec![
                    json!({"models":[{"name":"models/a"}],"nextPageToken":"a&secret=no"}),
                    json!({"models":[{"name":"models/b"}]}),
                ]
            };
            let (url, task) = server(
                replies
                    .into_iter()
                    .map(|v| ("200 OK", v.to_string()))
                    .collect(),
            )
            .await;
            assert_eq!(fetch(kind, url, "dummy", endpoint).await.unwrap().len(), 2);
            let requests = task.await.unwrap();
            assert_eq!(requests.len(), 2);
            assert!(requests[1].contains("a%26secret%3Dno"));
        }
    }
    #[tokio::test]
    async fn errors_redirects_and_malformed_responses_never_fallback() {
        for status in [
            "401 Unauthorized",
            "403 Forbidden",
            "429 Too Many Requests",
            "302 Found",
            "500 Internal Server Error",
            "200 OK",
        ] {
            let (url, task) = server(vec![(status, "sensitive provider error".into())]).await;
            let error = fetch(Kind::Xai, url, "dummy", XAI).await.err().unwrap();
            assert!(!error.contains("sensitive"));
            assert_eq!(task.await.unwrap().len(), 1);
        }
    }
    #[tokio::test]
    async fn repeated_cursor_and_large_response_fail_without_partial_list() {
        let reply = json!({"data":[{"id":"a"}],"has_more":true,"last_id":"a"}).to_string();
        let (url, task) = server(vec![("200 OK", reply.clone()), ("200 OK", reply)]).await;
        assert!(fetch(Kind::Claude, url, "dummy", CLAUDE).await.is_err());
        assert_eq!(task.await.unwrap().len(), 2);
        let (url, task) = server(vec![("200 OK", "x".repeat(8 * 1024 * 1024 + 1))]).await;
        assert_eq!(
            fetch(Kind::Xai, url, "dummy", XAI).await.err(),
            Some("api_catalog_limit")
        );
        task.await.unwrap();
    }
    #[test]
    fn unsupported_rows_visible_but_not_selectable_and_destinations_fixed() {
        let rows = json!({"models":[{"name":"models/embed","supportedGenerationMethods":["embedContent"]}]});
        assert!(
            !parse(Kind::Google, rows.to_string().as_bytes(), GOOGLE)
                .unwrap()
                .0[0]
                .selectable
        );
        let rows = json!({"data":[{"id":"openrouter/auto"},{"id":"vendor/image","architecture":{"output_modalities":["image"]}}]});
        assert!(parse(Kind::Router, rows.to_string().as_bytes(), ROUTER)
            .unwrap()
            .0
            .iter()
            .all(|m| !m.selectable));
        for endpoint in [
            "http://127.0.0.1/models",
            "https://api.x.ai/v1/chat/completions?key=secret",
            "https://evil.test/v1/chat/completions",
        ] {
            assert!(destination(endpoint).is_err());
        }
        assert!(parse(Kind::Xai, b"{\"data\":[{\"id\":12}]}", XAI).is_err());
    }
}
