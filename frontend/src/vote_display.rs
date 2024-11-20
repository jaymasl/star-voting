use yew::prelude::*;
use gloo_net::http::Request;
use crate::vote_ballot::VoteBallot;
use yew_router::prelude::*;
use crate::Route;
use crate::styles::*;
use crate::config::CONFIG;
use time::{OffsetDateTime, Duration};
use std::rc::Rc;
use futures::try_join;
use shared::models::*;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub id: String,
}

#[derive(Clone)]
enum State {
    Loading,
    Active { vote: Rc<Vote>, csrf_token: String, time_remaining: Duration },
    Error(String),
    NotFound,
}

pub struct VoteDisplay {
    state: State,
}

pub enum Msg {
    DataReceived(Vote, String),
    Error(String),
}

impl Component for VoteDisplay {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let id = ctx.props().id.clone();
        
        ctx.link().send_future(async move {
            match fetch_vote_data(&id).await {
                Ok((vote, token)) => Msg::DataReceived(vote, token),
                Err(e) => Msg::Error(e),
            }
        });

        Self { state: State::Loading }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::DataReceived(vote, token) => {
                let now = OffsetDateTime::now_utc();
                let voting_ends_at = vote.voting_ends_at;
                if now > voting_ends_at {
                    if let Some(navigator) = ctx.link().navigator() {
                        navigator.push(&Route::Results { id: vote.id.to_string() });
                    }
                    true
                } else {
                    let vote = Rc::new(vote);
                    let time_remaining = voting_ends_at - now;
                    self.state = State::Active {
                        vote: vote.clone(),
                        csrf_token: token,
                        time_remaining,
                    };
                    true
                }
            }
            Msg::Error(error) => {
                self.state = if error.contains("not found") {
                    State::NotFound
                } else {
                    State::Error(error)
                };
                true
            }
        }
    }

    fn view(&self, _: &Context<Self>) -> Html {
        match &self.state {
            State::Loading => render_loading(),
            State::Active { vote, csrf_token, time_remaining } => render_active(vote, csrf_token, *time_remaining),
            State::Error(error) => render_error(error),
            State::NotFound => render_not_found(),
        }
    }
}

async fn fetch_vote_data(id: &str) -> Result<(Vote, String), String> {
    let (vote_resp, token_resp) = try_join!(
        Request::get(&format!("{}/vote/{}", CONFIG.api_base_url, id)).send(),
        Request::get(&format!("{}/csrf-token", CONFIG.api_base_url)).send()
    ).map_err(|e| e.to_string())?;

    let vote = vote_resp.json::<Vote>().await
        .map_err(|_| "Failed to parse vote data".to_string())?;
    let token = token_resp.text().await
        .map_err(|_| "Failed to get CSRF token".to_string())?;

    Ok((vote, token))
}

fn render_loading() -> Html {
    html! {
        <div class="flex justify-center p-8">
            <div class="animate-pulse text-lg text-gray-400">{"Loading vote details..."}</div>
        </div>
    }
}

fn render_error(error: &str) -> Html {
    html! {
        <div class="container mx-auto px-4 py-8">
            <div class={alert_style("error")}>{error}</div>
        </div>
    }
}

fn render_not_found() -> Html {
    html! {
        <div class="container mx-auto px-4 py-8">
            <div class={alert_style("error")}>{"Vote not found"}</div>
        </div>
    }
}

fn render_active(vote: &Vote, csrf_token: &str, time_remaining: Duration) -> Html {
    html! {
        <div class="container mx-auto px-4 py-6 max-w-2xl">
            <div class="bg-gray-800 rounded-lg shadow-xl p-6 text-white">
                <h1 class="text-2xl font-bold mb-4 break-words text-gray-100">{&vote.title}</h1>
                <p class="mb-6 text-lg text-gray-300 break-words whitespace-pre-wrap">{&vote.description}</p>
                
                <div class="bg-gray-700/50 p-4 rounded-lg mb-6">
                    <h2 class="font-semibold mb-2">{"Time Remaining"}</h2>
                    <p class="text-lg">{render_time_text(time_remaining)}</p>
                </div>

                <div class="border-t border-gray-600 pt-4 mb-3">
                    {render_instructions(vote)}
                    <VoteBallot vote={vote.clone()} csrf_token={csrf_token.to_string()} />
                </div>
            </div>
        </div>
    }
}

fn render_instructions(vote: &Vote) -> Html {
    html! {
        <div class="bg-gray-700/30 p-4 rounded-lg mb-6">
            <h3 class="font-semibold mb-2">{"Voting Instructions"}</h3>
            <ul class="list-disc list-inside text-sm font-medium space-y-2 text-gray-300">
                <li>{"Rate each option from 0 (worst) to 5 (best)"}</li>
                <li>{"Identical scores show equal preference"}</li>
                <li>{"Unrated options receive 0"}</li>
                <li>{format!("Options: {}", vote.options.len())}</li>
            </ul>
        </div>
    }
}

fn render_time_text(d: Duration) -> String {
    if d.whole_hours() > 0 {
        format!("{}h {}m remaining", d.whole_hours(), d.whole_minutes() % 60)
    } else if d.whole_minutes() > 0 {
        format!("{}m {}s remaining", d.whole_minutes(), d.whole_seconds() % 60)
    } else {
        format!("{}s remaining", d.whole_seconds())
    }
}