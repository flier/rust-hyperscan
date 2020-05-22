use std::cell::RefCell;
use std::collections::VecDeque;
use std::str::pattern::{self, SearchStep};

use crate::common::BlockDatabase;
use crate::compile::{self, Builder, Flags};
use crate::runtime::{Matching, Scratch};

impl<'a> pattern::Pattern<'a> for compile::Pattern {
    type Searcher = Searcher<'a>;

    fn into_searcher(mut self, haystack: &'a str) -> Self::Searcher {
        self.flags |= Flags::SOM_LEFTMOST;
        let db = self.build().expect("build database");
        let scratch = db.alloc().expect("alloc scratch");

        Searcher {
            haystack,
            db,
            scratch,
            matches: None,
        }
    }
}

pub struct Searcher<'a> {
    haystack: &'a str,
    db: BlockDatabase,
    scratch: Scratch,
    matches: Option<RefCell<VecDeque<SearchStep>>>,
}

unsafe impl<'a> pattern::Searcher<'a> for Searcher<'a> {
    fn haystack(&self) -> &'a str {
        self.haystack
    }

    fn next(&mut self) -> SearchStep {
        if self.matches.is_none() {
            let matches = RefCell::new(VecDeque::new());

            self.db
                .scan(self.haystack, &self.scratch, Some(on_match), Some(&matches))
                .expect("scan");

            self.matches = Some(matches);
        }

        self.matches
            .as_mut()
            .unwrap()
            .borrow_mut()
            .pop_front()
            .unwrap_or(SearchStep::Done)
    }
}

fn on_match(_id: u32, from: u64, to: u64, matches: Option<&RefCell<VecDeque<SearchStep>>>) -> Matching {
    matches
        .expect("match")
        .borrow_mut()
        .push_back(SearchStep::Match(from as usize, to as usize));

    Matching::Continue
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
