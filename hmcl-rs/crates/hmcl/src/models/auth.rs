//! The models of the accounts the pages show.

use ferroui::base::data::core::ValueTypes;
use ferroui::base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_property, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

crate::display_model!(
    /// An account of the launcher, as the list of the accounts shows it.
    Account,
    namespace: "HMCL.Auth",
    properties: {
        /// The name of the player.
        Username: Option<String> => username_property,
        /// The address of the skin of the player, if there is one.
        SkinPath: Option<String> => skin_path_property,
        /// The kind of the account, as the list shows it.
        AccountType: Option<String> => account_type_property,
    }
);
