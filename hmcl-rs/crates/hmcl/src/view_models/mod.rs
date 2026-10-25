//! The view models of the pages of the launcher.
//!
//! The pages are written in XAML and bind to the view models, so the view
//! models are the classes of the framework: every value the pages show is a
//! property of a class, and the pages are told when it changes.
//!
//! The names of the classes and of their properties are the ones of the C#
//! version, so the XAML of that version works here as well.

// The wizard is wired to the pages step by step; the accessors the pages do
// not reach yet are part of the public surface of the view model.
#[allow(dead_code)]
pub mod export_wizard;
pub mod folder_entry;
pub mod main_window;

use std::rc::Rc;

use ferroui::base::collections::FerroList;
use ferroui::base::data::core::ValueTypes;
use ferroui::base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_property, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use ferroui::controls::ItemsSource;

/// The view model of a task in the task list.
///
/// It shows the state of a task of the launcher: its name, its progress and
/// the error it failed with.
#[repr(C)]
pub struct TaskItemViewModel {
    /// The base of the class.
    base: FerroObject,
}

ferro_class!(TaskItemViewModel: FerroObject);
ferro_impl_classes!(TaskItemViewModel: FerroObjectImpl);

ferro_class_info!(TaskItemViewModel {
    markup: {
        namespace: "HMCL.ViewModels",
        properties: [
            Name: Option<String> {
                get: |this: &Ref<TaskItemViewModel>| this.get_value(TaskItemViewModel::name_property()),
                set: |this: &Ref<TaskItemViewModel>, value: Option<String>| this.set_value(TaskItemViewModel::name_property(), value),
            },
            ProgressText: Option<String> {
                get: |this: &Ref<TaskItemViewModel>| this.get_value(TaskItemViewModel::progress_text_property()),
                set: |this: &Ref<TaskItemViewModel>, value: Option<String>| this.set_value(TaskItemViewModel::progress_text_property(), value),
            },
            StatusIcon: Option<String> {
                get: |this: &Ref<TaskItemViewModel>| this.get_value(TaskItemViewModel::status_icon_property()),
                set: |this: &Ref<TaskItemViewModel>, value: Option<String>| this.set_value(TaskItemViewModel::status_icon_property(), value),
            },
            ErrorMessage: Option<String> {
                get: |this: &Ref<TaskItemViewModel>| this.get_value(TaskItemViewModel::error_message_property()),
                set: |this: &Ref<TaskItemViewModel>, value: Option<String>| this.set_value(TaskItemViewModel::error_message_property(), value),
            },
            ProgressValue: f64 {
                get: |this: &Ref<TaskItemViewModel>| this.get_value(TaskItemViewModel::progress_value_property()),
                set: |this: &Ref<TaskItemViewModel>, value: f64| this.set_value(TaskItemViewModel::progress_value_property(), value),
            },
            IsIndeterminate: bool {
                get: |this: &Ref<TaskItemViewModel>| this.get_value(TaskItemViewModel::is_indeterminate_property()),
                set: |this: &Ref<TaskItemViewModel>, value: bool| this.set_value(TaskItemViewModel::is_indeterminate_property(), value),
            },
            CanCancel: bool {
                get: |this: &Ref<TaskItemViewModel>| this.get_value(TaskItemViewModel::can_cancel_property()),
                set: |this: &Ref<TaskItemViewModel>, value: bool| this.set_value(TaskItemViewModel::can_cancel_property(), value),
            },
        ],
    },
});

// The accessors of the view models are the values the pages bind to; the
// pages of the C# version use the same names.
#[allow(dead_code)]
impl TaskItemViewModel {
    /// Creates the view model of a task.
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }

    /// Registers the class and its properties.
    pub fn class_init() {
        ValueTypes::register_object::<TaskItemViewModel>();
    }

    /// Creates the view model of a task.
    pub fn new() -> Ref<Self> {
        Self::class_init();
        instantiate(Self::construct())
    }

    ferro_property!(
        /// The name of the task.
        pub fn name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<TaskItemViewModel, _>("Name", None)
        }
    );

    ferro_property!(
        /// The text of the progress of the task.
        pub fn progress_text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<TaskItemViewModel, _>("ProgressText", None)
        }
    );

    ferro_property!(
        /// The icon that shows the state of the task.
        pub fn status_icon_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<TaskItemViewModel, _>("StatusIcon", None)
        }
    );

    ferro_property!(
        /// The message of the error the task failed with.
        pub fn error_message_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<TaskItemViewModel, _>("ErrorMessage", None)
        }
    );

    ferro_property!(
        /// The progress of the task, from zero to a hundred.
        pub fn progress_value_property() -> StyledProperty<f64> {
            FerroProperty::register::<TaskItemViewModel, _>("ProgressValue", 0.0)
        }
    );

    ferro_property!(
        /// Whether the progress of the task is not known yet.
        pub fn is_indeterminate_property() -> StyledProperty<bool> {
            FerroProperty::register::<TaskItemViewModel, _>("IsIndeterminate", false)
        }
    );

    ferro_property!(
        /// Whether the task can be stopped.
        pub fn can_cancel_property() -> StyledProperty<bool> {
            FerroProperty::register::<TaskItemViewModel, _>("CanCancel", false)
        }
    );

    /// Returns the name of the task.
    pub fn name(&self) -> Option<String> {
        self.get_value(Self::name_property())
    }

    /// Sets the name of the task.
    pub fn set_name(&self, value: Option<String>) {
        self.set_value(Self::name_property(), value)
    }

    /// Returns the text of the progress of the task.
    pub fn progress_text(&self) -> Option<String> {
        self.get_value(Self::progress_text_property())
    }

    /// Sets the text of the progress of the task.
    pub fn set_progress_text(&self, value: Option<String>) {
        self.set_value(Self::progress_text_property(), value)
    }

    /// Returns the icon that shows the state of the task.
    pub fn status_icon(&self) -> Option<String> {
        self.get_value(Self::status_icon_property())
    }

    /// Sets the icon that shows the state of the task.
    pub fn set_status_icon(&self, value: Option<String>) {
        self.set_value(Self::status_icon_property(), value)
    }

    /// Returns the message of the error the task failed with.
    pub fn error_message(&self) -> Option<String> {
        self.get_value(Self::error_message_property())
    }

    /// Sets the message of the error the task failed with.
    pub fn set_error_message(&self, value: Option<String>) {
        self.set_value(Self::error_message_property(), value)
    }

    /// Returns the progress of the task, from zero to a hundred.
    pub fn progress_value(&self) -> f64 {
        self.get_value(Self::progress_value_property())
    }

    /// Sets the progress of the task.
    pub fn set_progress_value(&self, value: f64) {
        self.set_value(Self::progress_value_property(), value)
    }

    /// Returns whether the progress of the task is not known yet.
    pub fn is_indeterminate(&self) -> bool {
        self.get_value(Self::is_indeterminate_property())
    }

    /// Sets whether the progress of the task is not known yet.
    pub fn set_is_indeterminate(&self, value: bool) {
        self.set_value(Self::is_indeterminate_property(), value)
    }

    /// Returns whether the task can be stopped.
    pub fn can_cancel(&self) -> bool {
        self.get_value(Self::can_cancel_property())
    }

    /// Sets whether the task can be stopped.
    pub fn set_can_cancel(&self, value: bool) {
        self.set_value(Self::can_cancel_property(), value)
    }
}

/// The view model of the page that shows the tasks of the launcher.
#[repr(C)]
pub struct TaskListViewModel {
    /// The base of the class.
    base: FerroObject,
}

ferro_class!(TaskListViewModel: FerroObject);
ferro_impl_classes!(TaskListViewModel: FerroObjectImpl);

ferro_class_info!(TaskListViewModel {
    markup: {
        namespace: "HMCL.ViewModels",
        properties: [
            Tasks: Option<ItemsSource> {
                get: |this: &Ref<TaskListViewModel>| this.get_value(TaskListViewModel::tasks_property()),
                set: |this: &Ref<TaskListViewModel>, value: Option<ItemsSource>| this.set_value(TaskListViewModel::tasks_property(), value),
            },
            IsBusy: bool {
                get: |this: &Ref<TaskListViewModel>| this.get_value(TaskListViewModel::is_busy_property()),
                set: |this: &Ref<TaskListViewModel>, value: bool| this.set_value(TaskListViewModel::is_busy_property(), value),
            },
            RunningCount: i32 {
                get: |this: &Ref<TaskListViewModel>| this.get_value(TaskListViewModel::running_count_property()),
                set: |this: &Ref<TaskListViewModel>, value: i32| this.set_value(TaskListViewModel::running_count_property(), value),
            },
            CurrentTaskName: Option<String> {
                get: |this: &Ref<TaskListViewModel>| this.get_value(TaskListViewModel::current_task_name_property()),
                set: |this: &Ref<TaskListViewModel>, value: Option<String>| this.set_value(TaskListViewModel::current_task_name_property(), value),
            },
        ],
    },
});

#[allow(dead_code)]
impl TaskListViewModel {
    /// Creates the view model of the page.
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }

    /// Registers the class and its properties.
    pub fn class_init() {
        ValueTypes::register_object::<TaskListViewModel>();
    }

    /// Creates the view model of the page.
    pub fn new() -> Ref<Self> {
        Self::class_init();
        instantiate(Self::construct())
    }

    ferro_property!(
        /// The tasks the page shows.
        pub fn tasks_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<TaskListViewModel, _>("Tasks", None)
        }
    );

    ferro_property!(
        /// Whether at least one task is running.
        pub fn is_busy_property() -> StyledProperty<bool> {
            FerroProperty::register::<TaskListViewModel, _>("IsBusy", false)
        }
    );

    ferro_property!(
        /// The number of the tasks that are running.
        pub fn running_count_property() -> StyledProperty<i32> {
            FerroProperty::register::<TaskListViewModel, _>("RunningCount", 0)
        }
    );

    ferro_property!(
        /// The name of the task that was started last.
        pub fn current_task_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<TaskListViewModel, _>("CurrentTaskName", None)
        }
    );

    /// Returns the tasks the page shows.
    pub fn tasks(&self) -> Option<ItemsSource> {
        self.get_value(Self::tasks_property())
    }

    /// Sets the tasks the page shows.
    pub fn set_tasks(&self, value: Option<ItemsSource>) {
        self.set_value(Self::tasks_property(), value)
    }

    /// Returns whether at least one task is running.
    pub fn is_busy(&self) -> bool {
        self.get_value(Self::is_busy_property())
    }

    /// Sets whether at least one task is running.
    pub fn set_is_busy(&self, value: bool) {
        self.set_value(Self::is_busy_property(), value)
    }

    /// Returns the number of the tasks that are running.
    pub fn running_count(&self) -> i32 {
        self.get_value(Self::running_count_property())
    }

    /// Sets the number of the tasks that are running.
    pub fn set_running_count(&self, value: i32) {
        self.set_value(Self::running_count_property(), value)
    }

    /// Returns the name of the task that was started last.
    pub fn current_task_name(&self) -> Option<String> {
        self.get_value(Self::current_task_name_property())
    }

    /// Sets the name of the task that was started last.
    pub fn set_current_task_name(&self, value: Option<String>) {
        self.set_value(Self::current_task_name_property(), value)
    }

    /// Fills the page with the tasks of the launcher.
    pub fn set_task_items(&self, items: Vec<Ref<TaskItemViewModel>>) {
        let list: Rc<FerroList<Ref<TaskItemViewModel>>> = Rc::new(FerroList::new());
        for item in items {
            list.add(item);
        }
        self.set_tasks(Some(ItemsSource::from(list)));
    }
}
