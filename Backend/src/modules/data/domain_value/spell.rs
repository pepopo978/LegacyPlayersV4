#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq)]
pub struct Spell {
    pub id: u32,
    pub expansion_id: u8,
    pub name: String,
    pub subtext: String,
    pub cost: u16,
    pub cost_in_percent: u16,
    pub power_type: u8,
    pub cast_time: u32,
    pub school_mask: u16,
    pub dispel_type: u8,
    pub range_max: u32,
    pub cooldown: u32,
    pub duration: i32,
    pub icon: u16,
    pub description: String,
    pub aura: String,
}
