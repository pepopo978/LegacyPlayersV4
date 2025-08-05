use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod tests;

pub fn get_ts_from_now_in_secs(days: u64) -> u64 {
    SystemTime::now().add(Duration::from_secs(days * 24 * 60 * 60)).duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs()
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs()
}

pub fn format_ts_ms(timestamp_ms: u64) -> String {
    let timestamp_secs = timestamp_ms / 1000;
    let milliseconds = timestamp_ms % 1000;
    
    let duration = Duration::from_secs(timestamp_secs);
    let datetime = UNIX_EPOCH + duration;
    
    if let Ok(local_time) = datetime.duration_since(UNIX_EPOCH) {
        let total_seconds = local_time.as_secs();
        let hours = (total_seconds / 3600) % 24;
        let minutes = (total_seconds / 60) % 60;
        let seconds = total_seconds % 60;
        
        format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, milliseconds)
    } else {
        format!("{}ms", timestamp_ms)
    }
}
