use crate::modules::account::guard::IsModerator;
use crate::modules::armory::Armory;
use crate::modules::instance::dto::InstanceFailure;
use crate::modules::instance::tools::{create_ranking_export, UnrankAttempt};
use crate::modules::instance::{GzippedResponse, Instance};
use crate::MainDb;
use flate2::write::GzEncoder;
use flate2::Compression;
use rocket::State;
use rocket_contrib::json::Json;
use std::io::Write;

#[openapi]
#[get("/ranking/dps")]
pub fn get_instance_ranking_dps(me: State<Instance>, armory: State<Armory>) -> Result<GzippedResponse, InstanceFailure> {
    let instance_metas = me.instance_metas.read().unwrap();
    let rankings = match me.instance_rankings_dps.try_read() {
        Ok(rankings) => rankings,
        Err(_) => return Err(InstanceFailure::RankingsUpdating),
    };

    let json_data = create_ranking_export(&instance_metas.1, &rankings.1, &armory, None, None);

    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    Ok(GzippedResponse(compressed_data))
}

#[openapi]
#[get("/ranking/dps/by_season/<season>")]
pub fn get_instance_ranking_dps_by_season(me: State<Instance>, armory: State<Armory>, season: u8) -> Result<GzippedResponse, InstanceFailure> {
    let instance_metas = me.instance_metas.read().unwrap();
    let rankings = match me.instance_rankings_dps.try_read() {
        Ok(rankings) => rankings,
        Err(_) => return Err(InstanceFailure::RankingsUpdating),
    };

    let json_data = create_ranking_export(&instance_metas.1, &rankings.1, &armory, Some(season), None);

    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    Ok(GzippedResponse(compressed_data))
}

#[openapi]
#[get("/ranking/dps/by_server/<server_id>/by_season/<season>")]
pub fn get_instance_ranking_dps_by_server_and_season(me: State<Instance>, armory: State<Armory>, server_id: u32, season: u8) -> Result<GzippedResponse, InstanceFailure> {
    let instance_metas = me.instance_metas.read().unwrap();
    let rankings = match me.instance_rankings_dps.try_read() {
        Ok(rankings) => rankings,
        Err(_) => return Err(InstanceFailure::RankingsUpdating),
    };

    let json_data = create_ranking_export(&instance_metas.1, &rankings.1, &armory, Some(season), Some(server_id));
    // Serialize the data to JSON
    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    // Compress the JSON using gzip
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    Ok(GzippedResponse(compressed_data))
}

#[openapi]
#[get("/ranking/hps")]
pub fn get_instance_ranking_hps(me: State<Instance>, armory: State<Armory>) -> Result<GzippedResponse, InstanceFailure> {
    let instance_metas = me.instance_metas.read().unwrap();
    let rankings = match me.instance_rankings_hps.try_read() {
        Ok(rankings) => rankings,
        Err(_) => return Err(InstanceFailure::RankingsUpdating),
    };

    let json_data = create_ranking_export(&instance_metas.1, &rankings.1, &armory, None, None);
    // Serialize the data to JSON
    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    // Compress the JSON using gzip
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    Ok(GzippedResponse(compressed_data))
}

#[openapi]
#[get("/ranking/hps/by_season/<season>")]
pub fn get_instance_ranking_hps_by_season(me: State<Instance>, armory: State<Armory>, season: u8) -> Result<GzippedResponse, InstanceFailure> {
    let instance_metas = me.instance_metas.read().unwrap();
    let rankings = match me.instance_rankings_hps.try_read() {
        Ok(rankings) => rankings,
        Err(_) => return Err(InstanceFailure::RankingsUpdating),
    };

    let json_data = create_ranking_export(&instance_metas.1, &rankings.1, &armory, Some(season), None);
    // Serialize the data to JSON
    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    // Compress the JSON using gzip
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    Ok(GzippedResponse(compressed_data))
}

#[openapi]
#[get("/ranking/hps/by_server/<server_id>/by_season/<season>")]
pub fn get_instance_ranking_hps_by_server_and_season(me: State<Instance>, armory: State<Armory>, server_id: u32, season: u8) -> Result<GzippedResponse, InstanceFailure> {
    let instance_metas = me.instance_metas.read().unwrap();
    let rankings = match me.instance_rankings_hps.try_read() {
        Ok(rankings) => rankings,
        Err(_) => return Err(InstanceFailure::RankingsUpdating),
    };

    let json_data = create_ranking_export(&instance_metas.1, &rankings.1, &armory, Some(season), Some(server_id));
    // Serialize the data to JSON
    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    // Compress the JSON using gzip
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