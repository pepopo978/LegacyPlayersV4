use crate::modules::live_data_processor::dto::Unit;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct PlayersInCombat {
    pub unit: Unit,
    pub percentage: u32,
}
