use crate::modules::armory::tools::GetCharacter;
use crate::modules::armory::Armory;
use crate::modules::data::tools::RetrieveServer;
use crate::modules::data::Data;
use crate::modules::instance::domain_value::MetaType;
use crate::modules::instance::dto::{InstanceFailure, InstanceViewerAttempt, InstanceViewerGuild, InstanceViewerMeta, InstanceViewerParticipant};
use crate::modules::instance::material::Role;
use crate::modules::instance::tools::FindInstanceGuild;
use crate::modules::instance::Instance;
use crate::params;
use crate::util::database::Select;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub trait ExportInstance {
    fn export_instance_event_type(&self, instance_meta_id: u32, event_type: u8) -> Result<Vec<String>, InstanceFailure>;
    fn get_instance_meta(&self, db_main: &mut impl Select, data: &Data, armory: &Armory, instance_meta_id: u32) -> Result<InstanceViewerMeta, InstanceFailure>;
    fn get_instance_participants(&self, db_main: &mut impl Select, armory: &Armory, instance_meta_id: u32) -> Result<Vec<InstanceViewerParticipant>, InstanceFailure>;
    fn get_instance_attempts(&self, db_main: &mut impl Select, instance_meta_id: u32) -> Result<Vec<InstanceViewerAttempt>, InstanceFailure>;
}

impl ExportInstance for Instance {
    fn export_instance_event_type(&self, instance_meta_id: u32, event_type: u8) -> Result<Vec<String>, InstanceFailure> {
        let server_id = {
            let instance_metas = self.instance_metas.read().unwrap();
            let instance_meta = instance_metas.1.get(&instance_meta_id).ok_or(InstanceFailure::InvalidInput)?;
            instance_meta.server_id
        };

        let storage_path = std::env::var("INSTANCE_STORAGE_PATH").expect("storage path must be set");
        if Path::new(&format!("{}/{}/{}.zip", storage_path, server_id, instance_meta_id)).exists() {
            let reader = File::open(format!("{}/{}/{}.zip", storage_path, server_id, instance_meta_id)).unwrap();
            let zip = zip::ZipArchive::new(reader);
            if zip.is_err() {
                // Can happen if the zip file wants to be read before its ready
                return Err(InstanceFailure::Unknown);
            }
            let mut zip = zip.unwrap();

            for i in 0..zip.len() {
                let mut file = zip.by_index(i).unwrap();
                let evt_type = u8::from_str_radix(file.name(), 10).unwrap();
                if evt_type == event_type {
                    let mut content = String::with_capacity(file.size() as usize);
                    let read_result = file.read_to_string(&mut content);
                    if read_result.is_ok() {
                        let events = content.lines().into_iter().map(|cont| cont.to_owned()).collect::<Vec<String>>();
                        return Ok(events);
                    } else {
                        return Ok(Vec::new());
                    }
                }
            }
        } else {
            let event_path = format!("{}/{}/{}/{}", storage_path, server_id, instance_meta_id, event_type);
            if let Ok(file_content) = std::fs::read_to_string(event_path) {
                let events = file_content.lines().into_iter().map(|cont| cont.to_owned()).collect::<Vec<String>>();
                return Ok(events);
            }
        }
        Ok(vec![])
    }

    fn get_instance_meta(&self, db_main: &mut impl Select, data: &Data, armory: &Armory, instance_meta_id: u32) -> Result<InstanceViewerMeta, InstanceFailure> {
        let instance_metas = self.instance_metas.read().unwrap();
        if let Some(instance_meta) = instance_metas.1.get(&instance_meta_id) {
            let guild = instance_meta.participants.find_instance_guild(db_main, armory, instance_meta.start_ts);
            let map_difficulty = match instance_meta.instance_specific {
                MetaType::Raid { map_difficulty } => Some(map_difficulty),
                _ => None,
            };
            return Ok(InstanceViewerMeta {
                instance_meta_id,
                guild: guild.map(|guild| InstanceViewerGuild { guild_id: guild.id, guild_name: guild.name }),
                server_id: instance_meta.server_id,
                expansion_id: data.get_server(instance_meta.server_id).unwrap().expansion_id,
                map_id: instance_meta.map_id,
                map_difficulty,
                start_ts: instance_meta.start_ts,
                end_ts: instance_meta.end_ts,
                expired: instance_meta.expired,
                upload_id: instance_meta.upload_id
            });
        }
        Err(InstanceFailure::InvalidInput)
    }

    fn get_instance_participants(&self, db_main: &mut impl Select, armory: &Armory, instance_meta_id: u32) -> Result<Vec<InstanceViewerParticipant>, InstanceFailure> {
        let instance_metas = self.instance_metas.read().unwrap();
        if let Some(instance_meta) = instance_metas.1.get(&instance_meta_id) {
            return Ok(instance_meta
                .participants
                .iter()
                .map(|character_id| (character_id, armory.get_character_moment(db_main, *character_id, instance_meta.start_ts)))
                .map(|(char_id, char_history)| {
                    if let Some(char_history) = char_history {
                        InstanceViewerParticipant {
                            character_id: char_history.character_id,
                            name: char_history.character_name,
                            hero_class_id: char_history.character_info.hero_class_id,
                            role: Role::from_class_talent_string(char_history.character_info.hero_class_id, &char_history.character_info.talent_specialization.unwrap_or_else(|| String::from(""))),
                        }
                    } else {
                        InstanceViewerParticipant {
                            character_id: *char_id,
                            name: String::from("Unknown"),
                            hero_class_id: 1,
                            role: Role::Dps,
                        }
                    }
                })
                .collect());
        }
        Err(InstanceFailure::InvalidInput)
    }

    fn get_instance_attempts(&self, db_main: &mut impl Select, instance_meta_id: u32) -> Result<Vec<InstanceViewerAttempt>, InstanceFailure> {
        // Validate that the instance meta exists
        {
            let instance_metas = self.instance_metas.read().unwrap();
            instance_metas.1.get(&instance_meta_id).ok_or(InstanceFailure::InvalidInput)?;
        }

        let attempts: Vec<InstanceViewerAttempt> = db_main
            .select_wparams(
                "SELECT id, encounter_id, start_ts, end_ts, is_kill FROM `instance_attempt` WHERE instance_meta_id=:instance_meta_id",
                |mut row| InstanceViewerAttempt {
                    id: row.take(0).unwrap(),
                    encounter_id: row.take(1).unwrap(),
                    start_ts: row.take(2).unwrap(),
                    end_ts: row.take(3).unwrap(),
                    is_kill: row.take(4).unwrap(),
                },
                params!("instance_meta_id" => instance_meta_id),
            )
            .into_iter()
            .collect();

        Ok(attempts)
    }
}
