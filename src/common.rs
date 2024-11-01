use serde::{Serialize, Deserialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StarData {
    pub star_type: String,
    // Make it compatible with PepPy
    #[serde(rename = "star_id")]
    pub name: String,
}

impl StarData {
    pub fn new(star_type: &String, name: &String) -> StarData {
        StarData {
            star_type: star_type.clone(),
            name: name.clone()
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PepRun {
    pub filters: Vec<u8>,
    pub items: Vec<StarData>,
}

impl PepRun {
    pub fn new(filters: Vec<u8>, items: Vec<StarData>) -> PepRun {
        PepRun {
            filters,
            items,
        }
    }
}