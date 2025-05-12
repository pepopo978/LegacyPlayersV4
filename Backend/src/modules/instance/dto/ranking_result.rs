use serde::{Deserialize, Serialize};
use rocket_okapi::JsonSchema;
use crate::modules::instance::dto::RankingResultMeta;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RankingResult {
    pub id: u32,
    pub encounter_id: u32,
    pub server_id: u32,
    pub character_id: u32,
    pub character_name: String,
    pub hero_class_id: u8,
    pub instance_meta_id: u32,
    pub attempt_id: u32,
    pub amount: u32,
    pub duration: u64,
    pub difficulty_id: u8,
    pub character_spec: u8,
    pub season_index: u8,

    pub ranking_result: RankingResultMeta,
}