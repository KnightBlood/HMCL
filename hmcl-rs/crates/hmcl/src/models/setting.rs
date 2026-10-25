//! The models of the settings of the user.

use ferroui::base::data::core::ValueTypes;
use ferroui::base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_property, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

crate::display_model!(
    /// A game directory of the user.
    GameDirectory,
    namespace: "HMCL.Setting",
    properties: {
        /// The identifier of the directory.
        Id: Option<String> => id_property,
        /// The name of the directory that is shown to the user.
        DisplayName: Option<String> => display_name_property,
        /// The path of the directory.
        Path: Option<String> => path_property,
    }
);
