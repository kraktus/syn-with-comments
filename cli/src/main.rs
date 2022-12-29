use proc_macro2::{Group, LineColumn, Span, TokenStream, TokenTree};
use quote::quote;
use std::str::FromStr;
use syn::{visit_mut::VisitMut, ItemImpl};

// amazing if there is no better way
// start is included, end is excluded
fn span_start_and_end(s: Span) -> (usize, usize) {
    let debug_span = format!("{s:?}"); // allocating a string every time...
                                       // we know it's ASCII only
                                       // because of the form: `bytes(start..stop)`
    debug_span[6..debug_span.len() - 1]
        .split_once("..")
        .map(|(start, end)| {
            (
                usize::from_str(start).unwrap(),
                usize::from_str(end).unwrap(),
            )
        })
        .unwrap()
}

fn span_start(s: Span) -> usize {
    span_start_and_end(s).0
}

fn span_end(s: Span) -> usize {
    span_start_and_end(s).1
}

fn comments_between(input: &str, end_last_span: usize, end: Span) -> TokenStream {
    comments_between_raw(input, end_last_span, span_start(end))
}

fn comments_between_raw(input: &str, begin: usize, end: usize) -> TokenStream {
    dbg!("begin: {} end: {}", begin, end);
    let mut between = input[begin..end].trim();
    if between.is_empty() {
        TokenStream::new()
    } else {
        // FIXME why minus 1 needed?
        between = &between[..between.len() - 1];
        let comments = between
            .split('\n')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|c| &c[2..]) // remove the `//` characters. TODO handle it with more care, inner/outer
            .map(|comment| quote!(#[comment =  #comment]));
        quote!(#(#comments)*)
    }
}

// TODO generalise input
// TODO make this function not recursive
fn handle_token_tree(input: &str, tt: TokenTree, end_last_span: usize) -> TokenStream {
    println!("Handling token tree");
    match tt {
        TokenTree::Group(group) => {
            println!("In GROUP");
            // comments before the group
            let comments = comments_between(input, end_last_span, dbg!(group.span()));
            let last_span_boundary = span_start(group.span()) + 1; // plus 1 to get over the brace/parenthesis/space
            let inner_token_stream = if group.stream().is_empty() {
                // if the group is empty, the only thing that can be inside is a comment
                comments_between_raw(input, last_span_boundary, dbg!(span_end(group.span())))
            } else {
                handle_token_stream(input, group.stream(), last_span_boundary)
            };
            let stream = quote!(#inner_token_stream);
            let group_with_comments = Group::new(group.delimiter(), stream);
            quote!(#comments #group_with_comments)
        }
        terminal_token => {
            println!("{terminal_token:?}");
            let comments = comments_between(input, end_last_span, dbg!(terminal_token.span()));
            quote!(#comments #terminal_token)
        }
    }
}

fn handle_token_stream(input: &str, ts: TokenStream, mut end_last_span: usize) -> TokenStream {
    println!("Handling token stream");
    let inner_token_stream = ts.into_iter().map(|inner_tt| {
        let inner_span = inner_tt.span();
        let res = handle_token_tree(input, inner_tt, end_last_span);
        println!("res {res}");
        end_last_span = dbg!(span_end(inner_span));
        res
    });
    quote!(#(#inner_token_stream)*)
}

fn parse_str(input: &str) -> syn::Result<TokenStream> {
    dbg!(input.len());
    syn::parse_str(input).map(|ts: TokenStream| {
        println!("ts: {ts}");
        let ts_with_comments = handle_token_stream(&input, ts.clone(), 0);
        let trailing_comments = ts
            .into_iter()
            .last()
            .map(|token| comments_between_raw(input, span_end(token.span()), dbg!(input.len())))
            .unwrap_or_default();
        quote!(#ts_with_comments #trailing_comments)
    })
}

fn main() {
    let input = r#"
        // comment on Thing
        #[cfg(feature = foo)]
        impl Thing {
            // non-doc comment
            fn f(&self) {// foo
            }
            // also comment
            fn g(&self) {}
        }
        // trailing comment
    "#;
    // let input = r#"(0) // foo
    //     (// bar
    //     )
    // "#;

    let impl_block: TokenStream = syn::parse_str(input).unwrap();
    // println!("without comments debug: {:?}", quote!(#impl_block));
    // println!("without comments: {}", quote!(#impl_block));
    println!("with comments: {}", parse_str(input).unwrap());
}
