use yew::prelude::*;
use yew_router::prelude::*;
use crate::{Route, styles::*};

#[function_component]
pub fn Home() -> Html {
    html! {
        <div class={CONTAINER}>
            <h1 class={combine_classes(HEADING_LG, "text-white")}>{"STAR Voting"}</h1>
            <div class="text-center mb-6">
                <p class="text-gray-300 mb-4">
                    {"Alpha v0.1"}
                </p>
                <a href="https://jaykrown.com">
                    <span class="text-blue-400 hover:underline">{"jaykrown.com"}</span>
                </a>
            </div>
            
            <div class="space-y-8 max-w-3xl mx-auto">
                <div class="bg-gray-800 p-6 rounded-lg shadow-lg">
                    <p class="text-gray-300 mb-4">
                        {"This application lets you create votes and cast ballots using STAR voting (Score Then Automatic Runoff), 
                        a modern system that combines the flexibility of score voting with the majority consensus of an automatic runoff. 
                        It works for everything from simple Yes or No questions to preference votes with up to 20 options, 
                        always determining a clear winner unless there's a rare unbreakable tie."}
                    </p>
                    <p class="text-gray-300">
                        {"Built with Rust for speed, security, and reliability. Features include 
                        hCaptcha verification, browser fingerprinting, rate limiting, CSRF protection, 
                        and profanity filtering. Votes can run for up to 7 days, complete results 
                        archived for a month."}
                    </p>
                </div>

                <div class="bg-gray-800 p-6 rounded-lg shadow-lg">
                    <h2 class="text-xl font-semibold mb-4 text-white">{"How to Vote"}</h2>
                    <ul class="list-disc pl-6 space-y-3 text-gray-300">
                        <li>{"Rate each option from 0 (worst) to 5 (best)"}</li>
                        <li>{"Use different scores to show your preference order"}</li>
                        <li>{"Equal scores mean equal preference"}</li>
                        <li>{"Skipping an option is the same as giving it zero stars"}</li>
                    </ul>
                </div>

                <div class="bg-gray-800 p-6 rounded-lg shadow-lg">
                    <h2 class="text-xl font-semibold mb-4 text-white">{"Get Started"}</h2>
                    <div class="flex flex-col sm:flex-row gap-4 justify-center">
                        <Link<Route> to={Route::Votes}
                            classes="bg-blue-600 hover:bg-blue-700 text-white px-8 py-3 rounded-lg text-lg font-semibold text-center transition-colors">
                            {"View Votes"}
                        </Link<Route>>
                        <Link<Route> to={Route::CreateVote}
                            classes="bg-green-600 hover:bg-green-700 text-white px-8 py-3 rounded-lg text-lg font-semibold text-center transition-colors">
                            {"Create New Vote"}
                        </Link<Route>>
                    </div>
                </div>
            </div>
        </div>
    }
}