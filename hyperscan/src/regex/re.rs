use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use failure::Error;

use crate::common::BlockDatabase;
use crate::compile::{Builder, Pattern};
use crate::runtime::Matching;

/// A compiled regular expression for matching Unicode strings.
#[derive(Clone)]
pub struct Regex(pub(crate) Arc<BlockDatabase>);

impl Regex {
    /// Compiles a regular expression.
    /// Once compiled, it can be used repeatedly to search, split or replace text in a string.
    ///
    /// If an invalid expression is given, then an error is returned.
    pub fn new(re: &str) -> Result<Regex, Error> {
        Pattern::new(re)?.build().map(|db| Regex(Arc::new(db)))
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
}
