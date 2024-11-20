use gloo_net::http::Request;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use yew_router::prelude::*;
use wasm_bindgen::JsValue;
use crate::{vote_option_manager::VoteOptionManager, styles::*, Route, hcaptcha::HCaptcha};
use shared::{models::*, error::ErrorResponse, user_info::generate_browser_fingerprint};
use std::future::Future;
use std::pin::Pin;
use gloo_timers::callback::Timeout;
use crate::config::CONFIG;

const MAX_TITLE_LENGTH: usize = 100;
const MAX_DESCRIPTION_LENGTH: usize = 500;
const MAX_OPTION_LENGTH: usize = 40;
const MAX_OPTIONS: usize = 20;
const MAX_DURATION_DAYS: i64 = 6;

#[derive(Clone)]
pub struct FormState {
    title: String,
    description: String,
    options: Vec<String>,
    days: i32,
    hours: i32, 
    minutes: i32,
    csrf_token: Option<String>,
    captcha_token: Option<String>,
    error: Option<String>,
    submitting: bool,
}

impl Default for FormState {
    fn default() -> Self {
        Self {
            title: String::new(),
            description: String::new(),
            options: Vec::new(),
            days: 0,
            hours: 0,
            minutes: 0,
            csrf_token: None,
            captcha_token: None,
            error: None,
            submitting: false,
        }
    }
}

pub struct VoteCreate {
    state: FormState,
    navigator: Navigator,
}

pub enum Msg {
    UpdateField(String, String),
    UpdateOptions(Vec<String>),
    TokenReceived(String),
    Submit,
    SubmitResult(Result<Vote, String>),
    CaptchaVerified(String),
    CaptchaExpired,
    CaptchaError,
}

impl Component for VoteCreate {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let component = Self {
            state: FormState::default(),
            navigator: ctx.link().navigator().unwrap(),
        };
        
        ctx.link().send_future(async {
            let req = Request::get(&format!("{}/csrf-token", CONFIG.api_base_url));
            
            let response = match req.send().await {
                Ok(resp) => resp,
                Err(e) => return Msg::SubmitResult(Err(e.to_string())),
            };
        
            match response.text().await {
                Ok(token) => Msg::TokenReceived(token),
                Err(e) => Msg::SubmitResult(Err(e.to_string())),
            }
        });

        component
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::UpdateField(field, value) => {
                match field.as_str() {
                    "title" => self.state.title = value,
                    "description" => self.state.description = value,
                    "days" => self.state.days = value.parse().unwrap_or(0),
                    "hours" => self.state.hours = value.parse().unwrap_or(0),
                    "minutes" => self.state.minutes = value.parse().unwrap_or(0),
                    _ => return false,
                }
                true
            },
            Msg::UpdateOptions(options) => {
                self.state.options = options;
                true
            },
            Msg::TokenReceived(token) => {
                self.state.csrf_token = Some(token);
                true
            },
            Msg::Submit => {
                if self.state.captcha_token.is_none() {
                    self.state.error = Some("Please complete the captcha verification".into());
                    return true;
                }
                if let Err(error) = self.validate() {
                    self.state.error = Some(error);
                    return true;
                }
    
                let request = self.create_request();
                self.state.submitting = true;
                self.state.error = None;
    
                ctx.link().send_future(async move {
                    Msg::SubmitResult(submit_vote(request).await)
                });
                true
            },
            Msg::SubmitResult(result) => {
                match result {
                    Ok(vote) => {
                        self.navigator.push(&Route::Vote { id: vote.id.to_string() });
                        false
                    }
                    Err(error) => {
                        if error.contains("profanity") {
                            self.state.captcha_token = None;
                            self.state.error = Some(error);
                            self.state.submitting = false;

                            Timeout::new(100, || {
                                if let Some(window) = web_sys::window() {
                                    if let Ok(hcaptcha) = js_sys::Reflect::get(&window, &JsValue::from_str("hcaptcha")) {
                                        let _ = js_sys::Reflect::get(&hcaptcha, &JsValue::from_str("reset"))
                                            .and_then(|reset| {
                                                if reset.is_function() {
                                                    let func = js_sys::Function::from(reset);
                                                    let _ = func.call0(&hcaptcha);
                                                }
                                                Ok(JsValue::UNDEFINED)
                                            });
                                    }
                                }
                            }).forget();

                            true
                        } else {
                            self.state.error = Some(error);
                            self.state.submitting = false;
                            true
                        }
                    }
                }
            },
            Msg::CaptchaVerified(token) => {
                self.state.captcha_token = Some(token);
                self.state.error = None;
                true
            },
            Msg::CaptchaExpired => {
                self.state.captcha_token = None;
                if self.state.submitting {
                    self.state.error = Some("Captcha expired, please verify again".into());
                    self.state.submitting = false;
                }
                true
            },
            Msg::CaptchaError => {
                self.state.captcha_token = None;
                self.state.error = Some("Captcha verification failed. Please try again.".into());
                self.state.submitting = false;
                true
            },
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class={BG_PAGE}>
                <div class={CARD}>
                    <h1 class={HEADING_LG}>{"Create New Vote"}</h1>
                    {if let Some(error) = &self.state.error {
                        html! { <div class={alert_style("error")}>{error}</div> }
                    } else { html! {} }}
                    {self.render_form(ctx)}
                </div>
            </div>
        }
    }
}

impl VoteCreate {
    fn validate(&self) -> Result<(), String> {
        if self.state.captcha_token.is_none() {
            return Err("Please complete the captcha verification".into());
        }

        if self.state.title.trim().is_empty() {
            return Err("Title is required".into());
        }
        if self.state.options.len() < 2 {
            return Err("At least two options are required".into());
        }
        if self.state.options.len() > MAX_OPTIONS {
            return Err(format!("Maximum {} options allowed", MAX_OPTIONS));
        }

        let total_hours = (self.state.days * 24) + self.state.hours;
        if total_hours == 0 && self.state.minutes == 0 {
            return Err("Duration must be at least 1 minute".into());
        }
        if i64::from(self.state.days) >= MAX_DURATION_DAYS && 
        (self.state.hours > 23 || self.state.minutes > 59) {
            return Err("Duration cannot exceed 6 days, 23 hours, 59 minutes".into());
        }
        Ok(())
    }

    fn create_request(&self) -> CreateVoteRequest {
        CreateVoteRequest {
            csrf_token: self.state.csrf_token.clone().unwrap_or_default(),
            captcha_token: self.state.captcha_token.clone().unwrap_or_default(),
            title: self.state.title.clone(),
            description: self.state.description.clone(),
            options: self.state.options.clone(),
            duration_hours: (self.state.days * 24) + self.state.hours,
            duration_minutes: self.state.minutes,
            user_fingerprint: generate_browser_fingerprint(),
        }
    }

    fn render_form(&self, ctx: &Context<Self>) -> Html {
        let onsubmit = ctx.link().callback(|e: SubmitEvent| {
            e.prevent_default();
            Msg::Submit
        });
    
        let submit_disabled = self.state.submitting 
            || self.state.options.len() < 2 
            || self.state.options.len() > MAX_OPTIONS
            || self.state.captcha_token.is_none()
            || self.state.csrf_token.is_none()
            || self.state.title.trim().is_empty();

        web_sys::console::log_1(&format!(
            "Form state: submitting={}, options={}, captcha={}, csrf={}, title={}",
            self.state.submitting,
            self.state.options.len(),
            self.state.captcha_token.is_some(),
            self.state.csrf_token.is_some(),
            !self.state.title.trim().is_empty()
        ).into());
    
        html! {
            <form {onsubmit} class={SPACE_Y_LG}>
                {self.render_input(ctx, "title", "Title", MAX_TITLE_LENGTH)}
                {self.render_textarea(ctx, "description", "Description", MAX_DESCRIPTION_LENGTH)}
                {self.render_duration(ctx)}
                {self.render_options(ctx)}
    
                <div class="mb-4 mt-4">
                    <HCaptcha
                        site_key="ce22ff56-8b34-4c5c-8a2c-225ad14caba0"
                        on_verify={ctx.link().callback(Msg::CaptchaVerified)}
                        on_expire={ctx.link().callback(|_| Msg::CaptchaExpired)}
                        on_error={ctx.link().callback(|_| Msg::CaptchaError)}
                    />
                </div>
    
                <button 
                    type="submit" 
                    class={button_primary(true)}
                    disabled={submit_disabled}
                >
                    {if self.state.submitting { "Creating..." } else { "Create Vote" }}
                </button>

                <div class="text-sm text-gray-400 mt-2">
                    {"Captcha Status: "} {if self.state.captcha_token.is_some() { "Verified" } else { "Not Verified" }}
                </div>
            </form>
        }
    }

    fn render_input(&self, ctx: &Context<Self>, field_name: &'static str, label: &str, max_length: usize) -> Html {
        let value = match field_name {
            "title" => &self.state.title,
            "description" => &self.state.description,
            _ => return html! {},
        };

        let oninput = ctx.link().callback(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            Msg::UpdateField(field_name.to_string(), input.value())
        });

        html! {
            <div class={SPACE_Y_BASE}>
                <label class={TEXT_LABEL}>
                    {format!("{} ({}/{})", label, value.len(), max_length)}
                </label>
                <input type="text" class={INPUT_BASE} value={value.clone()}
                    maxlength={max_length.to_string()} {oninput}
                    placeholder={format!("Enter {}", label.to_lowercase())} />
            </div>
        }
    }

    fn render_textarea(&self, ctx: &Context<Self>, field: &str, label: &str, max_length: usize) -> Html {
        let field = field.to_string();
        let value = match field.as_str() {
            "description" => &self.state.description,
            _ => return html! {},
        };
        let oninput = ctx.link().callback(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            Msg::UpdateField(field.clone(), input.value())
        });
    
        html! {
            <div class={SPACE_Y_BASE}>
                <label class={TEXT_LABEL}>
                    {format!("{} ({}/{})", label, value.len(), max_length)}
                </label>
                <textarea class={INPUT_BASE} rows="4"
                    value={value.clone()}
                    maxlength={max_length.to_string()} 
                    oninput={oninput}
                    placeholder={format!("Enter {}", label.to_lowercase())} 
                />
            </div>
        }
    }

    fn render_duration(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class={SPACE_Y_BASE}>
                <label class={TEXT_LABEL}>{"Duration"}</label>
                <div class={GRID_COLS_3}>
                    {self.render_select(ctx, "days", "Days", 0..=MAX_DURATION_DAYS)}
                    {self.render_select(ctx, "hours", "Hours", 0..=23)}
                    {self.render_select(ctx, "minutes", "Minutes", 0..=59)}
                </div>
            </div>
        }
    }

    fn render_select(&self, ctx: &Context<Self>, field: &str, label: &str, range: std::ops::RangeInclusive<i64>) -> Html {
        let field = field.to_string();
        let value = match field.as_str() {
            "days" => self.state.days,
            "hours" => self.state.hours,
            "minutes" => self.state.minutes,
            _ => return html! {},
        };
        let onchange = ctx.link().callback(move |e: Event| {
            let select: HtmlSelectElement = e.target_unchecked_into();
            Msg::UpdateField(field.clone(), select.value())
        });
    
        html! {
            <div>
                <label class={TEXT_LABEL_SM}>{label}</label>
                <select class={INPUT_BASE} value={value.to_string()}
                    onchange={onchange}>
                    {for range.map(|v| html! {
                        <option value={v.to_string()} selected={i64::from(value) == v}>{v}</option>
                    })}
                </select>
            </div>
        }
    }

    fn render_options(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class={SPACE_Y_BASE}>
                <div class={FLEX_BETWEEN}>
                    <label class={TEXT_LABEL}>{"Vote Options"}</label>
                    <span class={TEXT_MUTED}>
                        {format!("Options: {}/{}", self.state.options.len(), MAX_OPTIONS)}
                    </span>
                </div>
                <VoteOptionManager
                    options={self.state.options.clone()}
                    error={self.state.error.clone()}
                    max_length={MAX_OPTION_LENGTH}
                    on_change={ctx.link().callback(Msg::UpdateOptions)}
                    can_add_more={self.state.options.len() < MAX_OPTIONS}
                />
            </div>
        }
    }
}

fn submit_vote(request: CreateVoteRequest) -> Pin<Box<dyn Future<Output = Result<Vote, String>> + 'static>> {
    Box::pin(async move {
        let response = Request::post(&format!("{}/vote", CONFIG.api_base_url))
            .json(&request)
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;

        match response.status() {
            200 => response.json::<Vote>().await.map_err(|e| e.to_string()),
            429 => Err("Please wait an hour before creating another vote".into()),
            400 => {
                let error = response.json::<ErrorResponse>().await
                    .map(|err| err.error)
                    .unwrap_or_else(|_| "Invalid request".into());
                Err(error)
            },
            403 => {
                let error = response.json::<ErrorResponse>().await
                    .map(|err| err.error)
                    .unwrap_or_else(|_| "Please try submitting again".into());

                if error.starts_with("CSRF token expired") {
                    if let Some(new_token) = error.split(": ").nth(1) {
                        let mut new_request = request.clone();
                        new_request.csrf_token = new_token.to_string();
                        return submit_vote(new_request).await;
                    }
                }
                Err(error)
            },
            _ => Err("An unexpected error occurred".into())
        }
    })
}