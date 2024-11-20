use yew::prelude::*;
use yew_router::prelude::*;

mod styles;
mod vote_display;
mod vote_ballot;
mod vote_results;
mod votes;
mod home;
mod vote_status;
mod vote_option_manager;
mod vote_create;
mod config;
pub mod hcaptcha;
pub mod render_results;

use crate::{
    vote_display::VoteDisplay,
    vote_results::VoteResults,
    votes::Votes,
    home::Home,
    vote_create::VoteCreate,
};

#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/")] Home,
    #[at("/votes")] Votes,
    #[at("/create")] CreateVote,
    #[at("/vote/:id")] Vote { id: String },
    #[at("/results/:id")] Results { id: String },
}

#[function_component(Navigation)]
fn navigation() -> Html {
    let current_route = use_route::<Route>();

    html! {
        <nav class="bg-gray-900 shadow-lg fixed top-0 w-full z-50">
            <div class="container mx-auto px-6 py-4 flex justify-center space-x-8">
                <Link<Route> to={Route::Home} classes={classes!(
                    "text-base", "md:text-lg", "font-medium", "px-4", "py-2", "rounded-md",
                    "transition-colors", "duration-200", "ease-in-out",
                    "text-gray-200", "border", "border-transparent", "hover:border-blue-400", "hover:text-blue-400",
                    if current_route == Some(Route::Home) {
                        "text-blue-400 border-blue-400 ring-2 ring-blue-500 ring-offset-1 ring-offset-gray-900"
                    } else {
                        ""
                    }
                )}>
                    {"Home"}
                </Link<Route>>
                <Link<Route> to={Route::Votes} classes={classes!(
                    "text-base", "md:text-lg", "font-medium", "px-4", "py-2", "rounded-md",
                    "transition-colors", "duration-200", "ease-in-out",
                    "text-gray-200", "border", "border-transparent", "hover:border-blue-400", "hover:text-blue-400",
                    if current_route == Some(Route::Votes) {
                        "text-blue-400 border-blue-400 ring-2 ring-blue-500 ring-offset-1 ring-offset-gray-900"
                    } else {
                        ""
                    }
                )}>
                    {"Votes"}
                </Link<Route>>
                <Link<Route> to={Route::CreateVote} classes={classes!(
                    "text-base", "md:text-lg", "font-medium", "px-4", "py-2", "rounded-md",
                    "transition-colors", "duration-200", "ease-in-out",
                    "text-gray-200", "border", "border-transparent", "hover:border-blue-400", "hover:text-blue-400",
                    if current_route == Some(Route::CreateVote) {
                        "text-blue-400 border-blue-400 ring-2 ring-blue-500 ring-offset-1 ring-offset-gray-900"
                    } else {
                        ""
                    }
                )}>
                    {"Create Vote"}
                </Link<Route>>
            </div>
        </nav>
    }
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <BrowserRouter>
            <div class="min-h-screen bg-gray-900">
                <Navigation />
                <div class="pt-16">
                    <Switch<Route> render={switch} />
                </div>
            </div>
        </BrowserRouter>
    }
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Home => html! { <Home /> },
        Route::Votes => html! { <Votes /> },
        Route::CreateVote => html! { <VoteCreate /> },
        Route::Vote { id } => html! { <VoteDisplay {id} /> },
        Route::Results { id } => html! { <VoteResults {id} /> },
    }
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    yew::Renderer::<App>::new().render();
}