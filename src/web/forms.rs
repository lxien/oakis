use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BatchIdsForm {
    #[serde(default)]
    pub ids: String,
}

#[derive(Debug, Deserialize)]
pub struct BatchTaxDeleteForm {
    pub kind: String,
    #[serde(default)]
    pub ids: String,
}

pub fn parse_ids(s: &str) -> Vec<i64> {
    s.split(',').filter_map(|x| x.trim().parse().ok()).collect()
}
