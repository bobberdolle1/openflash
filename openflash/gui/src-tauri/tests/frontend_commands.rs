//! Checks that the frontend and the backend agree on the command surface.
//!
//! Tauri resolves `invoke("name")` at runtime, so a frontend call to a command
//! that is not in `generate_handler!` fails only when a user clicks the button,
//! and a command that is implemented but not registered is silently
//! unreachable. Both had happened: the UI called `dump_nand` and
//! `add_network_device`, and `ai_compare_dumps`, `ai_search_keys` and
//! `ai_generate_report` were implemented but never registered.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Command names the frontend passes to `invoke`.
fn frontend_calls() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let source_dir = crate_dir().join("../src");

    let mut stack = vec![source_dir];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) => panic!("cannot list {}: {e}", dir.display()),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let is_source = path
                .extension()
                .is_some_and(|ext| ext == "ts" || ext == "tsx");
            if !is_source {
                continue;
            }
            names.extend(extract_invoke_names(&read(&path)));
        }
    }
    names
}

/// Pull the string literal out of every `invoke("name")` or
/// `invoke<T>("name")`.
fn extract_invoke_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = source;

    while let Some(position) = rest.find("invoke") {
        rest = &rest[position + "invoke".len()..];

        // Skip an optional type argument and any whitespace before the '('.
        let mut tail = rest.trim_start();
        if let Some(stripped) = tail.strip_prefix('<') {
            match stripped.find('>') {
                Some(end) => tail = stripped[end + 1..].trim_start(),
                None => continue,
            }
        }
        let Some(tail) = tail.strip_prefix('(') else {
            continue;
        };
        let tail = tail.trim_start();

        let quote = match tail.chars().next() {
            Some(c @ ('"' | '\'')) => c,
            // `invoke(someVariable)` cannot be checked statically.
            _ => continue,
        };
        if let Some(end) = tail[1..].find(quote) {
            names.push(tail[1..1 + end].to_string());
        }
    }
    names
}

/// Command names passed to `generate_handler!`.
fn registered_commands() -> BTreeSet<String> {
    let source = read(&crate_dir().join("src/lib.rs"));
    let mut names = BTreeSet::new();
    let mut rest = source.as_str();

    while let Some(position) = rest.find("command::") {
        rest = &rest[position + "command::".len()..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            names.insert(name);
        }
    }
    names
}

/// Command names defined with `#[tauri::command]`.
fn defined_commands() -> BTreeSet<String> {
    let source = read(&crate_dir().join("src/command.rs"));
    let mut names = BTreeSet::new();

    for block in source.split("#[tauri::command]").skip(1) {
        for line in block.lines() {
            let line = line.trim();
            if line.starts_with("#[") || line.is_empty() {
                continue;
            }
            let Some(rest) = line.strip_prefix("pub ") else {
                break;
            };
            let rest = rest.strip_prefix("async ").unwrap_or(rest);
            if let Some(rest) = rest.strip_prefix("fn ") {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    names.insert(name);
                }
            }
            break;
        }
    }
    names
}

#[test]
fn every_command_the_frontend_calls_is_registered() {
    let calls = frontend_calls();
    assert!(
        !calls.is_empty(),
        "found no invoke() calls at all; the scanner is probably looking in the wrong place"
    );

    let registered = registered_commands();
    let missing: Vec<_> = calls.difference(&registered).collect();

    assert!(
        missing.is_empty(),
        "the frontend calls commands that are not in generate_handler!, so they fail at \
         runtime: {missing:?}"
    );
}

#[test]
fn every_registered_command_exists() {
    let registered = registered_commands();
    let defined = defined_commands();
    let missing: Vec<_> = registered.difference(&defined).collect();

    assert!(
        missing.is_empty(),
        "generate_handler! names commands that command.rs does not define: {missing:?}"
    );
}

#[test]
fn every_implemented_command_is_reachable() {
    let defined = defined_commands();
    assert!(
        !defined.is_empty(),
        "found no #[tauri::command] functions; the scanner is probably broken"
    );

    let registered = registered_commands();
    let unreachable: Vec<_> = defined.difference(&registered).collect();

    assert!(
        unreachable.is_empty(),
        "these commands are implemented but absent from generate_handler!, so nothing can \
         ever call them: {unreachable:?}"
    );
}

#[test]
fn the_invoke_scanner_finds_both_call_spellings() {
    let source = r#"
        await invoke("plain_call");
        const data = await invoke<number[]>("typed_call", { a: 1 });
        await invoke('single_quoted');
        await invoke(dynamicName);
    "#;

    let found = extract_invoke_names(source);
    assert!(found.contains(&"plain_call".to_string()), "{found:?}");
    assert!(found.contains(&"typed_call".to_string()), "{found:?}");
    assert!(found.contains(&"single_quoted".to_string()), "{found:?}");
    assert_eq!(found.len(), 3, "a dynamic name must be skipped: {found:?}");
}
