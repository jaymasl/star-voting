use yew::prelude::*;
use gloo_net::http::Request;
use std::collections::HashMap;
use serde::Serialize;
use shared::models::{Vote, BallotResponse};
use yew_router::prelude::*;
use web_sys::window;
use gloo_timers::callback::Timeout;
use wasm_bindgen::JsValue;
use crate::Route;
use crate::styles::*;
use crate::hcaptcha::HCaptcha;
use crate::config::CONFIG;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BallotRequest {
    csrf_token: String,
    captcha_token: String,
    scores: HashMap<String, i8>,
    user_fingerprint: String,
}

#[derive(PartialEq)]
enum SubmissionState {
    Ready,
    Submitting,
    Success(BallotResponse),
    Error(String),
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub vote: Vote,
    pub csrf_token: String,
}

pub enum Msg {
    UpdateScore(String, i8),
    Submit,
    SubmissionComplete(Result<BallotResponse, String>),
    CaptchaVerified(String),
    CaptchaExpired,
    CaptchaError,
}

pub struct VoteBallot {
    scores: HashMap<String, i8>,
    state: SubmissionState,
    captcha_token: Option<String>,
}

impl Component for VoteBallot {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            scores: ctx.props().vote.options.iter()
                .map(|opt| (opt.clone(), 0))
                .collect(),
            state: SubmissionState::Ready,
            captcha_token: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::UpdateScore(option, score) => {
                if matches!(self.state, SubmissionState::Ready) {
                    self.scores.insert(option, score);
                    true
                } else {
                    false
                }
            }
            Msg::Submit => {
                if matches!(self.state, SubmissionState::Submitting) {
                    return false;
                }
                
                if self.captcha_token.is_none() {
                    self.state = SubmissionState::Error("Please complete the captcha verification".into());
                    return true;
                }

                self.state = SubmissionState::Submitting;
                let scores = self.scores.clone();
                let vote_id = ctx.props().vote.id;
                let csrf_token = ctx.props().csrf_token.clone();
                let captcha_token = self.captcha_token.clone().unwrap_or_default();
                let request = BallotRequest { 
                    csrf_token,
                    captcha_token,
                    scores,
                    user_fingerprint: shared::user_info::generate_browser_fingerprint(),
                };
                
                ctx.link().send_future(async move {
                    let req = match Request::post(&format!("{}/vote/{}/ballot", CONFIG.api_base_url, vote_id))
                        .json(&request)
                        .map_err(|e| e.to_string()) {
                        Ok(req) => req,
                        Err(e) => return Msg::SubmissionComplete(Err(e)),
                    };
                    let response = match req.send().await {
                        Ok(resp) => resp,
                        Err(e) => return Msg::SubmissionComplete(Err(e.to_string())),
                    };
                    match response.status() {
                        200 => {
                            match response.json::<BallotResponse>().await {
                                Ok(ballot_response) => Msg::SubmissionComplete(Ok(ballot_response)),
                                Err(e) => Msg::SubmissionComplete(Err(format!("Failed to parse response: {}", e)))
                            }
                        },
                        429 => Msg::SubmissionComplete(Err("You're voting too quickly. Please try again.".into())),
                        403 => Msg::SubmissionComplete(Err("Action not allowed: The voting period may have ended, or you may have already cast your ballot.".into())),
                        _ => Msg::SubmissionComplete(Err("Failed to submit ballot.".into()))
                    }
                });
                true
            }
            Msg::SubmissionComplete(result) => {
                match result {
                    Ok(ballot_response) => {
                        self.state = SubmissionState::Success(ballot_response);
                        let timeout = Timeout::new(100, move || {
                            if let Some(window) = window() {
                                window.scroll_to_with_x_and_y(0.0, 9999.0);
                            }
                        });
                        timeout.forget();
                    },
                    Err(error) => {
                        self.state = SubmissionState::Error(error);
                        self.captcha_token = None;

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
                    }
                }
                true
            }
            Msg::CaptchaVerified(token) => {
                self.captcha_token = Some(token);
                true
            }
            Msg::CaptchaExpired => {
                self.captcha_token = None;
                if matches!(self.state, SubmissionState::Submitting) {
                    self.state = SubmissionState::Error("Captcha expired, please verify again".into());
                }
                true
            }
            Msg::CaptchaError => {
                self.captcha_token = None;
                self.state = SubmissionState::Error("Captcha verification failed".into());
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="space-y-6">
                <div class="space-y-4">
                    {for ctx.props().vote.options.iter().map(|option| {
                        self.render_option(ctx, option)
                    })}
                </div>

                <div class="mb-4">
                    <HCaptcha
                        site_key="ce22ff56-8b34-4c5c-8a2c-225ad14caba0"
                        on_verify={ctx.link().callback(Msg::CaptchaVerified)}
                        on_expire={ctx.link().callback(|_| Msg::CaptchaExpired)}
                        on_error={ctx.link().callback(|_| Msg::CaptchaError)}
                    />
                </div>
    
                <div class="space-y-4">
                    {self.render_submission_status()}
                    {self.render_submission_controls(ctx)}
                </div>
            </div>
        }
    }
}

impl VoteBallot {
    fn render_option(&self, ctx: &Context<Self>, option: &str) -> Html {
        let current_score = *self.scores.get(option).unwrap_or(&0);
        let is_submitting = matches!(self.state, SubmissionState::Submitting);

        html! {
            <div class="space-y-4 p-6 mb-4 border border-gray-700 rounded-lg bg-gray-800 shadow-lg">
                <div class="text-xl font-semibold text-gray-200 tracking-wide break-words">
                    {option}
                </div>
                <div class="flex flex-col space-y-2 sm:flex-row sm:space-y-0 sm:space-x-4 sm:items-center">
                    <div class="flex items-center justify-center w-16 h-16 rounded-lg bg-gray-700 text-4xl font-bold text-center text-gray-300 border border-gray-500 shadow-md">
                        {current_score}
                    </div>
                    <div class="grid grid-cols-6 gap-2 w-full sm:flex sm:w-auto">
                        {(0..=5).map(|score| {
                            let opt = option.to_string();
                            let onclick = ctx.link().callback(move |_| {
                                Msg::UpdateScore(opt.clone(), score)
                            });

                            let button_classes = if current_score == score {
                                if score == 0 {
                                    "w-12 h-12 rounded-full flex items-center justify-center transition-all duration-150 ease-out bg-gray-600 text-gray-300 text-lg shadow ring-2 ring-white"
                                } else {
                                    "w-12 h-12 rounded-full flex items-center justify-center transform scale-105 transition-all duration-150 ease-out bg-blue-500 text-white text-lg shadow-lg ring-2 ring-blue-400 ring-offset-1 ring-offset-gray-800"
                                }
                            } else {
                                "w-12 h-12 rounded-full flex items-center justify-center transition-all duration-150 ease-out bg-gray-700 hover:bg-gray-600 text-gray-400 text-lg hover:shadow-md"
                            };

                            html! {
                                <button
                                    type="button"
                                    disabled={is_submitting}
                                    onclick={onclick}
                                    class={button_classes}
                                >
                                    {score}
                                </button>
                            }
                        }).collect::<Html>()}
                    </div>
                </div>
            </div>
        }
    }

    fn render_submission_controls(&self, ctx: &Context<Self>) -> Html {
        match &self.state {
            SubmissionState::Ready | SubmissionState::Error(_) => html! {
                <div class="flex flex-col sm:flex-row gap-4">
                    <button
                        type="button"
                        onclick={ctx.link().callback(|_| Msg::Submit)}
                        disabled={self.captcha_token.is_none()}
                        class={combine_classes(
                            "flex-1 bg-blue-600 hover:bg-blue-700 text-white px-8 py-4 rounded-lg text-lg font-semibold shadow-lg transform transition-all duration-150 hover:scale-105 focus:outline-none focus:ring-4 focus:ring-blue-500 focus:ring-opacity-50",
                            "disabled:opacity-50 disabled:cursor-not-allowed"
                        )}
                    >
                        {"Submit Ballot"}
                    </button>
                    <Link<Route> to={Route::Home}
                        classes="flex-1 bg-gray-600 hover:bg-gray-700 text-white px-8 py-4 rounded-lg text-lg font-semibold shadow-lg text-center transform transition-all duration-150 hover:scale-105">
                        {"Cancel"}
                    </Link<Route>>
                </div>
            },
            SubmissionState::Submitting => html! {
                <div class="flex justify-center">
                    <div class="animate-pulse text-blue-400">
                        {"Submitting ballot..."}
                    </div>
                </div>
            },
            SubmissionState::Success(_) => html! {
                <Link<Route> to={Route::Home}
                    classes="block w-full text-center bg-green-600 hover:bg-green-700 text-white px-8 py-4 rounded-lg text-lg font-semibold shadow-lg">
                    {"Return to Home"}
                </Link<Route>>
            },
        }
    }

    fn render_submission_status(&self) -> Html {
        match &self.state {
            SubmissionState::Error(error) => html! {
                <div class="text-center p-6 bg-red-900/50 border border-red-600 rounded-lg">
                    <p class="text-red-200">{error}</p>
                </div>
            },
            SubmissionState::Success(response) => html! {
                <div class="text-center p-6 bg-green-900/50 border border-green-600 rounded-lg">
                    <h3 class="text-xl font-semibold mb-2 text-green-400">{"Ballot Cast Successfully!"}</h3>
                    <div class="space-y-2">
                        <p class="text-gray-300">{"Your vote has been recorded."}</p>
                        <div class="bg-gray-800/50 p-3 rounded-lg">
                            <div class="flex justify-between items-center">
                                <span class="text-gray-400">{"Unique Ballot ID Number:"}</span>
                                <span class="text-gray-200 font-mono">{response.ballot_id}</span>
                            </div>
                        </div>
                    </div>
                </div>
            },
            _ => html! {}
        }
    }
}