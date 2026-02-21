use ratatui::style::Color;

use crate::provider::{Package, Provider, SearchResult};
use crate::util::{command_exists, escape_query, exec_command, sort_by_relevance};

pub struct NixProvider;

/// Strip "nixpkgs." prefix from attribute path.
fn extract_pkg_name(attr: &str) -> String {
    attr.strip_prefix("nixpkgs.").unwrap_or(attr).to_string()
}

/// Find the version in "name-version" by locating the last hyphen before a digit.
fn extract_version(name_version: &str) -> String {
    let bytes = name_version.as_bytes();
    let mut i = name_version.len();
    while i > 0 {
        i -= 1;
        if bytes[i] == b'-' && i + 1 < name_version.len() && bytes[i + 1].is_ascii_digit() {
            return name_version[i + 1..].to_string();
        }
    }
    String::new()
}

impl Provider for NixProvider {
    fn name(&self) -> &str {
        "nix"
    }

    fn is_available(&self) -> bool {
        command_exists("nix") || command_exists("nix-env")
    }

    fn search(&self, query: &str) -> SearchResult {
        if query.is_empty() {
            return SearchResult { packages: vec![], error: None };
        }

        let escaped = escape_query(query);
        // nix-env -qaP --description '.*query.*'
        let output = exec_command(&format!(
            "nix-env -qaP --description '.*{escaped}.*' 2>/dev/null"
        ));

        let mut packages = Vec::new();

        for line in output.lines() {
            if line.is_empty() {
                continue;
            }

            // Format: "nixpkgs.name    name-version    Description"
            let mut parts = line.splitn(3, |c: char| c == ' ' || c == '\t');
            let attr = match parts.next() {
                Some(a) if !a.is_empty() => a,
                _ => continue,
            };

            // Skip whitespace to find name-version
            let rest_raw = match parts.next() {
                Some(r) => r,
                None => continue,
            };
            let rest = rest_raw.trim_start();

            // Use full remaining as split on whitespace
            let mut tokens = rest.split_whitespace();
            let name_version = match tokens.next() {
                Some(nv) => nv,
                None => continue,
            };
            let description = tokens.collect::<Vec<_>>().join(" ");

            let name = extract_pkg_name(attr);
            if name.is_empty() || name_version.is_empty() {
                continue;
            }
            let version = extract_version(name_version);

            packages.push(Package {
                name,
                version,
                description,
                source: "nixpkgs".to_string(),
                installed: false,
            });
        }

        sort_by_relevance(&mut packages, query);
        SearchResult { packages, error: None }
    }

    fn install_command(&self, pkg: &Package) -> String {
        format!("nix-env -iA nixpkgs.{}", pkg.name)
    }

    fn source_color(&self, source: &str) -> Color {
        match source {
            "nixpkgs" => Color::Blue,
            "nixos" => Color::Cyan,
            _ => Color::Magenta,
        }
    }
}
