#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Unit {
    pub is_player: bool,
    pub is_self_damage: bool,
    pub is_mind_control: bool,
    pub unit_id: u64,
}

impl Default for Unit {
    fn default() -> Self {
        Unit { is_player: false, unit_id: 0, is_self_damage: false, is_mind_control: false }
    }
}
