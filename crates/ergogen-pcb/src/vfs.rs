use std::sync::{Mutex, OnceLock};

use indexmap::IndexMap;

static VIRTUAL_FS: OnceLock<Mutex<IndexMap<String, String>>> = OnceLock::new();

fn vfs() -> &'static Mutex<IndexMap<String, String>> {
    VIRTUAL_FS.get_or_init(|| Mutex::new(IndexMap::new()))
}

fn normalize_key(raw: &str) -> String {
    let mut s = raw.replace('\\', "/");
    while let Some(rest) = s.strip_prefix("./") {
        s = rest.to_string();
    }
    while s.contains("//") {
        s = s.replace("//", "/");
    }
    s
}

pub(crate) fn set(map: IndexMap<String, String>) {
    let mut normalized = IndexMap::new();
    for (k, v) in map {
        normalized.insert(normalize_key(&k), v);
    }
    *vfs().lock().expect("vfs lock") = normalized;
}

pub(crate) fn clear() {
    vfs().lock().expect("vfs lock").clear();
}

pub(crate) fn contains(candidate: &str) -> bool {
    let key = normalize_key(candidate);
    let guard = vfs().lock().expect("vfs lock");
    if guard.contains_key(&key) {
        return true;
    }
    guard.keys().any(|k| k.ends_with(&key) || key.ends_with(k))
}

pub(crate) fn read(candidate: &str) -> Option<String> {
    let key = normalize_key(candidate);
    let guard = vfs().lock().expect("vfs lock");
    if let Some(v) = guard.get(&key) {
        return Some(v.clone());
    }
    // Fallback: suffix match for callers that resolve to absolute paths (or vice-versa).
    guard
        .iter()
        .filter(|(k, _)| k.ends_with(&key) || key.ends_with(k.as_str()))
        .max_by_key(|(k, _)| k.len())
        .map(|(_, v)| v.clone())
}
