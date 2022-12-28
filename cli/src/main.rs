use itertools::Itertools;
use proc_macro2::{LineColumn, Span, TokenStream, TokenTree};
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
            println!("{start}..{end}");
            (
                usize::from_str(start).unwrap(),
                usize::from_str(end).unwrap(),
            )
        })
        .unwrap()
}

fn comments_between(input: &str, start: Span, end: Span) -> TokenStream {
    let mut between = input[span_start_end(start).1..span_start_end(end).0].trim();
    let mut buf = TokenStream::new();
    if between.is_empty() {
        TokenStream::new()
    } else {
        // FIXME why minus 1 needed?
        between = &between[..between.len() - 1];
        let comments = between
            .split('\n')
            .map(str::trim)
            .filter_map(|comment| (!comment.is_empty()).then(|| quote!(#[comment =  #comment])));
        quote!(#(#comments)*)
    }
}

// TODO generalise input
// TODO make this function not recursive
fn handle_token(input: &str, tt: &TokenTree, end: usize) -> TokenStream {
    match tt {
        TokenTree::Group(_) => todo!(),
        x => todo!(),
    }
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
    let input = r#"(
        
        ); // foo
        ()
    "#;

    let impl_block: TokenStream = syn::parse_str(input).unwrap();
    let mut cur = 0; //byte_offset(input, impl_block.brace_token.span.start()) + 1;
    println!("{:?}", quote!(#impl_block));
    for (one, two) in quote!(#impl_block).into_iter().tuple_windows() {
        let last_one = one.span();
        let first_two = two.span();
        // we need to check if there are comments between
        println!("{one:?}");
        println!("{two:?}");
        println!(
            "comments between: {}",
            comments_between(input, last_one, first_two)
        )
        // let comment = &input[cur..byte_offset(input, first.start())];
        // cur = byte_offset(input, last.end());
        // println!("comment: {:?}", comment.trim());
    }
}
