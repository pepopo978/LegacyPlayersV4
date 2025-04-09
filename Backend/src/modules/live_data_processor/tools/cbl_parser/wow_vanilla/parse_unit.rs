use crate::modules::data::Data;
use crate::modules::live_data_processor::dto::Unit;
use crate::modules::live_data_processor::tools::cbl_parser::wow_vanilla::hashed_unit_id::{get_hashed_player_unit_id, get_npc_unit_id};
use std::collections::HashMap;

pub fn parse_unit(cache: &mut HashMap<String, Unit>, data: &Data, unit_name: &str) -> Option<Unit> {
    if unit_name == "Unknown" {
        return None;
    }

    let unit_name = unit_name.to_string();
    if let Some(unit) = cache.get(&unit_name) {
        return Some(unit.clone());
    }

    let unit;
    if let Some(unit_id) = get_npc_unit_id(data, &unit_name) {
        let self_damage = unit_name.contains("self damage");

        unit = Unit { is_player: false, unit_id, is_self_damage: self_damage, is_mind_control:false };
    } else {
        // This indicates that something went terribly wrong during parsing
        if unit_name.contains("'s ") {
            return None;
        }
        let self_damage = unit_name.contains("self damage");
        unit = Unit {
            is_player: true,
            unit_id: get_hashed_player_unit_id(unit_name.as_str()),
            is_self_damage: self_damage,
            is_mind_control: !self_damage && unit_name.contains("(") && unit_name.contains(")"),
        }
    }

    cache.insert(unit_name, unit.clone());
    Some(unit)
}
