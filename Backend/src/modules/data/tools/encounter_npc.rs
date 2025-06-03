use crate::modules::data::{domain_value::EncounterNpc, Data};

pub trait RetrieveEncounterNpc {
    fn get_encounter_npc(&self, id: u32) -> Option<EncounterNpc>;
    fn get_all_encounter_npcs(&self) -> Vec<EncounterNpc>;
    fn get_required_death_creature_ids(&self, encounter_id: u32) -> Vec<u64>;
    fn encounter_has_pivot(&self, encounter_id: u32) -> bool;
}

impl RetrieveEncounterNpc for Data {
    fn get_encounter_npc(&self, id: u32) -> Option<EncounterNpc> {
        self.encounter_npcs.get(&id).cloned()
    }

    fn get_all_encounter_npcs(&self) -> Vec<EncounterNpc> {
        self.encounter_npcs.iter().map(|(_, encounter_npc)| encounter_npc.clone()).collect()
    }
    
    fn get_required_death_creature_ids(&self, encounter_id: u32) -> Vec<u64> {
        self.encounter_npcs
            .iter()
            .filter_map(|(_id, encounter_npc)| {
                if encounter_npc.encounter_id == encounter_id && encounter_npc.requires_death {
                    println!("EncounterNpc with id {} requires death", encounter_npc.npc_id);
                    Some(0xF130000000000000 + (encounter_npc.npc_id as u64).rotate_left(24))
                } else {
                    None
                }
            })
            .collect()
    }

    fn encounter_has_pivot(&self, encounter_id: u32) -> bool {
        self.encounter_npcs.iter().any(|(_, encounter_npc)| encounter_npc.is_pivot && encounter_npc.encounter_id == encounter_id)
    }
}
