use core::cell::RefCell;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use failure::Error;

use crate::common::BlockDatabase;
use crate::compile::{Builder, Flags, Pattern};
use crate::runtime::Matching;

/// Match represents a single match of a regex in a haystack.
///
/// The lifetime parameter `'t` refers to the lifetime of the matched text.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Match<'t> {
    text: &'t str,
    start: usize,
    end: usize,
}

impl<'t> Match<'t> {
    /// Returns the starting byte offset of the match in the haystack.
    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the ending byte offset of the match in the haystack.
    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }

    /// Returns the matched text.
    #[inline]
    pub fn as_str(&self) -> &'t str {
        &self.text[self.start..self.end]
    }

    /// Creates a new match from the given haystack and byte offsets.
    #[inline]
    fn new(haystack: &'t str, start: usize, end: usize) -> Match<'t> {
        Match {
            text: haystack,
            start: start,
            end: end,
        }
    }
}

impl<'t> From<Match<'t>> for &'t str {
    fn from(m: Match<'t>) -> &'t str {
        m.as_str()
    }
}

/// A compiled regular expression for matching Unicode strings.
#[derive(Clone)]
pub struct Regex(pub(crate) Arc<BlockDatabase>);

impl Regex {
    /// Compiles a regular expression.
    /// Once compiled, it can be used repeatedly to search, split or replace text in a string.
    ///
    /// If an invalid expression is given, then an error is returned.
    pub fn new(re: &str) -> Result<Regex, Error> {
        Self::with_flags(re, Flags::empty())
    }

    pub(crate) fn with_flags(re: &str, flags: Flags) -> Result<Regex, Error> {
        Pattern::with_flags(re, flags | Flags::SOM_LEFTMOST | Flags::UTF8)?
            .build()
            .map(|db| Regex(Arc::new(db)))
    }

    /// Returns true if and only if the regex matches the string given.
    ///
    /// It is recommended to use this method if all you need to do is test a match,
    /// since the underlying matching engine may be able to do less work.
    ///
    /// # Example
    ///
    /// Test if some text contains at least one word with exactly 13 Unicode word characters:
    ///
    /// ```   
    /// # use hyperscan::regex::Regex;
    /// let text = "I categorically deny having triskaidekaphobia.";
    /// assert!(Regex::new(r"\b\w{13}\b").unwrap().is_match(text));
    /// ```
    pub fn is_match(&self, text: &str) -> bool {
        let matched = AtomicBool::new(false);

        fn matching(_id: u32, _from: u64, _to: u64, data: Option<&AtomicBool>) -> Matching {
            data.unwrap().store(true, Ordering::Relaxed);

            Matching::Break
        }

        let s = self.0.alloc().unwrap();
        let _ = self.0.scan(text, &s, Some(matching), Some(&matched));

        matched.load(Ordering::Relaxed)
    }

    /// Returns the start and end byte range of the leftmost-first match in text. If no match exists, then None is returned.
    ///
    /// Note that this should only be used if you want to discover the position of the match. Testing the existence of a match is faster if you use is_match.
    ///
    /// # Example
    ///
    /// Find the start and end location of the first word with exactly 13 Unicode word characters:
    ///
    /// ```
    /// # use hyperscan::regex::Regex;
    /// let text = "I categorically deny having triskaidekaphobia.";
    /// let mat = Regex::new(r"\b\w{13}\b").unwrap().find(text).unwrap();
    /// assert_eq!(mat.start(), 2);
    /// assert_eq!(mat.end(), 15);
    /// ```
    pub fn find<'t>(&self, text: &'t str) -> Option<Match<'t>> {
        let matched = RefCell::new(vec![]);

        fn matching(_id: u32, from: u64, to: u64, data: Option<&RefCell<Vec<(usize, usize)>>>) -> Matching {
            data.unwrap().borrow_mut().push((from as usize, to as usize));

            Matching::Break
        }

        let s = self.0.alloc().unwrap();
        let _ = self.0.scan(text, &s, Some(matching), Some(&matched));

        matched
            .into_inner()
            .first()
            .map(|&(start, end)| Match::new(&text[start..end], start, end))
    }
}
