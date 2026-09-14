use serde::{Deserialize, Serialize};

/// Dialog Action types for Java 1.21.4+ client dialogs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DialogAction {
    OpenUrl { url: String },
    Custom { id: String, payload: Option<Vec<u8>> },
}

/// Action button displayed inside a dialog window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogButton {
    pub text: String,
    pub tooltip: Option<String>,
    pub width: Option<u32>,
    pub action: DialogAction,
}

impl DialogButton {
    pub fn new(text: impl Into<String>, action: DialogAction) -> Self {
        Self {
            text: text.into(),
            tooltip: None,
            width: None,
            action,
        }
    }

    pub fn action(text: impl Into<String>, action_id: impl Into<String>) -> Self {
        Self::new(
            text,
            DialogAction::Custom {
                id: action_id.into(),
                payload: None,
            },
        )
    }

    pub fn action_with_payload(
        text: impl Into<String>,
        action_id: impl Into<String>,
        payload: Vec<u8>,
    ) -> Self {
        Self::new(
            text,
            DialogAction::Custom {
                id: action_id.into(),
                payload: Some(payload),
            },
        )
    }

    pub fn url(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self::new(text, DialogAction::OpenUrl { url: url.into() })
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn width(mut self, width: u32) -> Self {
        self.width = Some(width);
        self
    }
}

/// Dialog input element types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DialogInput {
    Boolean {
        id: String,
        label: String,
        default_value: bool,
    },
    Text {
        id: String,
        label: String,
        placeholder: String,
        default_value: String,
    },
    NumberRange {
        id: String,
        label: String,
        min: f32,
        max: f32,
        initial: f32,
        step: f32,
        label_format: Option<String>,
    },
    SingleOption {
        id: String,
        label: String,
        options: Vec<String>,
        initial_index: u32,
    },
}

/// Body components inside a dialog window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DialogBody {
    PlainMessage(String),
    Item(i32),
}

/// Java 1.21.4+ Native Dialog Box representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dialog {
    pub id: String,
    pub title: String,
    pub body: Vec<DialogBody>,
    pub inputs: Vec<DialogInput>,
    pub buttons: Vec<DialogButton>,
    pub can_close_with_escape: bool,
    pub external_title: Option<String>,
}

impl Dialog {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: String::new(),
            body: Vec::new(),
            inputs: Vec::new(),
            buttons: Vec::new(),
            can_close_with_escape: true,
            external_title: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn external_title(mut self, ext_title: impl Into<String>) -> Self {
        self.external_title = Some(ext_title.into());
        self
    }

    pub fn body_text(mut self, message: impl Into<String>) -> Self {
        self.body.push(DialogBody::PlainMessage(message.into()));
        self
    }

    pub fn add_button(mut self, button: DialogButton) -> Self {
        self.buttons.push(button);
        self
    }

    pub fn add_action_button(self, text: impl Into<String>, action_id: impl Into<String>) -> Self {
        self.add_button(DialogButton::action(text, action_id))
    }

    pub fn add_url_button(self, text: impl Into<String>, url: impl Into<String>) -> Self {
        self.add_button(DialogButton::url(text, url))
    }

    pub fn add_bool_input(
        mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        default_value: bool,
    ) -> Self {
        self.inputs.push(DialogInput::Boolean {
            id: id.into(),
            label: label.into(),
            default_value,
        });
        self
    }

    pub fn add_text_input(
        mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        placeholder: impl Into<String>,
        default_value: impl Into<String>,
    ) -> Self {
        self.inputs.push(DialogInput::Text {
            id: id.into(),
            label: label.into(),
            placeholder: placeholder.into(),
            default_value: default_value.into(),
        });
        self
    }

    pub fn add_number_range(
        mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        min: f32,
        max: f32,
        initial: f32,
        step: f32,
    ) -> Self {
        self.inputs.push(DialogInput::NumberRange {
            id: id.into(),
            label: label.into(),
            min,
            max,
            initial,
            step,
            label_format: None,
        });
        self
    }

    pub fn add_single_option(
        mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        options: Vec<String>,
        initial_index: u32,
    ) -> Self {
        self.inputs.push(DialogInput::SingleOption {
            id: id.into(),
            label: label.into(),
            options,
            initial_index,
        });
        self
    }

    pub fn can_close_with_escape(mut self, can_close: bool) -> Self {
        self.can_close_with_escape = can_close;
        self
    }
}

// =========================================================================
// Bedrock Crossplay Form Dialogs (SimpleForm, ModalForm, CustomForm)
// =========================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormImage {
    pub r#type: String, // "url" or "path"
    pub data: String,
}

impl FormImage {
    pub fn url(url: impl Into<String>) -> Self {
        Self {
            r#type: "url".to_string(),
            data: url.into(),
        }
    }

    pub fn path(path: impl Into<String>) -> Self {
        Self {
            r#type: "path".to_string(),
            data: path.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimpleFormButton {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<FormImage>,
}

impl SimpleFormButton {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            image: None,
        }
    }

    pub fn with_image(mut self, image: FormImage) -> Self {
        self.image = Some(image);
        self
    }
}

/// Simple Bedrock Form Dialog with button choices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimpleForm {
    pub r#type: String,
    pub title: String,
    pub content: String,
    pub buttons: Vec<SimpleFormButton>,
}

impl SimpleForm {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            r#type: "form".to_string(),
            title: title.into(),
            content: String::new(),
            buttons: Vec::new(),
        }
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    pub fn button(mut self, text: impl Into<String>) -> Self {
        self.buttons.push(SimpleFormButton::new(text));
        self
    }

    pub fn button_with_image(mut self, text: impl Into<String>, image: FormImage) -> Self {
        self.buttons.push(SimpleFormButton::new(text).with_image(image));
        self
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Modal Bedrock Form Dialog (2-button prompt: Yes/No, Confirm/Cancel).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModalForm {
    pub r#type: String,
    pub title: String,
    pub content: String,
    pub button1: String,
    pub button2: String,
}

impl ModalForm {
    pub fn new(
        title: impl Into<String>,
        content: impl Into<String>,
        button1: impl Into<String>,
        button2: impl Into<String>,
    ) -> Self {
        Self {
            r#type: "modal".to_string(),
            title: title.into(),
            content: content.into(),
            button1: button1.into(),
            button2: button2.into(),
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Custom Bedrock Form Element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CustomFormElement {
    #[serde(rename = "label")]
    Label { text: String },
    #[serde(rename = "input")]
    Input {
        text: String,
        placeholder: String,
        default: String,
    },
    #[serde(rename = "toggle")]
    Toggle { text: String, default: bool },
    #[serde(rename = "slider")]
    Slider {
        text: String,
        min: f32,
        max: f32,
        step: f32,
        default: f32,
    },
    #[serde(rename = "step_slider")]
    StepSlider {
        text: String,
        steps: Vec<String>,
        default: u32,
    },
    #[serde(rename = "dropdown")]
    Dropdown {
        text: String,
        options: Vec<String>,
        default: u32,
    },
}

/// Custom Bedrock Form Dialog with flexible inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomForm {
    pub r#type: String,
    pub title: String,
    pub content: Vec<CustomFormElement>,
}

impl CustomForm {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            r#type: "custom_form".to_string(),
            title: title.into(),
            content: Vec::new(),
        }
    }

    pub fn label(mut self, text: impl Into<String>) -> Self {
        self.content.push(CustomFormElement::Label { text: text.into() });
        self
    }

    pub fn input(
        mut self,
        text: impl Into<String>,
        placeholder: impl Into<String>,
        default: impl Into<String>,
    ) -> Self {
        self.content.push(CustomFormElement::Input {
            text: text.into(),
            placeholder: placeholder.into(),
            default: default.into(),
        });
        self
    }

    pub fn toggle(mut self, text: impl Into<String>, default: bool) -> Self {
        self.content.push(CustomFormElement::Toggle {
            text: text.into(),
            default,
        });
        self
    }

    pub fn slider(
        mut self,
        text: impl Into<String>,
        min: f32,
        max: f32,
        step: f32,
        default: f32,
    ) -> Self {
        self.content.push(CustomFormElement::Slider {
            text: text.into(),
            min,
            max,
            step,
            default,
        });
        self
    }

    pub fn dropdown(
        mut self,
        text: impl Into<String>,
        options: Vec<String>,
        default: u32,
    ) -> Self {
        self.content.push(CustomFormElement::Dropdown {
            text: text.into(),
            options,
            default,
        });
        self
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

// =========================================================================
// Virtual Book GUI (Paper/Bukkit Player#openBook)
// =========================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Book {
    pub title: String,
    pub author: String,
    pub pages: Vec<String>,
}

impl Book {
    pub fn new(title: impl Into<String>, author: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            author: author.into(),
            pages: Vec::new(),
        }
    }

    pub fn add_page(mut self, page: impl Into<String>) -> Self {
        self.pages.push(page.into());
        self
    }

    pub fn pages(mut self, pages: &[impl AsRef<str>]) -> Self {
        for p in pages {
            self.pages.push(p.as_ref().to_string());
        }
        self
    }
}
