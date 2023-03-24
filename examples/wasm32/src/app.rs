use crate::actor;
use yew::prelude::*;

#[function_component(App)]
pub fn app() -> Html {
    let ping_node_ref = use_node_ref();
    let fibo_node_ref = use_node_ref();

    // TODO
    // link add and crate description brief
    //
    html! {
    <div class="w-full h-full">
      <div class="flex h-full flex-col" style="width: 15%; float: left">
        <img class="logo"
          style="margin: 1em; margin-top: 2.5em"
          src="assets/rust-logo.svg"
          alt="Yew logo" />
        <p class="text-center"
          style="margin: 9px; justify-content: center; place-content: center" >
          {{ "CrossBus Demo for Wasm" }}
        </p>
        <p class="text-center" style="font-size: 15px">
          <a href="https://github.com/hominee/crossbus"> {{ "CrossBus" }} </a>
          {{ " is a Platform-less Runtime-less Actor Computing Model" }}
        </p>
        <a class="normal-text text-center flex justify-center" href="https://github.com/hominee/crossbus" >
          <img class="mini-logo" src="assets/github-logo.jpeg" style="margin-right: 7px;"/>
          {{ " github" }}
        </a>
        <hr />
        <a class="normal-text text-center flex justify-center" href="https://crates.io/crates/crossbus" >
          <img class="mini-logo" src="assets/crates-logo.png" style="margin-right: 7px;"/>
          {{ " Crates" }}
        </a>
        <hr />
        <a class="normal-text text-center flex justify-center" href="https://docs.rs/crossbus" >
          <img class="mini-logo" src="assets/docs-logo.ico" style="margin-right: 7px;"/>
          {{ " Docs" }}
        </a>
        <hr />
        <a class="normal-text text-center flex justify-center" href="https://github.com/hominee/" >
          <img class="mini-logo" src="assets/author-logo.png" style="margin-right: 7px;"/>
          {{ " Creator" }}
        </a>
      </div>
      <div class="dark h-full" style="width: 85%; float: right; justify-items: center;">

        <div class="section text-center" style="justify-content: center; place-content: center;">
          <h3> {{ "Ping Demo" }} </h3>
          <div class="section-items flex flex-row w-full justify-center" >
            <div class="flex flex-col" style="margin-right: 40px;">
              <p class="normal-text"> {{ "the sum of " }}
                <span>{{ "1" }}<sup> {{ "7" }} </sup> </span> {{ "+" }}
                <span>{{ "2" }}<sup> {{ "7" }} </sup> </span> {{ "+" }}
                {{ " ... + " }}
                <span>{{ "n" }}<sup> {{ "7" }} </sup> </span>
              </p>
              <div class="flex flex-row" style="height: 30px;" >
                <input class="" ref={ping_node_ref.clone()} style="margin-right: 7px;" type="number" placeholder="the number add up to:" />
                <button class="" onclick={
                    Callback::from(move |_| {
                        if let Some(input) = ping_node_ref.cast::<web_sys::HtmlInputElement>() {
                            let num_str = input.value();
                            let num: u32 = num_str.parse().expect("invalid input");
                            log::info!("got target number: {}", num);
                            wasm_bindgen_futures::spawn_local(actor::run(num, "ping"));
                        }


                })} > {{ "Send" }} </button>
              </div>
            </div>
            <div class="flex flex-col" style="margin-right: 40px;">
              <p class="normal-text"> {{ "The Result: " }} </p>
              <textarea type="text" id="ping-result" rows="1" cols="20" style="height: 30%; 	align-self: center;"> </textarea>
            </div>
          </div>
          <hr />
        </div>

        <div class="section text-center" style="justify-content: center; place-content: center;">
          <h3> {{ "Fibonacci Sequence Demo" }} </h3>
          <div class="section-items flex flex-row w-full justify-center" >
            <div class="flex flex-col" style="margin-right: 40px;">
              <p class="normal-text"> {{ "the n-th term of Fibonacci Sequence" }} </p>
              <div class="flex flex-row" style="height: 30px;" >
                <input class="" ref={fibo_node_ref.clone()} style="margin-right: 7px;" type="number" placeholder="n-th term" />
                <button class="" onclick={
                    Callback::from(move |_| {
                        if let Some(input) = fibo_node_ref.cast::<web_sys::HtmlInputElement>() {
                            let num_str = input.value();
                            let num: u32 = num_str.parse().expect("invalid input");
                            log::info!("got target fibonacci index: {}", num);
                            wasm_bindgen_futures::spawn_local(actor::run(num, "fibonacci"));
                        }

                })
                }> {{ "Send" }} </button>
              </div>
            </div>
            <div class="flex flex-col" style="margin-right: 40px;">
              <p class="normal-text"> {{ "The Result: " }} </p>
              <textarea value="" id="fibonacci-result" rows="1" cols="20" style="height: 30%; 	align-self: center;"> </textarea>
            </div>
          </div>
          <hr />
        </div>

      </div>
    </div>
    }
}
