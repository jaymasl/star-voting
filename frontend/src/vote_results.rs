use yew::prelude::*;
use gloo_net::http::Request;
use shared::models::*;
use yew_router::prelude::*;
use crate::Route;
use crate::styles::*;
use std::rc::Rc;
use time::OffsetDateTime;
use futures::try_join;
use crate::render_results::render_results_view;
use crate::config::CONFIG;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub id: String,
}

#[derive(Default)]
enum State {
    #[default]
    Loading,
    Ready { vote: Rc<Vote>, result: Rc<VoteResult> },
    Error(String),
}

pub struct VoteResults {
    state: State,
}

impl Component for VoteResults {
    type Message = Result<(Vote, VoteResult), String>;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let id = ctx.props().id.clone();
        let navigator = ctx.link().navigator().unwrap();

        ctx.link().send_future(async move {
            match fetch_data(&id).await {
                Ok((vote, _)) if OffsetDateTime::now_utc() <= vote.voting_ends_at => {
                    navigator.push(&Route::Vote { id });
                    Err("Vote still active".into())
                }
                other => other
            }
        });

        Self { 
            state: State::default()
        }
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        self.state = match msg {
            Ok((vote, result)) => State::Ready { 
                vote: Rc::new(vote), 
                result: Rc::new(result) 
            },
            Err(err) => State::Error(err),
        };
        true
    }

    fn view(&self, _: &Context<Self>) -> Html {
        match &self.state {
            State::Loading => html! {
                <div class={CONTAINER}>
                    <div class="flex items-center justify-center p-8">
                        <div class="animate-spin rounded-full h-12 w-12 border-4 border-blue-500 border-t-transparent"/>
                    </div>
                </div>
            },
            State::Ready { vote, result } => render_results_view(vote, result),
            State::Error(err) => html! {
                <div class={CONTAINER}>
                    <div class={alert_style("error")}>
                        <p>{err}</p>
                        <Link<Route> to={Route::Home} classes={classes!(button_primary(false), "mt-4")}>
                            {"Return Home"}
                        </Link<Route>>
                    </div>
                </div>
            },
        }
    }
}

async fn fetch_data(id: &str) -> Result<(Vote, VoteResult), String> {
    let (vote_resp, result_resp) = try_join!(
        Request::get(&format!("{}/vote/{}", CONFIG.api_base_url, id)).send(),
        Request::get(&format!("{}/vote/{}/result", CONFIG.api_base_url, id)).send()
    ).map_err(|e| e.to_string())?;

    let vote = vote_resp.json::<Vote>().await
        .map_err(|_| "Failed to parse vote data".to_string())?;
    let result = result_resp.json::<VoteResult>().await
        .map_err(|_| "Failed to parse result data".to_string())?;

    Ok((vote, result))
}