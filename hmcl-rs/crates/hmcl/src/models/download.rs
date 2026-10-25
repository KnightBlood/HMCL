//! The models of the versions of the game the pages show.

use ferroui::base::data::core::ValueTypes;
use ferroui::base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_property, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

crate::display_model!(
    /// A version of the game, as the list of the versions shows it.
    VersionMetadata,
    namespace: "HMCL.Download",
    properties: {
        /// The identifier of the version.
        Id: Option<String> => id_property,
        /// The kind of the version.
        Type: Option<String> => type_property,
        /// The moment the version was released, as the list shows it.
        ReleaseTimeText: Option<String> => release_time_text_property,
    }
);
