//! Evaluate Mojang's `rules` (used on libraries and arguments) for the current
//! OS/arch, and expand `${...}` placeholders in argument templates.

use std::collections::HashMap;

use super::meta::{ArgEntry, Rule};

/// Mojang's OS name for the current platform.
pub fn os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

/// Mojang's OS arch string for the current platform.
fn os_arch() -> &'static str {
    if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86_64"
    }
}

/// Does a single rule match the current environment + feature flags?
fn rule_matches(rule: &Rule, features: &HashMap<String, bool>) -> bool {
    if let Some(os) = &rule.os {
        if let Some(name) = &os.name {
            if name != os_name() {
                return false;
            }
        }
        if let Some(arch) = &os.arch {
            if arch != os_arch() {
                return false;
            }
        }
        // OS version regex is ignored (rarely used; treating as match).
    }
    if let Some(req) = &rule.features {
        for (k, v) in req {
            if features.get(k).copied().unwrap_or(false) != *v {
                return false;
            }
        }
    }
    true
}

/// Standard rule resolution: no rules => allowed; otherwise the last matching
/// rule's action wins (default deny).
pub fn allowed(rules: &[Rule], features: &HashMap<String, bool>) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allow = false;
    for rule in rules {
        if rule_matches(rule, features) {
            allow = rule.action == "allow";
        }
    }
    allow
}

/// Expand a single argument template, substituting `${key}` from `vars`.
/// Unknown placeholders are left as-is.
pub fn expand(template: &str, vars: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find('}') {
            let key = &after[..end];
            match vars.get(key) {
                Some(val) => out.push_str(val),
                None => {
                    out.push_str("${");
                    out.push_str(key);
                    out.push('}');
                }
            }
            rest = &after[end + 1..];
        } else {
            out.push_str(&rest[start..]);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

/// Resolve an argument list (jvm or game) into concrete, substituted strings.
pub fn resolve_args(
    entries: &[ArgEntry],
    features: &HashMap<String, bool>,
    vars: &HashMap<String, String>,
) -> Vec<String> {
    let mut out = Vec::new();
    for entry in entries {
        match entry {
            ArgEntry::Literal(s) => out.push(expand(s, vars)),
            ArgEntry::Conditional { rules, value } => {
                if allowed(rules, features) {
                    // `value` is borrowed; clone its strings through expand.
                    match value {
                        super::meta::ArgValue::Single(s) => out.push(expand(s, vars)),
                        super::meta::ArgValue::Many(v) => {
                            for s in v {
                                out.push(expand(s, vars));
                            }
                        }
                    }
                }
            }
        }
    }
    out
}
