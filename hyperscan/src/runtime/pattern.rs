use core::pin::Pin;
use core::str::pattern::{Pattern, SearchStep, Searcher};
use std::collections::VecDeque;

use crate::common::BlockDatabase;
use crate::compile::{self, Builder, Flags};
use crate::runtime::Scratch;

impl<'a> Pattern<'a> for compile::Pattern {
    type Searcher = HsSearcher<'a>;

    fn into_searcher(mut self, haystack: &'a str) -> Self::Searcher {
        self.flags |= Flags::SOM_LEFTMOST;
        let db = self.build().expect("build database");
        let scratch = db.alloc().expect("alloc scratch");

        HsSearcher {
            haystack,
            db,
            scratch,
            matches: None,
        }
    }
}

pub struct HsSearcher<'a> {
    haystack: &'a str,
    db: BlockDatabase,
    scratch: Scratch,
    matches: Option<VecDeque<SearchStep>>,
}

unsafe impl<'a> Searcher<'a> for HsSearcher<'a> {
    fn haystack(&self) -> &'a str {
        self.haystack
    }

    fn next(&mut self) -> SearchStep {
        if self.matches.is_none() {
            let mut matches = VecDeque::new();

            self.db
                .scan(
                    self.haystack,
                    &self.scratch,
                    Some(on_match),
                    Some(Pin::new(&mut matches)),
                )
                .expect("scan");

            self.matches = Some(matches);
        }

        self.matches.as_mut().unwrap().pop_front().unwrap_or(SearchStep::Done)
    }
}

fn on_match<'a>(_id: u32, from: u64, to: u64, _flags: u32, matches: Option<Pin<&'a mut VecDeque<SearchStep>>>) -> u32 {
    matches
        .expect("match")
        .as_mut()
        .push_back(dbg!(SearchStep::Match(from as usize, to as usize)));

    0
}

#[cfg(test)]
pub mod tests {
    #[test]
    fn test_searcher() {
        assert_eq!("baaaab".find(pattern! { "a+" }), Some(1));
        assert_eq!(
            "baaaab".matches(pattern! { "a+" }).collect::<Vec<_>>(),
            vec!["a", "aa", "aaa", "aaaa"]
        );
    }
}
