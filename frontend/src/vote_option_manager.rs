use yew::prelude::*;
use web_sys::HtmlInputElement;
use crate::styles::*;

const MAX_OPTIONS: usize = 20;

#[derive(Properties, PartialEq)]
pub struct VoteOptionManagerProps {
    pub options: Vec<String>,
    pub error: Option<String>,
    pub max_length: usize,
    pub on_change: Callback<Vec<String>>,
    pub can_add_more: bool,
}

#[derive(Clone)]
pub enum Msg {
    AddOption,
    UpdateInput(String),
    StartEdit(usize),
    UpdateEdit(String),
    SaveEdit,
    DeleteOption(usize),
}

pub struct VoteOptionManager {
    options: Vec<String>,
    input_value: String,
    editing_index: Option<usize>,
    edit_value: String,
    duplicate_error: Option<String>,
}

impl Component for VoteOptionManager {
    type Message = Msg;
    type Properties = VoteOptionManagerProps;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            options: ctx.props().options.clone(),
            input_value: String::new(),
            editing_index: None,
            edit_value: String::new(),
            duplicate_error: None,
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class={SPACE_Y_LG}>
                <div class="flex gap-2">
                    <div class={INPUT_GROUP}>
                        <input
                            type="text"
                            value={self.input_value.clone()}
                            placeholder="Enter an option"
                            maxlength={ctx.props().max_length.to_string()}
                            class={INPUT_BASE}
                            oninput={ctx.link().callback(|e: InputEvent| {
                                let target = e.target_unchecked_into::<HtmlInputElement>();
                                Msg::UpdateInput(target.value())
                            })}
                            onkeypress={ctx.link().batch_callback(|e: KeyboardEvent| {
                                if e.key() == "Enter" {
                                    e.prevent_default();
                                    vec![Msg::AddOption]
                                } else {
                                    vec![]
                                }
                            })}
                        />
                        <div class={TEXT_MUTED}>
                            {format!("Characters: {}/{}", self.input_value.len(), ctx.props().max_length)}
                        </div>
                    </div>
                    <button 
                        type="button"
                        onclick={ctx.link().callback(|_| Msg::AddOption)}
                        disabled={self.input_value.trim().is_empty() 
                            || !ctx.props().can_add_more 
                            || self.input_value.len() > ctx.props().max_length}
                        class={button_primary(false)}
                    >
                        {"Add"}
                    </button>
                </div>

                {if let Some(error) = &self.duplicate_error {
                    html! {
                        <div class={TEXT_ERROR}>
                            {error}
                        </div>
                    }
                } else if !self.input_value.trim().is_empty() && self.input_value.len() > ctx.props().max_length {
                    html! {
                        <div class={TEXT_ERROR}>
                            {"Option text exceeds maximum length"}
                        </div>
                    }
                } else if !ctx.props().can_add_more {
                    html! {
                        <div class={TEXT_ERROR}>
                            {format!("Maximum number of options ({MAX_OPTIONS}) reached")}
                        </div>
                    }
                } else { html!{} }}

                <ul class={SPACE_Y_BASE}>
                    {for self.options.iter().enumerate().map(|(index, option)| {
                        let is_editing = self.editing_index == Some(index);
                        html! {
                            <li class={CARD_SECTION}>
                                if is_editing {
                                    <div class={INPUT_GROUP}>
                                        <div class="flex gap-2">
                                            <input
                                                type="text"
                                                value={self.edit_value.clone()}
                                                maxlength={ctx.props().max_length.to_string()}
                                                class={INPUT_BASE}
                                                oninput={ctx.link().callback(|e: InputEvent| {
                                                    let target = e.target_unchecked_into::<HtmlInputElement>();
                                                    Msg::UpdateEdit(target.value())
                                                })}
                                                onkeypress={ctx.link().batch_callback(|e: KeyboardEvent| {
                                                    if e.key() == "Enter" {
                                                        e.prevent_default();
                                                        vec![Msg::SaveEdit]
                                                    } else {
                                                        vec![]
                                                    }
                                                })}
                                            />
                                            <button 
                                                type="button"
                                                onclick={ctx.link().callback(|_| Msg::SaveEdit)}
                                                disabled={self.edit_value.trim().is_empty() || self.edit_value.len() > ctx.props().max_length}
                                                class={combine_classes(BUTTON_BASE, BUTTON_SUCCESS)}
                                            >
                                                {"Save"}
                                            </button>
                                        </div>
                                        <div class={TEXT_MUTED}>
                                            {format!("Characters: {}/{}", self.edit_value.len(), ctx.props().max_length)}
                                        </div>
                                    </div>
                                } else {
                                    <div class="flex gap-2 flex-wrap items-start">
                                        <span class="text-white break-words flex-grow">{option}</span>
                                        <div class="flex gap-2">
                                            <button 
                                                type="button"
                                                onclick={ctx.link().callback(move |_| Msg::StartEdit(index))}
                                                class={combine_classes(BUTTON_BASE, BUTTON_WARNING)}
                                            >
                                                {"Edit"}
                                            </button>
                                            <button 
                                                type="button"
                                                onclick={ctx.link().callback(move |_| Msg::DeleteOption(index))}
                                                class={combine_classes(BUTTON_BASE, BUTTON_DANGER)}
                                            >
                                                {"Delete"}
                                            </button>
                                        </div>
                                    </div>
                                }
                            </li>
                        }
                    })}
                </ul>
            </div>
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::AddOption => {
                let value = self.input_value.trim().to_string();
                if !value.is_empty() && ctx.props().can_add_more && value.len() <= ctx.props().max_length {
                    let is_duplicate = self.options.iter().any(|opt| 
                        opt.to_lowercase() == value.to_lowercase()
                    );
                    
                    if !is_duplicate {
                        self.options.push(value);
                        self.input_value.clear();
                        self.duplicate_error = None;
                        ctx.props().on_change.emit(self.options.clone());
                    } else {
                        self.duplicate_error = Some("Duplicate option".to_string());
                    }
                }
                true
            }
            Msg::UpdateInput(value) => {
                self.input_value = value;
                self.duplicate_error = None;
                true
            }
            Msg::StartEdit(index) => {
                if let Some(option) = self.options.get(index) {
                    self.editing_index = Some(index);
                    self.edit_value = option.clone();
                    self.duplicate_error = None;
                }
                true
            }
            Msg::UpdateEdit(value) => {
                self.edit_value = value;
                self.duplicate_error = None;
                true
            }
            Msg::SaveEdit => {
                if let Some(index) = self.editing_index {
                    let value = self.edit_value.trim().to_string();
                    if !value.is_empty() && value.len() <= ctx.props().max_length {
                        // Case insensitive check for duplicates, excluding the current option
                        let is_duplicate = self.options.iter().enumerate().any(|(i, opt)| 
                            i != index && opt.to_lowercase() == value.to_lowercase()
                        );
                        
                        if !is_duplicate {
                            self.options[index] = value;
                            self.editing_index = None;
                            self.edit_value.clear();
                            self.duplicate_error = None;
                            ctx.props().on_change.emit(self.options.clone());
                        } else {
                            self.duplicate_error = Some("Duplicate option".to_string());
                        }
                    }
                }
                true
            }
            Msg::DeleteOption(index) => {
                self.options.remove(index);
                self.editing_index = None;
                self.duplicate_error = None;
                ctx.props().on_change.emit(self.options.clone());
                true
            }
        }
    }
}