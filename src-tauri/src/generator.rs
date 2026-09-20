use crate::model::{AppData, GenerateRequest, PracticeSection, SessionPlan};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

const PAIRS: &[&str] = &[
    "fj", "dk", "sl", "a;", "gh", "ru", "ei", "wo", "qy", "tp", "vm", "c,", "x.", "z/", "bn",
];
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn pick<T: Copy>(&mut self, x: &[T]) -> T {
        x[(self.next() as usize) % x.len()]
    }
    fn range(&mut self, a: usize, b: usize) -> usize {
        a + (self.next() as usize % (b - a))
    }
}

pub fn generate(req: &GenerateRequest, data: &AppData) -> Result<SessionPlan, String> {
    if req.pairs.is_empty() {
        return Err("Select at least one key pair".into());
    }
    let mut seen = BTreeSet::new();
    let mut pairs = Vec::new();
    for raw in &req.pairs {
        let p = raw.to_lowercase();
        if !PAIRS.contains(&p.as_str()) {
            return Err(format!("Unknown key pair: {raw}"));
        }
        if seen.insert(p.clone()) {
            pairs.push(p)
        }
    }
    if req.mode == "standalone" && pairs.len() != 1 {
        return Err("Standalone practice requires exactly one pair".into());
    }
    if req.mode != "standalone" && req.mode != "mixed" {
        return Err("Unknown practice mode".into());
    }
    let seed = req.seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    });
    let mut rng = Rng::new(seed);
    let total = match req.length.as_str() {
        "short" => 24,
        "medium" => 48,
        "long" => 90,
        "endless" => 48,
        _ => return Err("Unknown session length".into()),
    };
    let names = [
        "Warm-up",
        "Basic patterns",
        "Alternation",
        "Transition practice",
        "Difficult combinations",
        "Longer sequences",
    ];
    let mut counts = [total / 6; 6];
    for count in counts.iter_mut().take(total % 6) {
        *count += 1
    }
    let weights = transition_weights(&pairs, data, req.adaptive);
    let mut used = BTreeSet::new();
    let mut sections = Vec::new();
    for (stage, name) in names.iter().enumerate() {
        let mut lines = Vec::new();
        for _ in 0..counts[stage] {
            let mut line = if req.mode == "standalone" {
                standalone(&pairs[0], stage, &mut rng)
            } else {
                mixed(&pairs, stage, &weights, &mut rng)
            };
            for _ in 0..5 {
                if used.insert(line.clone()) {
                    break;
                }
                line = if req.mode == "standalone" {
                    standalone(&pairs[0], stage, &mut rng)
                } else {
                    mixed(&pairs, stage, &weights, &mut rng)
                }
            }
            lines.push(line)
        }
        sections.push(PracticeSection {
            name: (*name).into(),
            lines,
        })
    }
    Ok(SessionPlan {
        mode: req.mode.clone(),
        pairs,
        sections,
        seed,
    })
}

fn groups<F: FnMut() -> String>(n: usize, mut f: F) -> String {
    (0..n).map(|_| f()).collect::<Vec<_>>().join(" ")
}
fn standalone(pair: &str, stage: usize, rng: &mut Rng) -> String {
    let c: Vec<char> = pair.chars().collect();
    groups(if stage < 2 { 6 } else { 5 }, || {
        let len = rng.range(2, 7);
        match stage {
            0 => std::iter::repeat_n(rng.pick(&c), len).collect(),
            1 => (0..len)
                .map(|i| c[(i + (rng.next() as usize & 1)) % 2])
                .collect(),
            2 => {
                let odd = rng.range(0, len);
                (0..len)
                    .map(|i| if i == odd { c[1] } else { c[0] })
                    .collect()
            }
            3 => (0..len)
                .map(|i| c[((i / 2) + (rng.next() as usize & 1)) % 2])
                .collect(),
            _ => structured(&c, len, rng, None),
        }
    })
}

fn mixed(pairs: &[String], stage: usize, weights: &BTreeMap<String, u64>, rng: &mut Rng) -> String {
    let keys: Vec<char> = pairs.iter().flat_map(|p| p.chars()).collect();
    let n = if stage < 2 { 6 } else { 5 };
    groups(n, || {
        let len = rng.range(2, 7);
        match stage {
            0 => {
                let p = &pairs[rng.range(0, pairs.len())];
                let pc: Vec<char> = p.chars().collect();
                (0..len)
                    .map(|i| if i < 2 { pc[i % 2] } else { rng.pick(&keys) })
                    .collect()
            }
            1 => (0..len)
                .map(|i| {
                    let side = i % 2;
                    let p = &pairs[rng.range(0, pairs.len())];
                    p.chars().nth(side).unwrap()
                })
                .collect(),
            2 => pair_transition(pairs, len, rng),
            3 => cross_pairs(pairs, len, rng),
            4 | 5 => structured(
                &keys,
                if stage == 5 { len + 3 } else { len },
                rng,
                Some(weights),
            ),
            _ => unreachable!(),
        }
    })
}
fn pair_transition(pairs: &[String], len: usize, rng: &mut Rng) -> String {
    let mut out = String::new();
    let mut order: (usize, usize) = (rng.range(0, pairs.len()), rng.range(0, pairs.len()));
    if pairs.len() > 1 && order.0 == order.1 {
        order.1 = (order.1 + 1) % pairs.len()
    }
    let a: Vec<char> = pairs[order.0].chars().collect();
    let b: Vec<char> = pairs[order.1].chars().collect();
    for i in 0..len {
        out.push(if i % 2 == 0 {
            a[(i / 2) % 2]
        } else {
            b[(i / 2) % 2]
        })
    }
    out
}
fn cross_pairs(pairs: &[String], len: usize, rng: &mut Rng) -> String {
    let left: Vec<char> = pairs.iter().map(|p| p.chars().next().unwrap()).collect();
    let right: Vec<char> = pairs.iter().map(|p| p.chars().nth(1).unwrap()).collect();
    (0..len)
        .map(|i| rng.pick(if (i / 2) % 2 == 0 { &left } else { &right }))
        .collect()
}
fn structured(
    keys: &[char],
    len: usize,
    rng: &mut Rng,
    weights: Option<&BTreeMap<String, u64>>,
) -> String {
    let mut out = String::new();
    out.push(rng.pick(keys));
    while out.len() < len {
        let prev = out.chars().last().unwrap();
        let mut pool = Vec::new();
        for &k in keys {
            let edge = format!("{prev}{k}");
            let w = weights
                .and_then(|x| x.get(&edge))
                .copied()
                .unwrap_or(1)
                .min(3);
            for _ in 0..w {
                pool.push(k)
            }
        }
        out.push(rng.pick(&pool))
    }
    out
}
fn transition_weights(pairs: &[String], data: &AppData, adaptive: bool) -> BTreeMap<String, u64> {
    if !adaptive {
        return BTreeMap::new();
    }
    let allowed: BTreeSet<char> = pairs.iter().flat_map(|p| p.chars()).collect();
    data.transition_errors
        .iter()
        .filter_map(|(k, &errors)| {
            let cs: Vec<char> = k.chars().collect();
            (cs.len() == 2 && cs.iter().all(|c| allowed.contains(c))).then(|| {
                let attempts = data
                    .transition_counts
                    .get(k)
                    .copied()
                    .unwrap_or(errors)
                    .max(1);
                let rate = errors as f64 / attempts as f64;
                let timing = data
                    .transition_timings
                    .get(k)
                    .and_then(|t| t.total_ms.checked_div(t.samples))
                    .unwrap_or(0);
                (
                    k.clone(),
                    (1 + (rate > 0.08) as u64 + (rate > 0.20 || timing > 450) as u64).min(3),
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn req(p: &[&str], mode: &str, seed: u64) -> GenerateRequest {
        GenerateRequest {
            pairs: p.iter().map(|x| x.to_string()).collect(),
            mode: mode.into(),
            length: "short".into(),
            seed: Some(seed),
            adaptive: false,
        }
    }
    fn text(plan: &SessionPlan) -> String {
        plan.sections
            .iter()
            .flat_map(|s| &s.lines)
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    }
    #[test]
    fn empty_is_safe() {
        assert!(generate(&req(&[], "mixed", 1), &AppData::default()).is_err())
    }
    #[test]
    fn standalone_uses_only_pair() {
        let t = text(&generate(&req(&["fj"], "standalone", 2), &AppData::default()).unwrap());
        assert!(t.chars().all(|c| c == 'f' || c == 'j' || c == ' '))
    }
    #[test]
    fn mixed_never_uses_unselected_keys_and_represents_all() {
        let t =
            text(&generate(&req(&["fj", "dk", "a;"], "mixed", 3), &AppData::default()).unwrap());
        assert!(t.chars().all(|c| "fjdka; ".contains(c)));
        for c in "fjdka;".chars() {
            assert!(t.matches(c).count() > 8, "missing {c}")
        }
    }
    #[test]
    fn punctuation_works() {
        let t =
            text(&generate(&req(&["c,", "x.", "z/"], "mixed", 4), &AppData::default()).unwrap());
        assert!(t.contains(',') && t.contains('.') && t.contains('/'))
    }
    #[test]
    fn seeds_vary_output() {
        assert_ne!(
            text(&generate(&req(&["fj", "dk"], "mixed", 5), &AppData::default()).unwrap()),
            text(&generate(&req(&["fj", "dk"], "mixed", 6), &AppData::default()).unwrap())
        )
    }
    #[test]
    fn adaptive_increases_problem_transition_and_normalizes() {
        let mut d = AppData::default();
        d.transition_errors.insert("fd".into(), 20);
        d.transition_counts.insert("fd".into(), 20);
        let mut r = req(&["fj", "dk"], "mixed", 7);
        r.length = "long".into();
        let plain = text(&generate(&r, &d).unwrap()).matches("fd").count();
        r.adaptive = true;
        let weighted = text(&generate(&r, &d).unwrap()).matches("fd").count();
        assert!(weighted > plain, "{weighted} not > {plain}");
        d.transition_counts.insert("fd".into(), 1000);
        assert_eq!(transition_weights(&r.pairs, &d, true)["fd"], 1)
    }
}
