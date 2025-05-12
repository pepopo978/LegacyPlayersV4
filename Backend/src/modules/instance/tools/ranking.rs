use std::collections::HashMap;

use crate::modules::instance::domain_value::{InstanceMeta, PrivacyType};
use crate::modules::instance::dto::{RankingResult, RankingCharacterMeta, RankingResultMeta};

pub fn create_ranking_export(
    instance_metas: &HashMap<u32, InstanceMeta>,
    rankings: &Vec<RankingResult>
) -> Vec<(u32, Vec<(u32, RankingCharacterMeta, Vec<RankingResultMeta>)>)> {
    // First, organize rankings by encounter_id and character_id
    let mut organized_rankings: HashMap<u32, HashMap<u32, (RankingCharacterMeta, Vec<RankingResultMeta>)>> = HashMap::new();

    for ranking in rankings {
        // Create character metadata
        let character_meta = RankingCharacterMeta {
            server_id: ranking.server_id,
            hero_class_id: ranking.hero_class_id,
            name: ranking.character_name.clone(), // Would need to be populated from elsewhere
        };

        // Add to organized structure
        organized_rankings
            .entry(ranking.encounter_id)
            .or_insert_with(HashMap::new)
            .entry(ranking.character_id)
            .or_insert_with(|| (character_meta, Vec::new()))
            .1.push(ranking.ranking_result.clone());
    }

    // Process the organized structure to generate the same output format
    organized_rankings
        .iter()
        .filter_map(|(npc_id, char_rankings)| {
            let res_char_rankings: Vec<(u32, RankingCharacterMeta, Vec<RankingResultMeta>)> = char_rankings
                .iter()
                .filter_map(|(character_id, (character_meta, rankings))| {
                    let res_rankings: Vec<RankingResultMeta> = rankings
                        .iter()
                        .filter_map(|rr| {
                            let instance_meta = instance_metas.get(&rr.instance_meta_id)?;
                            if instance_meta.privacy_type == PrivacyType::Public {
                                Some(rr.clone())
                            } else {
                                None
                            }
                        })
                        .collect();

                    if res_rankings.is_empty() {
                        None
                    } else {
                        Some((*character_id, character_meta.clone(), res_rankings))
                    }
                })
                .collect();

            if res_char_rankings.is_empty() {
                None
            } else {
                Some((*npc_id, res_char_rankings))
            }
        })
        .collect()
}

fn helper_get_best_ranking(ranking: Vec<RankingResultMeta>) -> RankingResultMeta {
    ranking.iter().fold(
        RankingResultMeta {
            instance_meta_id: 0,
            attempt_id: 0,
            amount: 0,
            duration: 1,
            difficulty_id: 0,
            character_spec: 0,
            season_index: 0,
        },
        |best, ranking_result| {
            if (best.amount as f64 / best.duration as f64) < (ranking_result.amount as f64 / ranking_result.duration as f64) {
                return ranking_result.clone();
            }
            best
        },
    )
}
