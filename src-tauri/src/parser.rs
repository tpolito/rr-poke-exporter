use serde::Serialize;
use std::fs;

use crate::charmap::decode_gen3_string;
use crate::data;

const SECTION_SIZE: usize = 0x1000;
const SECTION_COUNT: usize = 14;
const SLOT_SIZE: usize = SECTION_SIZE * SECTION_COUNT;

const PARTY_OFFSET: usize = 0x0038;
const POKEMON_SIZE: usize = 100;

const NATURES: [&str; 25] = [
    "Hardy", "Lonely", "Brave", "Adamant", "Naughty",
    "Bold", "Docile", "Relaxed", "Impish", "Lax",
    "Timid", "Hasty", "Serious", "Jolly", "Naive",
    "Modest", "Mild", "Quiet", "Bashful", "Rash",
    "Calm", "Gentle", "Sassy", "Careful", "Quirky",
];

fn u16_le(data: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([data[off], data[off + 1]])
}

fn u32_le(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

#[derive(Debug, Serialize, Clone)]
pub struct Pokemon {
    pub nickname: String,
    pub species: String,
    pub level: u8,
    pub item: Option<String>,
    pub nature: String,
    pub ability: String,
    pub moves: Vec<String>,
    pub display_text: String,
}

struct Section {
    id: u16,
    save_index: u32,
    data: Vec<u8>,
}

fn parse_save_slot(raw: &[u8], slot_offset: usize) -> Vec<Section> {
    (0..SECTION_COUNT)
        .map(|i| {
            let start = slot_offset + i * SECTION_SIZE;
            let data = raw[start..start + SECTION_SIZE].to_vec();
            Section {
                id: u16_le(&data, 0xFF4),
                save_index: u32_le(&data, 0xFFC),
                data,
            }
        })
        .collect()
}

fn get_active_slot(raw: &[u8]) -> Vec<Section> {
    let a = parse_save_slot(raw, 0);
    let b = parse_save_slot(raw, SLOT_SIZE);
    if a[0].save_index >= b[0].save_index { a } else { b }
}

fn find_section(sections: &[Section], id: u16) -> Result<&[u8], String> {
    sections
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.data.as_slice())
        .ok_or_else(|| format!("Section {} not found", id))
}

/// Parse a single party Pokemon from raw bytes (100 bytes).
/// CFRU/Radical Red uses fixed substructure order and no XOR encryption:
///   Growth(32), Attacks(44), EVs(56), Misc(68) — each 12 bytes.
fn parse_pokemon(pkmn: &[u8]) -> Option<Pokemon> {
    let personality = u32_le(pkmn, 0);
    if personality == 0 {
        return None;
    }

    let nickname = decode_gen3_string(&pkmn[8..18]);
    let level = pkmn[84];
    let nature_index = (personality % 25) as usize;
    let nature = NATURES[nature_index].to_string();

    // Growth substructure at fixed offset 32: species(u16), item(u16)
    let species_id = u16_le(pkmn, 32);
    let item_id = u16_le(pkmn, 34);

    // Attacks substructure at fixed offset 44: move1-4(u16 each)
    let moves: Vec<String> = (0..4)
        .map(|i| u16_le(pkmn, 44 + i * 2))
        .filter(|&m| m != 0)
        .map(|m| data::move_name(m).to_string())
        .collect();

    // Misc substructure at fixed offset 68: iv_egg_ability(u32 at +4 = offset 72)
    let iv_word = u32_le(pkmn, 72);
    let ability_bit = (iv_word >> 31) & 1;

    let species = data::species_name(species_id).to_string();

    // Ability slot: bit 31 set = hidden (2), else personality even = primary (0), odd = secondary (1)
    let ability_slot = if ability_bit == 1 {
        2
    } else if personality % 2 == 0 {
        0
    } else {
        1
    };
    let ability = data::ability_name(&species, ability_slot);

    let item = if item_id != 0 {
        Some(data::item_name(item_id).to_string())
    } else {
        None
    };

    // Build display text
    let mut text = String::new();
    match &item {
        Some(item_name) => text.push_str(&format!("{} ({}) @ {}\n", nickname, species, item_name)),
        None => text.push_str(&format!("{} ({})\n", nickname, species)),
    }
    text.push_str(&format!("Level: {}\n", level));
    text.push_str(&format!("{} Nature\n", nature));
    text.push_str(&format!("Ability: {}\n", ability));
    for m in &moves {
        text.push_str(&format!("- {}\n", m));
    }
    let display_text = text.trim_end().to_string();

    Some(Pokemon {
        nickname,
        species,
        level,
        item,
        nature,
        ability,
        moves,
        display_text,
    })
}

pub fn parse_sav(path: &str) -> Result<Vec<Pokemon>, String> {
    let raw = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;

    if raw.len() < SLOT_SIZE * 2 {
        return Err("File too small to be a valid .sav".to_string());
    }

    let sections = get_active_slot(&raw);
    let sec1 = find_section(&sections, 1)?;
    let party_count = u32_le(sec1, 0x0034) as usize;

    let mut party = Vec::new();
    for i in 0..party_count.min(6) {
        let off = PARTY_OFFSET + i * POKEMON_SIZE;
        if off + POKEMON_SIZE > sec1.len() {
            break;
        }
        if let Some(mon) = parse_pokemon(&sec1[off..off + POKEMON_SIZE]) {
            party.push(mon);
        }
    }

    Ok(party)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SAV: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../1636 - Pokemon Fire Red (U)(Squirrels) (patched).sav"
    );

    #[test]
    fn test_parse_party_from_sav() {
        let party = parse_sav(TEST_SAV).expect("Failed to parse .sav file");

        let expected: Vec<(&str, &str, u8, Option<&str>, &str, &[&str])> = vec![
            ("2Kewl", "Tentacruel", 28, None, "Relaxed",
             &["Water Pulse", "Wring Out", "Supersonic", "Acid"]),
            ("Smell", "Skuntank", 28, None, "Modest",
             &["Bite", "Acid Spray", "Toxic", "Flamethrower"]),
            ("Mimi", "Pawmo", 28, None, "Jolly",
             &["Arm Thrust", "Nuzzle", "Dig", "Bite"]),
            ("Kaeman", "Arbok", 28, Some("Oran Berry"), "Jolly",
             &["Thunder Fang", "Poison Jab", "Sucker Punch", "Fire Fang"]),
            ("Sparky", "Luxio", 28, None, "Adamant",
             &["Thunder Fang", "Swagger", "Spark", "Bite"]),
            ("horny", "Cetoddle", 28, None, "Careful",
             &["Ice Shard", "Rest", "Take Down", "Flail"]),
        ];

        assert_eq!(party.len(), expected.len(), "Party size mismatch");

        for (i, (mon, (exp_nick, exp_species, exp_level, exp_item, exp_nature, exp_moves))) in
            party.iter().zip(expected.iter()).enumerate()
        {
            assert_eq!(mon.nickname, *exp_nick, "Pokemon {}: nickname mismatch", i);
            assert_eq!(mon.species, *exp_species, "Pokemon {}: species mismatch", i);
            assert_eq!(mon.level, *exp_level, "Pokemon {}: level mismatch", i);
            assert_eq!(
                mon.item.as_deref(),
                *exp_item,
                "Pokemon {}: item mismatch", i
            );
            assert_eq!(mon.nature, *exp_nature, "Pokemon {}: nature mismatch", i);
            let move_strs: Vec<&str> = mon.moves.iter().map(|s| s.as_str()).collect();
            assert_eq!(
                move_strs.as_slice(),
                *exp_moves,
                "Pokemon {}: moves mismatch", i
            );
        }
    }

    #[test]
    fn test_display_text_format() {
        let party = parse_sav(TEST_SAV).expect("Failed to parse .sav file");

        let expected_first = "\
2Kewl (Tentacruel)
Level: 28
Relaxed Nature
Ability: Clear Body
- Water Pulse
- Wring Out
- Supersonic
- Acid";
        assert_eq!(party[0].display_text, expected_first, "First pokemon display_text mismatch");

        let expected_kaeman = "\
Kaeman (Arbok) @ Oran Berry
Level: 28
Jolly Nature
Ability: Intimidate
- Thunder Fang
- Poison Jab
- Sucker Punch
- Fire Fang";
        assert_eq!(party[3].display_text, expected_kaeman, "Kaeman display_text mismatch");
    }
}
