#![allow(clippy::type_complexity)]

use crate::modules::live_data_processor::material::interval_bucket::UniqueBucketId;

#[derive(Debug, Clone)]
pub struct Participant {
    pub id: u64,
    pub is_player: bool,
    pub is_self_damage: bool,
    pub is_mind_control: bool,
    pub name: String,
    pub hero_class_id: Option<u8>,
    pub gender_id: Option<bool>,
    pub race_id: Option<u8>,
    pub guild_args: Option<(String, String, u8)>,
    pub talents: Vec<(u64, Option<String>)>,
    pub last_seen_talents: String,
    pub server: Option<(u32, String)>,
    pub gear_setups: Option<Vec<(u64, Vec<Option<(u32, Option<u32>, Option<Vec<Option<u32>>>)>>)>>,
    pub active_intervals: Vec<(u64, u64)>,
    available_effective_heal: u32,

    // Technical
    pub first_seen: u64,
    pub last_seen: u64,
    pub last_brainwash: u64,
}

impl Participant {
    pub fn new(id: u64, is_player: bool, is_self_damage: bool, is_mind_control: bool, name: String, last_seen: u64) -> Self {
        Participant {
            id,
            is_player,
            is_self_damage,
            is_mind_control,
            hero_class_id: None,
            gender_id: None,
            race_id: None,
            name,
            server: None,
            gear_setups: None,
            active_intervals: vec![(last_seen, last_seen)],
            first_seen: last_seen,
            last_seen,
            guild_args: None,
            available_effective_heal: 0,
            talents: Vec::new(),
            last_seen_talents: String::new(),
            last_brainwash: 0,
        }
    }

    pub fn record_talents(&mut self, timestamp: u64, talent_string: &str) {
        if self.last_seen_talents != talent_string {
            if self.last_brainwash > 0 {
                // add an entry right 1s before brainwash with previous talents
                // don't bother marking talents if the last seen talents were empty
                if self.last_seen_talents != "" {
                    self.talents.push((self.last_brainwash - 1000, Some(self.last_seen_talents.clone())));
                }
                self.talents.push((self.last_brainwash, Some(talent_string.to_string())));
                self.last_brainwash = 0; // reset brainwash
            } else {
                self.talents.push((timestamp, Some(talent_string.to_string())));
            }
            self.last_seen_talents = talent_string.to_string();
        }
    }


    // Assumes that now > last_seen
    pub fn add_participation_point(&mut self, now: u64) {
        static PARTICIPATION_TIMEOUT: u64 = 5 * 60000;
        if now > self.last_seen {
            if now - self.last_seen <= PARTICIPATION_TIMEOUT {
                self.active_intervals.last_mut().unwrap().1 = now;
            } else {
                self.active_intervals.last_mut().unwrap().1 = self.last_seen + 30000;
                self.active_intervals.push((now, now));
            }
            self.last_seen = now;
        }
    }

    pub fn attribute_damage(&mut self, damage: u32) {
        self.available_effective_heal += damage;
    }

    pub fn attribute_heal(&mut self, heal: u32) -> u32 {
        let effective_heal;
        if heal > self.available_effective_heal {
            effective_heal = self.available_effective_heal;
            self.available_effective_heal = 0;
        } else {
            self.available_effective_heal -= heal;
            effective_heal = heal;
        }
        effective_heal
    }
}

impl UniqueBucketId for Participant {
    fn get_unique_bucket_id(&self) -> u64 {
        self.id
    }
}
