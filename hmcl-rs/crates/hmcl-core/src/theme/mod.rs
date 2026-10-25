//! The theme packs the user installs.
//!
//! A theme pack is a zip file that holds a manifest and the assets the
//! launcher reads to change its appearance. The manifest is written as
//! `manifest.json` and declares the themes the pack offers.
//!
//! The launcher of Java keeps the manifest and the assets of a pack in a file
//! named after the pack, so the same types are used to read and to write a
//! pack here.

pub mod theme_pack;
pub mod theme_pack_manager;

pub use theme_pack_manager::{
    InstalledThemePack, LoadedThemePack, ThemePackLocation, ThemePackManager,
    ThemePackManagerError, THEME_PACKS_DIRECTORY,
};
