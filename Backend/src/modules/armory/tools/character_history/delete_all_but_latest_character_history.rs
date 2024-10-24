use crate::params;
use crate::util::database::*;
use crate::modules::armory::{dto::ArmoryFailure, Armory};

pub trait DeleteMultipleCharacterHistory {
    fn delete_all_but_latest_character_history(&self, db_main: &mut (impl Execute + Select), character_id: u32) -> Result<(), ArmoryFailure>;
}

impl DeleteMultipleCharacterHistory for Armory {
    fn delete_all_but_latest_character_history(&self, db_main: &mut (impl Execute + Select), character_id: u32) -> Result<(), ArmoryFailure> {
        let mut characters = self.characters.write().unwrap();

        if let Some(character) = characters.get(&character_id) {
            if character.history_moments.len() <= 1 {
                return Ok(()); // No need to delete anything if there's only one or no history moments.
            }

            // Keep the most recent history moment
            let latest_history_id = character.history_moments.last().unwrap().id;

            // Delete all other history moments from the database
            let delete_query = "DELETE FROM armory_character_history WHERE character_id = :character_id AND id != :latest_id";
            if db_main.execute_wparams(
                delete_query,
                params! {
                    "character_id" => character_id,
                    "latest_id" => latest_history_id
                },
            ) {
                let character = characters.get_mut(&character_id).unwrap();
                // Keep only the latest history moment in memory
                character.history_moments.retain(|history_moment| history_moment.id == latest_history_id);

                return Ok(());
            }
            return Err(ArmoryFailure::Database("delete_character_history".to_owned()));
        }

        Err(ArmoryFailure::InvalidInput)
    }
}
