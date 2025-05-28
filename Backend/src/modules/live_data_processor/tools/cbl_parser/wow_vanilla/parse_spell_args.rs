use crate::modules::data::tools::RetrieveSpell;
use crate::modules::data::Data;
use std::collections::HashMap;

pub fn parse_spell_args_periodic(
    cache: &mut HashMap<String, Option<u32>>,
    data: &Data,
    spell_name: &str,
) -> Option<u32> {
    if spell_name == "Unknown" {
        return None;
    }

    let mut lookup = |name: &str| -> Option<u32> {
        if let Some(spell_id) = cache.get(name) {
            *spell_id
        } else {
            let spell_id = data.get_spell_by_name(1, &name.to_string()).map(|spell| spell.id);
            cache.insert(name.to_string(), spell_id);
            spell_id
        }
    };

    let periodic_spell_name = format!("{} (dot)", spell_name);
    if let Some(id) = lookup(&periodic_spell_name) {
        return Some(id);
    }

    lookup(spell_name)
}

pub fn parse_spell_args(cache: &mut HashMap<String, Option<u32>>, data: &Data, spell_name: &str) -> Option<u32> {
    if spell_name == "Unknown" {
        return None;
    }

    let spell_name = spell_name.to_string();
    if let Some(spell_id) = cache.get(&spell_name) {
        return *spell_id;
    }

    let spell_id = data.get_spell_by_name(1, &spell_name).map(|spell| spell.id);
    cache.insert(spell_name, spell_id);
    spell_id
}