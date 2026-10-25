//! The models of the addons the repositories offer.

use ferroui::base::data::core::ValueTypes;
use ferroui::base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_property, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

crate::display_model!(
    /// An addon a repository offers, as the list of the addons shows it.
    RemoteAddon,
    namespace: "HMCL.Addon",
    properties: {
        /// The title of the addon.
        Name: Option<String> => name_property,
        /// The description of the addon.
        Description: Option<String> => description_property,
        /// The author of the addon.
        Author: Option<String> => author_property,
        /// The name of the addon in its repository.
        Slug: Option<String> => slug_property,
        /// The address of the page of the addon.
        PageUrl: Option<String> => page_url_property,
        /// The address of the icon of the addon.
        IconUrl: Option<String> => icon_url_property,
    }
);
