use controls_module::controls::Controls;
use controls_module::models::{
    AlbumSimple, DiscoverPage, DiscoverSection, FavoriteIds, PlaylistSimple,
};
use futures::future::try_join_all;
use player_module::AppResult;
use player_module::client::{GenrePlaylistSlug, StreamClient};
use player_module::error::PlayerError;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    prelude::*,
    widgets::{ListState, Paragraph},
};

use crate::image_cache::ImageManager;
use crate::ui::{Pane, leaves_content, sidebar};
use crate::widgets::grid::Grid;
use crate::{
    app::{NotificationList, Output},
    ui::block,
};

/// An album section of a discover page, extended a page at a time as the selection reaches its end.
pub struct AlbumSection {
    pub title: &'static str,
    section: DiscoverSection,
    pub grid: Grid<AlbumSimple>,
    has_more: bool,
}

impl AlbumSection {
    /// The album sections of a discover page, in the order the page shows them.
    pub fn all(discover: DiscoverPage) -> Vec<Self> {
        [
            (
                "New releases",
                DiscoverSection::NewReleases,
                discover.new_releases,
            ),
            (
                "Qobuzissime",
                DiscoverSection::Qobuzissims,
                discover.qobuzissims,
            ),
            (
                "Essential Discography",
                DiscoverSection::IdealDiscography,
                discover.ideal_discography,
            ),
            (
                "Album of the week",
                DiscoverSection::AlbumOfTheWeek,
                discover.album_of_the_week,
            ),
            (
                "Press Accolades",
                DiscoverSection::PressAwards,
                discover.press_awards,
            ),
            (
                "Most streamed",
                DiscoverSection::MostStreamed,
                discover.most_streamed,
            ),
        ]
        .into_iter()
        .map(|(title, section, albums)| Self {
            title,
            section,
            grid: Grid::new(albums),
            has_more: true,
        })
        .collect()
    }

    /// Fetches the next page once a move put the selection on the last row.
    pub async fn load_more(
        &mut self,
        key: KeyCode,
        client: &StreamClient,
        genre_id: Option<u32>,
    ) -> AppResult<()> {
        let moved = matches!(
            key,
            KeyCode::Down | KeyCode::Right | KeyCode::Char('j' | 'l')
        );
        if !moved || !self.has_more || !self.grid.at_last_row() {
            return Ok(());
        }
        let offset = self.grid.all_items().len();
        let page = client
            .discover_section(self.section, genre_id, offset)
            .await?;
        self.has_more = page.has_more;
        self.grid.extend(page.albums);
        Ok(())
    }
}

#[derive(Default)]
pub struct DiscoverState {
    featured_albums: Vec<AlbumSection>,
    featured_playlists: Vec<(String, Grid<PlaylistSimple>)>,
    selected_sub_tab: usize,
    focus: Pane,
    pub loading: bool,
    pub loaded: bool,
}

impl DiscoverState {
    pub async fn new(client: &StreamClient) -> AppResult<Self> {
        let mut discover = client.discover_page(None).await?;
        let tags = std::mem::take(&mut discover.playlists_tags);

        let featured_playlists = try_join_all(tags.into_iter().map(|tag| async {
            let playlists = client
                .genre_playlists(GenrePlaylistSlug {
                    genre_id: None,
                    playlist_slug: Some(tag.clone().slug),
                })
                .await?;

            Ok::<_, PlayerError>((tag.name, Grid::new(playlists)))
        }))
        .await?;

        Ok(Self {
            featured_albums: AlbumSection::all(discover),
            featured_playlists,
            selected_sub_tab: 0,
            focus: Pane::default(),
            loading: false,
            loaded: true,
        })
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        favorites: &FavoriteIds,
        image_cache: &mut ImageManager,
    ) {
        let block = block(None);
        frame.render_widget(block, area);

        let tab_content_area = area.inner(Margin::new(1, 1));

        if !self.loaded {
            frame.render_widget(Paragraph::new("Loading..."), tab_content_area);
            return;
        }

        let labels = self
            .featured_albums
            .iter()
            .map(|section| section.title)
            .chain(
                self.featured_playlists
                    .iter()
                    .map(|(label, _)| label.as_str()),
            )
            .collect::<Vec<_>>();

        let (sidebar, sidebar_width) = sidebar(labels, self.focus == Pane::Sidebar);

        let [sidebar_area, content_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(sidebar_width), Constraint::Min(1)])
            .areas(tab_content_area);

        let mut sidebar_state = ListState::default();
        sidebar_state.select(Some(self.selected_sub_tab));

        frame.render_stateful_widget(sidebar, sidebar_area, &mut sidebar_state);

        let content_focused = self.focus == Pane::Content;

        if let Some(section) = self.selected_album_mut() {
            section.grid.render(
                content_area,
                frame.buffer_mut(),
                content_focused,
                &favorites.albums,
                image_cache,
            );
        } else if let Some((_, list)) = self.selected_playlist_mut() {
            list.render(
                content_area,
                frame.buffer_mut(),
                content_focused,
                &favorites.playlists,
                image_cache,
            );
        }
    }

    pub async fn handle_events(
        &mut self,
        event: Event,
        client: &StreamClient,
        controls: &Controls,
        notifications: &mut NotificationList,
    ) -> AppResult<Output> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => match self.focus {
                Pane::Sidebar => match key_event.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.cycle_subtab_backwards();
                        Ok(Output::Consumed)
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.cycle_subtab();
                        Ok(Output::Consumed)
                    }
                    KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                        self.focus = Pane::Content;
                        Ok(Output::Consumed)
                    }
                    _ => Ok(Output::NotConsumed),
                },
                Pane::Content => match key_event.code {
                    KeyCode::Esc => {
                        self.focus = Pane::Sidebar;
                        Ok(Output::Consumed)
                    }
                    code => {
                        let output = self
                            .handle_content_events(code, client, controls, notifications)
                            .await?;

                        if leaves_content(code, &output) {
                            self.focus = Pane::Sidebar;
                            return Ok(Output::Consumed);
                        }

                        Ok(output)
                    }
                },
            },
            _ => Ok(Output::NotConsumed),
        }
    }

    async fn handle_content_events(
        &mut self,
        key_code: KeyCode,
        client: &StreamClient,
        controls: &Controls,
        notifications: &mut NotificationList,
    ) -> AppResult<Output> {
        if let Some(section) = self.selected_album_mut() {
            let output = section
                .grid
                .handle_events(key_code, client, controls, notifications)
                .await?;
            section.load_more(key_code, client, None).await?;
            return Ok(output);
        }

        if let Some((_, list)) = self.selected_playlist_mut() {
            return list
                .handle_events(key_code, client, controls, notifications)
                .await;
        }

        Ok(Output::NotConsumed)
    }

    fn selected_album_mut(&mut self) -> Option<&mut AlbumSection> {
        self.featured_albums.get_mut(self.selected_sub_tab)
    }

    fn selected_playlist_mut(&mut self) -> Option<&mut (String, Grid<PlaylistSimple>)> {
        let index = self
            .selected_sub_tab
            .checked_sub(self.featured_albums.len())?;

        self.featured_playlists.get_mut(index)
    }

    fn cycle_subtab_backwards(&mut self) {
        let count = self
            .featured_albums
            .len()
            .saturating_add(self.featured_playlists.len());

        if count == 0 {
            return;
        }

        self.selected_sub_tab = self
            .selected_sub_tab
            .checked_sub(1)
            .unwrap_or_else(|| count.saturating_sub(1))
            .checked_rem(count)
            .unwrap_or(0);
    }

    fn cycle_subtab(&mut self) {
        let count = self
            .featured_albums
            .len()
            .saturating_add(self.featured_playlists.len());

        if count == 0 {
            return;
        }

        self.selected_sub_tab = self
            .selected_sub_tab
            .checked_add(1)
            .and_then(|value| value.checked_rem(count))
            .unwrap_or(0);
    }
}
