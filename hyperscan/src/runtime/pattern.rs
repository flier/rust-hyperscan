use std::str::pattern::{self, SearchStep};

use crate::common::BlockDatabase;
use crate::compile::{self, Builder, Flags};
use crate::runtime::Matching;

impl<'a> pattern::Pattern<'a> for compile::Pattern {
    type Searcher = Searcher<'a>;

    fn into_searcher(mut self, haystack: &'a str) -> Self::Searcher {
        self.flags |= Flags::SOM_LEFTMOST;
        let db: BlockDatabase = self.build().expect("build database");
        let scratch = db.alloc_scratch().expect("alloc scratch");
        let mut matches = Vec::new();

        db.scan(haystack, &scratch, |_, from, to, _| {
            let from = from as usize;
            let to = to as usize;

            match matches.last() {
                Some(&SearchStep::Match(start, end)) => {
                    if start == from && end < to {
                        // only the non-overlapping match should be return
                        *matches.last_mut().unwrap() = SearchStep::Match(from, to);
                    } else {
                        if end < from {
                            matches.push(SearchStep::Reject(end, from))
                        }

                        matches.push(SearchStep::Match(from, to))
                    }
                }
                None => {
                    matches.push(SearchStep::Reject(0, from));
                    matches.push(SearchStep::Match(from, to));
                }
                _ => matches.push(SearchStep::Match(from, to)),
            }

            Matching::Continue
        })
        .expect("scan");

        match matches.last() {
            Some(&SearchStep::Match(_, end)) if end < haystack.len() => {
                matches.push(SearchStep::Reject(end, haystack.len()));
            }
            Some(&SearchStep::Reject(start, end)) if end < haystack.len() => {
                *matches.last_mut().unwrap() = SearchStep::Match(start, haystack.len());
            }
            _ => {}
        }

        matches.reverse();

        Searcher { haystack, matches }
    }
}

pub struct Searcher<'a> {
    haystack: &'a str,
    matches: Vec<SearchStep>,
}

unsafe impl<'a> pattern::Searcher<'a> for Searcher<'a> {
    fn haystack(&self) -> &'a str {
        self.haystack
    }

    fn next(&mut self) -> SearchStep {
        self.matches.pop().unwrap_or(SearchStep::Done)
    }
}

#[cfg(test)]
pub mod tests {
    #[test]
    fn test_searcher() {
        assert_eq!("baaaab".find(pattern! { "a+" }), Some(1));
        assert_eq!("baaaab".matches(pattern! { "a+" }).collect::<Vec<_>>(), vec!["aaaa"]);

        let regex = regex::Regex::new("a+").unwrap();
        assert_eq!("baaaab".matches(&regex).collect::<Vec<_>>(), vec!["aaaa"]);
    }
}
