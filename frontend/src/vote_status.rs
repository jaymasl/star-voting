use yew::prelude::*;
use gloo_net::http::Request;
use shared::models::{Vote, VoteResult};
use yew_router::prelude::*;
use crate::Route;
use crate::config::CONFIG;
use time::OffsetDateTime;
use gloo_timers::callback::Interval;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub id: String,
}

#[function_component(VoteStatus)]
pub fn vote_status(props: &Props) -> Html {
    let vote_state = use_state(|| None::<Vote>);
    let result_state = use_state(|| None::<VoteResult>);
    let error_state = use_state(|| None::<String>);
    let time_remaining = use_state(String::new);
    let navigator = use_navigator().unwrap();

    {
        let vote_state = vote_state.clone();
        let time_remaining = time_remaining.clone();
        use_effect_with_deps(move |vote| {
            if let Some(vote) = &**vote {
                let update_time = {
                    let time_remaining = time_remaining.clone();
                    let voting_ends_at = vote.voting_ends_at;
                    move || {
                        let remaining = voting_ends_at - OffsetDateTime::now_utc();
                        time_remaining.set(format!("{} hours {} minutes", 
                            remaining.whole_hours(), 
                            remaining.whole_minutes() % 60));
                    }
                };
                let interval = Interval::new(1000, update_time.clone());
                update_time();
                interval.forget();
            }
            || ()
        }, vote_state.clone());
    }

    {
        let vote_state = vote_state.clone();
        let result_state = result_state.clone();
        let error_state = error_state.clone();
        let id = props.id.clone();
        use_effect_with_deps(move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                match Request::get(&format!("{}/vote/{}", CONFIG.api_base_url, id)).send().await {
                    Ok(response) => match response.json::<Option<Vote>>().await {
                        Ok(Some(vote)) => vote_state.set(Some(vote)),
                        Ok(None) => error_state.set(Some("Vote not found".into())),
                        Err(e) => error_state.set(Some(e.to_string())),
                    },
                    Err(e) => error_state.set(Some(e.to_string())),
                }

                if let Ok(response) = Request::get(&format!("{}/vote/{}/result", CONFIG.api_base_url, id)).send().await {
                    if let Ok(result) = response.json::<VoteResult>().await {
                        result_state.set(Some(result));
                    }
                }
            });
            || ()
        }, ());
    }

    let end_vote = {
        let id = props.id.clone();
        let navigator = navigator.clone();

        Callback::from(move |_| {
            let id = id.clone();
            let navigator = navigator.clone();
            
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(response) = Request::post(&format!("{}/vote/{}/end", CONFIG.api_base_url, id))
                    .send()
                    .await
                {
                    if response.status() == 200 {
                        navigator.push(&Route::Results { id });
                    }
                }
            });
        })
    };

    if let Some(error) = (*error_state).clone() {
        return html! {
            <div class="container mx-auto px-4 py-8">
                <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded">{error}</div>
            </div>
        };
    }

    match (&*vote_state, &*result_state) {
        (Some(vote), Some(result)) => html! {
            <div class="container mx-auto px-4 py-8 max-w-2xl">
                <div class="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-6">
                    <h1 class="text-2xl font-bold mb-4">{&vote.title}</h1>
                    <p class="mb-6">{&vote.description}</p>
                    <div class="bg-blue-50 dark:bg-blue-900 p-4 rounded-lg mb-6">
                        <h2 class="font-semibold mb-2">{"Time Remaining"}</h2>
                        <p>{&*time_remaining}</p>
                    </div>
                    <div class="mb-6">
                        <h2 class="font-semibold mb-2">{"Current Participation"}</h2>
                        <p>{format!("Total Ballots Cast: {}", result.stats.total_ballots)}</p>
                    </div>
                    <div class="mb-6">
                        <h2 class="font-semibold mb-2">{"Options"}</h2>
                        <div class="space-y-2">
                            {for vote.options.iter().map(|option| html! {
                                <div class="bg-gray-50 dark:bg-gray-700 p-3 rounded">{option}</div>
                            })}
                        </div>
                    </div>
                    <div class="flex justify-center space-x-4">
                        <Link<Route> to={Route::Vote { id: props.id.clone() }}
                            classes="bg-blue-500 hover:bg-blue-600 text-white px-6 py-2 rounded-lg transition-colors">
                            {"Cast Your Ballot"}
                        </Link<Route>>
                        <button
                            onclick={end_vote}
                            class="bg-red-500 hover:bg-red-600 text-white px-6 py-2 rounded-lg transition-colors"
                        >
                            {"End Vote Early"}
                        </button>
                        <Link<Route> to={Route::Home}
                            classes="bg-gray-500 hover:bg-gray-600 text-white px-6 py-2 rounded-lg transition-colors">
                            {"Back to Home"}
                        </Link<Route>>
                    </div>
                </div>
            </div>
        },
        _ => html! {
            <div class="flex justify-center p-8">
                <div class="animate-pulse text-lg text-gray-600 dark:text-gray-300">{"Loading..."}</div>
            </div>
        }
    }
}