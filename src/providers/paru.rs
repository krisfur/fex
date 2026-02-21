use crate::provider::{Package, Provider, SearchResult};
use crate::util::{command_exists, escape_query, exec_command_full, sort_by_relevance};

pub struct ParuProvider;

fn field_value(line: &str, key: &str) -> Option<String> {
    if line.starts_with(key) {
        if let Some(colon) = line.find(':') {
            return Some(line[colon + 1..].trim().to_string());
        }
    }
    None
}

fn parse_si_output(output: &str) -> Option<Package> {
    let mut pkg = Package {
        name: String::new(),
        version: String::new(),
        description: String::new(),
        source: String::new(),
        installed: false,
    };
    for line in output.lines() {
        if let Some(val) = field_value(line, "Repository") {
            pkg.source = val;
        } else if let Some(val) = field_value(line, "Name") {
            pkg.name = val;
        } else if let Some(val) = field_value(line, "Version") {
            pkg.version = val;
        } else if let Some(val) = field_value(line, "Description") {
            pkg.description = val;
        } else if line.contains("Install Reason") || line.contains("Installed Size") {
            pkg.installed = true;
        }
    }
    if pkg.name.is_empty() { None } else { Some(pkg) }
}

fn parse_ss_output(output: &str) -> Vec<Package> {
    let mut packages = Vec::new();
    let mut current: Option<Package> = None;

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        if !line.starts_with(' ') && !line.starts_with('\t') {
            if let Some(pkg) = current.take() {
                packages.push(pkg);
            }
            let Some(slash) = line.find('/') else { continue };
            let source = line[..slash].to_string();
            let rest = &line[slash + 1..];
            let Some(space) = rest.find(' ') else { continue };
            let name = rest[..space].to_string();
            let version = rest[space + 1..]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            let installed = line.contains("[installed]") || line.contains("[Installed]");
            current = Some(Package { name, version, description: String::new(), source, installed });
        } else if let Some(ref mut pkg) = current {
            let desc = line.trim_start();
            if !desc.is_empty() {
                pkg.description = desc.to_string();
            }
        }
    }
    if let Some(pkg) = current {
        packages.push(pkg);
    }
    packages
}

impl Provider for ParuProvider {
    fn name(&self) -> &str {
        "paru"
    }

    fn is_available(&self) -> bool {
        command_exists("paru")
    }

    fn search(&self, query: &str) -> SearchResult {
        if query.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let escaped = escape_query(query);

        // Try exact match via paru -Si
        let (info_stdout, _, info_exit) =
            exec_command_full(&format!("paru -Si '{escaped}' 2>/dev/null"));
        let mut exact_match: Option<Package> = None;
        if info_exit == 0 && !info_stdout.is_empty() {
            exact_match = parse_si_output(&info_stdout);
        }

        // Main search (capture stderr for AUR error messages)
        let (stdout, stderr, _) = exec_command_full(&format!("paru -Ss '{escaped}'"));

        if stderr.contains("Query arg too small")
            || stderr.contains("Too many package results")
            || stdout.contains("Query arg too small")
            || stdout.contains("Too many package results")
        {
            return SearchResult {
                packages: vec![],
                error: Some("Too many results! Try a more specific search.".to_string()),
            };
        }

        let mut packages = parse_ss_output(&stdout);

        if let Some(em) = exact_match {
            let already = packages.iter().any(|p| p.name == em.name && p.source == em.source);
            if !already {
                packages.insert(0, em);
            }
        }

        sort_by_relevance(&mut packages, query);
        SearchResult { packages, error: None }
    }

    fn install_command(&self, pkg: &Package) -> String {
        format!("paru -S {}", pkg.name)
    }
}
