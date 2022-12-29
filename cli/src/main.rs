use proc_macro2::{Group, LineColumn, Span, TokenStream, TokenTree};
use quote::quote;
use std::str::FromStr;
use syn::{visit_mut::VisitMut, ItemImpl};

struct CommentsRetriever<'a> {
    input: &'a str,
    ts: TokenStream,
}

impl<'a> CommentsRetriever<'a> {
    fn new(input: &'a str) -> syn::Result<Self> {
        let ts: TokenStream = syn::parse_str(input)?;
        Ok(Self { input, ts })
    }

    // return the byte offset bewteen `location` and the beginning and `input`
    fn byte_offset(&self, location: LineColumn) -> usize {
        let mut offset = 0;
        for _ in 1..location.line {
            offset += self.input[offset..].find('\n').unwrap() + 1;
        }
        offset
            + self.input[offset..]
                .chars()
                .take(location.column)
                .map(char::len_utf8)
                .sum::<usize>()
    }
    fn span_start(&self, s: Span) -> usize {
        self.byte_offset(s.start())
    }

    fn span_end(&self, s: Span) -> usize {
        self.byte_offset(s.end())
    }

    fn comments_between(&self, end_last_span: usize, end: Span) -> TokenStream {
        self.comments_between_raw(end_last_span, self.span_start(end))
    }

    fn comments_between_raw(&self, begin: usize, end: usize) -> TokenStream {
        let mut between = self.input[begin..end + 1].trim();
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

    // TODO make this function not recursive
    fn handle_token_tree(&self, tt: TokenTree, end_last_span: usize) -> TokenStream {
        match tt {
            TokenTree::Group(group) => {
                println!("In GROUP");
                // comments before the group
                let comments = self.comments_between(end_last_span, group.span());
                let last_span_boundary = self.span_start(group.span()) + 1; // plus 1 to get over the brace/parenthesis/space
                let inner_token_stream = if group.stream().is_empty() {
                    // if the group is empty, the only thing that can be inside is a comment
                    self.comments_between_raw(last_span_boundary, self.span_end(group.span()))
                } else {
                    self.handle_token_stream(group.stream(), last_span_boundary)
                };
                let stream = quote!(#inner_token_stream);
                let group_with_comments = Group::new(group.delimiter(), stream);
                quote!(#comments #group_with_comments)
            }
            terminal_token => {
                println!("In TERMINAL token");
                let comments = self.comments_between(end_last_span, dbg!(&terminal_token).span());
                quote!(#comments #terminal_token)
            }
        }
    }

    fn handle_token_stream(&self, ts: TokenStream, mut end_last_span: usize) -> TokenStream {
        let inner_token_stream = ts.into_iter().map(|inner_tt| {
            let inner_span = inner_tt.span();
            let res = self.handle_token_tree(inner_tt, end_last_span);
            println!("res {res}");
            end_last_span = self.span_end(inner_span);
            res
        });
        quote!(#(#inner_token_stream)*)
    }

    fn parse_str(self) -> TokenStream {
        self.handle_token_stream(self.ts.clone(), 0)
    }
}

fn parse_str(input: &str) -> syn::Result<TokenStream> {
    dbg!(input.len());
    CommentsRetriever::new(input).map(CommentsRetriever::parse_str)
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
    "#;
    // let input = r#"(0) // foo
    //     (// bar
    //     )
    // "#;

    // let impl_block: TokenStream = syn::parse_str(input).unwrap();
    // println!("without comments debug: {:?}", quote!(#impl_block));
    //println!("without comments: {}", quote!(#impl_block));
    //println!(
    //    "with comments: {}",
    //    handle_token_stream(&input, quote!(#impl_block), 0)
    //);
    println!("with comments parse_str: {}", parse_str(input).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        let input = r#"fn bar(x: usize) -> usize"#;
    }
}
