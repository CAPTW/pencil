//! Browser-owned local-only native messaging endpoint. No document persistence or Provider calls.
use codex_pencil::instant_selection::{
    AnalysisMode, AnalysisOptions, AnalysisRequest, InstantSelectionEngine, SourceIdentity,
    ENGINE_ID, ENGINE_VERSION,
};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

const MAX_FRAME: usize = 128 * 1024;
const MAX_RESPONSE: usize = 256 * 1024;
const MAX_TEXT: usize = 8192;
const CONFIG_NAME: &str = "grammar-chromium-host.origin.json";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginConfig {
    origin: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    version: u32,
    op: String,
    id: String,
    epoch: String,
    revision: u64,
    text: String,
}

#[derive(Serialize)]
struct Suggestion {
    start: usize,
    end: usize,
    replacement: String,
    rule: String,
    message: String,
}

#[derive(Serialize)]
struct Response {
    id: String,
    epoch: String,
    revision: u64,
    suggestions: Vec<Suggestion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'static str>,
}

fn valid_origin(origin: &str) -> bool {
    origin
        .strip_prefix("chrome-extension://")
        .and_then(|value| value.strip_suffix('/'))
        .is_some_and(|id| id.len() == 32 && id.bytes().all(|c| (b'a'..=b'p').contains(&c)))
}

fn authorized_origin(origin: &str, config: &[u8]) -> bool {
    valid_origin(origin)
        && serde_json::from_slice::<OriginConfig>(config).is_ok_and(|value| value.origin == origin)
}

// Distinguish clean port disconnect from truncated headers/bodies. Allocate only after bounds check.
fn read_frame(reader: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut header = [0; 4];
    loop {
        match reader.read(&mut header[..1]) {
            Ok(0) => return Ok(None),
            Ok(_) => break,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    reader.read_exact(&mut header[1..])?;
    let length = u32::from_le_bytes(header) as usize;
    if length == 0 || length > MAX_FRAME {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid_frame"));
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(Some(body))
}

fn write_frame(writer: &mut impl Write, response: &Response) -> io::Result<()> {
    let body = serde_json::to_vec(response).map_err(io::Error::other)?;
    if body.len() > MAX_RESPONSE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "response_limit"));
    }
    writer.write_all(&(body.len() as u32).to_le_bytes())?;
    writer.write_all(&body)?;
    writer.flush()
}

fn analyze(body: &[u8]) -> Result<Response, &'static str> {
    let request: Request = serde_json::from_slice(body).map_err(|_| "invalid_request")?;
    if request.version != 1
        || request.id.is_empty()
        || request.id.len() > 128
        || request.epoch.is_empty()
        || request.epoch.len() > 128
        || request.text.encode_utf16().count() > MAX_TEXT
    {
        return Err("invalid_request");
    }
    let mut response = Response {
        id: request.id,
        epoch: request.epoch,
        revision: request.revision,
        suggestions: Vec::new(),
        error: None,
    };
    if request.op != "analyze" {
        response.error = Some(if request.op == "deep" {
            "deep_unavailable"
        } else {
            "unsupported_op"
        });
        return Ok(response);
    }
    let identity = SourceIdentity::from_source(
        &response.epoch,
        request.revision,
        0,
        0,
        AnalysisMode::Correction,
        ENGINE_ID,
        ENGINE_VERSION,
        &request.text,
    );
    let result = InstantSelectionEngine::new()
        .analyze(&AnalysisRequest {
            source: request.text,
            identity,
            mode: AnalysisMode::Correction,
            protected_spans: vec![],
            options: AnalysisOptions::default(),
        })
        .map_err(|_| "analysis_unavailable")?;
    response.suggestions = result
        .suggestions
        .into_iter()
        .map(|item| Suggestion {
            start: item.range.start_utf16,
            end: item.range.end_utf16,
            replacement: item.replacement,
            rule: item.rule_code,
            message: item.message_code,
        })
        .collect();
    Ok(response)
}

fn run(reader: &mut impl Read, writer: &mut impl Write) -> io::Result<()> {
    while let Some(body) = read_frame(reader)? {
        let response =
            analyze(&body).map_err(|code| io::Error::new(io::ErrorKind::InvalidData, code))?;
        write_frame(writer, &response)?;
    }
    Ok(())
}

fn main() {
    // Never print parser errors/raw input. Chromium owns the port/process lifetime; EOF releases it.
    let result = (|| -> Result<(), ()> {
        let origin = std::env::args().nth(1).ok_or(())?;
        let path = std::env::current_exe()
            .map_err(|_| ())?
            .with_file_name(CONFIG_NAME);
        let mut config = Vec::new();
        std::fs::File::open(path)
            .map_err(|_| ())?
            .take(4097)
            .read_to_end(&mut config)
            .map_err(|_| ())?;
        if config.len() > 4096 || !authorized_origin(&origin, &config) {
            return Err(());
        }
        run(&mut io::stdin().lock(), &mut io::stdout().lock()).map_err(|_| ())
    })();
    if result.is_err() {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(text: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"version":1,"op":"analyze","id":"a","epoch":"b","revision":7,"text":text})).unwrap()
    }
    #[test]
    fn framing_rejects_truncation_and_oversize_and_accepts_eof() {
        assert!(read_frame(&mut &[][..]).unwrap().is_none());
        for data in [
            vec![1],
            vec![1, 0, 0, 0],
            vec![0, 0, 0, 0],
            ((MAX_FRAME + 1) as u32).to_le_bytes().to_vec(),
        ] {
            assert!(read_frame(&mut data.as_slice()).is_err());
        }
    }
    #[test]
    fn exact_single_origin_and_strict_schema() {
        let origin = "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/";
        let config = serde_json::to_vec(&serde_json::json!({"origin":origin})).unwrap();
        assert!(authorized_origin(origin, &config));
        assert!(!authorized_origin(
            "chrome-extension://bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb/",
            &config
        ));
        assert!(!valid_origin(
            "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/extra"
        ));
        assert!(!authorized_origin(origin, br#"{"origin":"*"}"#));
        assert!(analyze(&request(&"x".repeat(MAX_TEXT + 1))).is_err());
        let mut value: serde_json::Value = serde_json::from_slice(&request("hello")).unwrap();
        value["command"] = "calc.exe".into();
        assert!(analyze(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    #[test]
    fn production_engine_offsets_and_streaming() {
        let input = request("😀 seperate");
        let result = analyze(&input).unwrap();
        assert_eq!(result.revision, 7);
        assert!(!result.suggestions.is_empty());
        for item in &result.suggestions {
            assert!(item.start >= 3 && item.end <= 15);
        }
        let mut framed = Vec::new();
        for _ in 0..2 {
            framed.extend_from_slice(&(input.len() as u32).to_le_bytes());
            framed.extend_from_slice(&input);
        }
        let mut output = Vec::new();
        run(&mut framed.as_slice(), &mut output).unwrap();
        let mut reader = output.as_slice();
        assert!(read_frame(&mut reader).unwrap().is_some());
        assert!(read_frame(&mut reader).unwrap().is_some());
        assert!(read_frame(&mut reader).unwrap().is_none());
    }
    #[test]
    fn deep_never_invokes_provider() {
        let mut value: serde_json::Value = serde_json::from_slice(&request("hello")).unwrap();
        value["op"] = "deep".into();
        assert_eq!(
            analyze(&serde_json::to_vec(&value).unwrap()).unwrap().error,
            Some("deep_unavailable")
        );
    }
}
