use std::io::{Cursor, Write};
use crate::modules::account::guard::IsModerator;
use crate::modules::armory::Armory;
use crate::modules::instance::dto::{InstanceFailure, RankingCharacterMeta, RankingResult};
use crate::modules::instance::tools::{create_ranking_export, UnrankAttempt};
use crate::modules::instance::Instance;
use crate::MainDb;
use flate2::write::GzEncoder;
use flate2::Compression;
use okapi::openapi3::Responses;
use rocket::response::Responder;
use rocket::{Response, State};
use rocket::http::{ContentType, Status};
use rocket_contrib::json::Json;
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::response::OpenApiResponder;

#[derive(Debug)]
pub struct GzippedResponse(Vec<u8>);


impl<'r> Responder<'r> for GzippedResponse {
    fn respond_to(self, _: &rocket::Request<'_>) -> rocket::response::Result<'r> {
        let mut response = Response::build();
        response
            .header(ContentType::JSON)
            .raw_header("Content-Encoding", "gzip")
            .status(Status::Ok)
            .sized_body(Cursor::new(self.0));
        response.ok()
    }
}


impl<'r> OpenApiResponder<'r> for GzippedResponse {
    fn responses(gen: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        <Vec<u8> as OpenApiResponder>::responses(gen)
    }
}

#[openapi]
#[get("/ranking/dps")]
pub fn get_instance_ranking_dps(me: State<Instance>, armory: State<Armory>) -> GzippedResponse {
    let instance_metas = me.instance_metas.read().unwrap();
    let rankings = me.instance_rankings_dps.read().unwrap();

    let json_data = create_ranking_export(&instance_metas.1, &rankings.1, &armory, None, None);

    // Serialize the data to JSON
    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    // Compress the JSON using gzip
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    GzippedResponse(compressed_data)
}

#[openapi]
#[get("/ranking/dps/by_season/<season>")]
pub fn get_instance_ranking_dps_by_season(me: State<Instance>, armory: State<Armory>, season: u8) -> GzippedResponse {
    let instance_metas = me.instance_metas.read().unwrap();
    let rankings = me.instance_rankings_dps.read().unwrap();

    let json_data = create_ranking_export(&instance_metas.1, &rankings.1, &armory, Some(season), None);
    // Serialize the data to JSON
    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    // Compress the JSON using gzip
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    GzippedResponse(compressed_data)
}

#[openapi]
#[get("/ranking/dps/by_server/<server_id>/by_season/<season>")]
pub fn get_instance_ranking_dps_by_server_and_season(me: State<Instance>, armory: State<Armory>, server_id: u32, season: u8) -> GzippedResponse {
    let instance_metas = me.instance_metas.read().unwrap();
    let rankings = me.instance_rankings_dps.read().unwrap();

    let json_data = create_ranking_export(&instance_metas.1, &rankings.1, &armory, Some(season), Some(server_id));
    // Serialize the data to JSON
    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    // Compress the JSON using gzip
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    GzippedResponse(compressed_data)
}

#[openapi]
#[get("/ranking/hps")]
pub fn get_instance_ranking_hps(me: State<Instance>, armory: State<Armory>) -> GzippedResponse {
    let instance_metas = me.instance_metas.read().unwrap();
    let rankings = me.instance_rankings_hps.read().unwrap();

    let json_data = create_ranking_export(&instance_metas.1, &rankings.1, &armory, None, None);
    // Serialize the data to JSON
    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    // Compress the JSON using gzip
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    GzippedResponse(compressed_data)
}

#[openapi]
#[get("/ranking/hps/by_season/<season>")]
pub fn get_instance_ranking_hps_by_season(me: State<Instance>, armory: State<Armory>, season: u8) -> GzippedResponse {
    let instance_metas = me.instance_metas.read().unwrap();
    let rankings = me.instance_rankings_hps.read().unwrap();

    let json_data = create_ranking_export(&instance_metas.1, &rankings.1, &armory, Some(season), None);
    // Serialize the data to JSON
    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    // Compress the JSON using gzip
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    GzippedResponse(compressed_data)
}

#[openapi]
#[get("/ranking/hps/by_server/<server_id>/by_season/<season>")]
pub fn get_instance_ranking_hps_by_server_and_season(me: State<Instance>, armory: State<Armory>, server_id: u32, season: u8) -> GzippedResponse {
    let instance_metas = me.instance_metas.read().unwrap();
    let rankings = me.instance_rankings_hps.read().unwrap();

    let json_data = create_ranking_export(&instance_metas.1, &rankings.1, &armory, Some(season), Some(server_id));
    // Serialize the data to JSON
    let serialized_data = serde_json::to_vec(&json_data).expect("Serialization failed");

    // Compress the JSON using gzip
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    GzippedResponse(compressed_data)
}

#[openapi]
#[get("/ranking/tps")]
pub fn get_instance_ranking_tps(me: State<Instance>, armory: State<Armory>) -> Json<Vec<(u32, Vec<(u32, RankingCharacterMeta, Vec<RankingResult>)>)>> {
    let instance_metas = me.instance_metas.read().unwrap();
    let rankings = me.instance_rankings_tps.read().unwrap();
    Json(create_ranking_export(&instance_metas.1, &rankings.1, &armory, None, None))
}

#[openapi]
#[get("/ranking/tps/by_season/<season>")]
pub fn get_instance_ranking_tps_by_season(me: State<Instance>, armory: State<Armory>, season: u8) -> Json<Vec<(u32, Vec<(u32, RankingCharacterMeta, Vec<RankingResult>)>)>> {
    let instance_metas = me.instance_metas.read().unwrap();
    let rankings = me.instance_rankings_tps.read().unwrap();
    Json(create_ranking_export(&instance_metas.1, &rankings.1, &armory, Some(season), None))
}

#[openapi]
#[get("/ranking/tps/by_server/<server_id>/by_season/<season>")]
pub fn get_instance_ranking_tps_by_server_and_season(me: State<Instance>, armory: State<Armory>, server_id: u32, season: u8) -> Json<Vec<(u32, Vec<(u32, RankingCharacterMeta, Vec<RankingResult>)>)>> {
    let instance_metas = me.instance_metas.read().unwrap();
    let rankings = me.instance_rankings_tps.read().unwrap();
    Json(create_ranking_export(&instance_metas.1, &rankings.1, &armory, Some(season), Some(server_id)))
}

#[openapi]
#[delete("/ranking/unrank", data = "<data>")]
pub fn unrank_attempt(mut db_main: MainDb, me: State<Instance>, data: Json<u32>, _auth: IsModerator) -> Result<(), InstanceFailure> {
    me.unrank_attempt(&mut *db_main, data.into_inner())
}