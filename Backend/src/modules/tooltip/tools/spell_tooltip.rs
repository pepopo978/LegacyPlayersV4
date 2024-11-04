use crate::modules::{
    data::{
        tools::{RetrieveIcon, RetrieveLocalization, RetrievePowerType, RetrieveSpell},
        Data,
    },
    tooltip::{domain_value::SpellCost, dto::TooltipFailure, material::SpellTooltip, Tooltip},
};

pub trait RetrieveSpellTooltip {
    fn get_spell(&self, data: &Data, language_id: u8, expansion_id: u8, spell_id: u32) -> Result<SpellTooltip, TooltipFailure>;
}

impl RetrieveSpellTooltip for Tooltip {
    fn get_spell(&self, data: &Data, language_id: u8, expansion_id: u8, spell_id: u32) -> Result<SpellTooltip, TooltipFailure> {
        let spell_res = data.get_spell(expansion_id, spell_id);
        if spell_res.is_none() {
            return Err(TooltipFailure::InvalidInput);
        }
        let spell = spell_res.unwrap();
        let spell_cost = if spell.cost_in_percent > 0 {
            Some(SpellCost {
                cost: spell.cost_in_percent,
                cost_in_percent: true,
                power_type: data.get_power_type(spell.power_type + 1).and_then(|power_type| data.get_localization(language_id, power_type.localization_id)).unwrap().content,
            })
        } else if spell.cost > 0 {
            Some(SpellCost {
                cost: spell.cost,
                cost_in_percent: false,
                power_type: data.get_power_type(spell.power_type + 1).and_then(|power_type| data.get_localization(language_id, power_type.localization_id)).unwrap().content,
            })
        } else {
            None
        };

        Ok(SpellTooltip {
            name: spell.name,
            icon: data.get_icon(spell.icon).unwrap().name,
            subtext: spell.subtext,
            spell_cost,
            range: spell.range_max,
            description: spell.description,
        })
    }
}
