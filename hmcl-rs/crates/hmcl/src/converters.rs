//! The converters the pages use.
//!
//! The pages of the launcher show the values of the models in a way the
//! models do not know about: a yes or a no, the kind of a version of the
//! game. The converters do that work.

use std::rc::Rc;

use ferroui::base::data::converters::{cast_value, IValueConverter};
use ferroui::base::data::core::{ValueType, ValueTypes};
use ferroui::base::data::{BindingError, BindingOperations};
use ferroui::base::utilities::CultureInfo;
use ferroui::base::{ferro_markup_type, BoxedValue};

use crate::context;

/// The converters are reference types: an instance equals itself.
macro_rules! identity_eq {
    ($($type_:ty),*) => {
        $(
            impl PartialEq for $type_ {
                fn eq(&self, other: &Self) -> bool {
                    std::ptr::eq(self, other)
                }
            }
        )*
    };
}

identity_eq!(
    BoolToTextConverter,
    NullToBoolConverter,
    NotNullToBoolConverter,
    VersionTypeConverter
);

/// Converts a value into a text.
///
/// The converter is used to show whether an add-on is turned on.
#[derive(Default)]
pub struct BoolToTextConverter;

impl BoolToTextConverter {
    /// Creates the converter.
    pub fn new() -> Self {
        Self
    }
}

impl IValueConverter for BoolToTextConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let enabled = cast_value::<bool>(value) == Some(true);

        // The parameter of the converter is the text of the value that is
        // turned on.
        let text = match parameter.and_then(|parameter| cast_value::<String>(Some(parameter))) {
            Some(text) => {
                if enabled {
                    text
                } else {
                    String::new()
                }
            }
            None => {
                let i18n = &context::launcher().i18n();
                if enabled {
                    i18n.i18n("hmcl.booltotextconverter.828_")
                } else {
                    i18n.i18n("hmcl.booltotextconverter.830_")
                }
            }
        };

        Ok(Some(Rc::new(text)))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        // The converter cannot be used backwards, so the value is kept.
        Ok(Some(BindingOperations::do_nothing()))
    }
}

/// Converts a missing value into `false`.
#[derive(Default)]
pub struct NullToBoolConverter;

impl NullToBoolConverter {
    /// Creates the converter.
    pub fn new() -> Self {
        Self
    }
}

impl IValueConverter for NullToBoolConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let is_null = value.cloned().and_then(ValueTypes::normalize).is_none();
        Ok(Some(Rc::new(!is_null)))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        // The converter cannot be used backwards, so the value is kept.
        Ok(Some(BindingOperations::do_nothing()))
    }
}

/// Converts a missing value into `true`.
#[derive(Default)]
pub struct NotNullToBoolConverter;

impl NotNullToBoolConverter {
    /// Creates the converter.
    pub fn new() -> Self {
        Self
    }
}

impl IValueConverter for NotNullToBoolConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let is_null = value.cloned().and_then(ValueTypes::normalize).is_none();
        Ok(Some(Rc::new(is_null)))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        // The converter cannot be used backwards, so the value is kept.
        Ok(Some(BindingOperations::do_nothing()))
    }
}

/// Converts the kind of a version of the game into the text shown to the
/// user.
#[derive(Default)]
pub struct VersionTypeConverter;

impl VersionTypeConverter {
    /// Creates the converter.
    pub fn new() -> Self {
        Self
    }
}

impl IValueConverter for VersionTypeConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let i18n = &context::launcher().i18n();

        let text = match value.and_then(|value| cast_value::<String>(Some(value))) {
            Some(kind) => match kind.as_str() {
                "release" => i18n.i18n("instance.game.release"),
                "snapshot" => i18n.i18n("instance.game.snapshot"),
                "old_beta" | "old_alpha" => i18n.i18n("instance.game.old"),
                _ => kind,
            },
            None => String::new(),
        };

        Ok(Some(Rc::new(text)))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        // The converter cannot be used backwards, so the value is kept.
        Ok(Some(BindingOperations::do_nothing()))
    }
}

ferro_markup_type!(class BoolToTextConverter {
    namespace: "HMCL.Util",
    handles: [Rc<BoolToTextConverter>, Option<Rc<BoolToTextConverter>>],
    this: Rc<BoolToTextConverter>,
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(BoolToTextConverter::new())],
    fields: [Instance: Rc<BoolToTextConverter> => || Rc::new(BoolToTextConverter::new())],
});

ferro_markup_type!(class NullToBoolConverter {
    namespace: "HMCL.Util",
    handles: [Rc<NullToBoolConverter>, Option<Rc<NullToBoolConverter>>],
    this: Rc<NullToBoolConverter>,
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(NullToBoolConverter::new())],
    fields: [Instance: Rc<NullToBoolConverter> => || Rc::new(NullToBoolConverter::new())],
});

ferro_markup_type!(class NotNullToBoolConverter {
    namespace: "HMCL.Util",
    handles: [Rc<NotNullToBoolConverter>, Option<Rc<NotNullToBoolConverter>>],
    this: Rc<NotNullToBoolConverter>,
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(NotNullToBoolConverter::new())],
    fields: [Instance: Rc<NotNullToBoolConverter> => || Rc::new(NotNullToBoolConverter::new())],
});

ferro_markup_type!(class VersionTypeConverter {
    namespace: "HMCL.Util",
    handles: [Rc<VersionTypeConverter>, Option<Rc<VersionTypeConverter>>],
    this: Rc<VersionTypeConverter>,
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(VersionTypeConverter::new())],
    fields: [Instance: Rc<VersionTypeConverter> => || Rc::new(VersionTypeConverter::new())],
});

/// Registers the conversions of the values of the converters.
///
/// The bindings hold the converters as values, so the runtime has to know
/// that a converter is a converter.
pub fn register_value_types() {
    ValueTypes::register_cast::<Rc<BoolToTextConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
    ValueTypes::register_cast::<Rc<NullToBoolConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
    ValueTypes::register_cast::<Rc<NotNullToBoolConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
    ValueTypes::register_cast::<Rc<VersionTypeConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
}

/// The types with markup metadata of the converters of the launcher.
pub const TYPES: &[&ferroui::base::metadata::MarkupType] = &[
    <BoolToTextConverter as ferroui::base::metadata::MarkupTyped>::MARKUP,
    <NullToBoolConverter as ferroui::base::metadata::MarkupTyped>::MARKUP,
    <NotNullToBoolConverter as ferroui::base::metadata::MarkupTyped>::MARKUP,
    <VersionTypeConverter as ferroui::base::metadata::MarkupTyped>::MARKUP,
];
