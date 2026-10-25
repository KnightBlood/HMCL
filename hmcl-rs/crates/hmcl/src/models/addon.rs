//! The models of the add-ons of the game.

use ferroui::base::data::core::ValueTypes;
use ferroui::base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_property, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

crate::display_model!(
    /// A data pack of the game.
    DataPack,
    namespace: "HMCL.Addon",
    properties: {
        /// The description of the data pack, in the language of the user.
        Description: Option<String> => description_property,
        /// Whether the data pack is turned on.
        Enabled: bool => enabled_property,
        /// The name of the file of the data pack.
        FileName: Option<String> => file_name_property,
        /// The name of the data pack.
        Name: Option<String> => name_property,
    }
);

crate::display_model!(
    /// A resource pack of the game.
    ResourcePack,
    namespace: "HMCL.Addon",
    properties: {
        /// The description of the resource pack, in the language of the user.
        Description: Option<String> => description_property,
        /// Whether the resource pack is turned on.
        Enabled: bool => enabled_property,
        /// The name of the file of the resource pack.
        FileName: Option<String> => file_name_property,
        /// The name of the resource pack.
        Name: Option<String> => name_property,
        /// The warning of the resource pack, in the language of the user.
        Warning: Option<String> => warning_property,
    }
);
