use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, RwLock};
use sha2::{Sha256, Digest};

use crate::modules::armory::tools::{GetCharacter};
use crate::modules::armory::util::talent_tree::get_talent_tree;
use crate::modules::armory::Armory;
use crate::modules::data::Data;
use crate::modules::instance::domain_value::{InstanceAttempt, InstanceMeta, MetaType, PrivacyType};
use crate::modules::instance::dto::{SpeedKill, SpeedRun};
use crate::modules::instance::tools::FindInstanceGuild;
use crate::modules::live_data_processor::dto::LiveDataProcessorFailure;
use crate::modules::live_data_processor::tools::log_parser::parse_cbl;
use crate::mysql::Opts;
use crate::util::database::*;
use crate::{mysql, params};
use zip::ZipArchive;

pub struct Instance {
    pub instance_metas: Arc<RwLock<(u32, HashMap<u32, InstanceMeta>)>>,
    pub speed_runs: Arc<RwLock<Vec<SpeedRun>>>,
    pub speed_kills: Arc<RwLock<Vec<SpeedKill>>>,
}

impl Default for Instance {
    fn default() -> Self {
        Instance {
            instance_metas: Arc::new(RwLock::new((0, HashMap::new()))),
            speed_runs: Arc::new(RwLock::new(Vec::new())),
            speed_kills: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Instance {
    pub fn init(self, mut db_main: (impl Select + Send + Execute + 'static)) -> Self {
        let instance_metas_arc_clone = Arc::clone(&self.instance_metas);

        let dns = std::env::var("MYSQL_URL").unwrap();
        let opts = Opts::from_url(&dns).unwrap();
        let mut conn = mysql::Conn::new(opts.clone()).unwrap();

        let data = Data::default().init(&mut conn);
        let live_data_processor = crate::modules::live_data_processor::LiveDataProcessor::default().init(&mut conn);

        std::thread::spawn(move || {
            let mut armory_counter = 1;
            let armory = Armory::default().init(&mut db_main);

            loop {
                println!("[Update loop] starting update {}", time_util::now());
                delete_old_character_data(&mut db_main);
                println!("[Update loop] finish delete old character data");
                update_instance_metas(Arc::clone(&instance_metas_arc_clone), &mut db_main);
                println!("[Update loop] finish update_instance_metas");

                if armory_counter % 10 == 1 {
                    println!("Updating rankings at {}", time_util::now());
                    update_instance_rankings_dps( &mut db_main, &armory);
                    update_instance_rankings_hps( &mut db_main, &armory);
                    println!("Updating rankings complete at {}", time_util::now());
                }

                // update an instance metas that doesn't have updated specs
                if let Some(instance_meta) = db_main
                    .select(
                        "SELECT A.id, A.server_id, A.start_ts, A.end_ts, A.expired, A.map_id, B.map_difficulty, C.member_id, A.upload_id, A.privacy_type, A.privacy_ref, A.updated_specs FROM instance_meta A JOIN instance_raid B ON A.id = \
                         B.instance_meta_id JOIN instance_uploads C ON A.upload_id = C.id WHERE A.updated_specs = 0 LIMIT 10", // Restrict to 10 results
                        |mut row| InstanceMeta {
                            instance_meta_id: row.take(0).unwrap(),
                            server_id: row.take(1).unwrap(),
                            start_ts: row.take(2).unwrap(),
                            end_ts: row.take_opt(3).unwrap().ok(),
                            expired: row.take_opt(4).unwrap().ok(),
                            map_id: row.take(5).unwrap(),
                            participants: Vec::new(),
                            instance_specific: MetaType::Raid {
                                map_difficulty: row.take::<u8, usize>(6).unwrap(),
                            },
                            uploaded_user: row.take(7).unwrap(),
                            upload_id: row.take(8).unwrap(),
                            privacy_type: PrivacyType::new(row.take(9).unwrap(), row.take(10).unwrap()),
                            updated_specs: row.take(11).unwrap(),
                        },
                    )
                    .into_iter()
                    .next()
                // Extract the first (and only) result
                {
                    let storage_path = std::env::var("INSTANCE_STORAGE_PATH").expect("storage path must be set");
                    let zip_path = format!("{}/zips/upload_{}.zip", storage_path, instance_meta.upload_id);
                    let file = File::open(zip_path).map_err(|_| LiveDataProcessorFailure::InvalidZipFile);

                    if let Ok(file) = file {
                        // Create a ZipArchive from the file
                        let zip = ZipArchive::new(file).map_err(|_| LiveDataProcessorFailure::InvalidZipFile);

                        if let Ok(mut zip) = zip {
                            let log_file = zip.by_index(0).map_err(|_| LiveDataProcessorFailure::InvalidZipFile);

                            if let Ok(log_file) = log_file {
                                let bytes = log_file.bytes().filter_map(|byte| byte.ok()).collect::<Vec<u8>>();
                                let mut content = Vec::new();
                                for slice in bytes.split(|c| *c == 10) {
                                    if let Ok(parsed_str) = std::str::from_utf8(slice) {
                                        content.push(parsed_str);
                                    }
                                }
                                let content = content.join("\n");

                                println!("Updating specs for instance meta {}", instance_meta.instance_meta_id);

                                // delete all character histories within the start/end timestamp
                                db_main.execute_wparams(
                                    "DELETE FROM armory_character_history WHERE timestamp >= :start_ts AND timestamp <= :end_ts and character_id in (SELECT character_id FROM instance_participant WHERE instance_meta_id = :instance_meta_id)",
                                    params!(
                                        "start_ts" => instance_meta.start_ts/1000,
                                        "end_ts" => instance_meta.end_ts.unwrap_or(instance_meta.start_ts)/1000,
                                        "instance_meta_id" => instance_meta.instance_meta_id
                                    ),
                                );

                                let mut combat_log_parser = crate::modules::live_data_processor::material::WoWVanillaParser::new(instance_meta.server_id);

                                parse_cbl(
                                    &mut combat_log_parser,
                                    &live_data_processor,
                                    &mut conn,
                                    &data,
                                    &armory,
                                    &content,
                                    instance_meta.start_ts,
                                    instance_meta.end_ts.unwrap_or(instance_meta.start_ts),
                                    instance_meta.uploaded_user,
                                    false,
                                );

                                // mark instance meta as updated
                                db_main.execute_wparams("UPDATE instance_meta SET updated_specs = 1 WHERE id = :instance_meta_id", params!("instance_meta_id" => instance_meta.instance_meta_id));
                            }
                        }
                    }
                }
                println!("[Update loop] finish spec update");

                // Update hashes for instance_uploads with null hash
                let uploads_without_hash = db_main.select(
                    "SELECT id FROM instance_uploads WHERE hash IS NULL LIMIT 25",
                    |mut row| row.take::<u32, usize>(0).unwrap()
                );

                for upload_id in uploads_without_hash {
                    let storage_path = std::env::var("INSTANCE_STORAGE_PATH").expect("storage path must be set");
                    let zip_path = format!("{}/zips/upload_{}.zip", storage_path, upload_id);

                    println!("[Update loop] Updating hash for upload_id: {}", upload_id);
                    if let Ok(mut file) = File::open(zip_path) {
                        let mut buffer = Vec::new();
                        if file.read_to_end(&mut buffer).is_ok() {
                            // Calculate SHA256 hash
                            let mut hasher = Sha256::new();
                            hasher.update(&buffer);
                            let hash_result = hasher.finalize();
                            let hash_string = format!("{:x}", hash_result);
                            
                            // Update the database with the calculated hash
                            db_main.execute_wparams(
                                "UPDATE instance_uploads SET hash = :hash WHERE id = :id",
                                params!("hash" => hash_string.clone(), "id" => upload_id)
                            );
                            
                            // Get member_id for this upload
                            let member_id: u32 = db_main.select_wparams_value(
                                "SELECT member_id FROM instance_uploads WHERE id = :id",
                                |mut row| row.take::<u32, usize>(0).unwrap(),
                                params!("id" => upload_id)
                            ).unwrap();
                            
                            // Delete duplicate uploads with same member_id and hash but lower upload id
                            let duplicate_upload_ids: Vec<u32> = db_main.select_wparams(
                                "SELECT id FROM instance_uploads WHERE member_id = :member_id AND hash = :hash AND id < :upload_id",
                                |mut row| row.take::<u32, usize>(0).unwrap(),
                                params!("member_id" => member_id, "hash" => hash_string, "upload_id" => upload_id)
                            );
                            
                            for duplicate_id in duplicate_upload_ids {
                                // Delete instance_meta records first
                                db_main.execute_wparams(
                                    "DELETE FROM instance_meta WHERE upload_id = :upload_id",
                                    params!("upload_id" => duplicate_id)
                                );
                                
                                // Delete the duplicate instance_uploads record
                                db_main.execute_wparams(
                                    "DELETE FROM instance_uploads WHERE id = :id",
                                    params!("id" => duplicate_id)
                                );
                                
                                println!("Deleted duplicate upload_id: {} (same hash as {})", duplicate_id, upload_id);
                            }
                            
                            println!("Updated hash for upload_id: {}", upload_id);
                        }
                    } else {
                        // File doesn't exist, delete the instance_uploads record and any attached instance_meta
                        println!("Zip file not found for upload_id: {}, deleting from database", upload_id);
                        
                        // Delete any instance_meta records that reference this upload
                        db_main.execute_wparams(
                            "DELETE FROM instance_meta WHERE upload_id = :upload_id",
                            params!("upload_id" => upload_id)
                        );
                        
                        // Delete the instance_uploads record
                        db_main.execute_wparams(
                            "DELETE FROM instance_uploads WHERE id = :id",
                            params!("id" => upload_id)
                        );
                        
                        println!("Deleted instance_uploads and related instance_meta for upload_id: {}", upload_id);
                    }
                }
                println!("[Update loop] finish hash update");

                armory.update(&mut db_main);
                println!("[Update loop] finish armory update");

                armory_counter += 1;
                println!("[Update loop] Updating instance data done at {}", time_util::now());
                std::thread::sleep(std::time::Duration::from_secs(30));
            }
        });

        self
    }

    pub fn delete_instance_meta(&self, instance_meta_id: u32) {
        let mut instance_metas = self.instance_metas.write().unwrap();
        instance_metas.1.remove(&instance_meta_id);
    }
}

fn calculate_speed_runs(
    instance_metas: Arc<RwLock<(u32, HashMap<u32, InstanceMeta>)>>, instance_kill_attempts: Arc<RwLock<(u32, HashMap<u32, Vec<InstanceAttempt>>)>>, speed_runs: Arc<RwLock<Vec<SpeedRun>>>, db_main: &mut impl Select, armory: &Armory,
) {
    lazy_static! {
        static ref INSTANCE_ENCOUNTERS: HashMap<u16, Vec<u32>> = {
            let mut instance_encounters = HashMap::new();
            instance_encounters.insert(409, vec![80, 1, 2, 81, 82, 4, 5, 6, 7, 8, 9, 10]);
            instance_encounters.insert(249, vec![11]);
            instance_encounters.insert(309, vec![12, 13, 14, 15, 17, 19, 20, 21]);
            instance_encounters.insert(469, vec![22, 23, 24, 25, 26, 27, 28, 29]);
            instance_encounters.insert(509, vec![30, 31, 32, 33, 34, 35]);
            instance_encounters.insert(531, vec![36, 37, 38, 39, 40, 41, 42, 163, 164, 165]);
            instance_encounters.insert(533, vec![43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57]);
            instance_encounters.insert(532, vec![201, 202, 203, 204, 205]); // lower kara
            instance_encounters.insert(565, vec![69, 70]);
            instance_encounters.insert(544, vec![71]);
            instance_encounters.insert(550, vec![72, 73, 74, 75]);
            instance_encounters.insert(548, vec![76, 78, 79, 80, 81]);
            instance_encounters.insert(568, vec![82, 83, 84, 85, 86, 87]);
            instance_encounters.insert(534, vec![88, 89, 90, 91, 92]);
            instance_encounters.insert(564, vec![93, 94, 95, 96, 97, 98, 99, 100, 101]);
            instance_encounters.insert(580, vec![103, 104, 105, 106, 107]);
            instance_encounters.insert(615, vec![108]);
            instance_encounters.insert(616, vec![109]);
            instance_encounters.insert(624, vec![110, 111, 112, 113]);
            instance_encounters.insert(603, vec![114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126]);
            instance_encounters.insert(649, vec![128, 129, 130, 131, 132]);
            instance_encounters.insert(631, vec![133, 134, 136, 137, 138, 139, 140, 141, 143, 144]);
            instance_encounters.insert(724, vec![145]);
            instance_encounters.insert(807, vec![200]); // ES
            instance_encounters.insert(814, vec![206, 207, 208, 209, 210, 211, 212, 213, 214]); // upper kara

            instance_encounters
        };
    }

    let kill_attempts = instance_kill_attempts.read().unwrap();
    let instance_metas = instance_metas.read().unwrap();
    let mut speed_runs = speed_runs.write().unwrap();
    let already_calculated_speed_runs: Vec<u32> = speed_runs.iter().map(|speed_run| speed_run.instance_meta_id).collect();

    for (instance_meta_id, attempts) in kill_attempts.1.iter().filter(|(im_id, _)| !already_calculated_speed_runs.contains(*im_id)) {
        if !instance_metas.1.contains_key(instance_meta_id) || attempts.len() == 0 {
            continue;
        }

        let instance_meta = instance_metas.1.get(instance_meta_id).unwrap();
        if instance_meta.privacy_type != PrivacyType::Public {
            continue;
        }

        let has_killed_all_encounters = INSTANCE_ENCOUNTERS
            .get(&instance_meta.map_id)
            .unwrap()
            .iter()
            .all(|encounter_id| attempts.iter().any(|attempt| attempt.encounter_id == *encounter_id && attempt.rankable));
        let all_difficulties_are_same = attempts.iter().all(|attempt| attempt.difficulty_id == attempts[0].difficulty_id);
        if !has_killed_all_encounters || !all_difficulties_are_same {
            continue;
        }

        let start = attempts.iter().map(|attempt| attempt.start_ts).min().unwrap();
        let end = attempts.iter().map(|attempt| attempt.end_ts).max().unwrap();
        let (guild_id, guild_name) = instance_meta
            .participants
            .find_instance_guild(db_main, armory, instance_meta.end_ts.unwrap_or(instance_meta.start_ts))
            .map(|guild| (guild.id, guild.name))
            .unwrap_or((0, "Pug Raid".to_string()));

        speed_runs.push(SpeedRun {
            instance_meta_id: *instance_meta_id,
            map_id: instance_meta.map_id,
            guild_id,
            guild_name,
            server_id: instance_meta.server_id,
            duration: end - start,
            difficulty_id: attempts[0].difficulty_id,
            season_index: attempts[0].season_index,
        });
    }
}

fn calculate_speed_kills(
    instance_metas: Arc<RwLock<(u32, HashMap<u32, InstanceMeta>)>>, instance_kill_attempts: Arc<RwLock<(u32, HashMap<u32, Vec<InstanceAttempt>>)>>, speed_kills: Arc<RwLock<Vec<SpeedKill>>>, db_main: &mut impl Select, armory: &Armory,
) {
    let kill_attempts = instance_kill_attempts.read().unwrap();
    let instance_metas = instance_metas.read().unwrap();
    let mut speed_kills = speed_kills.write().unwrap();
    let already_calculated_speed_kills: Vec<u32> = speed_kills.iter().map(|speed_kill| speed_kill.attempt_id).collect();

    for (instance_meta_id, attempts) in kill_attempts.1.iter() {
        if !instance_metas.1.contains_key(instance_meta_id) {
            continue;
        }
        let instance_meta = instance_metas.1.get(instance_meta_id).unwrap();
        if instance_meta.privacy_type != PrivacyType::Public {
            continue;
        }

        let (guild_id, guild_name) = instance_meta
            .participants
            .find_instance_guild(db_main, armory, instance_meta.end_ts.unwrap_or(instance_meta.start_ts))
            .map(|guild| (guild.id, guild.name))
            .unwrap_or((0, "Pug Raid".to_string()));

        for attempt in attempts.iter().filter(|attempt| attempt.rankable && !already_calculated_speed_kills.contains(&attempt.attempt_id)) {
            speed_kills.push(SpeedKill {
                instance_meta_id: *instance_meta_id,
                attempt_id: attempt.attempt_id,
                encounter_id: attempt.encounter_id,
                guild_id,
                guild_name: guild_name.clone(),
                server_id: instance_meta.server_id,
                duration: attempt.end_ts - attempt.start_ts,
                difficulty_id: attempt.difficulty_id,
                season_index: attempt.season_index,
            });
        }
    }
}

fn calculate_season_index(ts: u64) -> u8 {
    let starting_unix_time = 1731470400000; // Wed Nov 13 2024 04:00:00 GMT+0000 1st instance reset after CC2
    let one_week_in_ms = 604800000;

    if ts < starting_unix_time {
        return 0;
    }

    (1 + (ts - starting_unix_time) / one_week_in_ms) as u8
}



fn update_instance_rankings_dps(db_main: &mut (impl Execute + Select), armory: &Armory) {
    let results = db_main
        .select(
            "SELECT A.id, A.character_id, B.encounter_id, A.attempt_id, A.damage, (B.end_ts - B.start_ts) as duration,
             B.instance_meta_id, C.map_difficulty, B.start_ts, D.server_id
             FROM instance_ranking_damage A
             JOIN instance_attempt B ON A.attempt_id = B.id
             JOIN instance_raid C ON B.instance_meta_id = C.instance_meta_id
             JOIN instance_meta D ON B.instance_meta_id = D.id
             WHERE B.rankable = 1 AND B.start_ts >= 1731470400000
             AND A.id not in (select id from ranking_results_damage)",
            |mut row| {
                let id: u32 = row.take(0).unwrap();
                let character_id: u32 = row.take(1).unwrap();
                let encounter_id: u32 = row.take(2).unwrap();
                let attempt_id: u32 = row.take(3).unwrap();
                let amount: u32 = row.take(4).unwrap();
                let duration: u64 = row.take(5).unwrap();
                let instance_meta_id: u32 = row.take(6).unwrap();
                let difficulty_id: u8 = row.take(7).unwrap();
                let start_ts: u64 = row.take(8).unwrap();
                let server_id: u32 = row.take(9).unwrap();
                (id, character_id, encounter_id, attempt_id, amount, duration, instance_meta_id, difficulty_id, start_ts, server_id)
            }
        );

    if results.is_empty() {
        return;
    }

    // Build value strings
    let mut value_strings = Vec::with_capacity(results.len());

    for (id, character_id, encounter_id, attempt_id, amount, duration, instance_meta_id, difficulty_id, start_ts, server_id) in results {
        let character_info = armory.get_character_moment(db_main, character_id, start_ts);

        let hero_class_id = character_info
            .as_ref()
            .map(|char_history| char_history.character_info.hero_class_id)
            .unwrap_or(0);

        if hero_class_id == 0 || hero_class_id == 12 {
            continue; // ignore unknown class
        }

        let character_name = character_info
            .as_ref()
            .map(|char_history| char_history.character_name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let character_spec = character_info
            .as_ref()
            .and_then(|char_history| char_history.character_info.talent_specialization.as_ref().map(|talents| get_talent_tree(&talents) + 1))
            .unwrap_or(0);

        let season_index = calculate_season_index(start_ts);

        // Create value string for this record
        value_strings.push(format!(
            "({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, '{}')",
            id, encounter_id, server_id, character_id, hero_class_id, instance_meta_id,
            attempt_id, amount, duration, difficulty_id, character_spec, season_index,
            character_name
        ));
    }

    // Execute batch inserts with max 1000 records per batch
    const BATCH_SIZE: usize = 1000;

    for chunk in value_strings.chunks(BATCH_SIZE) {
        if !chunk.is_empty() {
            let batch_insert = format!(
                "INSERT INTO ranking_results_damage (id, encounter_id, server_id, character_id, hero_class_id,
                instance_meta_id, attempt_id, amount, duration, difficulty_id, character_spec, season_index, character_name)
                VALUES {}",
                chunk.join(",")
            );

            db_main.execute_one(&batch_insert);
        }
    }
}

fn update_instance_rankings_hps(db_main: &mut (impl Execute + Select), armory: &Armory) {
    let results = db_main
        .select(
            "SELECT A.id, A.character_id, B.encounter_id, A.attempt_id, A.heal, (B.end_ts - B.start_ts) as duration,
             B.instance_meta_id, C.map_difficulty, B.start_ts, D.server_id
             FROM instance_ranking_heal A
             JOIN instance_attempt B ON A.attempt_id = B.id
             JOIN instance_raid C ON B.instance_meta_id = C.instance_meta_id
             JOIN instance_meta D ON B.instance_meta_id = D.id
             WHERE B.rankable = 1 AND B.start_ts >= 1731470400000
             AND A.id not in (select id from ranking_results_heal)",
            |mut row| {
                let id: u32 = row.take(0).unwrap();
                let character_id: u32 = row.take(1).unwrap();
                let encounter_id: u32 = row.take(2).unwrap();
                let attempt_id: u32 = row.take(3).unwrap();
                let amount: u32 = row.take(4).unwrap();
                let duration: u64 = row.take(5).unwrap();
                let instance_meta_id: u32 = row.take(6).unwrap();
                let difficulty_id: u8 = row.take(7).unwrap();
                let start_ts: u64 = row.take(8).unwrap();
                let server_id: u32 = row.take(9).unwrap();
                (id, character_id, encounter_id, attempt_id, amount, duration, instance_meta_id, difficulty_id, start_ts, server_id)
            }
        );

    if results.is_empty() {
        return;
    }

    // Build value strings
    let mut value_strings = Vec::with_capacity(results.len());

    for (id, character_id, encounter_id, attempt_id, amount, duration, instance_meta_id, difficulty_id, start_ts, server_id) in results {
        let character_info = armory.get_character_moment(db_main, character_id, start_ts);

        let hero_class_id = character_info
            .as_ref()
            .map(|char_history| char_history.character_info.hero_class_id)
            .unwrap_or(0);

        if hero_class_id == 0 || hero_class_id == 12 {
            continue; // ignore unknown class
        }

        let character_name = character_info
            .as_ref()
            .map(|char_history| char_history.character_name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let character_spec = character_info
            .as_ref()
            .and_then(|char_history| char_history.character_info.talent_specialization.as_ref().map(|talents| get_talent_tree(&talents) + 1))
            .unwrap_or(0);

        let season_index = calculate_season_index(start_ts);

        // Create value string for this record
        value_strings.push(format!(
            "({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, '{}')",
            id, encounter_id, server_id, character_id, hero_class_id, instance_meta_id,
            attempt_id, amount, duration, difficulty_id, character_spec, season_index,
            character_name
        ));
    }

    // Execute batch inserts with max 1000 records per batch
    const BATCH_SIZE: usize = 1000;

    for chunk in value_strings.chunks(BATCH_SIZE) {
        if !chunk.is_empty() {
            let batch_insert = format!(
                "INSERT INTO ranking_results_heal (id, encounter_id, server_id, character_id, hero_class_id,
                instance_meta_id, attempt_id, amount, duration, difficulty_id, character_spec, season_index, character_name)
                VALUES {}",
                chunk.join(",")
            );

            db_main.execute_one(&batch_insert);
        }
    }
}

fn delete_old_character_data(db_main: &mut (impl Select + Execute)) {
    // delete old armory_character_info
    db_main.execute_one("delete FROM main.armory_character_info where id not in (SELECT character_info_id FROM main.armory_character_history);");

    // delete old armory_gear
    db_main.execute_one("delete FROM main.armory_gear where id not in (SELECT gear_id FROM main.armory_character_info);");
}

fn update_instance_metas(instance_metas: Arc<RwLock<(u32, HashMap<u32, InstanceMeta>)>>, db_main: &mut impl Select) {
    let mut instance_metas = instance_metas.write().unwrap();
    let params = params!("saved_instance_meta_id" => instance_metas.0);

    // Raids
    db_main
        .select_wparams(
            "SELECT A.id, A.server_id, A.start_ts, A.end_ts, A.expired, A.map_id, B.map_difficulty, C.member_id, A.upload_id, A.privacy_type, A.privacy_ref, A.updated_specs FROM instance_meta A JOIN instance_raid B ON A.id = B.instance_meta_id \
             JOIN instance_uploads C ON A.upload_id = C.id WHERE A.id > :saved_instance_meta_id ORDER BY A.id",
            |mut row| InstanceMeta {
                instance_meta_id: row.take(0).unwrap(),
                server_id: row.take(1).unwrap(),
                start_ts: row.take(2).unwrap(),
                end_ts: row.take_opt(3).unwrap().ok(),
                expired: row.take_opt(4).unwrap().ok(),
                map_id: row.take(5).unwrap(),
                participants: Vec::new(),
                instance_specific: MetaType::Raid {
                    map_difficulty: row.take::<u8, usize>(6).unwrap(),
                },
                uploaded_user: row.take(7).unwrap(),
                upload_id: row.take(8).unwrap(),
                privacy_type: PrivacyType::new(row.take(9).unwrap(), row.take(10).unwrap()),
                updated_specs: row.take(11).unwrap(),
            },
            params.clone(),
        )
        .into_iter()
        .for_each(|result| {
            instance_metas.0 = if result.instance_meta_id > 50 { result.instance_meta_id - 50 } else { 0 }; // Always load previous 50 raids
            instance_metas.1.insert(result.instance_meta_id, result);
        });

    // Load participants
    db_main
        .select_wparams(
            "SELECT A.id, B.character_id FROM instance_meta A JOIN instance_participants B ON A.id = B.instance_meta_id WHERE A.id > :saved_instance_meta_id ORDER BY A.id",
            |mut row| (row.take::<u32, usize>(0).unwrap(), row.take::<u32, usize>(1).unwrap()),
            params,
        )
        .into_iter()
        .for_each(|(instance_meta_id, character_id)| {
            instance_metas.1.get_mut(&instance_meta_id).unwrap().participants.push(character_id);
        });
}

trait Winner {
    fn to_winner(&self) -> Option<bool>;
}

impl Winner for u8 {
    fn to_winner(&self) -> Option<bool> {
        // TODO: Find out what these values mean!
        if *self == 0 {
            return None;
        } else if *self == 1 {
            return Some(true);
        }
        Some(false)
    }
}
