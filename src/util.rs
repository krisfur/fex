use crate::provider::Package;

/// Escape shell special characters in a query string.
pub fn escape_query(query: &str) -> String {
    let mut escaped = String::with_capacity(query.len());
    for c in query.chars() {
        match c {
            '\'' | '"' | '\\' | '`' | '$' => {
                escaped.push('\\');
                escaped.push(c);
            }
            _ => escaped.push(c),
        }
    }
    escaped
}

/// Execute a shell command and return its stdout.
pub fn exec_command(cmd: &str) -> String {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

/// Execute a shell command and return (stdout, stderr, exit_code).
pub fn exec_command_full(cmd: &str) -> (String, String, i32) {
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
    {
        Ok(output) => (
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
            output.status.code().unwrap_or(-1),
        ),
        Err(_) => (String::new(), String::new(), -1),
    }
}

/// Check if a command exists on the system.
pub fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("sh")
        .args(["-c", &format!("command -v {cmd} > /dev/null 2>&1")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Sort packages by relevance to the query.
/// Priority: exact match > starts with > contains (shorter first) > alphabetical.
pub fn sort_by_relevance(packages: &mut Vec<Package>, query: &str) {
    let query_lower = query.to_lowercase();
    packages.sort_by(|a, b| {
        let a_lower = a.name.to_lowercase();
        let b_lower = b.name.to_lowercase();

        // Exact match gets highest priority
        let a_exact = a_lower == query_lower;
        let b_exact = b_lower == query_lower;
        if a_exact != b_exact {
            return b_exact.cmp(&a_exact);
        }

        // Starts with query gets next priority
        let a_starts = a_lower.starts_with(&query_lower);
        let b_starts = b_lower.starts_with(&query_lower);
        if a_starts != b_starts {
            return b_starts.cmp(&a_starts);
        }

        // Contains query (shorter names preferred)
        let a_contains = a_lower.contains(&query_lower);
        let b_contains = b_lower.contains(&query_lower);
        if a_contains != b_contains {
            return b_contains.cmp(&a_contains);
        }

        if a_contains && b_contains {
            return a.name.len().cmp(&b.name.len());
        }

        // Alphabetical fallback
        a_lower.cmp(&b_lower)
    });
}
