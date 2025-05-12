#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RankingResultMeta {
    #[serde(rename = "a")]
    pub instance_meta_id: u32,
    #[serde(rename = "b")]
    pub attempt_id: u32,
    #[serde(rename = "c")]
    pub amount: u32,
    #[serde(rename = "d")]
    pub duration: u64,
    #[serde(rename = "e")]
    pub difficulty_id: u8,
    #[serde(rename = "f")]
    pub character_spec: u8,
    #[serde(rename = "g")]
    pub season_index: u8
}
