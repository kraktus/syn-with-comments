use proc_macro2::{Group, LineColumn, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use syn::parse::Parse;

macro_rules! debug_println {
        ($tt:tt) => {
        #[cfg(feature = "debug")]
        println!($tt);
        };
    ($tt:tt, $($e:expr),+) => {
        #[cfg(feature = "debug")]
        println!($tt, $($e),*);
        }
}

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
        self.comments_between_raw(end_last_span, self.span_start(end), end)
    }

    fn comments_between_raw(&self, begin: usize, end: usize, span: Span) -> TokenStream {
        let between = self.input[begin..end].trim();
        let comments = between
            .split('\n')
            .map(str::trim)
            .filter(|s| s.len() > 1) // at least the `//` or /* chars
            .map(|c| &c[2..]) // remove the `//` characters. TODO handle it with more care, inner/outer
            .map(|comment| quote_spanned!(span=> #[comment =  #comment]));
        quote!(#(#comments)*)
    }

    // TODO make this function not recursive
    fn handle_token_tree(&self, tt: TokenTree, end_last_span: usize) -> TokenStream {
        let comments = if self.span_end(tt.span()) > end_last_span {
            self.comments_between(end_last_span, tt.span())
        } else {
            // this mean the token has the same span as the previous one, meaning
            // we're in expanded code. We will fast forward until returning to user code
            debug_println!("IN EXPANDED CODE");
            return tt.into();
        };

        match tt {
            TokenTree::Group(group) => {
                debug_println!("In GROUP: {:?}", group.span());
                let last_span_boundary = self.span_start(group.span()) + 1; // plus 1 to get over the brace/parenthesis/space
                let inner_token_stream = self.handle_token_stream(group.stream(), last_span_boundary);
                let group_with_comments = Group::new(group.delimiter(), inner_token_stream);
                quote!(#comments #group_with_comments)
            }
            terminal_token => {
                debug_println!("In TERMINAL token: {terminal_token:?}, span: {:?}", terminal_token.span());
                quote!(#comments #terminal_token)
            }
        }
    }

    fn handle_token_stream(&self, ts: TokenStream, mut end_last_span: usize) -> TokenStream {
        let inner_token_stream = ts.into_iter().map(|inner_tt| {
            let inner_span = inner_tt.span();
            let res = self.handle_token_tree(inner_tt, end_last_span);
            debug_println!("res {res}");
            end_last_span = self.span_end(inner_span);
            res
        });
        quote!(#(#inner_token_stream)*)
    }

    fn parse_str<T: Parse>(self) -> T {
        let mut ts = self.handle_token_stream(self.ts.clone(), 0);
        // now we must remove all
        loop {
            match syn::parse2(ts) {
                Ok(parsed) => return parsed,
                Err(e) => panic!("error: {e}, span: {:?}", e.span()),
            }
        }
    }
}

pub fn parse_str<T: Parse>(input: &str) -> syn::Result<T> {
    CommentsRetriever::new(input).map(CommentsRetriever::parse_str)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use syn::File;

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
        let x: File = parse_str(input).unwrap();
        debug_println!("RESULT: {x:?}");
        let res = prettyplease::unparse(&x);
        assert_eq!(input, res)
    }

    #[test]
    #[ignore = "comments in empty groups not handled for the moment"]
    fn test_parse_str_empty_group() {
        let input = r#"fn f(_: usize) {
    // foo
}
"#;
        let x: File = parse_str(input).unwrap();
        debug_println!("RESULT: {x:?}");
        let res = prettyplease::unparse(&x);
        assert_eq!(input, res)
    }

    #[test]
    fn test_parse_str2() {
        let input = r#"fn xxx() {
    let comments = between
        .filter() // bar
        .map(); // foo
}
"#;
        let x: File = parse_str(input).unwrap();
        debug_println!("RESULT: {x:?}");
        let res = prettyplease::unparse(&x);
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
        let x: File = parse_str(input).unwrap();
        let res = prettyplease::unparse(&x);
        assert_eq!(input, res)
    }
}
