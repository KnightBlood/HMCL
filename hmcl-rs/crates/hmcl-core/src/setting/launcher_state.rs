//! The state of the launcher the launcher keeps between its starts.
//!
//! The state is what the launcher of Java keeps in
//! `state/launcher-state.json`: the rectangle the window of the launcher had
//! the last time the player moved it, the version the launcher told the
//! player about and the tips the player was already shown, so the launcher
//! does not tell them again.
//!
//! The rectangle is kept the way the launcher of Java keeps it: the position
//! is a share of the size of the primary screen, and the size is in pixels,
//! so the window of the launcher is put in the same place on a screen of
//! another size. The state of a maximized window is not kept; the launcher
//! shows the window in the rectangle the player left it in before.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::schema::{Schema, SchemaVersion};
use crate::error::Error;

/// The schema of the file that holds the state of the launcher.
pub const LAUNCHER_STATE_SCHEMA: &str = "https://schemas.glavo.site/hmcl/launcher-state/1.0.0";

/// The least width the window of the launcher is shown with.
pub const MIN_CONTENT_WIDTH: f64 = 800.0;

/// The least height the window of the launcher is shown with.
///
/// The height of the page area least is 450, plus 40 for the bottom of the
/// pages, which is the way the launcher of Java computes it.
pub const MIN_CONTENT_HEIGHT: f64 = 490.0;

/// The share of a screen an edge of a window must keep next to the edge of
/// the screen for the window to be counted visible.
pub const MIN_VISIBLE_WINDOW_AREA: f64 = 20.0;

/// Returns the schema the state file keeps when the file does name none.
fn default_state_schema() -> String {
    LAUNCHER_STATE_SCHEMA.to_string()
}

/// The state of the launcher the launcher keeps between its starts.
///
/// The state is read from and written to `state/launcher-state.json` of the
/// directory of the launcher, the way the launcher of Java keeps it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LauncherState {
    /// The schema of the file, which the launcher compares with the one it
    /// knows.
    #[serde(rename = "$schema", default = "default_state_schema")]
    schema: String,
    /// The position of the window of the launcher on the primary screen, as
    /// a share of the width of the screen.
    #[serde(default)]
    x: f64,
    /// The position of the window of the launcher on the primary screen, as
    /// a share of the height of the screen.
    #[serde(default)]
    y: f64,
    /// The width of the window of the launcher, in pixels.
    #[serde(default)]
    width: f64,
    /// The height of the window of the launcher, in pixels.
    #[serde(default)]
    height: f64,
    /// The version the launcher told the player about last.
    #[serde(
        rename = "promptedVersion",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    prompted_version: Option<String>,
    /// The tips the player was already shown, by their identifiers.
    #[serde(
        rename = "shownTips",
        default,
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    shown_tips: BTreeMap<String, Value>,
    /// The fields of the state the launcher does not know yet.
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

impl LauncherState {
    /// Creates the state of a launcher that starts the first time.
    pub fn new() -> Self {
        Self {
            schema: LAUNCHER_STATE_SCHEMA.to_string(),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            prompted_version: None,
            shown_tips: BTreeMap::new(),
            extra: BTreeMap::new(),
        }
    }

    /// Returns the schema of the file.
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// Returns the position of the window on the primary screen, as a share
    /// of the width of the screen.
    pub fn x(&self) -> f64 {
        self.x
    }

    /// Returns the position of the window on the primary screen, as a share
    /// of the height of the screen.
    pub fn y(&self) -> f64 {
        self.y
    }

    /// Returns the width the window had the last time it moved, in pixels.
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Returns the height the window had the last time it moved, in pixels.
    pub fn height(&self) -> f64 {
        self.height
    }

    /// Returns the version the launcher told the player about last.
    pub fn prompted_version(&self) -> Option<&str> {
        self.prompted_version.as_deref()
    }

    /// Names the version the launcher told the player about.
    pub fn set_prompted_version(&mut self, prompted_version: Option<String>) {
        self.prompted_version = prompted_version;
    }

    /// Returns whether the player was shown a tip.
    pub fn is_tip_shown(&self, id: &str) -> bool {
        self.shown_tips.contains_key(id)
    }

    /// Names a tip the player was shown.
    pub fn mark_tip_shown(&mut self, id: impl Into<String>, value: Value) {
        self.shown_tips.insert(id.into(), value);
    }

    /// Drops a tip the player was shown, so the launcher shows it again.
    pub fn unmark_tip(&mut self, id: &str) -> bool {
        self.shown_tips.remove(id).is_some()
    }

    /// Names a field of the state the launcher does not know.
    pub fn set_extra(&mut self, name: impl Into<String>, value: Value) {
        self.extra.insert(name.into(), value);
    }

    /// Reads the state from `state/launcher-state.json` of the directory of
    /// the launcher.
    pub fn load(user_home: &std::path::Path) -> Result<super::storage::SettingsFile<Self>, Error> {
        super::storage::SettingsFile::load_with_schema(
            super::launcher_state_path(user_home),
            &Self::schema_of(),
        )
    }

    /// Writes the state to `state/launcher-state.json` of the directory of
    /// the launcher.
    pub fn save(&self, user_home: &std::path::Path) -> Result<(), Error> {
        super::storage::SettingsFile::new(super::launcher_state_path(user_home), self.clone())
            .save()
    }

    /// Returns the schema the launcher asks for.
    pub fn schema_of() -> Schema {
        Schema::new("launcher-state", SchemaVersion::new(1, 0, 0))
    }
}

impl Default for LauncherState {
    fn default() -> Self {
        Self::new()
    }
}

impl super::schema::SchemaSettings for LauncherState {
    fn schema(&self) -> &str {
        &self.schema
    }

    fn set_schema(&mut self, schema: &str) {
        self.schema = schema.to_string();
    }
}

/// A rectangle on the screen, in pixels.
///
/// The edges are the positions a corner of the rectangle is at; the name
/// matches the one the screens of the launcher of Java are named with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenBounds {
    /// The position of the left edge of the screen.
    pub min_x: f64,
    /// The position of the top edge of the screen.
    pub min_y: f64,
    /// The position of the right edge of the screen.
    pub max_x: f64,
    /// The position of the bottom edge of the screen.
    pub max_y: f64,
}

impl ScreenBounds {
    /// Returns whether a corner of a window of a size can be seen on the
    /// screen.
    ///
    /// This is the way the launcher of Java decides a window is on the
    /// screen the player can see: a part of the window at least as large as
    /// [`MIN_VISIBLE_WINDOW_AREA`] must be next to the corner the player
    /// looks at.
    fn shows(&self, x: f64, y: f64, width: f64) -> bool {
        self.min_x + MIN_VISIBLE_WINDOW_AREA <= x + width
            && x <= self.max_x - MIN_VISIBLE_WINDOW_AREA
            && self.min_y + MIN_VISIBLE_WINDOW_AREA <= y
            && y <= self.max_y - MIN_VISIBLE_WINDOW_AREA
    }
}

/// The rectangle the window of the launcher is shown in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContentBounds {
    /// The position of the left edge of the window.
    pub x: f64,
    /// The position of the top edge of the window.
    pub y: f64,
    /// The width of the window.
    pub width: f64,
    /// The height of the window.
    pub height: f64,
}

/// Returns the rectangle the window of the launcher starts in.
///
/// The size the player left the window in is kept, but never smaller than
/// the size the pages of the launcher need [`MIN_CONTENT_WIDTH`] and
/// [`MIN_CONTENT_HEIGHT`]; the position the player left the window in is
/// measured from the size of the primary screen, and is put another way when
/// the window would not be seen on any of the screens: then the launcher of
/// Java puts the window in the middle of the primary screen, which this one
/// does too.
pub fn initial_content_bounds(
    state: &LauncherState,
    primary_size: (f64, f64),
    screens: &[ScreenBounds],
) -> ContentBounds {
    let width = MIN_CONTENT_WIDTH.max(state.width());
    let height = MIN_CONTENT_HEIGHT.max(state.height());
    let mut x = state.x() * primary_size.0;
    let mut y = state.y() * primary_size.1;

    let visible = screens.iter().any(|screen| screen.shows(x, y, width));
    if !visible {
        x = (primary_size.0 - width) / 2.0;
        y = (primary_size.1 - height) / 2.0;
    }

    ContentBounds {
        x,
        y,
        width,
        height,
    }
}

impl LauncherState {
    /// Names the rectangle the window of the launcher is in now.
    ///
    /// The position is kept as a share of the size of the primary screen,
    /// which is the way the launcher of Java keeps it.
    pub fn record_content_bounds(&mut self, bounds: &ContentBounds, primary_size: (f64, f64)) {
        self.x = bounds.x / primary_size.0;
        self.y = bounds.y / primary_size.1;
        self.width = bounds.width;
        self.height = bounds.height;
    }

    /// Names the rectangle the window of an older launcher of the player was
    /// in.
    ///
    /// The older launcher kept the size of the window the same way the
    /// launcher of today does, so the size is taken as it is; the place was
    /// written in pixels, and the launcher of Java keeps the number as it is
    /// rather than measuring it against the screen, which this does too, so
    /// the two launchers put the window of a player who comes from the older
    /// launcher in the same place.
    pub fn set_legacy_bounds(&mut self, x: f64, y: f64, width: f64, height: f64) {
        self.x = finite_or_zero(x);
        self.y = finite_or_zero(y);
        self.width = finite_or_zero(width);
        self.height = finite_or_zero(height);
    }
}

/// Returns the number, or zero when it is not a finite number.
///
/// A state file that holds a number the launcher cannot use would put the
/// window of the launcher nowhere, so the number is read as the one of a
/// launcher that was never moved.
fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the bounds of a screen of a size.
    fn screen(width: f64, height: f64) -> ScreenBounds {
        ScreenBounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: width,
            max_y: height,
        }
    }

    #[test]
    fn a_window_the_player_left_in_the_middle_of_a_larger_screen_is_back() {
        let mut state = LauncherState::default();
        state.record_content_bounds(
            &ContentBounds {
                x: 400.0,
                y: 200.0,
                width: 1200.0,
                height: 700.0,
            },
            (1920.0, 1080.0),
        );

        // The recorded position is a share of the size of the screen.
        assert_eq!(state.x(), 400.0 / 1920.0);
        assert_eq!(state.y(), 200.0 / 1080.0);
        assert_eq!(state.width(), 1200.0);
        assert_eq!(state.height(), 700.0);

        let bounds = initial_content_bounds(&state, (1920.0, 1080.0), &[screen(1920.0, 1080.0)]);
        assert_eq!(
            bounds,
            ContentBounds {
                x: 400.0,
                y: 200.0,
                width: 1200.0,
                height: 700.0
            }
        );
    }

    #[test]
    fn a_window_remembered_on_a_smaller_screen_is_back_at_its_share() {
        let mut state = LauncherState::default();
        state.record_content_bounds(
            &ContentBounds {
                x: 960.0,
                y: 540.0,
                width: 800.0,
                height: 490.0,
            },
            (1920.0, 1080.0),
        );

        // The launcher of the player is now on a screen that is half as
        // large; the same share is used.
        let bounds = initial_content_bounds(&state, (960.0, 540.0), &[screen(960.0, 540.0)]);
        assert_eq!(bounds.x, 480.0);
        assert_eq!(bounds.y, 270.0);
        assert_eq!(bounds.width, 800.0);
        assert_eq!(bounds.height, 490.0);
    }

    #[test]
    fn a_window_smaller_than_the_pages_need_is_grown() {
        let mut state = LauncherState::default();
        state.record_content_bounds(
            &ContentBounds {
                x: 0.0,
                y: 0.0,
                width: 640.0,
                height: 400.0,
            },
            (1920.0, 1080.0),
        );

        let bounds = initial_content_bounds(&state, (1920.0, 1080.0), &[screen(1920.0, 1080.0)]);
        assert_eq!(bounds.width, MIN_CONTENT_WIDTH);
        assert_eq!(bounds.height, MIN_CONTENT_HEIGHT);
    }

    #[test]
    fn the_window_of_the_first_start_is_in_the_middle_of_the_primary_screen() {
        let state = LauncherState::default();

        let bounds = initial_content_bounds(&state, (1920.0, 1080.0), &[screen(1920.0, 1080.0)]);
        assert_eq!(bounds.x, 560.0, "(1920 - 800) / 2");
        assert_eq!(bounds.y, 295.0, "(1080 - 490) / 2");
        assert_eq!(bounds.width, MIN_CONTENT_WIDTH);
        assert_eq!(bounds.height, MIN_CONTENT_HEIGHT);
    }

    #[test]
    fn a_window_off_the_screens_is_put_in_the_middle() {
        let mut state = LauncherState::default();
        state.record_content_bounds(
            &ContentBounds {
                x: 5000.0,
                y: 100.0,
                width: 900.0,
                height: 600.0,
            },
            (1920.0, 1080.0),
        );

        // The position is not on any screen of the player; the launcher puts
        // the window in the middle of the primary screen.
        let bounds = initial_content_bounds(&state, (1920.0, 1080.0), &[screen(1920.0, 1080.0)]);
        assert_eq!(bounds.x, (1920.0 - 900.0) / 2.0);
        assert_eq!(bounds.y, (1080.0 - 600.0) / 2.0);

        // A window whose corner is beyond a screen but near the margin of
        // the second one is kept, which is what the launcher of Java counts
        // visible: 0.96 * 1920 = 1843.2 is inside the screen that reaches to
        // 1920 with a margin of 20.
        state.x = 0.96;
        let bounds = initial_content_bounds(
            &state,
            (1920.0, 1080.0),
            &[
                ScreenBounds {
                    min_x: 0.0,
                    min_y: 0.0,
                    max_x: 960.0,
                    max_y: 540.0,
                },
                ScreenBounds {
                    min_x: 960.0,
                    min_y: 0.0,
                    max_x: 1920.0,
                    max_y: 540.0,
                },
            ],
        );
        assert!((bounds.x - 1843.2).abs() < 1e-6, "{:?}", bounds.x);
    }

    #[test]
    fn the_tips_are_marked_and_unmarked() {
        let mut state = LauncherState::default();

        assert!(!state.is_tip_shown("javaVersionTip"));
        state.mark_tip_shown("javaVersionTip", serde_json::json!(21));
        assert!(state.is_tip_shown("javaVersionTip"));
        assert!(state.unmark_tip("javaVersionTip"));
        assert!(!state.is_tip_shown("javaVersionTip"));
        assert!(!state.unmark_tip("javaVersionTip"));
    }

    #[test]
    fn the_version_of_the_update_the_player_was_told_about_is_kept() {
        let mut state = LauncherState::default();

        assert_eq!(state.prompted_version(), None);
        state.set_prompted_version(Some("3.6.15".to_string()));
        assert_eq!(state.prompted_version(), Some("3.6.15"));
        state.set_prompted_version(None);
        assert_eq!(state.prompted_version(), None);
    }

    #[test]
    fn the_state_is_written_in_the_format_of_the_java_launcher() {
        let mut state = LauncherState::default();
        state.record_content_bounds(
            &ContentBounds {
                x: 400.0,
                y: 200.0,
                width: 1200.0,
                height: 700.0,
            },
            (1920.0, 1080.0),
        );
        state.set_prompted_version(Some("3.6.15".to_string()));
        state.mark_tip_shown("javaVersionTip", serde_json::json!(21));

        let json = serde_json::to_string(&state).expect("the state should be written");

        assert!(
            json.contains("\"$schema\":\"https://schemas.glavo.site/hmcl/launcher-state/1.0.0\"")
        );
        assert!(json.contains("\"x\":"));
        assert!(json.contains("\"y\":"));
        assert!(json.contains("\"width\":1200"));
        assert!(json.contains("\"height\":700"));
        assert!(json.contains("\"promptedVersion\":\"3.6.15\""));
        assert!(json.contains("\"shownTips\":{\"javaVersionTip\":21}"));

        let read: LauncherState = serde_json::from_str(&json).expect("the state should be read");

        // The position is written as a share of the screen, which the text
        // cannot hold exactly; the parts are compared near enough.
        assert_eq!(read.width, state.width);
        assert_eq!(read.height, state.height);
        assert_eq!(read.prompted_version, state.prompted_version);
        assert_eq!(read.shown_tips, state.shown_tips);
        assert_eq!(read.schema, state.schema);
        assert!(
            (read.x - state.x).abs() < 1e-9,
            "{:?} != {:?}",
            read.x,
            state.x
        );
        assert!(
            (read.y - state.y).abs() < 1e-9,
            "{:?} != {:?}",
            read.y,
            state.y
        );
    }

    #[test]
    fn the_state_without_the_schema_is_read() {
        let read: LauncherState =
            serde_json::from_str(r#"{"x": 0.25, "y": 0.5, "width": 1280.0, "height": 720.0}"#)
                .expect("the state should be read");

        assert_eq!(read.schema(), LAUNCHER_STATE_SCHEMA);
        assert_eq!(read.x(), 0.25);
        assert_eq!(read.width(), 1280.0);
        assert!(read.prompted_version.is_none());
    }

    #[test]
    fn the_state_is_kept_under_the_state_directory() {
        let user_home = std::path::Path::new("/home/user/.hmcl");
        let path = super::super::launcher_state_path(user_home);
        assert!(path.ends_with("state/launcher-state.json"), "{path:?}");
    }
}
