use crate::modules::armory::Armory;
use crate::modules::data::Data;
use crate::modules::instance::dto::{InstanceFailure, InstanceViewerAttempt, InstanceViewerMeta, InstanceViewerParticipant};
use crate::modules::instance::tools::ExportInstance;
use crate::modules::instance::{GzippedResponse, Instance};
use crate::MainDb;
use flate2::write::GzEncoder;
use flate2::Compression;
use rocket::State;
use rocket_contrib::json::Json;
use std::io::Write;
use serde_json::{from_str, Value};

#[openapi(skip)]
#[get("/export/<instance_meta_id>/<event_type>/<_last_event_id>")]
pub fn get_instance_event_type(me: State<Instance>, instance_meta_id: u32, event_type: u8, _last_event_id: u32) -> Result<GzippedResponse, InstanceFailure> {
    let event_data = me.export_instance_event_type(instance_meta_id, event_type);

    if event_data.is_err() {
        return Err(event_data.err().unwrap());
    }

    let event_data_ref: &Vec<String> = &event_data?;

    let formatted_data: Vec<Value> = event_data_ref
        .iter()
        .filter_map(|s| from_str::<Value>(s).ok()) // Each `s` is a &String
        .collect();

    let serialized_data = serde_json::to_vec(&formatted_data).expect("Serialization failed");

    // Compress the JSON using gzip
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized_data).expect("Gzip compression failed");
    let compressed_data = encoder.finish().expect("Gzip finalization failed");

    Ok(GzippedResponse(compressed_data))
}

#[openapi]
#[get("/export/<instance_meta_id>")]
pub fn get_instance_meta(mut db_main: MainDb, me: State<Instance>, data: State<Data>, armory: State<Armory>, instance_meta_id: u32) -> Result<Json<InstanceViewerMeta>, InstanceFailure> {
    me.get_instance_meta(&mut *db_main, &data, &armory, instance_meta_id).map(Json)
}

#[openapi]
#[get("/export/participants/<instance_meta_id>")]
pub fn get_instance_participants(mut db_main: MainDb, me: State<Instance>, armory: State<Armory>, instance_meta_id: u32) -> Result<Json<Vec<InstanceViewerParticipant>>, InstanceFailure> {
    me.get_instance_participants(&mut *db_main, &armory, instance_meta_id).map(Json)
}

#[openapi]
#[get("/export/attempts/<instance_meta_id>")]
pub fn get_instance_attempts(me: State<Instance>, mut db_main: MainDb, instance_meta_id: u32) -> Result<Json<Vec<InstanceViewerAttempt>>, InstanceFailure> {
    me.get_instance_attempts(&mut (*db_main), instance_meta_id).map(Json)
}
