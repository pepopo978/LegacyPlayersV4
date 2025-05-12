use crate::modules::account::guard::IsModerator;
use crate::modules::instance::dto::{InstanceFailure, RankingResult, RankingResultMeta};
use crate::modules::instance::tools::{create_ranking_export, UnrankAttempt};
use crate::modules::instance::{GzippedResponse, Instance};
use crate::MainDb;
use flate2::write::GzEncoder;
use flate2::Compression;
use rocket::State;
use rocket_contrib::json::Json;
use std::io::Write;
use crate::params;
use crate::util::database::Select;

/// Fetches ranking results (damage or healing) based on optional season and server filters
///
/// # Arguments
/// * `db_main` - Database connection
/// * `table_name` - Either "ranking_results_damage" or "ranking_results_heal"
/// * `season` - Optional season filter
/// * `server_id` - Optional server filter
pub fn fetch_ranking_results(
    db_main: &mut MainDb,
    table_name: &str,
    season: Option<u8>,
    server_id: Option<u32>,
) -> Vec<RankingResult> {
    let mut query = format!(
        "SELECT id, encounter_id, server_id, character_id, character_name, hero_class_id, instance_meta_id,
         attempt_id, amount, duration, difficulty_id, character_spec, season_index
         FROM {}", table_name
    );

    // Build the where clause based on provided filters
    let mut conditions = Vec::new();

    if season.is_some() {
        conditions.push("season_index = :season");
    }

    if server_id.is_some() {
        conditions.push("server_id = :server_id");
    }

    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&conditions.join(" AND "));
    }

    // Execute the query with appropriate parameters
    if let (Some(s), Some(srv)) = (season, server_id) {
        db_main.select_wparams(
            &query,
            |mut row| create_ranking_result(&mut row),
            params!("season" => s, "server_id" => srv),
        )
    } else if let Some(s) = season {
        db_main.select_wparams(
            &query,
            |mut row| create_ranking_result(&mut row),
            params!("season" => s),
        )
    } else if let Some(srv) = server_id {
        db_main.select_wparams(
            &query,
            |mut row| create_ranking_result(&mut row),
            params!("server_id" => srv),
        )
    } else {
        db_main.select(
            &query,
            |mut row| create_ranking_result(&mut row),
        )
    }
}

// Helper function to create RankingResult from row data
// Helper function to create RankingResult from row data
fn create_ranking_result(row: &mut rocket_contrib::databases::mysql::Row) -> RankingResult
{
    // Read values once to avoid duplicate take() calls
    let id = row.take(0).unwrap();
    let encounter_id = row.take(1).unwrap();
    let server_id = row.take(2).unwrap();
    let character_id = row.take(3).unwrap();
    let character_name = row.take(4).unwrap();
    let hero_class_id = row.take(5).unwrap();
    let instance_meta_id = row.take(6).unwrap();
    let attempt_id = row.take(7).unwrap();
    let amount = row.take(8).unwrap();
    let duration = row.take(9).unwrap();
    let difficulty_id = row.take(10).unwrap();
    let character_spec = row.take(11).unwrap();
    let season_index = row.take(12).unwrap();

    RankingResult {
        id,
        encounter_id,
        server_id,
        character_id,
        character_name,
        hero_class_id,
        instance_meta_id,
        attempt_id,
        amount,
        duration,
        difficulty_id,
        character_spec,
        season_index,
        ranking_result: RankingResultMeta {
            instance_meta_id,
            attempt_id,
            amount,
            duration,
            difficulty_id,
            character_spec,
            season_index,
        },
    }
}

#[openapi]
#[get("/ranking/dps")]
pub fn get_instance_ranking_dps(mut db_main: MainDb, me: State<Instance>) -> Result<GzippedResponse, InstanceFailure> {
    let instance_metas = me.instance_metas.read().unwrap();

    let results = fetch_ranking_results(&mut db_main, "ranking_results_damage", None, None);

    let json_data = create_ranking_export(&instance_metas.1, &results);

    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    Ok(GzippedResponse(compressed_data))
}

#[openapi]
#[get("/ranking/dps/by_season/<season>")]
pub fn get_instance_ranking_dps_by_season(mut db_main: MainDb, me: State<Instance>, season: u8) -> Result<GzippedResponse, InstanceFailure> {
    let instance_metas = me.instance_metas.read().unwrap();

    let results = fetch_ranking_results(&mut db_main, "ranking_results_damage", Some(season), None);

    let json_data = create_ranking_export(&instance_metas.1, &results);

    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    Ok(GzippedResponse(compressed_data))
}

#[openapi]
#[get("/ranking/dps/by_server/<server_id>/by_season/<season>")]
pub fn get_instance_ranking_dps_by_server_and_season(mut db_main: MainDb, me: State<Instance>, server_id: u32, season: u8) -> Result<GzippedResponse, InstanceFailure> {
    let instance_metas = me.instance_metas.read().unwrap();

    let results = fetch_ranking_results(&mut db_main, "ranking_results_damage", Some(season), Some(server_id));

    let json_data = create_ranking_export(&instance_metas.1, &results);

    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    Ok(GzippedResponse(compressed_data))
}

#[openapi]
#[get("/ranking/hps")]
pub fn get_instance_ranking_hps(mut db_main: MainDb, me: State<Instance>) -> Result<GzippedResponse, InstanceFailure> {
    let instance_metas = me.instance_metas.read().unwrap();

    let results = fetch_ranking_results(&mut db_main, "ranking_results_heal", None, None);

    let json_data = create_ranking_export(&instance_metas.1, &results);

    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    Ok(GzippedResponse(compressed_data))
}

#[openapi]
#[get("/ranking/hps/by_season/<season>")]
pub fn get_instance_ranking_hps_by_season(mut db_main: MainDb, me: State<Instance>, season: u8) -> Result<GzippedResponse, InstanceFailure> {
    let instance_metas = me.instance_metas.read().unwrap();

    let results = fetch_ranking_results(&mut db_main, "ranking_results_heal", Some(season), None);

    let json_data = create_ranking_export(&instance_metas.1, &results);

    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    Ok(GzippedResponse(compressed_data))
}

#[openapi]
#[get("/ranking/hps/by_server/<server_id>/by_season/<season>")]
pub fn get_instance_ranking_hps_by_server_and_season(mut db_main: MainDb, me: State<Instance>, server_id: u32, season: u8) -> Result<GzippedResponse, InstanceFailure> {
    let instance_metas = me.instance_metas.read().unwrap();

    let results = fetch_ranking_results(&mut db_main, "ranking_results_heal", Some(season), Some(server_id));

    let json_data = create_ranking_export(&instance_metas.1, &results);

    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    Ok(GzippedResponse(compressed_data))
}

#[openapi]
#[delete("/ranking/unrank", data = "<data>")]
pub fn unrank_attempt(mut db_main: MainDb, me: State<Instance>, data: Json<u32>, _auth: IsModerator) -> Result<(), InstanceFailure> {
    me.unrank_attempt(&mut *db_main, data.into_inner())
}