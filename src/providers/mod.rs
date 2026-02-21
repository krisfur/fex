pub mod apk;
pub mod apt;
pub mod brew;
pub mod dnf;
pub mod flatpak;
pub mod nix;
pub mod pacman;
pub mod paru;
pub mod snap;
pub mod xbps;
pub mod yay;
pub mod zerobrew;
pub mod zypper;

use crate::provider::BoxedProvider;

/// Auto-detection priority order:
/// paru → yay → pacman → xbps → zerobrew → brew → dnf → apk → zypper → nix → apt → snap → flatpak
pub fn auto_detect_provider() -> Option<BoxedProvider> {
    let candidates: Vec<BoxedProvider> = vec![
        Box::new(paru::ParuProvider),
        Box::new(yay::YayProvider),
        Box::new(pacman::PacmanProvider),
        Box::new(xbps::XbpsProvider),
        Box::new(zerobrew::ZerobrewProvider),
        Box::new(brew::BrewProvider),
        Box::new(dnf::DnfProvider),
        Box::new(apk::ApkProvider),
        Box::new(zypper::ZypperProvider),
        Box::new(nix::NixProvider),
        Box::new(apt::AptProvider),
        Box::new(snap::SnapProvider),
        Box::new(flatpak::FlatpakProvider),
    ];
    candidates.into_iter().find(|p| p.is_available())
}

/// Create a provider by name.
pub fn create_provider(name: &str) -> Option<BoxedProvider> {
    match name {
        "paru" => Some(Box::new(paru::ParuProvider)),
        "yay" => Some(Box::new(yay::YayProvider)),
        "pacman" => Some(Box::new(pacman::PacmanProvider)),
        "xbps" => Some(Box::new(xbps::XbpsProvider)),
        "zerobrew" => Some(Box::new(zerobrew::ZerobrewProvider)),
        "brew" => Some(Box::new(brew::BrewProvider)),
        "dnf" => Some(Box::new(dnf::DnfProvider)),
        "apk" => Some(Box::new(apk::ApkProvider)),
        "zypper" => Some(Box::new(zypper::ZypperProvider)),
        "nix" => Some(Box::new(nix::NixProvider)),
        "apt" => Some(Box::new(apt::AptProvider)),
        "snap" => Some(Box::new(snap::SnapProvider)),
        "flatpak" => Some(Box::new(flatpak::FlatpakProvider)),
        _ => None,
    }
}

/// Returns a list of (name, provider) for every available provider.
pub fn get_available_providers() -> Vec<(&'static str, BoxedProvider)> {
    let candidates: Vec<(&'static str, BoxedProvider)> = vec![
        ("paru", Box::new(paru::ParuProvider)),
        ("yay", Box::new(yay::YayProvider)),
        ("pacman", Box::new(pacman::PacmanProvider)),
        ("xbps", Box::new(xbps::XbpsProvider)),
        ("zerobrew", Box::new(zerobrew::ZerobrewProvider)),
        ("brew", Box::new(brew::BrewProvider)),
        ("dnf", Box::new(dnf::DnfProvider)),
        ("apk", Box::new(apk::ApkProvider)),
        ("zypper", Box::new(zypper::ZypperProvider)),
        ("nix", Box::new(nix::NixProvider)),
        ("apt", Box::new(apt::AptProvider)),
        ("snap", Box::new(snap::SnapProvider)),
        ("flatpak", Box::new(flatpak::FlatpakProvider)),
    ];
    candidates.into_iter().filter(|(_, p)| p.is_available()).collect()
}
