//! Personal accounts: a key its owner chose, kept on each of their devices. A file put on the
//! server without the key gets an owner token; its owner can then put it in an account
//! ([`Admin::Claim`](figcraft_collab::message::Admin::Claim)). The server keeps only the SHA-256 of
//! the key, in the file's `meta.json`. Every device with the key lists the account's files
//! ([`Admin::Account`](figcraft_collab::message::Admin::Account)) and manages them: the key opens a
//! hosting connection on such a file as its owner token does ([`Store::owns`]).

use figcraft_collab::message::{AccountFile, MAX_ACCOUNT, MIN_ACCOUNT_KEY};

use super::{AdminReply, META, Store, hash, same_secret, write_json};

/// What a key too short to be a password answers.
pub const SHORT_KEY: &str = "an account key has at least 16 characters";

/// A usable account key: long enough, and nothing in it that isn't printable.
fn usable(key: &str) -> bool {
    key.len() >= MIN_ACCOUNT_KEY && key.len() <= 512 && !key.contains(|c: char| c.is_control() || c.is_whitespace())
}

impl Store {
    /// Put `file` in the account whose key is `key` (its owner asked: the caller checked the owner
    /// token). Another account the file was in loses it.
    pub(super) fn claim(&mut self, file: &str, key: &str) -> AdminReply {
        if !usable(key) {
            return AdminReply::Failed { reason: SHORT_KEY.into() };
        }
        let path = self.dir(file).join(META);
        let entry = match self.entry(file) {
            Ok(e) => e,
            Err(reason) => return AdminReply::Failed { reason },
        };
        let account = Some(hash(key));
        if entry.meta.account == account {
            return AdminReply::Done;
        }
        let before = std::mem::replace(&mut entry.meta.account, account);
        if let Err(e) = write_json(&path, &entry.meta) {
            entry.meta.account = before;
            return AdminReply::Failed { reason: format!("couldn't keep the file's account: {e}") };
        }
        log::info!("event=claimed file={file}");
        AdminReply::Done
    }

    /// Key `key` is the account key of `file`.
    pub(super) fn in_account(&self, file: &str, key: &str) -> bool {
        if !usable(key) {
            return false;
        }
        let presented = hash(key);
        self.files.get(file).and_then(|e| e.meta.account.as_deref()).is_some_and(|h| same_secret(&presented, h))
    }

    /// The files of the account whose key is `key`, most recently changed first. A key no file
    /// is in lists none (which doesn't say whether the key was ever used).
    pub(super) fn account(&self, key: &str) -> AdminReply {
        if !usable(key) {
            return AdminReply::Failed { reason: SHORT_KEY.into() };
        }
        let presented = hash(key);
        let mut files: Vec<AccountFile> = self
            .files
            .iter()
            .filter(|(_, e)| e.meta.account.as_deref().is_some_and(|h| same_secret(&presented, h)))
            .map(|(id, e)| AccountFile {
                id: id.clone(),
                name: e.meta.name.clone(),
                created: e.meta.created,
                updated: e.meta.updated,
                expires: self.expires(&e.meta),
            })
            .collect();
        files.sort_by(|a, b| b.updated.cmp(&a.updated).then_with(|| a.name.cmp(&b.name)));
        files.truncate(MAX_ACCOUNT);
        AdminReply::Account(files)
    }
}
