//! The models the pages show.
//!
//! The pages of the launcher are written in XAML and bind to the models of
//! the launcher: the data of a blueprint, of a data pack, of a world and so
//! on. The models are the classes of the framework, because the loader needs
//! to know the properties of a model to bind to them.

/// Declares a model the pages bind to.
///
/// The declaration gives the class of the model, the namespace it is
/// addressed by and the properties the pages bind to. Every property names the
/// accessor of the property, which the pages and the code use to read and to
/// write it.
#[macro_export]
macro_rules! display_model {
    (
        $(#[$meta:meta])*
        $name:ident,
        namespace: $namespace:literal,
        properties: {
            $(
                $(#[$property_meta:meta])*
                $property:ident : $type:ty => $accessor:ident
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[repr(C)]
        pub struct $name {
            /// The base of the class.
            base: FerroObject,
        }

        ferro_class!($name: FerroObject);
        ferro_impl_classes!($name: FerroObjectImpl);

        ferro_class_info!($name {
            markup: {
                namespace: $namespace,
                properties: [
                    $(
                        $property: $type {
                            get: |this: &Ref<$name>| this.get_value($name::$accessor()),
                            set: |this: &Ref<$name>, value: $type| this.set_value($name::$accessor(), value),
                        },
                    )*
                ],
            },
        });

        #[allow(dead_code)]
        impl $name {
            /// Creates the class data of the model.
            pub fn construct() -> Self {
                Self {
                    base: FerroObject::construct(),
                }
            }

            /// Registers the class and its properties.
            pub fn class_init() {
                ValueTypes::register_object::<$name>();
            }

            /// Creates an instance of the model.
            pub fn new() -> Ref<Self> {
                Self::class_init();
                instantiate(Self::construct())
            }

            $(
                ferro_property!(
                    $(#[$property_meta])*
                    pub fn $accessor() -> StyledProperty<$type> {
                        FerroProperty::register::<$name, _>(stringify!($property), Default::default())
                    }
                );
            )*
        }
    };
}

pub mod addon;
pub mod auth;
pub mod download;
pub mod game;
pub mod remote_addon;
pub mod setting;
