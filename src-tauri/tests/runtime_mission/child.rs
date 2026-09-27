// Synthetic process fixture: finite lifetime/output, no network/accounts/user data.
use std::{
    env, fs,
    io::{self, Write},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

/// Minimal JSON string encoding; the fixture is compiled without crates.
fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let descendant = args.iter().any(|arg| arg == "--fixture-descendant");
    if !descendant {
        if let Ok(dir) = env::var("P01_CASE_DIR") {
            // Every root spawn and its exact argv (synthetic payloads only), so
            // tests can prove what the selected Provider received and that no
            // fallback or replay started another process.
            let dir = std::path::Path::new(&dir);
            let argv = args[1..].iter().map(|arg| json_string(arg)).collect::<Vec<_>>().join(",");
            fs::write(dir.join("argv.json"), format!("[{argv}]")).unwrap();
            let mut spawns = fs::OpenOptions::new().create(true).append(true).open(dir.join("spawns.log")).unwrap();
            writeln!(spawns, "{}", std::process::id()).unwrap();
        }
        if let Ok(dir) = env::var("P01_CASE_DIR") {
            let flags = format!(
                "{{\"no_session_persistence\":{},\"bare\":{},\"disallowed_all_tools\":{},\"disable_slash_commands\":{}}}",
                args.iter().any(|arg| arg == "--no-session-persistence"),
                args.iter().any(|arg| arg == "--bare"),
                args.windows(2).any(|pair| pair[0] == "--disallowedTools" && pair[1] == "*"),
                args.iter().any(|arg| arg == "--disable-slash-commands"),
            );
            fs::write(std::path::Path::new(&dir).join("flags.json"), flags).unwrap();
        }
    }
    // The built-app flow switches the behaviour between its steps through a
    // task-owned file; the other tests set P01_FIXTURE_MODE.
    let mode = env::var("P01_FIXTURE_MODE_FILE")
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|text| text.trim().to_string())
        .or_else(|| env::var("P01_FIXTURE_MODE").ok())
        .unwrap_or_else(|| "slow".into());
    if let Ok(dir) = env::var("P01_CASE_DIR") {
        fs::write(
            std::path::Path::new(&dir).join(if descendant {
                "descendant.pid"
            } else {
                "root.pid"
            }),
            std::process::id().to_string(),
        )
        .unwrap();
    }
    if descendant {
        thread::sleep(Duration::from_secs(4));
        return;
    }
    match mode.as_str() {
        "pipe-holder" | "early-descendant" => {
            let mut child = Command::new(env::current_exe().unwrap())
                .arg("--fixture-descendant")
                .stdin(Stdio::null())
                .spawn()
                .unwrap();
            if mode == "early-descendant" {
                let _ = child.wait();
            }
        }
        "cleanup-lock" => {
            let dir = std::path::PathBuf::from(env::var("P01_CASE_DIR").unwrap());
            fs::write(
                dir.join("owned-cwd.txt"),
                env::current_dir().unwrap().to_str().unwrap(),
            )
            .unwrap();
            let start = std::time::Instant::now();
            while !dir.join("release-child").exists() && start.elapsed() < Duration::from_secs(3) {
                thread::sleep(Duration::from_millis(5));
            }
        }
        "probe-hang" => thread::sleep(Duration::from_secs(12)),
        "invalid-utf8" => {
            let _ = io::stdout().write_all(&[255]);
        }
        "flood-slow" => {
            for _ in 0..40 {
                if io::stdout().write_all(&[b'x'; 8192]).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
        "dual-flood" => {
            let writer = thread::spawn(|| {
                let _ = io::stdout().write_all(&vec![b'x'; 2 * 1024 * 1024]);
            });
            let _ = io::stderr().write_all(&vec![b'x'; 2 * 1024 * 1024]);
            let _ = writer.join();
        }
        "stdout-flood" | "stderr-flood" => {
            let count: usize = env::var("P01_FIXTURE_BYTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1024 * 1024);
            let bytes = vec![b'x'; count.min(32 * 1024 * 1024)];
            if mode != "stderr-flood" {
                let _ = io::stdout().write_all(&bytes);
            }
            if mode != "stdout-flood" {
                let _ = io::stderr().write_all(&bytes);
            }
            // Keep the synthetic writer alive so the exact overflow cause is
            // observable before normal child-exit teardown races the reader.
            thread::sleep(Duration::from_millis(250));
        }
        "success" | "slow-success" => {
            if mode == "slow-success" {
                // Late enough for the local Instant draft to arrive first.
                thread::sleep(Duration::from_millis(2500));
            }
            println!("{{\"result\":\"{{\\\"replacement\\\":\\\"Synthetic.\\\",\\\"summary\\\":\\\"fixture\\\",\\\"edits\\\":[]}}\"}}");
        }
        _ => thread::sleep(Duration::from_secs(4)),
    }
}
