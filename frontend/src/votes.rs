use yew::prelude::*;
use yew_router::prelude::*;
use gloo_net::http::Request;
use gloo_timers::callback::Interval;
use crate::{Route, styles::*};
use shared::models::Vote;
use time::{OffsetDateTime, Duration};
use std::rc::Rc;
use crate::config::CONFIG;

#[derive(Clone, Default)]
pub struct VotesState {
    votes: Vec<Vote>,
    error: Option<String>,
    last_fetch: Option<OffsetDateTime>,
}

impl Reducible for VotesState {
    type Action = Msg;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut next = (*self).clone();
        match action {
            Msg::Fetch => {
                next.last_fetch = Some(OffsetDateTime::now_utc());
            },
            Msg::VotesReceived(votes) => {
                next.votes = votes;
                next.error = None;
            },
            Msg::Error(error) => {
                next.error = Some(error);
            },
        }
        Rc::new(next)
    }
}

pub enum Msg {
    Fetch,
    VotesReceived(Vec<Vote>),
    Error(String),
}

#[function_component]
pub fn Votes() -> Html {
    let state = use_reducer(VotesState::default);
    
    use_effect_with_deps({
        let state = state.clone();
        move |_| {
            let timer_state = state.clone();
            let interval = Interval::new(1_000, move || {
                timer_state.dispatch(Msg::Fetch);
            });
    
            wasm_bindgen_futures::spawn_local(async move {
                match Request::get(&format!("{}/votes", CONFIG.api_base_url)).send().await {
                    Ok(response) => match response.json::<Vec<Vote>>().await {
                        Ok(votes) => state.dispatch(Msg::VotesReceived(votes)),
                        Err(e) => state.dispatch(Msg::Error(e.to_string())),
                    },
                    Err(e) => state.dispatch(Msg::Error(e.to_string())),
                }
            });
    
            move || drop(interval)
        }
    }, ());

    let truncate = |text: &str, limit: usize| {
        if text.chars().count() > limit {
            format!("{}...", text.chars().take(limit).collect::<String>())
        } else {
            text.to_string()
        }
    };

    html! {
        <div class={CONTAINER}>
            <h1 class={combine_classes(HEADING_LG, "text-white")}>{"Votes"}</h1>
            
            {if let Some(error) = &state.error {
                html! { <div class={alert_style("error")}>{error}</div> }
            } else { html! {} }}

            <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {state.votes.iter().map(|vote| {
                    let now = OffsetDateTime::now_utc();
                    let is_ended = now > vote.voting_ends_at;
                    let time_remaining = if is_ended { Duration::ZERO } else { vote.voting_ends_at - now };
                    let route = if is_ended {
                        Route::Results { id: vote.id.to_string() }
                    } else {
                        Route::Vote { id: vote.id.to_string() }
                    };

                    html! {
                        <Link<Route> to={route} 
                            classes={classes!(
                                CARD_HOVER_SCALE,
                                "hover:shadow-lg",
                                "transition-shadow"
                            )}>
                            <div class="h-full flex flex-col">
                                <h2 class={HEADING_SM} title={vote.title.clone()}>
                                    {truncate(&vote.title, 15)}
                                </h2>
                                <p class={combine_classes(TEXT_MUTED, "mb-2")} title={vote.description.clone()}>
                                    {truncate(&vote.description, 20)}
                                </p>
                                <div class="mt-auto space-y-1">
                                    <p class={TEXT_MUTED}>{"Ballots Cast: "}{vote.ballots.len()}</p>
                                    <p class={TEXT_MUTED}>{"Options: "}{vote.options.len()}</p>
                                    <div class={FLEX_BETWEEN}>
                                        <div class={combine_classes("font-medium", 
                                            if is_ended { "text-orange-400" } else { MEGA_PULSE }
                                        )}>
                                            {if is_ended { "Vote Concluded" }
                                             else if time_remaining.whole_hours() > 0 { "Active Vote" }
                                             else { "Ending Soon" }}
                                        </div>
                                        {if !is_ended {
                                            html! { <div class={TEXT_MUTED}>{render_time(time_remaining)}</div> }
                                        } else { html! {} }}
                                    </div>
                                </div>
                            </div>
                        </Link<Route>>
                    }
                }).collect::<Html>()}
            </div>

            {if state.votes.is_empty() && state.error.is_none() {
                html! {
                    <div class="flex justify-center p-8">
                        <div class={combine_classes("animate-pulse", TEXT_MUTED)}>{"Loading votes..."}</div>
                    </div>
                }
            } else { html! {} }}
        </div>
    }
}

fn render_time(d: Duration) -> String {
    if d.whole_hours() > 0 {
        format!("{}h {}m", d.whole_hours(), d.whole_minutes() % 60)
    } else if d.whole_minutes() > 0 {
        format!("{}m {}s", d.whole_minutes(), d.whole_seconds() % 60)
    } else {
        format!("{}s", d.whole_seconds())
    }
}