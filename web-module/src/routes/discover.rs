use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use controls_module::models::{AlbumSimple, DiscoverPage, DiscoverSection};
use player_module::client::GenrePlaylistSlug;
use serde::Deserialize;
use serde_json::json;
use tokio::try_join;

use crate::{AppState, ResponseResult, ok_or_error_page};

pub fn routes() -> Router<std::sync::Arc<crate::AppState>> {
    Router::new()
        .route("/discover", get(genre_page))
        .route("/discover/playlist", get(playlist_partial))
        .route("/discover/section/{section}", get(section_partial))
}

const SECTIONS: [DiscoverSection; 6] = [
    DiscoverSection::NewReleases,
    DiscoverSection::Qobuzissims,
    DiscoverSection::IdealDiscography,
    DiscoverSection::AlbumOfTheWeek,
    DiscoverSection::PressAwards,
    DiscoverSection::MostStreamed,
];

/// The name a section's links use.
const fn name(section: DiscoverSection) -> &'static str {
    match section {
        DiscoverSection::NewReleases => "new-releases",
        DiscoverSection::Qobuzissims => "qobuzissims",
        DiscoverSection::IdealDiscography => "ideal-discography",
        DiscoverSection::AlbumOfTheWeek => "album-of-the-week",
        DiscoverSection::PressAwards => "press-awards",
        DiscoverSection::MostStreamed => "most-streamed",
    }
}

/// The link that fetches the albums of a section beyond the `offset` shown.
fn more_link(section: DiscoverSection, genre_id: Option<u32>, offset: usize) -> String {
    let name = name(section);
    let genre_id = genre_id.map(|id| id.to_string()).unwrap_or_default();
    format!("/discover/section/{name}?offset={offset}&genre_id={genre_id}")
}

fn more_links(discover: &DiscoverPage, genre_id: Option<u32>) -> serde_json::Value {
    let link = |section, albums: &[AlbumSimple]| more_link(section, genre_id, albums.len());
    json!({
        "new_releases": link(DiscoverSection::NewReleases, &discover.new_releases),
        "qobuzissims": link(DiscoverSection::Qobuzissims, &discover.qobuzissims),
        "ideal_discography": link(DiscoverSection::IdealDiscography, &discover.ideal_discography),
        "album_of_the_week": link(DiscoverSection::AlbumOfTheWeek, &discover.album_of_the_week),
        "press_awards": link(DiscoverSection::PressAwards, &discover.press_awards),
        "most_streamed": link(DiscoverSection::MostStreamed, &discover.most_streamed),
    })
}

#[derive(Debug, Deserialize)]
struct GenreQuery {
    genre_id: Option<String>,
}

async fn genre_page(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GenreQuery>,
) -> ResponseResult {
    let genre_id = query
        .genre_id
        .as_deref()
        .and_then(|s| s.parse::<u32>().ok());

    let (discover, genres) = ok_or_error_page(
        &state,
        try_join!(state.client.discover_page(genre_id), state.client.genres(),),
    )?;

    Ok(state.render(
        "discover.html",
        &json! ({
            "more": more_links(&discover, genre_id),
            "selected_genre": genre_id.map(|id| id.to_string()).unwrap_or_default(),
            "discover": discover,
            "active_tab": "discover",
            "genres": genres,
            "playlists": discover.playlists
        }),
    ))
}

#[derive(Deserialize)]
struct SectionQuery {
    genre_id: Option<String>,
    offset: Option<usize>,
}

/// A page of a section's albums, with the link to the next one while there is more.
async fn section_partial(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(query): Query<SectionQuery>,
) -> ResponseResult {
    let Some(section) = SECTIONS
        .into_iter()
        .find(|&section| self::name(section) == name)
    else {
        return Err(StatusCode::NOT_FOUND.into_response());
    };
    let genre_id = query
        .genre_id
        .as_deref()
        .and_then(|s| s.parse::<u32>().ok());
    let offset = query.offset.unwrap_or_default();
    let page = ok_or_error_page(
        &state,
        state
            .client
            .discover_section(section, genre_id, offset)
            .await,
    )?;
    let more = page
        .has_more
        .then(|| more_link(section, genre_id, offset.saturating_add(page.albums.len())));

    Ok(state.render(
        "discover-album-more.html",
        &json!({"albums": page.albums, "more": more}),
    ))
}

#[derive(Deserialize)]
struct PlaylistQuery {
    genre_id: Option<String>,
    playlist_tag_slug: Option<String>,
}

async fn playlist_partial(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PlaylistQuery>,
) -> ResponseResult {
    let genre_id = query
        .genre_id
        .as_deref()
        .and_then(|s| s.parse::<u32>().ok());

    let (discover, playlists) = ok_or_error_page(
        &state,
        try_join!(
            state.client.discover_page(genre_id),
            state.client.genre_playlists(GenrePlaylistSlug {
                genre_id,
                playlist_slug: query.playlist_tag_slug,
            })
        ),
    )?;

    let tags = discover.playlists_tags;

    Ok(state.render(
        "discover-playlists.html",
        &json! ({
            "tags": tags,
            "playlists": playlists,
        }),
    ))
}
