use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
    pub pairs: Vec<String>,
    pub mode: String,
    pub length: String,
    pub seed: Option<u64>,
    pub adaptive: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPlan {
    pub mode: String,
    pub pairs: Vec<String>,
    pub sections: Vec<PracticeSection>,
    pub seed: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PracticeSection {
    pub name: String,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub theme: String,
    pub font_size: u8,
    pub stop_on_error: bool,
    pub show_keyboard: bool,
    pub show_live_wpm: bool,
    pub adaptive: bool,
    pub allow_backspace: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            font_size: 34,
            stop_on_error: false,
            show_keyboard: false,
            show_live_wpm: true,
            adaptive: true,
            allow_backspace: true,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PairStats {
    pub sessions: u64,
    pub correct: u64,
    pub errors: u64,
    pub total_ms: u64,
    pub typed: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TimingStat {
    pub total_ms: u64,
    pub samples: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SessionSummary {
    pub timestamp_ms: u64,
    pub mode: String,
    pub pairs: Vec<String>,
    pub wpm: f64,
    pub accuracy: f64,
    pub duration_ms: u64,
}
impl Default for SessionSummary {
    fn default() -> Self {
        Self {
            timestamp_ms: 0,
            mode: String::new(),
            pairs: vec![],
            wpm: 0.0,
            accuracy: 0.0,
            duration_ms: 0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppData {
    pub settings: Settings,
    pub sessions_completed: u64,
    pub total_practice_ms: u64,
    pub total_correct: u64,
    pub total_errors: u64,
    pub pair_stats: BTreeMap<String, PairStats>,
    pub key_errors: BTreeMap<String, u64>,
    pub transition_errors: BTreeMap<String, u64>,
    pub transition_counts: BTreeMap<String, u64>,
    pub key_timings: BTreeMap<String, TimingStat>,
    pub transition_timings: BTreeMap<String, TimingStat>,
    pub recent_sessions: Vec<SessionSummary>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    pub timestamp_ms: u64,
    pub mode: String,
    pub pairs: Vec<String>,
    pub wpm: f64,
    pub accuracy: f64,
    pub duration_ms: u64,
    pub correct: u64,
    pub errors: u64,
    pub key_errors: BTreeMap<String, u64>,
    pub transition_errors: BTreeMap<String, u64>,
    pub transition_counts: BTreeMap<String, u64>,
    pub key_timings: BTreeMap<String, TimingStat>,
    pub transition_timings: BTreeMap<String, TimingStat>,
}

impl AppData {
    pub fn absorb(&mut self, r: SessionRecord) {
        self.sessions_completed += 1;
        self.total_practice_ms += r.duration_ms;
        self.total_correct += r.correct;
        self.total_errors += r.errors;
        for pair in &r.pairs {
            let s = self.pair_stats.entry(pair.clone()).or_default();
            s.sessions += 1;
            s.correct += r.correct;
            s.errors += r.errors;
            s.total_ms += r.duration_ms;
            s.typed += r.correct + r.errors;
        }
        merge_counts(&mut self.key_errors, r.key_errors);
        merge_counts(&mut self.transition_errors, r.transition_errors);
        merge_counts(&mut self.transition_counts, r.transition_counts);
        merge_times(&mut self.key_timings, r.key_timings);
        merge_times(&mut self.transition_timings, r.transition_timings);
        self.recent_sessions.insert(
            0,
            SessionSummary {
                timestamp_ms: r.timestamp_ms,
                mode: r.mode,
                pairs: r.pairs,
                wpm: r.wpm,
                accuracy: r.accuracy,
                duration_ms: r.duration_ms,
            },
        );
        self.recent_sessions.truncate(20);
    }
}
fn merge_counts(a: &mut BTreeMap<String, u64>, b: BTreeMap<String, u64>) {
    for (k, v) in b {
        *a.entry(k).or_default() += v;
    }
}
fn merge_times(a: &mut BTreeMap<String, TimingStat>, b: BTreeMap<String, TimingStat>) {
    for (k, v) in b {
        let x = a.entry(k).or_default();
        x.total_ms += v.total_ms;
        x.samples += v.samples;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aggregation_is_bounded_and_correct() {
        let mut d = AppData::default();
        for i in 0..25 {
            d.absorb(SessionRecord {
                timestamp_ms: i,
                mode: "mixed".into(),
                pairs: vec!["fj".into()],
                wpm: 30.0,
                accuracy: 90.0,
                duration_ms: 1000,
                correct: 9,
                errors: 1,
                key_errors: BTreeMap::from([("f".into(), 1)]),
                transition_errors: BTreeMap::new(),
                transition_counts: BTreeMap::new(),
                key_timings: BTreeMap::new(),
                transition_timings: BTreeMap::new(),
            });
        }
        assert_eq!(d.sessions_completed, 25);
        assert_eq!(d.key_errors["f"], 25);
        assert_eq!(d.recent_sessions.len(), 20);
    }
}
