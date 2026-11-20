//! Personal accounts against a real server, at the protocol level: a file's owner puts it in an
//! account (`Admin::Claim`), the account's key lists it (`Admin::Account`) and manages it as the
//! owner token does, a wrong or short key learns nothing, and the account survives a restart.

use figcraft_collab::message::{Access, Admin, AdminReply, ClientMsg, OpenHosting, PROTOCOL, ServerMsg};

use crate::config::Config;
use crate::test_util::{Raw, TempDir, config, live, pump, same, session};

const KEY: &str = "my-personal-key-0123456789";

fn open(dir: &std::path::Path) -> Config {
    Config { open_publish: true, ..config(dir) }
}

/// A hosting connection on `file` with `owner` (empty: none named), or why it was refused.
fn hosting(addr: std::net::SocketAddr, file: &str, owner: &str) -> Result<Raw, String> {
    let mut raw = Raw::connect(addr);
    raw.send(&ClientMsg::OpenHosting(OpenHosting { protocol: PROTOCOL, client_version: "test".into(), file: file.into(), owner: owner.into() }));
    match raw.next() {
        ServerMsg::Hosting(_) => Ok(raw),
        other => Err(format!("{other:?}")),
    }
}

fn ask(raw: &mut Raw, req: Admin) -> AdminReply {
    raw.send(&ClientMsg::Admin(req));
    match raw.next() {
        ServerMsg::Admin(r) => *r,
        other => panic!("answered {other:?}"),
    }
}

fn listed(addr: std::net::SocketAddr, key: &str) -> Vec<String> {
    let mut raw = hosting(addr, "", "").unwrap();
    match ask(&mut raw, Admin::Account { key: key.into() }) {
        AdminReply::Account(files) => files.into_iter().map(|f| f.id).collect(),
        other => panic!("answered {other:?}"),
    }
}

/// Ada publishes a file through the server at `url`: its id and owner token.
fn publish(url: &str) -> (String, String) {
    let mut a = session("Ada");
    a.execute("pref.publishServer", &serde_json::json!({"url": url})).unwrap();
    a.execute("shape.rectangle", &serde_json::json!({"x": 10, "y": 10, "width": 40, "height": 30})).unwrap();
    a.execute("collab.share", &serde_json::json!({})).unwrap();
    pump(&mut [&mut a], "the share", |s| live(s[0]) && s[0].live().share.as_ref().is_some_and(|sh| sh.links.is_some()));
    let f = &a.prefs.online_files[0];
    (f.file.clone(), f.owner.clone())
}

#[test]
fn an_owner_puts_a_file_in_an_account_and_the_key_lists_and_manages_it() {
    let dir = TempDir::new("account");
    let cfg = open(dir.path());
    let mut server = crate::start(&cfg).unwrap();
    let addr = server.addr();
    let (file, owner) = publish(&format!("ws://{addr}/"));
    assert!(listed(addr, KEY).is_empty(), "nothing in the account yet");

    // Only the file's owner may put it in an account, with a key long enough to be a password.
    let mut stranger = hosting(addr, "", "").unwrap();
    assert!(matches!(ask(&mut stranger, Admin::Claim { file: file.clone(), account: KEY.into() }), AdminReply::Failed { .. }));
    let mut own = hosting(addr, &file, &owner).unwrap();
    assert!(
        matches!(ask(&mut own, Admin::Claim { file: file.clone(), account: "short".into() }), AdminReply::Failed { reason } if reason.contains("16"))
    );
    assert_eq!(ask(&mut own, Admin::Claim { file: file.clone(), account: KEY.into() }), AdminReply::Done);
    assert_eq!(ask(&mut own, Admin::Claim { file: file.clone(), account: KEY.into() }), AdminReply::Done, "again: nothing changes");
    let meta = std::fs::read_to_string(dir.path().join("files").join(&file).join("meta.json")).unwrap();
    assert!(meta.contains("\"account\"") && !meta.contains(KEY), "only the key's hash is kept: {meta}");

    // The key lists it; another key, a short one or junk lists nothing.
    assert_eq!(listed(addr, KEY), vec![file.clone()]);
    assert!(listed(addr, "another-key-0123456789").is_empty());
    let mut raw = hosting(addr, "", "").unwrap();
    assert!(matches!(ask(&mut raw, Admin::Account { key: "short".into() }), AdminReply::Failed { .. }));
    assert!(matches!(ask(&mut raw, Admin::Account { key: "x".repeat(100_000) }), AdminReply::Failed { .. }));

    // The key opens hosting on the file as its owner token does: new links, a rename, `Owned`.
    let mut by_key = hosting(addr, &file, KEY).unwrap();
    assert!(matches!(ask(&mut by_key, Admin::CreateLink { file: file.clone(), access: Access::Edit, label: "phone".into() }), AdminReply::Link(_)));
    assert_eq!(ask(&mut by_key, Admin::Rename { file: file.clone(), name: "Renamed".into() }), AdminReply::Done);
    assert!(
        matches!(ask(&mut by_key, Admin::Owned { files: vec![(file.clone(), KEY.into())] }), AdminReply::Owned(l) if l[0].as_ref().is_some_and(|f| f.name == "Renamed"))
    );
    assert!(hosting(addr, &file, "another-key-0123456789").is_err(), "a wrong key manages nothing");

    // The account is kept with the file: a restart keeps it.
    server.stop();
    let mut server = crate::start(&Config { listen: addr.to_string(), ..open(dir.path()) }).unwrap();
    let addr = server.addr();
    assert_eq!(listed(addr, KEY), vec![file.clone()]);

    // Put in another account, it leaves the first one.
    let mut own = hosting(addr, &file, &owner).unwrap();
    assert_eq!(ask(&mut own, Admin::Claim { file: file.clone(), account: "another-key-0123456789".into() }), AdminReply::Done);
    assert!(listed(addr, KEY).is_empty());
    assert_eq!(listed(addr, "another-key-0123456789"), [file]);
    server.stop();
}

fn ex(s: &mut figcraft_engine::Session, cmd: &str, p: serde_json::Value) -> serde_json::Value {
    s.execute(cmd, &p).unwrap_or_else(|e| panic!("{cmd}: {e}"))
}

/// The computer and the phone of one person, end to end through the apps' commands: the computer
/// makes a key and shares a file; the phone, with only the key and the server, lists the file,
/// opens it as its owner and edits it with the computer in it.
#[test]
fn a_file_put_online_on_one_device_opens_on_another_with_the_same_key() {
    let dir = TempDir::new("account-devices");
    let mut server = crate::start(&open(dir.path())).unwrap();
    let url = format!("ws://{}/", server.addr());
    let mut pc = session("Ada");
    ex(&mut pc, "pref.publishServer", serde_json::json!({"url": url}));
    let made = ex(&mut pc, "collab.account", serde_json::json!({"create": true}));
    let key = made["key"].as_str().unwrap().to_string();
    ex(&mut pc, "shape.rectangle", serde_json::json!({"x": 10, "y": 10, "width": 40, "height": 30}));
    ex(&mut pc, "collab.share", serde_json::json!({}));
    pump(&mut [&mut pc], "the share", |s| live(s[0]) && s[0].live().share.as_ref().is_some_and(|sh| sh.links.is_some()));
    let file = pc.prefs.online_files[0].file.clone();
    assert!(pc.account_pending(), "not in the account until the list is asked for");
    let list = ex(&mut pc, "collab.files", serde_json::json!({"refresh": true}));
    assert!(!pc.account_pending() && pc.prefs.online_files[0].in_account);
    assert!(!list.to_string().contains(&key.replace('-', "")), "the key is never in the list");

    // The phone: the key typed in lower case without dashes, and the server.
    let mut phone = session("Ada (phone)");
    ex(&mut phone, "pref.publishServer", serde_json::json!({"url": url}));
    ex(&mut phone, "collab.account", serde_json::json!({"key": key.replace('-', " ").to_lowercase()}));
    let list = ex(&mut phone, "collab.files", serde_json::json!({"refresh": true}));
    assert_eq!(list["files"][0]["file"], file.as_str(), "{list}");
    assert_eq!(list["files"][0]["state"], "online");
    assert!(!phone.prefs.online_files[0].edit.is_empty(), "the phone made its own edit link");
    ex(&mut phone, "collab.openFile", serde_json::json!({"file": file}));
    pump(&mut [&mut pc, &mut phone], "the phone in the file", |s| live(s[1]) && same(s[0].doc().unwrap(), s[1].doc().unwrap()));
    ex(&mut phone, "shape.ellipse", serde_json::json!({"x": 100, "y": 100, "width": 20, "height": 20}));
    pump(&mut [&mut pc, &mut phone], "the phone's edit on the computer", |s| same(s[0].doc().unwrap(), s[1].doc().unwrap()));
    // The phone manages the file with the key alone: a rename shows on the computer's list.
    ex(&mut phone, "collab.manageFile", serde_json::json!({"file": file, "action": "rename", "name": "Trip"}));
    assert_eq!(ex(&mut pc, "collab.files", serde_json::json!({"refresh": true}))["files"][0]["name"], "Trip");
    // Listing again makes no new link: the phone keeps the one it made.
    let edit = phone.prefs.online_files[0].edit.clone();
    ex(&mut phone, "collab.files", serde_json::json!({"refresh": true}));
    assert_eq!(phone.prefs.online_files[0].edit, edit);
    // Taking the key off the phone drops what it knew only through the account.
    ex(&mut phone, "collab.account", serde_json::json!({"clear": true}));
    assert!(phone.prefs.online_files.is_empty());
    server.stop();
}
