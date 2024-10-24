use crate::util::database::*;

use crate::{
    dto::CheckPlausability,
    modules::armory::{
        dto::{ArmoryFailure, CharacterHistoryDto},
        material::CharacterHistory,
        tools::{CreateCharacterHistory, CreateGuild, GetCharacter},
        Armory,
    },
};
use crate::modules::armory::tools::character_history::delete_all_but_latest_character_history::DeleteMultipleCharacterHistory;

pub trait SetCharacterHistory {
    fn set_character_history(&self, db_main: &mut (impl Execute + Select), server_id: u32, update_character_history: CharacterHistoryDto, character_uid: u64, timestamp: u64) -> Result<CharacterHistory, ArmoryFailure>;
}

impl SetCharacterHistory for Armory {
    fn set_character_history(&self, db_main: &mut (impl Execute + Select), server_id: u32, update_character_history: CharacterHistoryDto, character_uid: u64, timestamp: u64) -> Result<CharacterHistory, ArmoryFailure> {
        // Validation
        if !update_character_history.is_plausible() {
            return Err(ArmoryFailure::ImplausibleInput);
        }

        // Check if this character exists
        let character_id_res = self.get_character_id_by_uid(server_id, character_uid);
        if character_id_res.is_none() {
            return Err(ArmoryFailure::InvalidInput);
        }
        let character_id = character_id_res.unwrap();

        update_character_history
            .character_guild
            .as_ref()
            .and_then(|chr_guild_dto| self.create_guild(db_main, server_id, chr_guild_dto.guild.clone()).ok().map(|gld| gld.id));


        let result = self.delete_all_but_latest_character_history(db_main, character_id);
        if !result.is_ok() {
            return Err(ArmoryFailure::Database("delete_character_history".to_owned()));
        }

        self.create_character_history(db_main, server_id, update_character_history, character_uid, timestamp)
    }
}
