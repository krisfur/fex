use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::provider::{Package, SearchResult};
use crate::util::{escape_query, exec_command};

const FORMULA_API_URL: &str = "https://formulae.brew.sh/api/formula.json";
const CASK_API_URL: &str = "https://formulae.brew.sh/api/cask.json";

static HOMEBREW_INDEX: OnceLock<Result<Vec<HomebrewEntry>, String>> = OnceLock::new();

#[derive(Clone)]
struct HomebrewEntry {
    name: String,
    description: String,
    source: String,
    search_terms: Vec<String>,
}

#[derive(Deserialize)]
struct FormulaApiEntry {
    name: String,
    #[serde(default)]
    desc: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    oldnames: Vec<String>,
}

#[derive(Deserialize)]
struct CaskApiEntry {
    token: String,
    #[serde(default)]
    desc: Option<String>,
    #[serde(default)]
    old_tokens: Vec<String>,
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum MatchKind {
    Exact,
    StartsWith,
    Contains,
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct MatchRank {
    kind: MatchKind,
    term_len: usize,
    name_len: usize,
}

impl Ord for MatchRank {
    fn cmp(&self, other: &Self) -> Ordering {
        self.kind
            .cmp(&other.kind)
            .then_with(|| other.term_len.cmp(&self.term_len))
            .then_with(|| other.name_len.cmp(&self.name_len))
    }
}

impl PartialOrd for MatchRank {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MatchKind {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority().cmp(&other.priority())
    }
}

impl PartialOrd for MatchKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl MatchKind {
    fn priority(self) -> u8 {
        match self {
            MatchKind::Contains => 0,
            MatchKind::StartsWith => 1,
            MatchKind::Exact => 2,
        }
    }
}

pub fn is_initialized() -> bool {
    HOMEBREW_INDEX.get().is_some()
}

pub fn search(query: &str) -> SearchResult {
    if query.is_empty() {
        return SearchResult {
            packages: vec![],
            error: None,
        };
    }

    let installed = get_installed();

    match load_index() {
        Ok(index) => SearchResult {
            packages: search_index(index, query, &installed),
            error: None,
        },
        Err(_) => {
            let mut result = cli_search(query, &installed);
            let count = result.packages.len();
            result.error = Some(if count == 0 {
                "Homebrew index unavailable, using slower CLI search.".to_string()
            } else {
                format!(
                    "Homebrew index unavailable, using slower CLI search ({count} result{}).",
                    if count == 1 { "" } else { "s" }
                )
            });
            result
        }
    }
}

fn get_installed() -> HashSet<String> {
    let mut installed = HashSet::new();
    for output in [
        exec_command("brew list --formula 2>/dev/null"),
        exec_command("brew list --cask 2>/dev/null"),
    ] {
        for line in output.lines() {
            if !line.is_empty() {
                installed.insert(line.to_string());
            }
        }
    }
    installed
}

fn load_index() -> Result<&'static Vec<HomebrewEntry>, String> {
    HOMEBREW_INDEX
        .get_or_init(fetch_index)
        .as_ref()
        .map_err(|err| err.clone())
}

fn fetch_index() -> Result<Vec<HomebrewEntry>, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("fex/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|err| format!("failed to create HTTP client: {err}"))?;

    let formulae: Vec<FormulaApiEntry> = client
        .get(FORMULA_API_URL)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|err| format!("failed to fetch formula index: {err}"))?
        .json()
        .map_err(|err| format!("failed to parse formula index: {err}"))?;

    let casks: Vec<CaskApiEntry> = client
        .get(CASK_API_URL)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|err| format!("failed to fetch cask index: {err}"))?
        .json()
        .map_err(|err| format!("failed to parse cask index: {err}"))?;

    let mut entries = Vec::with_capacity(formulae.len() + casks.len());
    entries.extend(formulae.into_iter().map(|item| {
        let FormulaApiEntry {
            name,
            desc,
            aliases,
            oldnames,
        } = item;

        let search_terms = normalize_terms(
            std::iter::once(&name)
                .chain(aliases.iter())
                .chain(oldnames.iter()),
        );

        HomebrewEntry {
            name,
            description: desc.unwrap_or_default(),
            source: "formula".to_string(),
            search_terms,
        }
    }));
    entries.extend(casks.into_iter().map(|item| {
        let CaskApiEntry {
            token,
            desc,
            old_tokens,
        } = item;

        let search_terms = normalize_terms(std::iter::once(&token).chain(old_tokens.iter()));

        HomebrewEntry {
            name: token,
            description: desc.unwrap_or_default(),
            source: "cask".to_string(),
            search_terms,
        }
    }));

    Ok(entries)
}

fn normalize_terms<'a>(terms: impl IntoIterator<Item = &'a String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for term in terms {
        let term = term.to_lowercase();
        if !normalized.contains(&term) {
            normalized.push(term);
        }
    }
    normalized
}

fn search_index(index: &[HomebrewEntry], query: &str, installed: &HashSet<String>) -> Vec<Package> {
    let query_lower = query.to_lowercase();
    let mut matches: Vec<(MatchRank, &HomebrewEntry)> = index
        .iter()
        .filter_map(|entry| best_match(entry, &query_lower).map(|rank| (rank, entry)))
        .collect();

    matches.sort_by(|(a_rank, a_entry), (b_rank, b_entry)| {
        b_rank
            .cmp(a_rank)
            .then_with(|| a_entry.name.cmp(&b_entry.name))
    });

    matches
        .into_iter()
        .map(|(_, entry)| Package {
            name: entry.name.clone(),
            version: String::new(),
            description: entry.description.clone(),
            source: entry.source.clone(),
            installed: installed.contains(&entry.name),
        })
        .collect()
}

fn best_match(entry: &HomebrewEntry, query: &str) -> Option<MatchRank> {
    entry
        .search_terms
        .iter()
        .filter_map(|term| {
            if term == query {
                Some(MatchRank {
                    kind: MatchKind::Exact,
                    term_len: term.len(),
                    name_len: entry.name.len(),
                })
            } else if term.starts_with(query) {
                Some(MatchRank {
                    kind: MatchKind::StartsWith,
                    term_len: term.len(),
                    name_len: entry.name.len(),
                })
            } else if term.contains(query) {
                Some(MatchRank {
                    kind: MatchKind::Contains,
                    term_len: term.len(),
                    name_len: entry.name.len(),
                })
            } else {
                None
            }
        })
        .max()
}

fn cli_search(query: &str, installed: &HashSet<String>) -> SearchResult {
    let escaped = escape_query(query);
    let output = exec_command(&format!("brew search --desc '{escaped}' 2>/dev/null"));

    let mut packages = Vec::new();
    let mut current_source = "formula";

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        if line.contains("==> Formulae") {
            current_source = "formula";
            continue;
        }
        if line.contains("==> Casks") {
            current_source = "cask";
            continue;
        }
        if line.starts_with('=') || line.starts_with("No ") {
            continue;
        }

        let (name, description) = if let Some(colon) = line.find(": ") {
            (line[..colon].to_string(), line[colon + 2..].to_string())
        } else {
            (line.trim().to_string(), String::new())
        };

        if name.is_empty() {
            continue;
        }

        packages.push(Package {
            installed: installed.contains(&name),
            name,
            version: String::new(),
            description,
            source: current_source.to_string(),
        });
    }

    SearchResult {
        packages,
        error: None,
    }
}
