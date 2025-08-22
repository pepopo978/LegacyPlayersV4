use crate::modules::instance::dto::InstanceFailure;
use crate::modules::instance::Instance;
use crate::params;
use crate::util::database::{Execute, Select};

pub trait UnrankAttempt {
    fn unrank_attempt(&self, db_main: &mut (impl Execute + Select), attempt_id: u32) -> Result<(), InstanceFailure>;
}

impl UnrankAttempt for Instance {
    fn unrank_attempt(&self, db_main: &mut (impl Execute + Select), attempt_id: u32) -> Result<(), InstanceFailure> {
        // Query the database to get the instance_meta_id for this attempt
        let i_m_i = db_main
            .select_wparams(
                "SELECT instance_meta_id FROM instance_attempt WHERE id = :attempt_id",
                |mut row| row.take::<u32, usize>(0).unwrap(),
                params!("attempt_id" => attempt_id)
            )
            .into_iter()
            .next()
            .ok_or_else(|| InstanceFailure::InvalidInput)?;

        // Update the database to mark as unrankable
        let _ = db_main.execute_wparams("UPDATE `main`.`instance_attempt` SET rankable = 0 WHERE id=:attempt_id", params!("attempt_id" => attempt_id));

        {
            let mut speed_runs = self.speed_runs.write().unwrap();
            if let Some(index) = speed_runs.iter().position(|speed_run| speed_run.instance_meta_id == i_m_i) {
                speed_runs.remove(index);
            }
        }

        {
            let mut speed_kills = self.speed_kills.write().unwrap();
            if let Some(index) = speed_kills.iter().position(|speed_kill| speed_kill.attempt_id == attempt_id) {
                speed_kills.remove(index);
            }
        }

        Ok(())
    }
}