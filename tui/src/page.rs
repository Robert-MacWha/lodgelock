use tlock_hdk::tlock_api::component::Component;

#[derive(Clone, Debug)]
pub enum PageItem {
    Label(String),
    Button {
        text: String,
        id: String,
    },
    Input {
        label: String,
        id: String,
        form_id: String,
    },
    Dropdown {
        label: String,
        options: Vec<String>,
        id: String,
        form_id: String,
    },
    Submit {
        text: String,
        form_id: String,
    },
}

impl PageItem {
    pub fn is_interactive(&self) -> bool {
        !matches!(self, PageItem::Label(_))
    }
}

pub fn flatten(component: &Component) -> Vec<PageItem> {
    let mut out = Vec::new();
    flatten_inner(component, None, &mut out);
    out
}

fn flatten_inner(component: &Component, form_ctx: Option<&str>, out: &mut Vec<PageItem>) {
    match component {
        Component::Container { children } => {
            for child in children {
                flatten_inner(child, form_ctx, out);
            }
        }
        Component::Heading { text } => out.push(PageItem::Label(format!("## {text}"))),
        Component::Heading2 { text } => out.push(PageItem::Label(format!("# {text}"))),
        Component::Text { text } => out.push(PageItem::Label(text.clone())),
        Component::UnorderedList { items } => {
            for (key, child) in items {
                out.push(PageItem::Label(format!("• {key}")));
                flatten_inner(child, form_ctx, out);
            }
        }
        Component::Form { id, fields } => {
            for field in fields {
                flatten_inner(field, Some(id), out);
            }
        }
        Component::ButtonInput { text, id } => out.push(PageItem::Button {
            text: text.clone(),
            id: id.clone(),
        }),
        Component::TextInput { label, id, .. } => {
            if let Some(form_id) = form_ctx {
                out.push(PageItem::Input {
                    label: label.clone(),
                    id: id.clone(),
                    form_id: form_id.to_string(),
                });
            } else {
                out.push(PageItem::Label(format!("[input: {label}]")));
            }
        }
        Component::DropdownInput {
            label, options, id, ..
        } => {
            if let Some(form_id) = form_ctx {
                out.push(PageItem::Dropdown {
                    label: label.clone(),
                    options: options.clone(),
                    id: id.clone(),
                    form_id: form_id.to_string(),
                });
            } else {
                out.push(PageItem::Label(format!("[dropdown: {label}]")));
            }
        }
        Component::SubmitInput { text } => {
            if let Some(form_id) = form_ctx {
                out.push(PageItem::Submit {
                    text: text.clone(),
                    form_id: form_id.to_string(),
                });
            }
        }
        Component::Chain { id } => out.push(PageItem::Label(format!("Chain: {id}"))),
        Component::Account { id } => out.push(PageItem::Label(format!("Account: {id}"))),
        Component::Asset { id, balance } => {
            let bal = balance
                .map(|b| b.to_string())
                .unwrap_or_else(|| "—".to_string());
            out.push(PageItem::Label(format!("Asset: {id}  balance: {bal}")));
        }
        Component::EntityId { id } => out.push(PageItem::Label(format!("Entity: {id}"))),
        Component::Hex { data } => out.push(PageItem::Label(format!("0x{}", hex::encode(data)))),
    }
}
