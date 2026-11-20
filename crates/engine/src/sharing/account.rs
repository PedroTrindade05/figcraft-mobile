//! Your personal account: one key, kept on each of your devices, that makes your online files the
//! same list everywhere.
//!
//! The key is a password you make once (`collab.account {create: true}`) and type or paste on
//! each device (`collab.account {key}`). Every file you put online is then put in the account on
//! its server ([`Admin::Claim`], with the file's owner token), and listing your online files asks
//! each server for the account's files too ([`Admin::Account`]): a file put online from the
//! computer shows on the phone, and the other way round. The server keeps only a hash of the key,
//! and the key manages the account's files as their owner tokens do, so a device that only has the
//! key opens them as their owner: it makes its own edit link the first time
//! ([`Admin::CreateLink`]). Like owner tokens, the key is kept with the preferences, is never
//! logged, and no command result carries it unless asked for (`reveal`).

use figcraft_collab::message::{Access, Admin, AdminReply, MIN_ACCOUNT_KEY};
use serde_json::{Value, json};

use super::online::{FileState, OnlineFile, plain_server, same_server};
use crate::cmd::CommandSpec;
use crate::live::{NetReply, NetRequest};
use crate::{Result, Session, bad, spec};

/// The characters a made key is written with (Crockford's base 32: no I, L, O or U).
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// How many characters a made key has (5 bits each: 160 random bits).
const MADE_LENGTH: usize = 32;

/// What a link this device makes for itself is called on the server.
const DEVICE_LINK: &str = "Account (this device)";

/// A key as typed: spaces and dashes go, letters are capitals, and the letters that read as digits
/// are those digits (O as 0, I and L as 1), so a key read off one screen and typed on another
/// matches. `None`: too short or long to be one, or with characters a key doesn't have.
pub fn normalize(key: &str) -> Option<String> {
    let mut out = String::new();
    for c in key.chars().filter(|c| !c.is_whitespace() && *c != '-') {
        let c = match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            c => c,
        };
        if !c.is_ascii_alphanumeric() {
            return None;
        }
        out.push(c);
    }
    (MIN_ACCOUNT_KEY..=128).contains(&out.len()).then_some(out)
}

/// A key in groups of four, easier to read and type (`7K2M-9QXT-…`).
pub fn grouped(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    chars.chunks(4).map(|c| c.iter().collect::<String>()).collect::<Vec<_>>().join("-")
}

/// A new key: [`MADE_LENGTH`] characters from the system's secure random numbers.
fn make() -> Result<String> {
    let mut bytes = [0u8; MADE_LENGTH];
    getrandom::fill(&mut bytes).map_err(|e| crate::EngineError::Other(format!("no secure random numbers: {e}")))?;
    Ok(bytes.iter().map(|b| char::from(ALPHABET[usize::from(b & 31)])).collect())
}

/// The edit link a device makes for itself joins with.
fn edit_link(server: &str, file: &str, token: &str) -> String {
    format!("figcraft://join?server={}&file={file}&token={token}", plain_server(server))
}

impl Session {
    /// The account key set on this device, if any.
    pub fn account_key(&self) -> Option<&str> {
        Some(self.prefs.account_key.as_str()).filter(|k| !k.is_empty())
    }

    /// Some recorded file isn't in the account yet: listing the online files again puts it there
    /// (the UI does so in the background after a file goes online).
    pub fn account_pending(&self) -> bool {
        self.account_key().is_some() && self.prefs.online_files.iter().any(|f| !f.in_account && f.state != FileState::Gone)
    }

    /// Bring this device's list and the account on `server` together (part of listing the online
    /// files): put the files recorded here in the account, add the account's files recorded on
    /// other devices, and make an edit link for each of those this device has none for yet.
    pub(super) fn account_sync(&mut self, server: &str) -> Result<()> {
        let Some(key) = self.account_key().map(str::to_string) else { return Ok(()) };
        // Put this device's files in the account (once each).
        let unclaimed: Vec<(String, String)> = self
            .prefs
            .online_files
            .iter()
            .filter(|f| same_server(&f.server, server) && !f.in_account && f.state != FileState::Gone)
            .map(|f| (f.file.clone(), f.owner.clone()))
            .collect();
        for (file, owner) in unclaimed {
            let done = owner == key || {
                let req = Admin::Claim { file: file.clone(), account: key.clone() };
                let (reply, _) = self.net(NetRequest::Hosting { server: server.to_string(), file: file.clone(), owner, req: Some(req) })?;
                match reply {
                    NetReply::Hosting(Ok((_, Some(AdminReply::Done)))) => true,
                    // A server older than accounts can't read the request: nothing more to do there.
                    NetReply::Hosting(Err(why)) if why.contains("couldn't be read") => return Ok(()),
                    _ => false,
                }
            };
            if done {
                self.account_mark(server, &file);
            }
        }
        // The account's files from the other devices.
        let req = NetRequest::Hosting {
            server: server.to_string(),
            file: String::new(),
            owner: String::new(),
            req: Some(Admin::Account { key: key.clone() }),
        };
        let (reply, _) = self.net(req)?;
        let NetReply::Hosting(Ok((_, Some(AdminReply::Account(files))))) = reply else { return Ok(()) };
        let mut changed = false;
        for f in files {
            if let Some(known) = self.prefs.online_files.iter_mut().find(|r| r.file == f.id && same_server(&r.server, server)) {
                changed |= !known.in_account;
                known.in_account = true;
                continue;
            }
            if self.prefs.online_files.len() >= super::online::MAX_ONLINE_FILES {
                break;
            }
            let record = OnlineFile {
                server: plain_server(server),
                file: f.id,
                name: f.name,
                owner: key.clone(),
                created: f.created,
                opened: 0,
                state: FileState::Online,
                expires: f.expires,
                in_account: true,
                ..OnlineFile::default()
            };
            if let Some(record) = record.clean() {
                self.prefs.online_files.push(record);
                changed = true;
            }
        }
        // An edit link of this device's own for each account file it has none for.
        let linkless: Vec<(String, String)> = self
            .prefs
            .online_files
            .iter()
            .filter(|f| same_server(&f.server, server) && f.edit.is_empty() && f.state != FileState::Gone)
            .map(|f| (f.file.clone(), f.owner.clone()))
            .collect();
        for (file, owner) in linkless {
            let req = Admin::CreateLink { file: file.clone(), access: Access::Edit, label: DEVICE_LINK.into() };
            let (reply, _) = self.net(NetRequest::Hosting { server: server.to_string(), file: file.clone(), owner, req: Some(req) })?;
            if let NetReply::Hosting(Ok((_, Some(AdminReply::Link(made))))) = reply
                && let Some(record) = self.prefs.online_files.iter_mut().find(|r| r.file == file && same_server(&r.server, server))
            {
                // The app's transport hands the new link over whole; a bare token is made into one.
                record.edit = if made.token.contains("://") { made.token } else { edit_link(server, &file, &made.token) };
                changed = true;
            }
        }
        if changed {
            self.live.unsaved = true;
        }
        Ok(())
    }

    /// Recorded file `file` on `server` is in the account now.
    fn account_mark(&mut self, server: &str, file: &str) {
        if let Some(f) = self.prefs.online_files.iter_mut().find(|f| f.file == file && same_server(&f.server, server)) {
            f.in_account = true;
            self.live.unsaved = true;
        }
    }
}

/// `collab.account`.
fn account(s: &mut Session, p: &Value) -> Result<Value> {
    const CMD: &str = "collab.account";
    let flag = |k: &str| p.get(k).and_then(Value::as_bool).unwrap_or(false);
    let new = match (p.get("key").and_then(Value::as_str), flag("create"), flag("clear")) {
        (Some(_), true, _) | (Some(_), _, true) | (None, true, true) => return Err(bad(CMD, "give one of `key`, `create` and `clear`")),
        (Some(typed), false, false) => Some(
            normalize(typed)
                .ok_or_else(|| bad(CMD, format!("a key has {MIN_ACCOUNT_KEY} to 128 letters and digits (dashes and spaces don't count)")))?,
        ),
        (None, true, false) => Some(make()?),
        (None, false, true) => {
            // Files known here only through the account can't be managed without its key.
            let old = std::mem::take(&mut s.prefs.account_key);
            if !old.is_empty() {
                s.prefs.online_files.retain(|f| f.owner != old.as_str());
                s.prefs.online_files.iter_mut().for_each(|f| f.in_account = false);
                s.live.unsaved = true;
            }
            None
        }
        (None, false, false) => None,
    };
    if let Some(key) = new.filter(|k| k.as_str() != s.prefs.account_key.as_str()) {
        s.prefs.account_key = crate::prefs::AccountKey(key);
        // Every file goes in the new account at the next listing.
        s.prefs.online_files.iter_mut().for_each(|f| f.in_account = false);
        s.live.unsaved = true;
    }
    let shown = (flag("create") || flag("reveal")).then(|| s.account_key().map(grouped)).flatten();
    Ok(json!({"set": s.account_key().is_some(), "key": shown, "pending": s.account_pending()}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![spec!(
        "collab.account",
        "Personal account key",
        "{key? (set this device's key: as made on another device; dashes, spaces and case don't matter), create? (true: make a new key and set it), clear? (true: take the key off this device; the files known here only through it leave the list), reveal? (true: say the key)} → {set (a key is set), key (in groups of four: only with create or reveal, else null), pending (some file of this device isn't in the account yet: `collab.files {refresh: true}` puts it there)}: one key on each of your devices makes your online files the same list everywhere (each file you put online goes in the account on its server, and listing your files brings in the ones from your other devices, which open as their owner). The key is a password: keep it to yourself",
        false,
        account
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typed_key_reads_as_made() {
        let made = make().unwrap();
        assert_eq!(made.len(), MADE_LENGTH);
        assert_eq!(normalize(&grouped(&made)).as_deref(), Some(made.as_str()));
        assert_eq!(normalize(" ab cd-ef gh-ij kl-mn op ").as_deref(), Some("ABCDEFGH1JK1MN0P"));
        for bad in ["short", "has a ! mark in it 1234", &"x".repeat(200), "ÁÁÁÁÁÁÁÁÁÁÁÁÁÁÁÁÁ"] {
            assert_eq!(normalize(bad), None, "{bad}");
        }
        assert_ne!(make().unwrap(), make().unwrap());
    }

    #[test]
    fn the_command_sets_shows_and_clears_the_key() {
        let mut s = Session::new();
        assert_eq!(account(&mut s, &json!({})).unwrap()["set"], false);
        let made = account(&mut s, &json!({"create": true})).unwrap();
        let shown = made["key"].as_str().unwrap().to_string();
        assert!(made["set"] == true && shown.contains('-'));
        assert_eq!(account(&mut s, &json!({})).unwrap()["key"], Value::Null, "not said unless asked");
        assert_eq!(account(&mut s, &json!({"reveal": true})).unwrap()["key"], shown.as_str());
        // The same key typed on another device, in lower case and without dashes.
        let mut other = Session::new();
        account(&mut other, &json!({"key": shown.replace('-', "").to_lowercase()})).unwrap();
        assert_eq!(other.prefs.account_key, s.prefs.account_key);
        assert!(account(&mut other, &json!({"key": "short"})).is_err());
        assert!(account(&mut other, &json!({"key": shown, "create": true})).is_err());
        assert_eq!(account(&mut s, &json!({"clear": true})).unwrap()["set"], false);
        assert!(s.prefs.account_key.is_empty());
    }
}
