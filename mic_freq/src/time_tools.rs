//! Clocks independent of rendering: monotonic durations and a wall-clock alarm.
use chrono::{DateTime, Duration as CalendarDuration, Local, LocalResult, NaiveTime, TimeZone};
use std::time::{Duration, Instant};

pub fn parse_duration(text: &str) -> Result<Duration, &'static str> {
    let parts: Vec<_> = text.split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return Err("Use MM:SS or HH:MM:SS");
    }
    let mut seconds = 0u64;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() || part.len() > 2 || !part.bytes().all(|b| b.is_ascii_digit()) {
            return Err("Use two-digit time fields");
        }
        let n: u64 = part.parse().map_err(|_| "Invalid duration")?;
        if i > 0 && n >= 60 {
            return Err("Minutes and seconds must be below 60");
        }
        seconds = seconds * 60 + n;
    }
    if seconds == 0 {
        return Err("Choose a duration above zero");
    }
    Ok(Duration::from_secs(seconds))
}

pub fn display_duration(duration: Duration, hundredths: bool) -> String {
    let s = duration.as_secs();
    let base = format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60);
    if hundredths {
        format!("{base}.{:02}", duration.subsec_millis() / 10)
    } else {
        base
    }
}

/// Resolve the next occurrence of a local time, including ambiguous DST hours.
pub fn next_alarm(text: &str, now: DateTime<Local>) -> Result<DateTime<Local>, &'static str> {
    if text.len() != 5
        || text.as_bytes()[2] != b':'
        || !text
            .bytes()
            .enumerate()
            .all(|(i, b)| i == 2 || b.is_ascii_digit())
    {
        return Err("Use local 24-hour time HH:MM");
    }
    let time =
        NaiveTime::parse_from_str(text, "%H:%M").map_err(|_| "Use local 24-hour time HH:MM")?;
    for offset in 0..=2 {
        let date = now.date_naive() + CalendarDuration::days(offset);
        let candidates = match Local.from_local_datetime(&date.and_time(time)) {
            LocalResult::Single(t) => vec![t],
            LocalResult::Ambiguous(a, b) => vec![a, b],
            LocalResult::None => vec![],
        };
        if let Some(next) = candidates.into_iter().filter(|t| *t > now).min() {
            return Ok(next);
        }
    }
    Err("That local time does not occur in the next two days")
}

#[derive(Default)]
pub struct Alarm {
    pub due: Option<DateTime<Local>>,
    pub ringing: bool,
}

impl Alarm {
    pub fn arm(&mut self, text: &str, now: DateTime<Local>) -> Result<(), &'static str> {
        self.due = Some(next_alarm(text, now)?);
        self.ringing = false;
        Ok(())
    }
    pub fn tick(&mut self, now: DateTime<Local>) -> bool {
        if self.due.is_some_and(|due| now >= due) {
            self.due = None;
            self.ringing = true;
            true
        } else {
            false
        }
    }
    pub fn snooze(&mut self, now: DateTime<Local>) {
        self.due = Some(now + CalendarDuration::minutes(5));
        self.ringing = false;
    }
}

#[derive(Default)]
pub struct Timer {
    remaining: Duration,
    started: Option<Instant>,
    pub ringing: bool,
}

impl Timer {
    pub fn remaining(&self, now: Instant) -> Duration {
        self.remaining.saturating_sub(
            self.started
                .map_or(Duration::ZERO, |t| now.saturating_duration_since(t)),
        )
    }
    pub fn running(&self) -> bool {
        self.started.is_some()
    }
    pub fn start(&mut self, duration: Duration, now: Instant) {
        self.remaining = duration;
        self.started = Some(now);
        self.ringing = false;
    }
    pub fn pause(&mut self, now: Instant) {
        self.remaining = self.remaining(now);
        self.started = None;
    }
    pub fn resume(&mut self, now: Instant) {
        if !self.remaining.is_zero() {
            self.started = Some(now);
        }
    }
    pub fn tick(&mut self, now: Instant) -> bool {
        if self.running() && self.remaining(now).is_zero() {
            self.remaining = Duration::ZERO;
            self.started = None;
            self.ringing = true;
            true
        } else {
            false
        }
    }
}

#[derive(Default)]
pub struct Stopwatch {
    elapsed: Duration,
    started: Option<Instant>,
    pub laps: Vec<Duration>,
    pub lap_count: usize,
}

impl Stopwatch {
    pub fn elapsed(&self, now: Instant) -> Duration {
        self.elapsed
            + self
                .started
                .map_or(Duration::ZERO, |t| now.saturating_duration_since(t))
    }
    pub fn running(&self) -> bool {
        self.started.is_some()
    }
    pub fn toggle(&mut self, now: Instant) {
        if self.running() {
            self.elapsed = self.elapsed(now);
            self.started = None;
        } else {
            self.started = Some(now);
        }
    }
    pub fn lap(&mut self, now: Instant) {
        if self.running() {
            self.lap_count += 1;
            self.laps.insert(0, self.elapsed(now));
            self.laps.truncate(4);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn durations_validate_ranges_and_format() {
        assert_eq!(parse_duration("01:02:03").unwrap().as_secs(), 3723);
        assert_eq!(parse_duration("90:00").unwrap().as_secs(), 5400);
        for invalid in ["00:00", "-1:20", "1:60", "1:99:00", "1", "999:00", "a:20"] {
            assert!(parse_duration(invalid).is_err());
        }
        assert_eq!(
            display_duration(Duration::from_millis(3723456), true),
            "01:02:03.45"
        );
    }
    #[test]
    fn countdown_pauses_resumes_and_fires_only_once_after_a_stall() {
        let now = Instant::now();
        let mut timer = Timer::default();
        timer.start(Duration::from_secs(10), now);
        timer.pause(now + Duration::from_secs(3));
        assert_eq!(
            timer.remaining(now + Duration::from_secs(100)),
            Duration::from_secs(7)
        );
        timer.resume(now + Duration::from_secs(100));
        assert!(!timer.tick(now + Duration::from_secs(106)));
        assert!(timer.tick(now + Duration::from_secs(200)));
        assert!(!timer.tick(now + Duration::from_secs(201)));
    }
    #[test]
    fn stopwatch_excludes_paused_time_and_bounds_laps() {
        let now = Instant::now();
        let mut watch = Stopwatch::default();
        watch.toggle(now);
        watch.toggle(now + Duration::from_secs(2));
        watch.toggle(now + Duration::from_secs(10));
        for second in 11..20 {
            watch.lap(now + Duration::from_secs(second));
        }
        assert_eq!(watch.elapsed(now + Duration::from_secs(20)).as_secs(), 12);
        assert_eq!(watch.laps.len(), 4);
        assert_eq!(watch.lap_count, 9);
    }
    #[test]
    fn alarm_rolls_forward_fires_once_and_snoozes() {
        let now = Local::now();
        let text = now.format("%H:%M").to_string();
        let mut alarm = Alarm::default();
        alarm.arm(&text, now).unwrap();
        assert!(alarm.due.unwrap() > now);
        assert!(!alarm.tick(now));
        assert!(alarm.tick(alarm.due.unwrap() + CalendarDuration::seconds(1)));
        assert!(!alarm.tick(now + CalendarDuration::days(3)));
        alarm.snooze(now);
        assert!(!alarm.ringing);
        assert!(alarm.tick(now + CalendarDuration::minutes(5)));
        for invalid in ["24:00", "12:60", "1:00", "hello"] {
            assert!(next_alarm(invalid, now).is_err());
        }
    }
}
