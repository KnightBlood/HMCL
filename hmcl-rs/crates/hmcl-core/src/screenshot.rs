//! The screenshots the game writes into a game directory.
//!
//! The game writes a screenshot of every press of the key of the screenshots
//! into `screenshots`, as a PNG file named after the moment it was taken. The
//! launcher lists them, and the player opens one to look at it, to copy it or to
//! choose it as the background of the launcher.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The directory of the screenshots of a game directory.
pub const SCREENSHOTS_DIRECTORY: &str = "screenshots";

/// A screenshot of a game directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screenshot {
    /// The file of the screenshot.
    path: PathBuf,
    /// The name of the file of the screenshot.
    file_name: String,
    /// The moment the file was created, in milliseconds since the first of
    /// January 1970, if the system tells it.
    creation_time: Option<i64>,
    /// The size of the file, if the system tells it.
    file_size: Option<u64>,
}

impl Screenshot {
    /// Returns whether a file is a screenshot.
    ///
    /// The game writes the screenshots as PNG files, so a file of another kind
    /// is not shown, even when the player put it in the directory.
    pub fn is_file_screenshot(path: &Path) -> bool {
        path.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    }

    /// Reads a screenshot of a file.
    ///
    /// Nothing is returned when the file is not a screenshot. A file whose
    /// attributes cannot be read is still a screenshot, so the file is shown
    /// without the moment and the size it was written.
    pub fn open(path: impl AsRef<Path>) -> Option<Self> {
        let path = path.as_ref();
        if !Self::is_file_screenshot(path) {
            return None;
        }

        let metadata = std::fs::metadata(path).ok();
        let creation_time = metadata.as_ref().and_then(|metadata| {
            metadata
                .created()
                // The systems of Unix do not always tell when a file was
                // created; the moment it was written is used instead, which is
                // the same moment for a screenshot.
                .or_else(|_| metadata.modified())
                .ok()
                .and_then(milliseconds)
        });

        Some(Self {
            path: path.to_path_buf(),
            file_name: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("")
                .to_string(),
            creation_time,
            file_size: metadata.map(|metadata| metadata.len()),
        })
    }

    /// Returns the file of the screenshot.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the name of the file of the screenshot.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Returns the moment the screenshot was created, in milliseconds since the
    /// first of January 1970.
    pub fn creation_time(&self) -> Option<i64> {
        self.creation_time
    }

    /// Returns the size of the file of the screenshot, in bytes.
    pub fn file_size(&self) -> Option<u64> {
        self.file_size
    }

    /// Reads the file of the screenshot.
    pub fn read(&self) -> std::io::Result<Vec<u8>> {
        std::fs::read(&self.path)
    }

    /// Chooses the screenshot as the background of the launcher.
    ///
    /// The launcher shows a picture of the choice of the player when the kind
    /// of the background is the one of the custom picture, so the kind is
    /// changed and the path of the file is written into the settings, which is
    /// what the launcher of Java does when a player chooses a screenshot.
    pub fn set_as_background(&self, settings: &mut crate::setting::LauncherSettings) {
        settings.background_type = crate::setting::BackgroundType::Custom;
        settings.custom_background_image_path = Some(self.path.display().to_string());
    }

    /// Deletes the file of the screenshot.
    pub fn delete(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.path) {
            // The file may be gone already, which is what the caller wants.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }
}

/// Returns the screenshots of a directory, newest first.
///
/// The game names the files after the moment of the screenshot, so the files of
/// the names that sort last are the newest ones. A directory that cannot be
/// read holds no screenshot.
pub fn list_screenshots(directory: &Path) -> Vec<Screenshot> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };

    let mut screenshots = entries
        .flatten()
        .filter_map(|entry| Screenshot::open(entry.path()))
        .collect::<Vec<_>>();

    // The screenshots are shown newest first, like the page of the launcher.
    screenshots.sort_by(|first, second| second.path.cmp(&first.path));
    screenshots
}

/// Deletes the screenshots of a directory.
///
/// The files of the other kinds are left in the directory, because the player
/// may have put them there.
///
/// Returns the number of the screenshots that are deleted when every one of
/// them could be deleted, and the failure of the first file that cannot be
/// deleted.
pub fn clear_screenshots(directory: &Path) -> Result<usize, ScreenShotError> {
    let mut deleted = 0;

    for screenshot in list_screenshots(directory) {
        screenshot
            .delete()
            .map_err(|error| ScreenShotError::Delete {
                path: screenshot.path().to_path_buf(),
                error,
            })?;
        deleted += 1;
    }

    Ok(deleted)
}

/// The error of managing the screenshots of a game directory.
#[derive(Debug)]
pub enum ScreenShotError {
    /// A screenshot cannot be deleted.
    Delete {
        /// The file of the screenshot.
        path: PathBuf,
        /// The reason the file cannot be deleted.
        error: std::io::Error,
    },
}

impl std::fmt::Display for ScreenShotError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Delete { path, error } => {
                write!(
                    formatter,
                    "the screenshot {} cannot be deleted: {error}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ScreenShotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Delete { error, .. } => Some(error),
        }
    }
}

/// Returns the number of milliseconds of a moment.
fn milliseconds(time: SystemTime) -> Option<i64> {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a directory the tests work in.
    fn directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-screenshot-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        directory
    }

    #[test]
    fn the_screenshots_of_a_directory_are_read() {
        let base = directory("list");
        std::fs::write(base.join("2026-01-02_03.04.05.png"), b"first").expect("write");
        std::fs::write(base.join("2026-02-03_04.05.06.png"), b"second").expect("write");
        std::fs::write(base.join("notes.txt"), b"not a screenshot").expect("write");
        std::fs::create_dir_all(base.join("2026-03-04_05.06.07.png")).expect("dir");

        let screenshots = list_screenshots(&base);
        // The newest file comes first, and the files of the other kinds and
        // the directories are left out.
        let names = screenshots
            .iter()
            .map(Screenshot::file_name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec!["2026-02-03_04.05.06.png", "2026-01-02_03.04.05.png"]
        );

        assert_eq!(screenshots[0].file_size(), Some(6));
        assert_eq!(screenshots[0].read().expect("read"), b"second");
        assert!(screenshots[0].creation_time().is_some());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_screenshot_is_chosen_as_the_background_of_the_launcher() {
        let base = directory("background");
        let file = base.join("2026-01-02_03.04.05.png");
        std::fs::write(&file, b"picture").expect("the file should be written");

        let screenshot = Screenshot::open(&file).expect("the screenshot should be read");
        let mut settings = crate::setting::LauncherSettings::default();

        screenshot.set_as_background(&mut settings);

        assert_eq!(
            settings.background_type,
            crate::setting::BackgroundType::Custom
        );
        assert_eq!(
            settings.custom_background_image_path.as_deref(),
            Some(file.display().to_string().as_str())
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_missing_directory_holds_no_screenshot() {
        let base = directory("missing");
        assert!(list_screenshots(&base.join("screenshots")).is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_file_that_is_not_a_png_is_not_a_screenshot() {
        let base = directory("kinds");
        std::fs::write(base.join("image.txt"), b"text").expect("write");
        std::fs::write(base.join("image.PNG"), b"png").expect("write");

        assert!(Screenshot::open(base.join("image.txt")).is_none());
        // The extension of the file is compared without regard to the case.
        assert!(Screenshot::open(base.join("image.PNG")).is_some());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_screenshot_is_deleted() {
        let base = directory("delete");
        std::fs::write(base.join("shot.png"), b"png").expect("write");

        let screenshot = Screenshot::open(base.join("shot.png")).expect("the screenshot");
        screenshot
            .delete()
            .expect("the screenshot should be deleted");
        assert!(!base.join("shot.png").exists());

        // Deleting a screenshot that is gone is not a failure.
        screenshot
            .delete()
            .expect("the screenshot should be deleted");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_screenshots_of_a_directory_are_cleared() {
        let base = directory("clear");
        std::fs::write(base.join("first.png"), b"png").expect("write");
        std::fs::write(base.join("second.png"), b"png").expect("write");
        std::fs::write(base.join("notes.txt"), b"text").expect("write");

        let deleted = clear_screenshots(&base).expect("the screenshots should be cleared");
        assert_eq!(deleted, 2);
        assert!(!base.join("first.png").exists());
        assert!(!base.join("second.png").exists());
        // The files of the other kinds are kept.
        assert!(base.join("notes.txt").is_file());

        let _ = std::fs::remove_dir_all(&base);
    }
}
