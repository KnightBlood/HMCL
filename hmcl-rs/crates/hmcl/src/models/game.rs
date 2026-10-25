//! The models of the files and the instances of the game.

use ferroui::base::data::core::ValueTypes;
use ferroui::base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_property, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

crate::display_model!(
    /// A blueprint of the game.
    BlueprintInfo,
    namespace: "HMCL.Game",
    properties: {
        /// The name of the file of the blueprint.
        FileName: Option<String> => file_name_property,
        /// The moment the blueprint was changed, in the language of the user.
        ModifiedAtText: Option<String> => modified_at_text_property,
        /// The size of the blueprint, in the language of the user.
        SizeText: Option<String> => size_text_property,
        /// The kind of the blueprint, in the language of the user.
        TypeText: Option<String> => type_text_property,
    }
);

crate::display_model!(
    /// A component of the game, like a library or a resource pack.
    GameComponentMark,
    namespace: "HMCL.Game",
    properties: {
        /// The kind of the component, in the language of the user.
        ComponentType: Option<String> => component_type_property,
        /// The version of the component.
        Version: Option<String> => version_property,
    }
);

crate::display_model!(
    /// A screenshot of the game.
    ScreenshotInfo,
    namespace: "HMCL.Game",
    properties: {
        /// The name of the file of the screenshot.
        FileName: Option<String> => file_name_property,
        /// The path of the file of the screenshot.
        Path: Option<String> => path_property,
        /// The moment the screenshot was taken, in the language of the user.
        TakenAtText: Option<String> => taken_at_text_property,
    }
);

crate::display_model!(
    /// A world of the game.
    WorldInfo,
    namespace: "HMCL.Game",
    properties: {
        /// The name of the world.
        Name: Option<String> => name_property,
        /// The path of the folder of the world.
        Path: Option<String> => path_property,
    }
);

crate::display_model!(
    /// An instance of the game.
    GameInstance,
    namespace: "HMCL.Game",
    properties: {
        /// The identifier of the instance.
        Id: Option<String> => id_property,
        /// The path of the folder of the instance.
        Path: Option<String> => path_property,
        /// The kind of the loader of the instance.
        Loader: Option<String> => loader_property,
        /// The version of the game of the instance.
        Version: Option<String> => version_property,
        /// The path of the icon of the instance.
        IconPath: Option<String> => icon_path_property,
        /// The moment the instance was played last, in the language of the
        /// user.
        LastPlayedText: Option<String> => last_played_text_property,
        /// The game directory the instance belongs to.
        GameDirectory: Option<String> => game_directory_property,
        /// The profile of the instance.
        Profile: Option<String> => profile_property,
    }
);
