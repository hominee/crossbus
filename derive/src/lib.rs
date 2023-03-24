extern crate proc_macro;

use core::{ops::ControlFlow, str::FromStr};
use proc_macro::{TokenStream, TokenTree};

#[proc_macro_derive(Message)]
pub fn message(stream: TokenStream) -> TokenStream {
    let mut name = "".to_string();
    let nonident = ["enum".to_string(), "struct".to_string()];
    stream
        .into_iter()
        // take the first ident as the
        // name of the struct
        // then stop iterating
        .try_for_each(|tree| {
            if let TokenTree::Ident(ident) = tree {
                name = ident.to_string();
                if !nonident.contains(&&name) {
                    return ControlFlow::Break(());
                }
            }
            ControlFlow::Continue(())
        });
    assert_ne!(name, "", "struct/enum name NOT FOUND");
    let raw_token =
        r#"impl crossbus::message::Message for <+ident+> { }"#.replace("<+ident+>", &name);
    TokenStream::from_str(&raw_token).unwrap()
}

#[proc_macro_attribute]
pub fn main(attr: TokenStream, stream: TokenStream) -> TokenStream {
    let mut name = "".to_string();
    let raw_stream = stream.to_string();
    if !raw_stream.contains("async fn ") {
        panic!("attribute for async function ONLY");
    }
    let raw_attr = attr.to_string();
    let rt_vec: Vec<&str> = raw_attr.split(",").filter(|en| *en != "").collect();
    if rt_vec.len() != 1 {
        panic!("ONLY one type runtime is allowed");
    }
    let rt_str_ = rt_vec[0].replace(" ", "");
    let rt_str = rt_str_.strip_prefix("runtime=");
    if rt_str.is_none() {
        panic!("Runtime not found, HELP: #[main(runtime = \"xx::xx\")]");
    }
    let rt_str = rt_str.unwrap();
    let nonident = [
        "fn".to_owned(),
        "async".to_owned(),
        "pub".to_owned(),
        "mod".to_owned(),
        "in".to_owned(),
        "super".to_owned(),
        "crate".to_owned(),
    ];
    stream
        .into_iter()
        // take the first ident as the
        // name of the struct
        // then stop iterating
        .try_for_each(|tree| {
            if let TokenTree::Ident(ident) = tree {
                name = ident.to_string();
                if !nonident.contains(&&name) {
                    return ControlFlow::Break(());
                }
            }
            ControlFlow::Continue(())
        });
    assert_eq!(name, "main", "attribute for main function ONLY");
    let runtime;
    match rt_str {
        "async-std" => {
            runtime = "crossbus::rt::runtime_async_std::Runtime::block_on";
        }
        "tokio" => {
            runtime = "crossbus::rt::runtime_tokio::Runtime::block_on";
        }
        "wasm32" | "wasm64" => {
            runtime = "crossbus::rt::runtime_wasm32::Runtime::spawn_local";
        }
        _ => {
            runtime = rt_str;
        }
    }
    let raw_token = r#"fn main() {
<+fn+>

let _ = <+runtime+>(async {

    main().await;
    crossbus::reactor::Reactor::as_future().await;
}); }"#
        .replace("<+fn+>", &raw_stream)
        .replace("<+runtime+>", runtime);
    TokenStream::from_str(&raw_token).unwrap()
}
