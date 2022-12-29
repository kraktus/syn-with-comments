use proc_macro2::{Group, LineColumn, Span, TokenStream, TokenTree};
use quote::quote;
use std::str::FromStr;
use syn::{visit_mut::VisitMut, ItemImpl};

// amazing if there is no better way
// start is included, end is excluded
fn span_start_end(s: Span) -> (usize, usize) {
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

fn comments_between(input: &str, end_last_span: usize, end: Span) -> TokenStream {
    comments_between_raw(input, end_last_span, span_start_end(end).0)
}

fn comments_between_raw(input: &str, begin: usize, end: usize) -> TokenStream {
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
    match tt {
        TokenTree::Group(group) => {
            println!("In GROUP");
            // comments before the group
            let comments = comments_between(input, end_last_span, group.span());
            let last_span_boundary = span_start_end(group.span()).0 + 1; // plus 1 to get over the brace/parenthesis/space
            let inner_token_stream = if group.stream().is_empty() {
                // if the group is empty, the only thing that can be inside is a comment
                comments_between_raw(input, last_span_boundary, span_start_end(group.span()).1)
            } else {
                handle_token_stream(input, group.stream(), last_span_boundary)
            };
            let stream = quote!(#inner_token_stream);
            let group_with_comments = Group::new(group.delimiter(), stream);
            quote!(#comments #group_with_comments)
        }
        terminal_token => {
            let comments = comments_between(input, end_last_span, terminal_token.span());
            quote!(#comments #terminal_token)
        }
    }
}

fn handle_token_stream(input: &str, ts: TokenStream, mut end_last_span: usize) -> TokenStream {
    let inner_token_stream = ts.into_iter().map(|inner_tt| {
        let inner_span = inner_tt.span();
        let res = handle_token_tree(input, inner_tt, end_last_span);
        println!("res {res}");
        end_last_span = span_start_end(inner_span).1;
        res
    });
    quote!(#(#inner_token_stream)*)
}

fn main() {
    // let input = r#"
    //     // comment on Thing
    //     #[cfg(feature = foo)]
    //     impl Thing {
    //         // non-doc comment
    //         fn f(&self) {// foo
    //         }
    //         // also comment
    //         fn g(&self) {}
    //     }
    // "#;
    let input = r#"(0) // foo
        (// bar
        )
    "#;

    let impl_block: TokenStream = syn::parse_str(input).unwrap();
    println!("without comments debug: {:?}", quote!(#impl_block));
    println!("without comments: {}", quote!(#impl_block));
    println!(
        "with comments: {}",
        handle_token_stream(&input, quote!(#impl_block), 0)
    );
}
