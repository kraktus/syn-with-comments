use itertools::Itertools;
use proc_macro2::{LineColumn, Span, TokenStream, TokenTree,  Group};
use quote::quote;
use std::str::FromStr;
use syn::{visit_mut::VisitMut, ItemImpl};

fn byte_offset(input: &str, location: LineColumn) -> usize {
    let mut offset = 0;
    for _ in 1..location.line {
        offset += input[offset..].find('\n').unwrap() + 1;
    }
    offset
        + input[offset..]
            .chars()
            .take(location.column)
            .map(char::len_utf8)
            .sum::<usize>()
}

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
    let mut between = input[end_last_span..span_start_end(end).0].trim();
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
            // comments between the first delimiter and the first token
            let comments = comments_between(input, end_last_span, group.span());
            let last_span_boundary = span_start_end(group.span()).0 + 1; // plus 1 to get over the brace/parenthesis/space
            let inner_token_stream = handle_token_stream(input, group.stream(), last_span_boundary);
            let stream = quote!(#comments #inner_token_stream);
            let group_with_comments = Group::new(group.delimiter(), stream);
            quote!(#group_with_comments)

        }
        terminal_token => {
            let comments = comments_between(input, end_last_span, terminal_token.span());
            quote!(#comments #terminal_token)
        }
    }
}

fn handle_token_stream(input: &str, ts: TokenStream, mut end_last_span: usize) -> TokenStream {
    let inner_token_stream = ts.into_iter().map(|(inner_tt)| {
        let inner_span = inner_tt.span();
        let res = handle_token_tree(input, inner_tt, end_last_span);
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
    let input = r#"() // foo
        ()
    "#;

    let impl_block: TokenStream = syn::parse_str(input).unwrap();
    println!("without comments debug: {:?}", quote!(#impl_block));
    println!("without comments: {}", quote!(#impl_block));
    println!("with comments: {}", handle_token_stream(&input, quote!(#impl_block), 0));

    for (one, two) in quote!(#impl_block).into_iter().tuple_windows() {
        let last_one = one.span();
        let first_two = two.span();
        // we need to check if there are comments between
        println!("{one:?}");
        println!("{two:?}");
        println!(
            "comments between: {}",
            comments_between(input, span_start_end(last_one).1, first_two)
        )
        // let comment = &input[cur..byte_offset(input, first.start())];
        // cur = byte_offset(input, last.end());
        // println!("comment: {:?}", comment.trim());
    }
}
