//! What the page keeps across reloads, in the browser's local storage: your name (Preferences ▸
//! Your name), so a reload or a later visit joins live sessions and comments as you; and your
//! personal account key with your online files and the server in Preferences, so the phone stays
//! in your account.

use figcraft_engine::Session;
use serde_json::json;

/// Put back the name the page kept, checked as one typed in Preferences: the page's
/// storage is anyone's to change.
pub fn restore_name(session: &mut Session, kept: Option<String>) {
    if let Some(name) = kept
        && let Err(e) = session.execute("pref.userName", &json!({"name": name}))
    {
        log::warn!("the kept name wasn't used: {e}");
    }
}

/// Your personal account key, online files and the server in Preferences as the page keeps them
/// (`None`: nothing to keep).
pub fn account(prefs: &figcraft_engine::Prefs) -> Option<String> {
    let usual = prefs.publish_server == figcraft_engine::live::DEFAULT_SERVER;
    if prefs.account_key.is_empty() && prefs.online_files.is_empty() && usual {
        return None;
    }
    let server = (!usual).then_some(prefs.publish_server.as_str());
    serde_json::to_string(&json!({"account_key": prefs.account_key, "online_files": prefs.online_files, "server": server})).ok()
}

/// Put back the account key and online files the page kept. They are read as the preferences
/// file is (the page's storage is anyone's to change): what can't be a key or a file is dropped.
pub fn restore_account(session: &mut Session, kept: Option<String>) {
    let Some(kept) = kept else { return };
    match serde_json::from_str::<figcraft_engine::Prefs>(&kept) {
        Ok(p) => {
            session.prefs.account_key = p.account_key;
            session.prefs.online_files = p.online_files;
        }
        Err(e) => log::warn!("the kept account wasn't used: {e}"),
    }
    // The server chosen in Preferences, checked as one typed there.
    let server = serde_json::from_str::<serde_json::Value>(&kept).ok().and_then(|v| v["server"].as_str().map(str::to_string));
    if let Some(url) = server
        && let Err(e) = session.execute("pref.publishServer", &json!({"url": url}))
    {
        log::warn!("the kept server wasn't used: {e}");
    }
}

#[cfg(test)]
mod tests {
    use figcraft_engine::Session;
    use serde_json::json;

    use super::{account, restore_account, restore_name};

    /// The account key and online files survive a reload; junk in the storage is dropped.
    #[test]
    fn the_account_survives_a_reload() {
        let mut before = Session::new();
        assert_eq!(account(&before.prefs), None, "nothing kept without an account");
        before.execute("collab.account", &json!({"create": true})).unwrap();
        let kept = account(&before.prefs);
        let mut after = Session::new();
        restore_account(&mut after, kept);
        assert_eq!(after.prefs.account_key, before.prefs.account_key);
        // The server typed in Preferences is kept too.
        before.execute("pref.publishServer", &json!({"url": "wss://my-pc.example.ts.net/"})).unwrap();
        let mut after = Session::new();
        restore_account(&mut after, account(&before.prefs));
        assert_eq!(after.prefs.publish_server, before.prefs.publish_server);
        let mut junk = Session::new();
        restore_account(&mut junk, Some(r#"{"account_key": "x", "online_files": [{"server": "nope"}]}"#.into()));
        assert!(junk.prefs.account_key.is_empty() && junk.prefs.online_files.is_empty());
        restore_account(&mut junk, Some("not json".into()));
        assert!(junk.prefs.account_key.is_empty());
    }

    /// What one page kept, the next one restores; junk in the storage is cleaned like typing.
    #[test]
    fn the_name_survives_a_reload() {
        let mut before = Session::new();
        before.execute("pref.userName", &json!({"name": "  Alice "})).unwrap();
        let kept = before.prefs.user_name.clone();
        let mut after = Session::new();
        restore_name(&mut after, Some(kept));
        assert_eq!(after.prefs.user_name, "Alice");
        let mut fresh = Session::new();
        restore_name(&mut fresh, None);
        assert_eq!(fresh.prefs.user_name, Session::new().prefs.user_name, "nothing kept: the default");
        restore_name(&mut fresh, Some(format!("Bob\u{7}{}", "x".repeat(500))));
        assert!(fresh.prefs.user_name.starts_with("Bobx") && fresh.prefs.user_name.chars().count() == 64);
    }
}
