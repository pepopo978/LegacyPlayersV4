use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, RwLock};

use crate::material::Cachable;
use crate::modules::armory::Armory;
use crate::modules::armory::tools::{GetArenaTeam, GetCharacter};
use crate::modules::armory::util::talent_tree::get_talent_tree;
use crate::modules::instance::domain_value::{InstanceAttempt, InstanceMeta, MetaType, PrivacyType};
use crate::modules::instance::dto::{InstanceViewerAttempt, RankingResult, SpeedKill, SpeedRun};
use crate::modules::instance::tools::{FindInstanceGuild};
use crate::{mysql, params};
use crate::util::database::*;
use zip::ZipArchive;
use crate::modules::data::Data;
use crate::modules::live_data_processor::tools::log_parser::parse_cbl;
use crate::modules::live_data_processor::dto::{LiveDataProcessorFailure};
use crate::mysql::Opts;

pub struct Instance {
    pub instance_metas: Arc<RwLock<(u32, HashMap<u32, InstanceMeta>)>>,
    pub instance_exports: Arc<RwLock<HashMap<(u32, u8), Cachable<Vec<String>>>>>,
    pub instance_attempts: Arc<RwLock<HashMap<u32, Cachable<Vec<InstanceViewerAttempt>>>>>,
    pub speed_runs: Arc<RwLock<Vec<SpeedRun>>>,
    pub speed_kills: Arc<RwLock<Vec<SpeedKill>>>,

    // encounter_id => character_id => Vec<Ranking>
    pub instance_rankings_dps: Arc<RwLock<(u32, HashMap<u32, HashMap<u32, Vec<RankingResult>>>)>>,
    pub instance_rankings_hps: Arc<RwLock<(u32, HashMap<u32, HashMap<u32, Vec<RankingResult>>>)>>,
    pub instance_rankings_tps: Arc<RwLock<(u32, HashMap<u32, HashMap<u32, Vec<RankingResult>>>)>>,
    // attempt_id => (instance_meta_id => Vec<Attempt>)
    pub instance_kill_attempts: Arc<RwLock<(u32, HashMap<u32, Vec<InstanceAttempt>>)>>,
}

impl Default for Instance {
    fn default() -> Self {
        Instance {
            instance_metas: Arc::new(RwLock::new((0, HashMap::new()))),
            instance_exports: Arc::new(RwLock::new(HashMap::new())),
            instance_attempts: Arc::new(RwLock::new(HashMap::new())),
            instance_rankings_dps: Arc::new(RwLock::new((0, HashMap::new()))),
            instance_rankings_hps: Arc::new(RwLock::new((0, HashMap::new()))),
            instance_rankings_tps: Arc::new(RwLock::new((0, HashMap::new()))),
            instance_kill_attempts: Arc::new(RwLock::new((0, HashMap::new()))),
            speed_runs: Arc::new(RwLock::new(Vec::new())),
            speed_kills: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Instance {
    pub fn init(self, mut db_main: (impl Select + Send + Execute + 'static)) -> Self {
        let instance_metas_arc_clone = Arc::clone(&self.instance_metas);
        let instance_exports_arc_clone = Arc::clone(&self.instance_exports);
        let instance_attempts_arc_clone = Arc::clone(&self.instance_attempts);
        let instance_rankings_dps_arc_clone = Arc::clone(&self.instance_rankings_dps);
        let instance_rankings_hps_arc_clone = Arc::clone(&self.instance_rankings_hps);
        let instance_rankings_tps_arc_clone = Arc::clone(&self.instance_rankings_tps);
        let instance_kill_attempts_clone = Arc::clone(&self.instance_kill_attempts);
        let speed_runs_arc_clone = Arc::clone(&self.speed_runs);
        let speed_kills_arc_clone = Arc::clone(&self.speed_kills);

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
                evict_attempts_cache(Arc::clone(&instance_attempts_arc_clone));
                println!("[Update loop] finish evict_attempts_cache");
                evict_export_cache(Arc::clone(&instance_exports_arc_clone));
                println!("[Update loop] finish evict_export_cache");
                update_instance_metas(Arc::clone(&instance_metas_arc_clone), &mut db_main, &armory);
                println!("[Update loop] finish update_instance_metas");
                update_instance_kill_attempts(Arc::clone(&instance_kill_attempts_clone), &mut db_main);
                println!("[Update loop] finish update_instance_kill_attempts");

                if armory_counter == 2 || armory_counter >= 1000 {
                    // purge old character info
                    println!("Updating rankings at {}", time_util::now());
                    update_instance_rankings_dps(Arc::clone(&instance_rankings_dps_arc_clone), &mut db_main, &armory);
                    update_instance_rankings_hps(Arc::clone(&instance_rankings_hps_arc_clone), &mut db_main, &armory);
                    update_instance_rankings_tps(Arc::clone(&instance_rankings_tps_arc_clone), &mut db_main, &armory);
                    println!("Updating rankings complete at {}", time_util::now());

                    println!("Updating speed runs at {}", time_util::now());
                    calculate_speed_runs(Arc::clone(&instance_metas_arc_clone),
                                         Arc::clone(&instance_kill_attempts_clone),
                                         Arc::clone(&speed_runs_arc_clone), &mut db_main, &armory);
                    calculate_speed_kills(Arc::clone(&instance_metas_arc_clone),
                                          Arc::clone(&instance_kill_attempts_clone),
                                          Arc::clone(&speed_kills_arc_clone), &mut db_main, &armory);
                    println!("Updating speed runs complete at {}", time_util::now());
                    armory_counter = 0;
                }

                // update an instance metas that doesn't have updated specs
                if let Some(instance_meta) = db_main
                    .select(
                        "SELECT A.id, A.server_id, A.start_ts, A.end_ts, A.expired, A.map_id, B.map_difficulty, C.member_id, A.upload_id, A.privacy_type, A.privacy_ref, A.updated_specs \
         FROM instance_meta A \
         JOIN instance_raid B ON A.id = B.instance_meta_id \
         JOIN instance_uploads C ON A.upload_id = C.id \
         WHERE A.updated_specs = 0 \
         LIMIT 10", // Restrict to 10 results
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
                    .next() // Extract the first (and only) result
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
                                db_main.execute_wparams("DELETE FROM armory_character_history WHERE timestamp >= :start_ts AND timestamp <= :end_ts and character_id in (SELECT character_id FROM instance_participant WHERE instance_meta_id = :instance_meta_id)",
                                                        params!(
                                                            "start_ts" => instance_meta.start_ts/1000,
                                                            "end_ts" => instance_meta.end_ts.unwrap_or(instance_meta.start_ts)/1000,
                                                            "instance_meta_id" => instance_meta.instance_meta_id
                                                        ));

                                let mut combat_log_parser = crate::modules::live_data_processor::material::WoWVanillaParser::new(instance_meta.server_id);

                                parse_cbl(&mut combat_log_parser,
                                          &live_data_processor,
                                          &mut conn,
                                          &data,
                                          &armory,
                                          &content,
                                          instance_meta.start_ts,
                                          instance_meta.end_ts.unwrap_or(instance_meta.start_ts),
                                          instance_meta.uploaded_user,
                                          false);


                                // mark instance meta as updated
                                db_main.execute_wparams("UPDATE instance_meta SET updated_specs = 1 WHERE id = :instance_meta_id",
                                                        params!("instance_meta_id" => instance_meta.instance_meta_id));

                            }
                        }
                    }
                }
                println!("[Update loop] finish spec update");

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

fn calculate_speed_runs(instance_metas: Arc<RwLock<(u32, HashMap<u32, InstanceMeta>)>>,
                        instance_kill_attempts: Arc<RwLock<(u32, HashMap<u32, Vec<InstanceAttempt>>)>>,
                        speed_runs: Arc<RwLock<Vec<SpeedRun>>>,
                        db_main: &mut impl Select, armory: &Armory) {
    lazy_static! {
        static ref INSTANCE_ENCOUNTERS: HashMap<u16, Vec<u32>> = {
            let mut instance_encounters = HashMap::new();
            instance_encounters.insert(409, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
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
            instance_encounters.insert(814, vec![206, 207, 208, 209, 210]); // upper kara up to echo

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

        let has_killed_all_encounters = INSTANCE_ENCOUNTERS.get(&instance_meta.map_id).unwrap()
            .iter().all(|encounter_id| attempts.iter().any(|attempt| attempt.encounter_id == *encounter_id && attempt.rankable));
        let all_difficulties_are_same = attempts.iter().all(|attempt| attempt.difficulty_id == attempts[0].difficulty_id);
        if !has_killed_all_encounters || !all_difficulties_are_same {
            continue;
        }

        let start = attempts.iter().map(|attempt| attempt.start_ts).min().unwrap();
        let end = attempts.iter().map(|attempt| attempt.end_ts).max().unwrap();
        let (guild_id, guild_name) = instance_meta.participants
            .find_instance_guild(db_main, armory, instance_meta.end_ts.unwrap_or(instance_meta.start_ts))
            .map(|guild| (guild.id, guild.name)).unwrap_or((0, "Pug Raid".to_string()));

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

fn calculate_speed_kills(instance_metas: Arc<RwLock<(u32, HashMap<u32, InstanceMeta>)>>,
                         instance_kill_attempts: Arc<RwLock<(u32, HashMap<u32, Vec<InstanceAttempt>>)>>,
                         speed_kills: Arc<RwLock<Vec<SpeedKill>>>,
                         db_main: &mut impl Select, armory: &Armory) {
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

        let (guild_id, guild_name) = instance_meta.participants
            .find_instance_guild(db_main, armory, instance_meta.end_ts.unwrap_or(instance_meta.start_ts))
            .map(|guild| (guild.id, guild.name)).unwrap_or((0, "Pug Raid".to_string()));

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

    (1 + (ts-starting_unix_time) / one_week_in_ms) as u8
}

fn update_instance_kill_attempts(instance_kill_attempts: Arc<RwLock<(u32, HashMap<u32, Vec<InstanceAttempt>>)>>, db_main: &mut impl Select) {
    let mut kill_attempts = instance_kill_attempts.write().unwrap();
    db_main.select_wparams("SELECT A.instance_meta_id, A.id, A.encounter_id, A.start_ts, A.end_ts, B.map_difficulty, A.rankable FROM instance_attempt A \
    JOIN instance_raid B ON A.instance_meta_id = B.instance_meta_id \
    WHERE A.is_kill = 1 AND A.id > :saved_attempt_id ORDER BY A.id",
                           |mut row|
                               {
                                   let start_ts = row.take(3).unwrap();
                                   (row.take::<u32, usize>(0).unwrap(), InstanceAttempt {
                                       attempt_id: row.take(1).unwrap(),
                                       encounter_id: row.take(2).unwrap(),
                                       start_ts,
                                       end_ts: row.take(4).unwrap(),
                                       is_kill: true,
                                       difficulty_id: row.take(5).unwrap(),
                                       rankable: row.take(6).unwrap(),
                                       season_index: calculate_season_index(start_ts),
                                   })
                               }, params!("saved_attempt_id" => kill_attempts.0))
        .into_iter()
        .for_each(|(instance_meta_id, instance_attempt)| {
            kill_attempts.0 = instance_attempt.attempt_id;
            let attempt_container = kill_attempts.1.entry(instance_meta_id).or_insert_with(Vec::new);
            attempt_container.push(instance_attempt);
        });
}

fn update_instance_rankings_dps(instance_rankings_dps: Arc<RwLock<(u32, HashMap<u32, HashMap<u32, Vec<RankingResult>>>)>>, db_main: &mut impl Select, armory: &Armory) {
    let mut rankings_dps = instance_rankings_dps.write().unwrap();
    db_main
        .select_wparams(
            "SELECT A.id, A.character_id, B.encounter_id, A.attempt_id, A.damage, \
            (B.end_ts - B.start_ts) as duration, B.instance_meta_id, C.map_difficulty, B.start_ts FROM instance_ranking_damage A \
            JOIN instance_attempt B ON A.attempt_id = B.id \
            JOIN instance_raid C ON B.instance_meta_id = C.instance_meta_id \
            WHERE A.id > :last_queried_id AND B.rankable = 1 AND B.start_ts >= 1731470400000  ORDER BY A.id", // filter stuff before CC2
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
                (
                    id,
                    character_id,
                    encounter_id,
                    attempt_id,
                    amount,
                    duration,
                    instance_meta_id,
                    difficulty_id,
                    start_ts
                )
            },
            params!("last_queried_id" => rankings_dps.0),
        )
        .into_iter()
        .for_each(|(id, character_id, encounter_id, attempt_id, amount, duration, instance_meta_id, difficulty_id, start_ts)| {
            rankings_dps.0 = id;
            let characters_rankings = rankings_dps.1.entry(encounter_id).or_insert_with(HashMap::new);
            let rankings = characters_rankings.entry(character_id).or_insert_with(|| Vec::with_capacity(1));
            rankings.push(RankingResult {
                attempt_id,
                amount,
                duration,
                instance_meta_id,
                difficulty_id,
                character_spec: armory.get_character_moment(db_main, character_id, start_ts)
                    .and_then(|char_history| char_history.character_info.talent_specialization.as_ref().map(|talents| get_talent_tree(&talents) + 1))
                    .unwrap_or(0),
                season_index: calculate_season_index(start_ts),
            });
        });
}

fn update_instance_rankings_hps(instance_rankings_hps: Arc<RwLock<(u32, HashMap<u32, HashMap<u32, Vec<RankingResult>>>)>>, db_main: &mut impl Select, armory: &Armory) {
    let mut rankings_hps = instance_rankings_hps.write().unwrap();
    db_main
        .select_wparams(
            "SELECT A.id, A.character_id, B.encounter_id, A.attempt_id, A.heal, \
            (B.end_ts - B.start_ts) as duration, B.instance_meta_id, C.map_difficulty, B.start_ts FROM instance_ranking_heal A \
            JOIN instance_attempt B ON A.attempt_id = B.id \
            JOIN instance_raid C ON B.instance_meta_id = C.instance_meta_id \
            WHERE A.id > :last_queried_id AND B.rankable = 1 AND B.start_ts >= 1731470400000 ORDER BY A.id",  // filter stuff before CC2
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
                (
                    id,
                    character_id,
                    encounter_id,
                    attempt_id,
                    amount,
                    duration,
                    instance_meta_id,
                    difficulty_id,
                    start_ts
                )
            },
            params!("last_queried_id" => rankings_hps.0),
        )
        .into_iter()
        .for_each(|(id, character_id, encounter_id, attempt_id, amount, duration, instance_meta_id, difficulty_id, start_ts)| {
            rankings_hps.0 = id;
            let characters_rankings = rankings_hps.1.entry(encounter_id).or_insert_with(HashMap::new);
            let rankings = characters_rankings.entry(character_id).or_insert_with(|| Vec::with_capacity(1));
            rankings.push(RankingResult {
                attempt_id,
                amount,
                duration,
                instance_meta_id,
                difficulty_id,
                character_spec: armory.get_character_moment(db_main, character_id, start_ts)
                    .and_then(|char_history| char_history.character_info.talent_specialization.as_ref().map(|talents| get_talent_tree(&talents) + 1))
                    .unwrap_or(0),
                season_index: calculate_season_index(start_ts),
            });
        });
}

fn update_instance_rankings_tps(instance_rankings_tps: Arc<RwLock<(u32, HashMap<u32, HashMap<u32, Vec<RankingResult>>>)>>, db_main: &mut impl Select, armory: &Armory) {
    let mut rankings_tps = instance_rankings_tps.write().unwrap();
    db_main
        .select_wparams(
            "SELECT A.id, A.character_id, B.encounter_id, A.attempt_id, A.threat, \
            (B.end_ts - B.start_ts) as duration, B.instance_meta_id, C.map_difficulty, B.start_ts FROM instance_ranking_threat A \
            JOIN instance_attempt B ON A.attempt_id = B.id \
            JOIN instance_raid C ON B.instance_meta_id = C.instance_meta_id \
            WHERE A.id > :last_queried_id AND B.rankable = 1 AND B.start_ts >= 1731470400000 ORDER BY A.id", // filter stuff before CC2
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
                (
                    id,
                    character_id,
                    encounter_id,
                    attempt_id,
                    amount,
                    duration,
                    instance_meta_id,
                    difficulty_id,
                    start_ts
                )
            },
            params!("last_queried_id" => rankings_tps.0),
        )
        .into_iter()
        .for_each(|(id, character_id, encounter_id, attempt_id, amount, duration, instance_meta_id, difficulty_id, start_ts)| {
            rankings_tps.0 = id;
            let characters_rankings = rankings_tps.1.entry(encounter_id).or_insert_with(HashMap::new);
            let rankings = characters_rankings.entry(character_id).or_insert_with(|| Vec::with_capacity(1));
            rankings.push(RankingResult {
                attempt_id,
                amount,
                duration,
                instance_meta_id,
                difficulty_id,
                character_spec: armory.get_character_moment(db_main, character_id, start_ts)
                    .and_then(|char_history| char_history.character_info.talent_specialization.as_ref().map(|talents| get_talent_tree(&talents) + 1))
                    .unwrap_or(0),
                season_index: calculate_season_index(start_ts),
            });
        });
}

fn evict_attempts_cache(instance_attempts: Arc<RwLock<HashMap<u32, Cachable<Vec<InstanceViewerAttempt>>>>>) {
    let now = time_util::now();
    let mut instance_attempts = instance_attempts.write().unwrap();
    for instance_meta_id in instance_attempts
        .iter()
        .filter(|(_, cachable)| cachable.get_last_access() + 21600 < now)
        .map(|(instance_meta_id, _)| *instance_meta_id)
        .collect::<Vec<u32>>()
    {
        instance_attempts.remove(&instance_meta_id);
    }
}

fn evict_export_cache(instance_exports: Arc<RwLock<HashMap<(u32, u8), Cachable<Vec<String>>>>>) {
    let now = time_util::now();
    let mut instance_exports = instance_exports.write().unwrap();
    for instance_meta_id in instance_exports
        .iter()
        .filter(|(_, cachable)| cachable.get_last_access() + 21600 < now)
        .map(|(instance_meta_id, _)| *instance_meta_id)
        .collect::<Vec<(u32, u8)>>()
    {
        instance_exports.remove(&instance_meta_id);
    }
}

fn delete_old_character_data(db_main: &mut (impl Select + Execute)) {
    // delete old armory_character_info
    db_main.execute_one(
        "delete FROM main.armory_character_info where id not in (SELECT character_info_id FROM main.armory_character_history);"
    );

    // delete old armory_gear
    db_main.execute_one(
        "delete FROM main.armory_gear where id not in (SELECT gear_id FROM main.armory_character_info);"
    );
}

fn update_instance_metas(instance_metas: Arc<RwLock<(u32, HashMap<u32, InstanceMeta>)>>, db_main: &mut impl Select, armory: &Armory) {
    let mut instance_metas = instance_metas.write().unwrap();
    let params = params!("saved_instance_meta_id" => instance_metas.0);

    // Raids
    db_main
        .select_wparams(
            "SELECT A.id, A.server_id, A.start_ts, A.end_ts, A.expired, A.map_id, B.map_difficulty, C.member_id, A.upload_id, A.privacy_type, A.privacy_ref, A.updated_specs FROM instance_meta A \
            JOIN instance_raid B ON A.id = B.instance_meta_id \
            JOIN instance_uploads C ON A.upload_id = C.id \
            WHERE A.id > :saved_instance_meta_id ORDER BY A.id",
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
            }, params.clone(),
        )
        .into_iter()
        .for_each(|result| {
            instance_metas.0 = if result.instance_meta_id > 50 { result.instance_meta_id - 50 } else { 0 }; // Always load previous 50 raids
            instance_metas.1.insert(result.instance_meta_id, result);
        });

    // Rated Arenas
    // TODO: Rename team_change1 to team1_change
    db_main
        .select_wparams(
            "SELECT A.id, A.server_id, A.start_ts, A.end_ts, A.expired, A.map_id, B.winner, \
            B.team_id1, B.team_id2, B.team_change1, B.team_change2, C.member_id, A.upload_id, A.privacy_type, A.privacy_ref, A.updated_specs FROM instance_meta A \
            JOIN instance_rated_arena B ON A.id = B.instance_meta_id \
            JOIN instance_uploads C ON A.upload_id = C.id \
            WHERE A.id > :saved_instance_meta_id ORDER BY A.id",
            |mut row| {
                (
                    row.take::<u32, usize>(0).unwrap(),
                    row.take::<u32, usize>(1).unwrap(),
                    row.take::<u64, usize>(2).unwrap(),
                    row.take_opt::<u64, usize>(3).unwrap().ok(),
                    row.take_opt::<u64, usize>(4).unwrap().ok(),
                    row.take::<u16, usize>(5).unwrap(),
                    row.take::<u8, usize>(6).unwrap().to_winner(),
                    row.take::<u32, usize>(7).unwrap(),
                    row.take::<u32, usize>(8).unwrap(),
                    row.take::<i32, usize>(9).unwrap(),
                    row.take::<i32, usize>(10).unwrap(),
                    row.take::<u32, usize>(11).unwrap(),
                    row.take::<u32, usize>(12).unwrap(),
                    row.take::<u8, usize>(13).unwrap(),
                    row.take::<u32, usize>(14).unwrap(),
                    row.take::<bool, usize>(15).unwrap(),
                )
            }, params.clone(),
        )
        .into_iter()
        .for_each(|(instance_meta_id, server_id, start_ts, end_ts, expired, map_id, winner, team_id1, team_id2, team1_change, team2_change, uploaded_user, upload_id, privacy_type, privacy_ref, updated_specs)| {
            instance_metas.1.insert(
                instance_meta_id,
                InstanceMeta {
                    instance_meta_id,
                    server_id,
                    start_ts,
                    end_ts,
                    map_id,
                    expired,
                    participants: Vec::new(),
                    instance_specific: MetaType::RatedArena {
                        winner,
                        team1: armory.get_arena_team_by_id(db_main, team_id1).expect("Foreign key constraint"),
                        team2: armory.get_arena_team_by_id(db_main, team_id2).expect("Foreign key constraint"),
                        team1_change,
                        team2_change,
                    },
                    uploaded_user,
                    upload_id,
                    privacy_type: PrivacyType::new(privacy_type, privacy_ref),
                    updated_specs,
                },
            );
        });

    // Skirmishes
    db_main
        .select_wparams(
            "SELECT A.id, A.server_id, A.start_ts, A.end_ts, A.expired, A.map_id, B.winner, C.member_id, A.upload_id, A.privacy_type, A.privacy_ref, A.updated_specs FROM instance_meta A \
            JOIN instance_skirmish B ON A.id = B.instance_meta_id \
            JOIN instance_uploads C ON A.upload_id = C.id \
            WHERE A.id > :saved_instance_meta_id ORDER BY A.id",
            |mut row| InstanceMeta {
                instance_meta_id: row.take(0).unwrap(),
                server_id: row.take(1).unwrap(),
                start_ts: row.take(2).unwrap(),
                end_ts: row.take_opt(3).unwrap().ok(),
                expired: row.take_opt(4).unwrap().ok(),
                map_id: row.take(5).unwrap(),
                participants: Vec::new(),
                instance_specific: MetaType::Skirmish {
                    winner: row.take::<u8, usize>(6).unwrap().to_winner(),
                },
                uploaded_user: row.take(7).unwrap(),
                upload_id: row.take(8).unwrap(),
                privacy_type: PrivacyType::new(row.take(9).unwrap(), row.take(10).unwrap()),
                updated_specs: row.take(11).unwrap(),
            }, params.clone(),
        )
        .into_iter()
        .for_each(|result| {
            instance_metas.1.insert(result.instance_meta_id, result);
        });

    // Battlegrounds
    db_main
        .select_wparams(
            "SELECT A.id, A.server_id, A.start_ts, A.end_ts, A.expired, A.map_id, B.winner, \
            B.score_alliance, B.score_horde, C.member_id, A.upload_id, A.privacy_type, A.privacy_ref, A.updated_specs FROM instance_meta A \
            JOIN instance_battleground B ON A.id = B.instance_meta_id \
            JOIN instance_uploads C ON A.upload_id = C.id \
            WHERE A.id > :saved_instance_meta_id ORDER BY A.id",
            |mut row| InstanceMeta {
                instance_meta_id: row.take(0).unwrap(),
                server_id: row.take(1).unwrap(),
                start_ts: row.take(2).unwrap(),
                end_ts: row.take_opt(3).unwrap().ok(),
                expired: row.take_opt(4).unwrap().ok(),
                map_id: row.take(5).unwrap(),
                participants: Vec::new(),
                instance_specific: MetaType::Battleground {
                    winner: row.take::<u8, usize>(6).unwrap().to_winner(),
                    score_alliance: row.take(7).unwrap(),
                    score_horde: row.take(8).unwrap(),
                },
                uploaded_user: row.take(9).unwrap(),
                upload_id: row.take(10).unwrap(),
                privacy_type: PrivacyType::new(row.take(11).unwrap(), row.take(12).unwrap()),
                updated_specs: row.take(13).unwrap(),
            }, params.clone(),
        )
        .into_iter()
        .for_each(|result| {
            instance_metas.1.insert(result.instance_meta_id, result);
        });

    // Load participants
    db_main
        .select_wparams("SELECT A.id, B.character_id FROM instance_meta A \
        JOIN instance_participants B ON A.id = B.instance_meta_id \
        WHERE A.id > :saved_instance_meta_id ORDER BY A.id", |mut row| {
            (row.take::<u32, usize>(0).unwrap(), row.take::<u32, usize>(1).unwrap())
        }, params)
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
