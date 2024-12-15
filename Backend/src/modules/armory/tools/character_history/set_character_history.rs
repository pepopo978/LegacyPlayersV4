use crate::util::database::*;

use crate::{
    modules::armory::{
        dto::{ArmoryFailure, CharacterHistoryDto},
        material::CharacterHistory,
        tools::{CreateCharacterHistory, CreateGuild, GetCharacter},
        Armory,
    },
};

pub trait SetCharacterHistory {
    fn set_character_history(&self, db_main: &mut (impl Execute + Select), server_id: u32, update_character_history: CharacterHistoryDto, character_uid: u64, timestamp: u64) -> Result<CharacterHistory, ArmoryFailure>;
}

impl SetCharacterHistory for Armory {
    fn set_character_history(&self, db_main: &mut (impl Execute + Select), server_id: u32, update_character_history: CharacterHistoryDto, character_uid: u64, timestamp: u64) -> Result<CharacterHistory, ArmoryFailure> {
        // Check if this character exists
        let character_id_res = self.get_character_id_by_uid(server_id, character_uid);
        if character_id_res.is_none() {
            return Err(ArmoryFailure::InvalidInput);
        }

        update_character_history
            .character_guild
            .as_ref()
            .and_then(|chr_guild_dto| self.create_guild(db_main, server_id, chr_guild_dto.guild.clone()).ok().map(|gld| gld.id));

        // delete any history in the database with the same timestamp
        let character_id = character_id_res.unwrap();

        let query = format!("DELETE FROM armory_character_history WHERE character_id = {} AND timestamp = {}", character_id, timestamp/1000);
        db_main.execute_one(&query);

        self.create_character_history(db_main, server_id, update_character_history, character_uid, timestamp)
    }
}
