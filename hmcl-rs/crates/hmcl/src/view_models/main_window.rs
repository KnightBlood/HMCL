//! The view model of the main window.
//!
//! The pages of the launcher are bound to the view model of the main
//! window; the view model holds the data the pages show and the choices the
//! user made. The names of the properties are the ones of the C# version, so
//! the pages of that version bind to them.

use ferroui::base::data::core::ValueTypes;
use ferroui::base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_property, instantiate, BoxedValue,
    FerroObject, FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use ferroui::controls::ItemsSource;

crate::display_model!(
    /// The view model of the main window.
    ///
    /// The pages of the launcher are bound to it: the lists of the files and
    /// of the instances, and the items the user selected.
    MainWindowViewModel,
    namespace: "HMCL.ViewModels",
    properties: {
        // The state of the launcher.
        /// The message that tells what the launcher is doing.
        StatusMessage: Option<String> => status_message_property,
        /// Whether the launcher is starting the game.
        IsLaunching: bool => is_launching_property,
        /// Whether the launcher is busy.
        IsBusy: bool => is_busy_property,
        /// The state of the loading of the page of the downloads.
        LoadState: Option<String> => load_state_property,
        /// The error of the loading of the page of the downloads.
        LoadError: Option<String> => load_error_property,
        /// The text the user searched for.
        SearchText: Option<String> => search_text_property,
        /// The text the user searched for the add-ons.
        ModSearchQuery: Option<String> => mod_search_query_property,
        /// The text the user searched for the modpacks.
        ModpackSearchQuery: Option<String> => modpack_search_query_property,
        /// The text the user searched for the instances.
        InstanceSearchText: Option<String> => instance_search_text_property,
        /// The state of the installation of the add-on.
        ModInstallStatus: Option<String> => mod_install_status_property,
        /// The description of the hardware of the machine.
        HardwareDescription: Option<String> => hardware_description_property,
        /// The state of the memory of the machine.
        MemoryStatus: Option<String> => memory_status_property,
        /// The font of the log views.
        LogFontFamily: Option<String> => log_font_family_property,
        /// The size of the font of the log views.
        LogFontSize: f64 => log_font_size_property,
        /// Whether the log view shows the reasons of the crashes.
        HasCrashCauses: bool => has_crash_causes_property,
        /// Whether the window of the update is shown.
        ShowUpdateBubble: bool => show_update_bubble_property,
        /// The version of the launcher that is the newest.
        LatestVersionText: Option<String> => latest_version_text_property,
        /// The update of the launcher that is the newest.
        LatestUpdate: Option<BoxedValue> => latest_update_property,
        /// The kind of the version of the game that is shown.
        TypeFilter: Option<String> => type_filter_property,
        /// The repository of the add-ons that is shown.
        SelectedModRepository: Option<String> => selected_mod_repository_property,

        // The accounts.
        /// The accounts of the user.
        Accounts: Option<ItemsSource> => accounts_property,
        /// The account the user selected.
        SelectedAccount: Option<BoxedValue> => selected_account_property,
        /// The authentication servers of the user.
        AuthServers: Option<ItemsSource> => auth_servers_property,
        /// The authentication server the user selected.
        SelectedAuthServer: Option<BoxedValue> => selected_auth_server_property,

        // The instances.
        /// The instances of the game.
        GameInstances: Option<ItemsSource> => game_instances_property,
        /// The instances that match the text the user searched for.
        FilteredInstances: Option<ItemsSource> => filtered_instances_property,
        /// The instance the user selected.
        SelectedInstance: Option<BoxedValue> => selected_instance_property,
        /// The game directories of the user.
        GameDirectories: Option<ItemsSource> => game_directories_property,
        /// The game directory the user selected.
        SelectedGameDirectory: Option<BoxedValue> => selected_game_directory_property,

        // The files of the game.
        /// The resource packs of the game.
        ResourcePacks: Option<ItemsSource> => resource_packs_property,
        /// The resource pack the user selected.
        SelectedResourcePack: Option<BoxedValue> => selected_resource_pack_property,
        /// The data packs of the game.
        DataPacks: Option<ItemsSource> => data_packs_property,
        /// The data pack the user selected.
        SelectedDataPack: Option<BoxedValue> => selected_data_pack_property,
        /// The shader packs of the game.
        ShaderPacks: Option<ItemsSource> => shader_packs_property,
        /// The shader pack the user selected.
        SelectedShaderPack: Option<BoxedValue> => selected_shader_pack_property,
        /// The screenshots of the game.
        Screenshots: Option<ItemsSource> => screenshots_property,
        /// The blueprints of the game.
        Blueprints: Option<ItemsSource> => blueprints_property,
        /// The blueprint the user selected.
        SelectedBlueprint: Option<BoxedValue> => selected_blueprint_property,
        /// The worlds of the game.
        Worlds: Option<ItemsSource> => worlds_property,
        /// The world the user selected.
        SelectedWorld: Option<BoxedValue> => selected_world_property,

        // The downloads.
        /// The components of the game.
        Installers: Option<ItemsSource> => installers_property,
        /// The component the user selected.
        SelectedInstaller: Option<BoxedValue> => selected_installer_property,
        /// The kinds of the versions of the game that are shown.
        TypeFilters: Option<ItemsSource> => type_filters_property,
        /// The add-ons the user searched for.
        RemoteModResults: Option<ItemsSource> => remote_mod_results_property,
        /// The add-on the user selected.
        SelectedRemoteMod: Option<BoxedValue> => selected_remote_mod_property,
        /// The modpacks the user searched for.
        RemoteModpackResults: Option<ItemsSource> => remote_modpack_results_property,
        /// The modpack the user selected.
        SelectedRemoteModpack: Option<BoxedValue> => selected_remote_modpack_property,
        /// The repositories of the add-ons.
        ModRepositories: Option<ItemsSource> => mod_repositories_property,
        /// The version of the game the user selected.
        SelectedVersion: Option<BoxedValue> => selected_version_property,

        // The log of the game.
        /// The lines of the log of the game.
        GameLines: Option<ItemsSource> => game_lines_property,
        /// The reasons of the last crash of the game.
        CrashCauses: Option<ItemsSource> => crash_causes_property,

        // The tasks of the launcher.
        /// The tasks of the launcher.
        TaskList: Option<BoxedValue> => task_list_property,

        // The hint of the launcher.
        /// The hint the launcher shows.
        Hint: Option<BoxedValue> => hint_property,

        // The Terracotta of the launcher.
        /// The state of the Terracotta of the user.
        TerracottaStatus: Option<String> => terracotta_status_property,
        /// The description of the state of the Terracotta of the user.
        TerracottaStatusDescription: Option<String> => terracotta_status_description_property,
        /// The nodes of the Terracotta of the user.
        TerracottaNodes: Option<ItemsSource> => terracotta_nodes_property,
        /// The profiles of the Terracotta of the user.
        TerracottaProfiles: Option<ItemsSource> => terracotta_profiles_property,
        /// The code of the room of the Terracotta of the user.
        TerracottaRoomCode: Option<String> => terracotta_room_code_property,
        /// The name of the room of the Terracotta of the user.
        TerracottaRoomName: Option<String> => terracotta_room_name_property,
        /// The difficulty of the Terracotta of the user.
        TerracottaDifficulty: Option<String> => terracotta_difficulty_property,
        /// The guide of the packages of the Terracotta of the user.
        TerracottaPackageGuide: Option<String> => terracotta_package_guide_property,
        /// Whether the Terracotta of the user is installed.
        IsTerracottaInstalled: bool => is_terracotta_installed_property,
        /// Whether the Terracotta of the user is ready.
        IsTerracottaReady: bool => is_terracotta_ready_property,
    }
);
