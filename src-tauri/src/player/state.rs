use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub song_cid: Option<String>,
    pub song_name: Option<String>,
    pub artists: Vec<String>,
    pub cover_url: Option<String>,
    pub is_playing: bool,
    pub is_loading: bool,
    pub progress: f64,
    pub duration: f64,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            song_cid: None,
            song_name: None,
            artists: Vec::new(),
            cover_url: None,
            is_playing: false,
            is_loading: false,
            progress: 0.0,
            duration: 0.0,
        }
    }
}