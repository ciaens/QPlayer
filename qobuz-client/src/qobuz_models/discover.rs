use serde::{Deserialize, Serialize};

use crate::qobuz_models::{album_suggestion::AlbumSuggestion, playlist::PlaylistSimple};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Discover {
    pub containers: Containers,
}

/// The album sections of the discover page; each has an endpoint that pages past what the index shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiscoverSection {
    NewReleases,
    Qobuzissims,
    IdealDiscography,
    AlbumOfTheWeek,
    MostStreamed,
    PressAwards,
}

impl DiscoverSection {
    #[must_use]
    pub const fn path(self) -> &'static str {
        match self {
            Self::NewReleases => "discover/newReleases",
            Self::Qobuzissims => "discover/qobuzissims",
            Self::IdealDiscography => "discover/idealDiscography",
            Self::AlbumOfTheWeek => "discover/albumOfTheWeek",
            Self::MostStreamed => "discover/mostStreamed",
            Self::PressAwards => "discover/pressAward",
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverSectionPage {
    pub has_more: Option<bool>,
    pub items: Vec<AlbumSuggestion>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Containers {
    pub new_releases: AlbumContainer,
    pub qobuzissims: AlbumContainer,
    pub ideal_discography: AlbumContainer,
    pub album_of_the_week: AlbumContainer,
    pub most_streamed: AlbumContainer,
    pub press_awards: AlbumContainer,
    pub playlists: PlaylistContainer,
    pub playlists_tags: PlaylistTagsContainer,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AlbumContainer {
    pub data: AlbumData,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AlbumData {
    pub items: Vec<AlbumSuggestion>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistContainer {
    pub data: PlaylistData,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistData {
    pub items: Vec<PlaylistSimple>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistTagsContainer {
    pub data: PlaylistTagData,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistTagData {
    pub items: Vec<PlaylistTag>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistTag {
    pub slug: String,
    pub name: String,
}
