//! The types of the crate, as the markup loader sees them.
//!
//! The loader looks the types of the XAML up by their namespace, so every
//! module of the crate that holds a type the pages name is mapped to the
//! namespace of that module. The namespaces are the ones of the C# version,
//! so the XAML of that version works here as well.

use ferroui::base::data::core::ValueTypes;
use ferroui::base::metadata::MarkupType;
use ferroui::base::metadata::{MarkupAssembly, XmlnsDefinition};
use ferroui::base::{StaticType, TypeInfo};

/// The XML namespace of the framework, which the documents of the launcher
/// use as their default namespace.
const FERRO_XML_NAMESPACE: &str = "https://github.com/ferroui";

/// The namespaces of the modules of the crate that hold the types the pages
/// name.
const NAMESPACES: &[(&str, &str)] = &[
    ("hmcl::views", "HMCL.Views"),
    ("hmcl::view_models::folder_entry", "HMCL.Views"),
    ("hmcl::view_models", "HMCL.ViewModels"),
    ("hmcl::controls", "HMCL.Controls"),
    ("hmcl::util", "HMCL.Util"),
    ("hmcl::models::game", "HMCL.Game"),
    ("hmcl::models::addon", "HMCL.Addon"),
    ("hmcl::models::setting", "HMCL.Setting"),
    ("hmcl::models::auth", "HMCL.Auth"),
    ("hmcl::models::download", "HMCL.Download"),
    ("hmcl::models::remote_addon", "HMCL.Addon"),
];

/// The types of the crate the pages name.
const TYPES: &[&TypeInfo] = &[
    <crate::view_models::TaskItemViewModel as StaticType>::TYPE,
    <crate::view_models::TaskListViewModel as StaticType>::TYPE,
    <crate::view_models::main_window::MainWindowViewModel as StaticType>::TYPE,
    <crate::view_models::folder_entry::FolderEntry as StaticType>::TYPE,
    <crate::view_models::export_wizard::ModpackExportWizardViewModel as StaticType>::TYPE,
    <crate::view_models::export_wizard::ModpackFileEntryViewModel as StaticType>::TYPE,
    <crate::models::game::BlueprintInfo as StaticType>::TYPE,
    <crate::models::game::GameComponentMark as StaticType>::TYPE,
    <crate::models::game::GameInstance as StaticType>::TYPE,
    <crate::models::game::ScreenshotInfo as StaticType>::TYPE,
    <crate::models::game::WorldInfo as StaticType>::TYPE,
    <crate::models::addon::DataPack as StaticType>::TYPE,
    <crate::models::addon::ResourcePack as StaticType>::TYPE,
    <crate::models::setting::GameDirectory as StaticType>::TYPE,
    <crate::models::auth::Account as StaticType>::TYPE,
    <crate::models::download::VersionMetadata as StaticType>::TYPE,
    <crate::models::remote_addon::RemoteAddon as StaticType>::TYPE,
];

/// The assembly of the views of the pages.
pub static VIEWS_ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "HMCL.Views",
    crate_name: "hmcl",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "HMCL.Views",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The assembly of the view models of the pages.
pub static VIEW_MODELS_ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "HMCL.ViewModels",
    crate_name: "hmcl",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "HMCL.ViewModels",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The assembly of the controls of the pages.
pub static CONTROLS_ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "HMCL.Controls",
    crate_name: "hmcl",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "HMCL.Controls",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The assembly of the helpers of the pages.
pub static UTIL_ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "HMCL.Util",
    crate_name: "hmcl",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: FERRO_XML_NAMESPACE,
        namespace: "HMCL.Util",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// Declares the types of the crate to the markup loader.
///
/// The method must be called before the first document is loaded; it is
/// cheap and does nothing after the first call.
pub fn register() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        MarkupType::register_all(crate::converters::TYPES);
        ValueTypes::register_global(crate::converters::register_value_types);

        MarkupAssembly::register(&VIEWS_ASSEMBLY);
        MarkupAssembly::register(&VIEW_MODELS_ASSEMBLY);
        MarkupAssembly::register(&CONTROLS_ASSEMBLY);
        MarkupAssembly::register(&UTIL_ASSEMBLY);
    });
}
