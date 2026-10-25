//! The view model of the folders of an instance.

use ferroui::base::data::core::ValueTypes;
use ferroui::base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_property, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

crate::display_model!(
    /// A folder of an instance of the game.
    FolderEntry,
    namespace: "HMCL.Views",
    properties: {
        /// The name of the folder.
        Name: Option<String> => name_property,
        /// The path of the folder.
        Path: Option<String> => path_property,
    }
);
