use yew::prelude::*;
use yew_router::prelude::*;
use shared::models::{Vote, VoteResult, VoteOptionStats, HeadToHeadResult};
use crate::{styles::*, Route};
use std::cmp::Ordering;

fn render_head_to_head_results(head_to_head: &HeadToHeadResult) -> Html {
    html! {
        <div class="space-y-4">
            <div class="bg-gray-800/50 rounded-lg p-4">
                <div class="relative mb-1">
                    <div class="max-w-full break-words pr-12 font-medium">{&head_to_head.finalist1}</div>
                    <div class="absolute right-0 top-0 font-bold text-xl">{head_to_head.finalist1_votes}</div>
                </div>
                <div class="my-2 border-t border-gray-400"/>
                <div class="relative mt-1">
                    <div class="max-w-full break-words pr-12 font-medium">{&head_to_head.finalist2}</div>
                    <div class="absolute right-0 top-0 font-bold text-xl">{head_to_head.finalist2_votes}</div>
                </div>
            </div>
        </div>
    }
}

fn render_winner_details(
    _winner: &str,
    _head_to_head: &HeadToHeadResult,
    is_tie: bool,
    (finalist1, f1_nonzero, f1_stats): (&str, usize, &VoteOptionStats),
    (finalist2, f2_nonzero, f2_stats): (&str, usize, &VoteOptionStats)
) -> Html {
    html! {
        <div class="bg-gray-800/30 rounded-lg p-4">
            <div class="font-medium mb-3 text-sm text-gray-300">{"Final Round Details"}</div>
            <div class="space-y-4">
                <div>
                    <div class="max-w-full break-words mb-1">{finalist1}</div>
                    <div class="text-sm text-gray-400 ml-3 space-y-0.5">
                        {format!("{} non-zero votes", f1_nonzero)}
                        <div>{format!("{} five-star ratings", f1_stats.frequency.get(&5).unwrap_or(&0))}</div>
                    </div>
                </div>
                <div>
                    <div class="max-w-full break-words mb-1">{finalist2}</div>
                    <div class="text-sm text-gray-400 ml-3 space-y-0.5">
                        {format!("{} non-zero votes", f2_nonzero)}
                        <div>{format!("{} five-star ratings", f2_stats.frequency.get(&5).unwrap_or(&0))}</div>
                    </div>
                </div>
                {if is_tie {
                    html! {
                        <div class="mt-3 text-sm text-yellow-300/90">
                            {"Tie resolved by tiebreaker rules"}
                        </div>
                    }
                } else {
                    html! {}
                }}
            </div>
        </div>
    }
}

pub fn render_results_view(vote: &Vote, result: &VoteResult) -> Html {
    html! {
        <div class={CONTAINER_SM}>
            <div class={CARD}>
                <h1 class={classes!(HEADING_MD, "break-words")}>{&vote.title}</h1>
                <p class={classes!("mb-2", "text-white", "break-words")}>{&vote.description}</p>
                {render_vote_duration(result)}
                {render_runoff_round(result.winner.as_deref(), result.error.as_deref(), result)}
                {render_score_distributions(result, vote)}
                {render_ballots(vote, result)}
                <div class="mt-6 flex justify-center">
                    <Link<Route> to={Route::Home} 
                        classes={classes!(button_primary(false))}>
                        {"Back to Home"}
                    </Link<Route>>
                </div>
            </div>
        </div>
    }
}

fn render_tie_resolution(tied_options: &[(&str, &VoteOptionStats)]) -> Html {
    let first_stats = &tied_options[0].1;
    let all_identical = tied_options.iter().all(|(_, stats)| {
        stats.total_score == first_stats.total_score && 
        stats.frequency == first_stats.frequency
    });
 
    if all_identical {
        return html! {
            <div class="mt-4 pt-4 border-t border-blue-700">
                <p class="font-medium mb-2">{"True Tie - Identical Statistics:"}</p>
                <div class="ml-4 space-y-3">
                    <ol class="list-decimal list-inside space-y-1">
                        <li>{format_scores(tied_options)}</li>
                        <li>{"All options have identical vote distributions"}</li>
                    </ol>
                    <p class="font-medium">{"Unable to determine advancement - true tie"}</p>
                </div>
            </div>
        };
    }
 
    let nonzero_counts: Vec<_> = tied_options.iter()
        .map(|(name, stats)| (*name, (1..=5).map(|i| stats.frequency.get(&i).unwrap_or(&0)).sum()))
        .collect();
    let max_nonzero = nonzero_counts.iter().map(|(_, count)| count).max().unwrap_or(&0);
    let nonzero_winners: Vec<_> = nonzero_counts.iter()
        .filter(|(_, count)| count == max_nonzero)
        .collect();
    let five_star_counts: Vec<_> = tied_options.iter()
        .map(|(name, stats)| (*name, *stats.frequency.get(&5).unwrap_or(&0)))
        .collect();
    let max_fives = five_star_counts.iter().map(|(_, count)| count).max().unwrap_or(&0);
    let five_winners: Vec<_> = five_star_counts.iter()
        .filter(|(_, count)| count == max_fives)
        .collect();
 
    html! {
        <div class="mt-4 pt-4 border-t border-blue-700">
            <p class="font-medium mb-2">{
                if tied_options.len() > 2 {
                    format!("{}-Way Second Place Tie Resolution:", tied_options.len())
                } else {
                    "Second Place Resolution:".to_string()
                }
            }</p>
            <div class="ml-4 space-y-3">
                <ol class="list-decimal list-inside space-y-1">
                    <li>{format_scores(tied_options)}</li>
                    <li>{format_nonzero_votes(&nonzero_counts)}</li>
                    <li>{format_five_star_ratings(&five_star_counts)}</li>
                </ol>
                <p class="font-medium">{determine_winner_text(&nonzero_winners, &five_winners)}</p>
            </div>
        </div>
    }
}

fn format_scores(options: &[(&str, &VoteOptionStats)]) -> String {
    format!("Total scores: {}", options.iter().enumerate()
        .map(|(i, (name, stats))| format!("{}: {} points{}", 
            name, stats.total_score,
            if i < options.len() - 1 { ", " } else { "" }))
        .collect::<String>())
}

fn format_nonzero_votes(counts: &[(&str, usize)]) -> String {
    format!("Non-zero votes: {}", counts.iter().enumerate()
        .map(|(i, (name, count))| format!("{}: {}{}", 
            name, count,
            if i < counts.len() - 1 { ", " } else { "" }))
        .collect::<String>())
}

fn format_five_star_ratings(counts: &[(&str, usize)]) -> String {
    format!("Five-star ratings: {}", counts.iter().enumerate()
        .map(|(i, (name, count))| format!("{}: {}{}", 
            name, count,
            if i < counts.len() - 1 { ", " } else { "" }))
        .collect::<String>())
}

fn determine_winner_text(nonzero_winners: &[&(&str, usize)], five_winners: &[&(&str, usize)]) -> String {
    if nonzero_winners.len() == 1 && nonzero_winners[0].1 > 0 {
        format!("{} advances (by non-zero votes)", nonzero_winners[0].0)
    } else if five_winners.len() == 1 && five_winners[0].1 > 0 {
        format!("{} advances (by five-star ratings)", five_winners[0].0)
    } else if !five_winners.is_empty() && five_winners[0].1 > 0 {
        format!("{} advances (by tie-breaking rules)", five_winners[0].0)
    } else {
        "Unable to determine perfect tie".to_string()
    }
}

fn render_score_distributions(result: &VoteResult, vote: &Vote) -> Html {
    let mut options: Vec<_> = vote.options.iter()
        .filter_map(|opt| result.stats.option_scores.get(opt)
            .map(|stats| (opt.to_string(), stats)))
        .collect();
    options.sort_by(|(_, a), (_, b)|
        b.average_score.partial_cmp(&a.average_score).unwrap_or(Ordering::Equal));

    let second_place_ties = if options.len() >= 2 {
        options.iter()
            .skip(1)
            .take_while(|(_, stats)| 
                (stats.average_score - options[1].1.average_score).abs() < f64::EPSILON)
            .map(|(opt, stats)| (opt.as_str(), *stats))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    html! {
        <div class={combine_classes(STATS_CARD, STATS_CARD_INFO)}>
            <h3 class={HEADING_SM}>{"Complete Score Distribution"}</h3>
            <div class="space-y-6">
                {for options.iter().map(|(opt, stats)| html! {
                    <div class="pb-4 border-b border-blue-700/30 last:border-0">
                        <div class="font-medium mb-1 break-words">
                            {&*opt}
                        </div>
                        <div class="text-sm text-gray-300 mb-1">
                            {format!("Average: {:.2}", stats.average_score)}
                        </div>
                        <div class="text-sm text-gray-400 mb-3">
                            {format!("Total Score: {}", stats.total_score)}
                        </div>
                        <div class="grid grid-cols-6 text-center">
                            {for (0..=5).map(|score| {
                                let count = stats.frequency.get(&score).copied().unwrap_or(0);
                                html! {
                                    <div>
                                        <div class="text-l text-gray-400">
                                            {if count > 0 { format!("{}×", count) } else { "-".to_string() }}
                                        </div>
                                        <div class="text-s font-medium mt-1">
                                            {score}
                                        </div>
                                    </div>
                                }
                            })}
                        </div>
                    </div>
                })}
            </div>
            {if second_place_ties.len() > 1 {
                render_tie_resolution(&second_place_ties)
            } else {
                html! {}
            }}
            {render_tiebreak_rules()}
        </div>
    }
}

fn render_tiebreak_rules() -> Html {
    html! {
        <div class="mt-4 pt-4 border-t border-blue-700">
            <h4 class={combine_classes(HEADING_SM, "mb-2")}>{"Tie-Breaking Rules"}</h4>
            <ol class="list-decimal list-inside space-y-1 text-sm">
                <li>{"Most unique voters (non-zero)"}</li>
                <li>{"Most high scores (5s, then 4s)"}</li>
                <li>{"Fewest low scores (0s, then 1s)"}</li>
            </ol>
        </div>
    }
}

fn render_ballots(vote: &Vote, result: &VoteResult) -> Html {
    let mut options: Vec<_> = vote.options.iter()
        .filter_map(|opt| result.stats.option_scores.get(opt)
            .map(|stats| (opt.as_str(), stats)))
        .collect();

    options.sort_by(|(_, a), (_, b)| 
        b.average_score.partial_cmp(&a.average_score).unwrap_or(Ordering::Equal));

    let ordered_options: Vec<&str> = options.iter().map(|(opt, _)| *opt).collect();

    html! {
        <div class="mt-4">
            {render_ballot_header(&ordered_options)}
            {render_ballot_table(&ordered_options, vote)}
        </div>
    }
}

fn render_ballot_header(ordered_options: &[&str]) -> Html {
    html! {
        <>
            <h2 class={combine_classes(HEADING_MD, "mb-1")}>{"Individual Ballot Results"}</h2>
            <div class="mb-4 bg-gray-800 rounded-lg p-3">
                <div class="font-medium mb-2 text-gray-300">{"Options Legend:"}</div>
                <div class="border border-gray-700 rounded-lg p-2 bg-gray-800/50">
                    <div class="space-y-1">
                        {for ordered_options.iter().enumerate().map(|(i, opt)| html! {
                            <div class="flex items-baseline">
                                <span class="font-mono text-gray-400 w-8 shrink-0 text-right pr-1">{format!("{}.", i + 1)}</span>
                                <span class="text-gray-300 min-w-0 break-words">{opt}</span>
                            </div>
                        })}
                    </div>
                </div>
            </div>
        </>
    }
}

fn render_ballot_table(ordered_options: &[&str], vote: &Vote) -> Html {
    html! {
        <div class="rounded-lg border border-gray-300 overflow-x-auto">
            <table class="w-full text-sm">
                <thead class="bg-gray-700/50">
                    <tr>
                        <th class="px-2 py-1 border-b border-r border-gray-300 text-center w-16 text-white">
                            {"Ballot"}
                        </th>
                        {for ordered_options.iter().enumerate().map(|(i, _)| html! {
                            <th class="px-2 py-1 border-b border-r last:border-r-0 border-gray-300 text-center w-12 text-white">
                                {i + 1}
                            </th>
                        })}
                    </tr>
                </thead>
                <tbody>
                    {for vote.ballots.iter().enumerate().map(|(i, ballot)| html! {
                        <tr class="hover:bg-gray-700/30">
                            <td class="px-2 py-1 border-r border-gray-300 text-center text-white font-mono text-xs">
                                {format!("#{}", i + 1)}
                            </td>
                            {for ordered_options.iter().map(|opt| html! {
                                <td class="px-2 py-1 border-r last:border-r-0 border-gray-300 text-center text-white">
                                    {ballot.scores.get(*opt).map_or("-".to_string(), |score| score.to_string())}
                                </td>
                            })}
                        </tr>
                    })}
                </tbody>
            </table>
        </div>
    }
}

fn render_runoff_round(winner: Option<&str>, error: Option<&str>, result: &VoteResult) -> Html {
    match (&result.head_to_head, winner, error) {
        (Some(head_to_head), Some(_), None) => {
            render_winner_section(result.winner.as_deref().unwrap(), head_to_head, result)
        }
        (_, _, Some(error_msg)) => render_error_section(error_msg),
        _ => html! {}
    }
}

fn render_winner_section(winner: &str, head_to_head: &HeadToHeadResult, result: &VoteResult) -> Html {
    let is_tie = head_to_head.finalist1_votes == head_to_head.finalist2_votes;
    let (f1_stats, f2_stats) = (
        result.stats.option_scores.get(&head_to_head.finalist1).unwrap(),
        result.stats.option_scores.get(&head_to_head.finalist2).unwrap()
    );
    
    let (f1_nonzero, f2_nonzero) = (
        (1..=5).map(|i| f1_stats.frequency.get(&i).unwrap_or(&0)).sum::<usize>(),
        (1..=5).map(|i| f2_stats.frequency.get(&i).unwrap_or(&0)).sum::<usize>()
    );
 
    html! {
        <div class={combine_classes(STATS_CARD, STATS_CARD_SUCCESS)}>
            <h3 class={HEADING_SM}>{"STAR Voting Runoff Round"}</h3>
            <div class="mb-4">
                <div class="text-xl font-bold">{"🏆 Winner:"}</div>
                <div class="text-xl font-bold overflow-hidden truncate" title={winner.to_string()}>
                    {winner}
                </div>
            </div>
            <div class={SPACE_Y_BASE}>
                {render_head_to_head_results(head_to_head)}
                {render_winner_details(winner, head_to_head, is_tie, (&head_to_head.finalist1, f1_nonzero, f1_stats),
                                     (&head_to_head.finalist2, f2_nonzero, f2_stats))}
            </div>
        </div>
    }
}

fn render_error_section(error_msg: &str) -> Html {
    html! {
        <div class={combine_classes(STATS_CARD, STATS_CARD_WARNING)}>
            <h3 class={HEADING_SM}>{"Unable to Determine Winner"}</h3>
            <p>{error_msg}</p>
        </div>
    }
}

fn render_vote_duration(result: &VoteResult) -> Html {
    if let Some(duration_hours) = result.duration_hours {
        let days = duration_hours / 24;
        let hours = duration_hours % 24;
        let minutes = result.duration_minutes.unwrap_or(0);
        
        let duration_text = format_duration(days, hours, minutes);
        html! { <p class="mb-4 text-gray-400 text-sm italic">{duration_text}</p> }
    } else {
        html! {}
    }
}

fn format_duration(days: i64, hours: i64, minutes: i64) -> String {
    fn plural(n: i64, word: &str) -> String {
        format!("{} {}{}", n, word, if n == 1 { "" } else { "s" })
    }

    let parts: Vec<_> = [
        (days, "day"),
        (hours, "hour"),
        (minutes, "minute")
    ].iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, word)| plural(*n, word))
        .collect();

    format!("Duration: {}", match parts.len() {
        0 => "0 minutes".to_string(),
        1 => parts[0].clone(),
        n => format!("{} and {}", 
            parts[..n-1].join(", "), 
            parts[n-1])
    })
}