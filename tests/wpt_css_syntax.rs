//! WPT `css/css-syntax` parser-level test suite harness.
//!
//! Drives the MusKitty CSS parser through fixtures extracted from
//! upstream WPT `css/css-syntax/*.html` (`tests/data/wpt/*.json`). The
//! upstream tests assert through the CSSOM (`style.color`,
//! `cssRules.length`, `selectorText`); the portable essence is rule and
//! declaration survival, which is what is asserted here. The JSON
//! `note` fields document the exact mapping per file.
//!
//! Not ported (upstream needs features outside this crate):
//! - custom-property-rule-ambiguity stylesheets 3/4: need
//!   CSSNestedDeclarations materialization + `&`-selector handling.
//! - urange-parsing / unicode-range-selector: property-grammar level.
//! - missing-semicolon: a reftest with no script assertion.
//!
//! Run with:
//!   cargo test --test wpt_css_syntax -- --nocapture

use muskitty_css_parser::{parse_a_declaration, parse_a_stylesheet, parse_a_stylesheets_contents};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

struct Case {
    file: String,
    desc: String,
    run: Box<dyn Fn() -> Result<(), String>>,
}

/// Collect (name → count) for every declaration in a parsed stylesheet,
/// including declarations inside at-rules and qualified rules.
fn collect_declarations(rules: &[muskitty_css_parser::Rule], counts: &mut HashMap<String, usize>) {
    use muskitty_css_parser::Rule;
    for rule in rules {
        match rule {
            Rule::AtRule(at) => {
                if let Some(decls) = &at.declarations {
                    for d in decls {
                        *counts.entry(d.name.clone()).or_default() += 1;
                    }
                }
                if let Some(children) = &at.child_rules {
                    collect_declarations(children, counts);
                }
            }
            Rule::QualifiedRule(q) => {
                for d in &q.declarations {
                    *counts.entry(d.name.clone()).or_default() += 1;
                }
                collect_declarations(&q.child_rules, counts);
            }
            Rule::Declarations(decls) => {
                for d in decls {
                    *counts.entry(d.name.clone()).or_default() += 1;
                }
            }
        }
    }
}

fn build_cases(file: String, root: &Value) -> Vec<Case> {
    let source = root
        .get("source")
        .and_then(|v| v.as_str())
        .unwrap_or("?")
        .to_string();
    let cases = root
        .get("cases")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for c in &cases {
        let kind = c
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let desc = c
            .get("desc")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| format!("{source} {kind}"));
        let kind_clone = kind.clone();
        let c = c.clone();
        out.push(Case {
            file: file.clone(),
            desc,
            run: Box::new(move || match kind_clone.as_str() {
                "stylesheet-rule-count" => {
                    let input = c.get("input").and_then(|v| v.as_str()).unwrap_or("");
                    let want = c.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let got = parse_a_stylesheets_contents(input).len();
                    if got == want {
                        Ok(())
                    } else {
                        Err(format!("expected {want} rules, got {got}"))
                    }
                }
                "declaration-survives-at-rule" => {
                    let input = c.get("input").and_then(|v| v.as_str()).unwrap_or("");
                    let want_name = c.get("declaration").and_then(|v| v.as_str()).unwrap_or("");
                    let sheet = parse_a_stylesheet(input);
                    let mut counts = HashMap::new();
                    collect_declarations(&sheet.rules, &mut counts);
                    if counts.contains_key(want_name) {
                        Ok(())
                    } else {
                        Err(format!(
                            "declaration {want_name:?} did not survive; got {counts:?}"
                        ))
                    }
                }
                "declaration-name-count" => {
                    let input = c.get("input").and_then(|v| v.as_str()).unwrap_or("");
                    let want_name = c.get("declaration").and_then(|v| v.as_str()).unwrap_or("");
                    let want = c.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let sheet = parse_a_stylesheet(input);
                    let mut counts = HashMap::new();
                    collect_declarations(&sheet.rules, &mut counts);
                    let got = counts.get(want_name).copied().unwrap_or(0);
                    if got == want {
                        Ok(())
                    } else {
                        Err(format!(
                            "expected {want} {want_name:?} declaration(s), got {got}"
                        ))
                    }
                }
                "declaration-name-counts" => {
                    let input = c.get("input").and_then(|v| v.as_str()).unwrap_or("");
                    let sheet = parse_a_stylesheet(input);
                    let mut got: HashMap<String, usize> = HashMap::new();
                    collect_declarations(&sheet.rules, &mut got);
                    let want_json = c
                        .get("declaration_counts")
                        .and_then(|v| v.as_object())
                        .cloned()
                        .unwrap_or_default();
                    let mut problems = Vec::new();
                    for (name, want) in &want_json {
                        let want = want.as_u64().unwrap_or(0) as usize;
                        let have = got.get(name).copied().unwrap_or(0);
                        if have != want {
                            problems.push(format!("{name}: want {want}, got {have}"));
                        }
                    }
                    if problems.is_empty() {
                        Ok(())
                    } else {
                        Err(problems.join("; "))
                    }
                }
                "declaration-valid" => {
                    let input = c.get("input").and_then(|v| v.as_str()).unwrap_or("");
                    match parse_a_declaration(input) {
                        Some(_) => Ok(()),
                        None => Err("expected a valid declaration, got None".to_string()),
                    }
                }
                other => Err(format!("unknown case kind {other:?}")),
            }),
        });
    }
    out
}

#[test]
fn wpt_css_syntax_parser_suite() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("wpt");
    let mut entries: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read wpt dir {dir:?}: {e}"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    entries.sort();

    let mut total_pass = 0usize;
    let mut total_fail = 0usize;
    let mut per_file: Vec<(String, usize, usize)> = Vec::new();
    let mut failures: Vec<(String, String, String)> = Vec::new();

    for path in &entries {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let root: Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path:?}: {e}"));
        let cases = build_cases(name.clone(), &root);
        let mut file_pass = 0usize;
        let mut file_fail = 0usize;
        for case in &cases {
            match (case.run)() {
                Ok(()) => file_pass += 1,
                Err(detail) => {
                    file_fail += 1;
                    failures.push((case.file.clone(), case.desc.clone(), detail));
                }
            }
        }
        total_pass += file_pass;
        total_fail += file_fail;
        per_file.push((name, file_pass, file_fail));
    }

    eprintln!("\n═══════════════════════════════════════════════════════════════");
    eprintln!(" WPT css/css-syntax (parser level) — results");
    eprintln!("═══════════════════════════════════════════════════════════════");
    eprintln!(
        " {:<36} {:>8} {:>8} {:>8}",
        "fixture", "pass", "fail", "total"
    );
    eprintln!(" ─────────────────────────────────────────────────────────────────");
    for (name, p, f) in &per_file {
        eprintln!(" {:<36} {:>8} {:>8} {:>8}", name, p, f, p + f);
    }
    eprintln!(" ─────────────────────────────────────────────────────────────────");
    let total = total_pass + total_fail;
    let pct = if total == 0 {
        0.0
    } else {
        100.0 * total_pass as f64 / total as f64
    };
    eprintln!(
        " {:<36} {:>8} {:>8} {:>8}   ({:.1}%)",
        "TOTAL", total_pass, total_fail, total, pct
    );
    eprintln!("\n── failures ──");
    for (file, desc, detail) in &failures {
        eprintln!("\n[{file}] {desc}\n  {detail}");
    }
    eprintln!("═══════════════════════════════════════════════════════════════\n");

    assert!(
        total > 0,
        "no test cases were loaded — fixture data missing?"
    );
    eprintln!(
        "PASS RATE: {:.1}% ({}/{}) — informational; not asserting a hard threshold yet.",
        pct, total_pass, total
    );
}
