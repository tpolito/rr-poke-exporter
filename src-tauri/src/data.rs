use std::collections::HashMap;
use std::sync::LazyLock;

const SPECIES_TXT: &str = include_str!("../data/Species.txt");
const MOVES_TXT: &str = include_str!("../data/Moves.txt");
const ITEMS_TXT: &str = include_str!("../data/Items.txt");
const ABILITIES_CSV: &str = include_str!("../data/species_abilities.csv");

/// Build a lookup vec from a 1-indexed text file (one name per line).
/// Prepends a dummy entry at index 0 so that vec[id] works directly.
fn build_lookup(text: &'static str) -> Vec<&'static str> {
    let mut v = vec![""];
    v.extend(text.lines().map(|l| l.trim()));
    v
}

/// Species names indexed by species ID. Index 0 = dummy, index 1 = Bulbasaur, etc.
pub static SPECIES: LazyLock<Vec<&'static str>> = LazyLock::new(|| build_lookup(SPECIES_TXT));

/// Move names indexed by move ID. Index 0 = dummy, index 1 = Pound, etc.
pub static MOVES: LazyLock<Vec<&'static str>> = LazyLock::new(|| build_lookup(MOVES_TXT));

/// Item names indexed by item ID. Index 0 = dummy, index 1 = Master Ball, etc.
pub static ITEMS: LazyLock<Vec<&'static str>> = LazyLock::new(|| build_lookup(ITEMS_TXT));

/// Map from species name (lowercase) to (primary, secondary, hidden) ability names.
pub static ABILITIES: LazyLock<HashMap<String, (String, String, String)>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for line in ABILITIES_CSV.lines().skip(1) {
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() >= 4 {
            map.insert(
                cols[0].trim().to_lowercase(),
                (
                    cols[1].trim().to_string(),
                    cols[2].trim().to_string(),
                    cols[3].trim().to_string(),
                ),
            );
        }
    }
    map
});

pub fn species_name(id: u16) -> &'static str {
    SPECIES.get(id as usize).copied().unwrap_or("???")
}

pub fn move_name(id: u16) -> &'static str {
    MOVES.get(id as usize).copied().unwrap_or("???")
}

pub fn item_name(id: u16) -> &'static str {
    ITEMS.get(id as usize).copied().unwrap_or("???")
}

/// Look up ability name given species name and ability slot (0=primary, 1=secondary, 2=hidden).
pub fn ability_name(species: &str, slot: u8) -> String {
    match ABILITIES.get(&species.to_lowercase()) {
        Some((primary, secondary, hidden)) => match slot {
            2 => hidden.clone(),
            1 => secondary.clone(),
            _ => primary.clone(),
        },
        None => "???".to_string(),
    }
}
