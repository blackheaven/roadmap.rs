use std::collections::HashSet;
use web_sys::HtmlInputElement;
use yew::prelude::*;

const secret: &str = "haskell";

#[derive(Clone)]
struct State {
    // remaining
    count: u8,
    guesses: HashSet<char>,
}

fn mk_display(state: State) -> String {
    let guessed = String::from_iter(String::from(secret).chars().map(|c| {
        if state.guesses.contains(&c) {
            '_'
        } else {
            c
        }
    }));

    let score = if state.count == 0 {
        String::from("lost!")
    } else {
        format!("{} guess remaining", state.count)
    };

    return format!("{} {}", guessed, score);
}

#[function_component]
fn App() -> Html {
    let state = use_state(|| State {
        count: 4,
        guesses: HashSet::from_iter(String::from(secret).chars()),
    });
    let display_handle = use_state(String::default);
    let display = (*display_handle).clone();
    let disabled_handle = use_state(|| false);
    let disabled = (*disabled_handle).clone();

    let input_node_ref = use_node_ref();
    let onchange = {
        let input_node_ref = input_node_ref.clone();
        let state = state.clone();

        Callback::from(move |_| {
            let input = input_node_ref.cast::<HtmlInputElement>();
            if let Some(guess) = input.and_then(|x| x.value().chars().next()) {
                let mut new_state = (*state).clone();
                if new_state.count > 0 && !new_state.guesses.remove(&guess) {
                    new_state.count -= 1;
                }

                state.set(new_state.clone());
                display_handle.set(mk_display(new_state.clone()));
                disabled_handle.set(new_state.clone().count == 0);
            }
        })
    };

    html! {
        <>
            <p>
                <label for="guess">
                    { "My guess:" }
                    <input ref={input_node_ref}
                        {onchange}
                        id="guess"
                        type="text"
                        disabled={disabled}
                    />
                </label>
            </p>
            <p>
                <p>{ display }</p>
            </p>
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
