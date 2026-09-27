//! Fixed installation-owned cleanup receipts. No document-derived data belongs here.
use serde::{Deserialize, Serialize};
#[cfg(any(test, not(windows)))]
use std::fs;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
};
const MAX_GENERATION: u64 = 9_007_199_254_740_991;
pub const LEDGER: &str = "grammar-cleanup.json";
pub const LOCK: &str = "grammar-cleanup.lock";
type Result<T> = std::result::Result<T, &'static str>;
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Ticket {
    pub installation: String,
    pub slot: usize,
    pub generation: u64,
    pub token: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Slot {
    generation: u64,
    token: String,
    state: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    version: u32,
    installation: String,
    slots: [Slot; 4],
}
#[derive(Clone)]
pub struct Store {
    pub directory: PathBuf,
    pub installation: String,
}
pub fn valid_installation(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn valid_token(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|v| v.hyphenated().to_string() == value)
}
impl Store {
    fn lock(&self) -> Result<File> {
        #[cfg(not(windows))]
        return Err("cleanup_unavailable");
        #[cfg(windows)]
        {
            if !valid_installation(&self.installation) {
                return Err("cleanup_unavailable");
            }
            let mut options = OpenOptions::new();
            options.read(true).write(true);
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                options.share_mode(0);
            }
            // Two documents can reach the ledger within milliseconds. A short,
            // bounded wait keeps that from failing a cleanup step (which blocks
            // Deep until the user checks cleanup) without waiting indefinitely.
            for attempt in 0..40 {
                match options.open(self.directory.join(LOCK)) {
                    Ok(file) => return Ok(file),
                    Err(_) if attempt < 39 => std::thread::sleep(std::time::Duration::from_millis(25)),
                    Err(_) => break,
                }
            }
            Err("cleanup_busy")
        }
    }
    fn read(&self) -> Result<Ledger> {
        let mut bytes = Vec::new();
        File::open(self.directory.join(LEDGER))
            .map_err(|_| "cleanup_unavailable")?
            .take(4097)
            .read_to_end(&mut bytes)
            .map_err(|_| "cleanup_unavailable")?;
        if bytes.len() > 4096 {
            return Err("cleanup_unavailable");
        }
        let ledger: Ledger = serde_json::from_slice(&bytes).map_err(|_| "cleanup_unavailable")?;
        if ledger.version != 1
            || ledger.installation != self.installation
            || ledger.slots.iter().enumerate().any(|(i, s)| {
                !s.token.is_empty() && ledger.slots[..i].iter().any(|p| p.token == s.token)
            })
            || ledger.slots.iter().any(|s| {
                s.generation > MAX_GENERATION
                    || !matches!(
                        s.state.as_str(),
                        "FREE" | "RESERVED" | "PENDING" | "COMPLETE"
                    )
                    || (s.generation == 0 && (s.state != "FREE" || !s.token.is_empty()))
                    || (s.generation > 0 && !valid_token(&s.token))
            })
        {
            return Err("cleanup_unavailable");
        }
        Ok(ledger)
    }
    fn write(&self, ledger: &Ledger) -> Result<()> {
        let temporary = self.directory.join("grammar-cleanup.tmp");
        let bytes = serde_json::to_vec(ledger).map_err(|_| "cleanup_unavailable")?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "cleanup_unavailable")?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "cleanup_unavailable")?;
        drop(file);
        atomic_replace(&temporary, &self.directory.join(LEDGER)).map_err(|_| "cleanup_unavailable")
    }
    pub fn reserve(&self, token: &str) -> Result<Ticket> {
        if !valid_token(token) {
            return Err("cleanup_invalid_ticket");
        }
        let _lock = self.lock()?;
        let mut ledger = self.read()?;
        if ledger.slots.iter().any(|s| s.token == token) {
            return Err("cleanup_duplicate_token");
        }
        let index = ledger
            .slots
            .iter()
            .position(|s| s.state == "FREE" && s.generation < MAX_GENERATION)
            .ok_or("cleanup_slots_full")?;
        let slot = &mut ledger.slots[index];
        slot.generation += 1;
        slot.token = token.into();
        slot.state = "RESERVED".into();
        let ticket = Ticket {
            installation: self.installation.clone(),
            slot: index,
            generation: slot.generation,
            token: token.into(),
        };
        self.write(&ledger)?;
        Ok(ticket)
    }
    pub fn find(&self, token: &str) -> Result<Ticket> {
        if !valid_token(token) {
            return Err("cleanup_invalid_ticket");
        }
        let _lock = self.lock()?;
        let ledger = self.read()?;
        let slot = ledger
            .slots
            .iter()
            .position(|s| s.token == token && s.state != "FREE")
            .ok_or("cleanup_unknown")?;
        Ok(Ticket {
            installation: self.installation.clone(),
            slot,
            generation: ledger.slots[slot].generation,
            token: token.into(),
        })
    }
    fn change(&self, ticket: &Ticket, op: &str) -> Result<()> {
        if ticket.installation != self.installation
            || ticket.slot >= 4
            || ticket.generation == 0
            || ticket.generation > MAX_GENERATION
            || !valid_token(&ticket.token)
        {
            return Err("cleanup_invalid_ticket");
        }
        let _lock = self.lock()?;
        let mut ledger = self.read()?;
        let slot = &mut ledger.slots[ticket.slot];
        if slot.generation != ticket.generation || slot.token != ticket.token {
            return Err("cleanup_unknown");
        }
        let next = match (op, slot.state.as_str()) {
            ("start", "RESERVED") => "PENDING",
            ("complete", "PENDING") => "COMPLETE",
            ("query", "RESERVED") => "COMPLETE",
            ("query", "COMPLETE") => return Ok(()),
            ("ack", "COMPLETE") => "FREE",
            ("ack", "FREE") => return Ok(()),
            _ => return Err("cleanup_unknown"),
        };
        slot.state = next.into();
        self.write(&ledger)
    }
    pub fn start(&self, t: &Ticket) -> Result<()> {
        self.change(t, "start")
    }
    pub fn complete(&self, t: &Ticket) -> Result<()> {
        self.change(t, "complete")
    }
    pub fn query(&self, t: &Ticket) -> Result<()> {
        self.change(t, "query")
    }
    pub fn ack(&self, t: &Ticket) -> Result<()> {
        self.change(t, "ack")
    }
}
#[cfg(windows)]
fn atomic_replace(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
    }
    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 1 | 8) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
#[cfg(not(windows))]
fn atomic_replace(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    fs::rename(from, to)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    fn fixture() -> Store {
        let directory =
            std::env::temp_dir().join(format!("grammar-receipt-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&directory).unwrap();
        let store = Store {
            directory,
            installation: uuid::Uuid::new_v4().simple().to_string(),
        };
        fs::write(store.directory.join(LOCK), b"").unwrap();
        let ledger = Ledger {
            version: 1,
            installation: store.installation.clone(),
            slots: std::array::from_fn(|_| Slot {
                generation: 0,
                token: String::new(),
                state: "FREE".into(),
            }),
        };
        fs::write(
            store.directory.join(LEDGER),
            serde_json::to_vec(&ledger).unwrap(),
        )
        .unwrap();
        store
    }
    fn token() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    #[test]
    fn reserved_query_retires_before_late_admission_and_ack_is_exact_idempotent() {
        let store = fixture();
        let ticket = store.reserve(&token()).unwrap();
        assert_eq!(store.find(&ticket.token).unwrap(), ticket);
        store.query(&ticket).unwrap();
        assert!(store.start(&ticket).is_err());
        store.ack(&ticket).unwrap();
        store.ack(&ticket).unwrap();
        assert_eq!(store.find(&ticket.token), Err("cleanup_unknown"));
        assert_eq!(store.reserve(&ticket.token), Err("cleanup_duplicate_token"));
        let newer = store.reserve(&token()).unwrap();
        assert_eq!(newer.slot, ticket.slot);
        assert!(newer.generation > ticket.generation);
        assert!(store.query(&ticket).is_err());
        assert!(store.ack(&ticket).is_err());
        fs::remove_dir_all(&store.directory).unwrap();
    }
    #[test]
    fn pending_survives_new_store_and_only_checked_completion_is_queryable() {
        let store = fixture();
        let ticket = store.reserve(&token()).unwrap();
        store.start(&ticket).unwrap();
        let restarted = store.clone();
        assert!(restarted.query(&ticket).is_err());
        assert!(restarted.start(&ticket).is_err());
        assert!(restarted.ack(&ticket).is_err());
        store.complete(&ticket).unwrap();
        restarted.query(&ticket).unwrap();
        restarted.ack(&ticket).unwrap();
        fs::remove_dir_all(&store.directory).unwrap();
    }
    #[test]
    fn four_slots_bounded_and_corrupt_or_missing_receipts_never_reset() {
        let store = fixture();
        for _ in 0..4 {
            store.reserve(&token()).unwrap();
        }
        assert_eq!(store.reserve(&token()), Err("cleanup_slots_full"));
        fs::write(store.directory.join(LEDGER), b"broken").unwrap();
        assert_eq!(store.reserve(&token()), Err("cleanup_unavailable"));
        fs::remove_file(store.directory.join(LEDGER)).unwrap();
        assert!(store.reserve(&token()).is_err());
        assert!(!store.directory.join(LEDGER).exists());
        fs::remove_dir_all(&store.directory).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn exclusive_os_lock_fails_closed_without_wait_or_write() {
        let store = fixture();
        let lock = store.lock().unwrap();
        assert_eq!(store.reserve(&token()), Err("cleanup_busy"));
        drop(lock);
        assert!(store.reserve(&token()).is_ok());
        fs::remove_dir_all(&store.directory).unwrap();
    }
    #[test]
    fn wrong_identity_and_stale_temporary_fail_closed() {
        let store = fixture();
        let ticket = store.reserve(&token()).unwrap();
        let mut wrong = ticket.clone();
        wrong.installation = uuid::Uuid::new_v4().simple().to_string();
        assert!(store.query(&wrong).is_err());
        wrong = ticket.clone();
        wrong.generation += 1;
        assert!(store.query(&wrong).is_err());
        wrong = ticket.clone();
        wrong.token = token();
        assert!(store.query(&wrong).is_err());
        fs::write(store.directory.join("grammar-cleanup.tmp"), b"partial").unwrap();
        assert!(store.start(&ticket).is_err());
        assert_eq!(store.read().unwrap().slots[ticket.slot].state, "RESERVED");
        fs::remove_dir_all(&store.directory).unwrap();
    }
    #[test]
    fn duplicate_tokens_are_corrupt_not_a_matching_receipt() {
        let store = fixture();
        let ticket = store.reserve(&token()).unwrap();
        let mut ledger = store.read().unwrap();
        ledger.slots[1] = Slot {
            generation: 1,
            token: ticket.token.clone(),
            state: "COMPLETE".into(),
        };
        fs::write(
            store.directory.join(LEDGER),
            serde_json::to_vec(&ledger).unwrap(),
        )
        .unwrap();
        assert_eq!(store.query(&ticket), Err("cleanup_unavailable"));
        fs::remove_dir_all(&store.directory).unwrap();
    }
}
