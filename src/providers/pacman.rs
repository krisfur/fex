use crate::provider::{Package, Provider, SearchResult};
use crate::util::{command_exists, escape_query, exec_command, sort_by_relevance};

pub struct PacmanProvider;

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
        }
    }
    if pkg.name.is_empty() { None } else { Some(pkg) }
}

fn field_value(line: &str, key: &str) -> Option<String> {
    if line.starts_with(key) {
        if let Some(colon) = line.find(':') {
            return Some(line[colon + 1..].trim().to_string());
        }
    }
    None
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
            // Parse: repo/name version [installed]
            let Some(slash) = line.find('/') else { continue };
            let source = line[..slash].to_string();
            let rest = &line[slash + 1..];
            let Some(space) = rest.find(' ') else { continue };
            let name = rest[..space].to_string();
            let after_name = &rest[space + 1..];
            let version = after_name
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

impl Provider for PacmanProvider {
    fn name(&self) -> &str {
        "pacman"
    }

    fn is_available(&self) -> bool {
        command_exists("pacman")
    }

    fn search(&self, query: &str) -> SearchResult {
        if query.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let escaped = escape_query(query);

        // Try exact match via pacman -Si
        let info_output = exec_command(&format!("pacman -Si '{escaped}' 2>/dev/null"));
        let mut exact_match: Option<Package> = None;
        if !info_output.is_empty() && !info_output.contains("error:") {
            if let Some(mut pkg) = parse_si_output(&info_output) {
                // Check installed
                let q_out = exec_command(&format!("pacman -Q '{escaped}' 2>/dev/null"));
                if !q_out.is_empty() && !q_out.contains("error:") {
                    pkg.installed = true;
                }
                exact_match = Some(pkg);
            }
        }

        let output = exec_command(&format!("pacman -Ss '{escaped}' 2>/dev/null"));
        if output.is_empty() && exact_match.is_none() {
            return SearchResult { packages: vec![], error: None };
        }

        let mut packages = parse_ss_output(&output);

        // Add exact match if not already present
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
        format!("sudo pacman -S {}", pkg.name)
    }
}
