use proc_macro2::{Group, LineColumn, Span, TokenStream, TokenTree};
use quote::quote;

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
        let between = self.input[begin..end].trim();
        let comments = between
            .split('\n')
            .map(str::trim)
            .filter(|s| s.len() > 1) // at least the `//` or /* chars
            .map(|c| &c[2..]) // remove the `//` characters. TODO handle it with more care, inner/outer
            .map(|comment| quote!(#[comment =  #comment]));
        quote!(#(#comments)*)
    }

    // TODO make this function not recursive
    fn handle_token_tree(&self, tt: TokenTree, end_last_span: usize) -> TokenStream {
        let comments = if self.span_end(tt.span()) > end_last_span {
            self.comments_between(end_last_span, tt.span())
        } else {
            // this mean the token has the same span as the previous one, meaning
            // we're in expanded code. We will fast forward until returning to user code
            println!("IN EXPANDED CODE");
            return tt.into();
        };

        match tt {
            TokenTree::Group(group) => {
                println!("In GROUP: {:?}", group.span());
                let last_span_boundary = self.span_start(group.span()) + 1; // plus 1 to get over the brace/parenthesis/space
                let inner_token_stream = if group.stream().is_empty() {
                    // if the group is empty, the only thing that can be inside is a comment
                    self.comments_between_raw(last_span_boundary, self.span_end(group.span()))
                } else {
                    self.handle_token_stream(group.stream(), last_span_boundary)
                };
                let group_with_comments = Group::new(group.delimiter(), inner_token_stream);
                quote!(#comments #group_with_comments)
            }
            terminal_token => {
                println!("In TERMINAL token: {:?}", terminal_token.span());
                quote!(#comments #terminal_token)
            }
        }
    }

    fn handle_token_stream(&self, ts: TokenStream, mut end_last_span: usize) -> TokenStream {
        let inner_token_stream = ts.into_iter().map(|inner_tt| {
            let inner_span = inner_tt.span();
            let res = self.handle_token_tree(inner_tt, end_last_span);
            println!("res {res}");
            end_last_span = dbg!(self.span_end(inner_span));
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
    println!("with comments parse_str: {}", parse_str(input).unwrap());
}
#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_parse_str() {
        let input = r#"// comment on Thing
#[cfg(feature = foo)]
impl Thing {
    // non-doc comment
    fn f(&self) {
        // foo
        todo!()
    }
    // also comment
    fn g(&self) {
        todo!()
    }
}
"#;
        let x = parse_str(input).unwrap();
        println!("RESULT: {x}");
        let res = prettyplease::unparse(&parse_quote!(#x));
        assert_eq!(input, res)
    }

    #[test]
    #[ignore = "trailing comments not handled for the moment"]
    fn test_trailing_comments() {
        let input = r#"// comment on Thing
#[cfg(feature = foo)]
impl Thing {
    // non-doc comment
    fn f(&self) {
        // foo
        todo!()
    }
    // also comment
    fn g(&self) {
        todo!()
    }
}


// this is a trailing comment, added after the last token
"#;
        let x = parse_str(input).unwrap();
        println!("RESULT: {x}");
        let res = prettyplease::unparse(&parse_quote!(#x));
        assert_eq!(input, res)
    }
}
