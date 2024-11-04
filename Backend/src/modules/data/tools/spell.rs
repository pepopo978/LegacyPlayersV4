use crate::modules::data::{domain_value::Spell, Data};

pub trait RetrieveSpell {
    fn get_spell(&self, expansion_id: u8, spell_id: u32) -> Option<Spell>;
    fn get_spell_by_name(&self, expansion_id: u8, spell_name: &String) -> Option<Spell>;
}

impl RetrieveSpell for Data {
    fn get_spell(&self, expansion_id: u8, spell_id: u32) -> Option<Spell> {
        if expansion_id == 0 {
            return None;
        }

        self.spells.get(expansion_id as usize - 1).and_then(|map| map.get(&spell_id).cloned())
    }

    fn get_spell_by_name(&self, expansion_id: u8, spell_name: &String) -> Option<Spell> {
        if expansion_id == 0 {
            return None;
        }

        self.spells
            .get(expansion_id as usize - 1)
            .and_then(|map| map.iter().find(|(_, spell)| spell.name.contains(spell_name)))
            .map(|(_, spell)| spell.clone())
    }
}
