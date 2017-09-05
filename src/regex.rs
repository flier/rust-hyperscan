use std::cell::RefCell;

use api::{BlockScanner, DatabaseBuilder, ScratchAllocator};
use common::BlockDatabase;
use compile::Pattern;
use constants::HS_FLAG_SOM_LEFTMOST;
use errors::Result;

/// A compiled regular expression for matching Unicode strings.
///
/// It is represented as either a sequence of bytecode instructions (dynamic)
/// or as a specialized Rust function (native). It can be used to search, split
/// or replace text. All searching is done with an implicit `.*?` at the
/// beginning and end of an expression. To force an expression to match the
/// whole string (or a prefix or a suffix), you must use an anchor like `^` or
/// `$` (or `\A` and `\z`).
///
/// While this crate will handle Unicode strings (whether in the regular
/// expression or in the search text), all positions returned are **byte
/// indices**. Every byte index is guaranteed to be at a Unicode code point
/// boundary.
///
/// The lifetimes `'r` and `'t` in this crate correspond to the lifetime of a
/// compiled regular expression and text to search, respectively.
///
/// The only methods that allocate new strings are the string replacement
/// methods. All other methods (searching and splitting) return borrowed
/// pointers into the string given.
///
/// # Examples
///
/// Find the location of a US phone number:
///
/// ```rust
/// # use hyperscan::regex::Regex;
/// let re = Regex::new("[0-9]{3}-[0-9]{3}-[0-9]{4}").unwrap();
/// let mat = re.find("phone: 111-222-3333").unwrap();
/// assert_eq!((mat.start(), mat.end()), (7, 19));
/// ```
pub struct Regex {
    pattern: Pattern,
    db: BlockDatabase,
}

/// Core regular expression methods.
impl Regex {
    /// Compiles a regular expression. Once compiled, it can be used repeatedly
    /// to search, split or replace text in a string.
    ///
    /// If an invalid expression is given, then an error is returned.
    pub fn new(re: &str) -> Result<Self> {
        let mut pattern: Pattern = re.parse()?;

        pattern.flags |= HS_FLAG_SOM_LEFTMOST;

        let db: BlockDatabase = pattern.build()?;

        Ok(Regex { pattern, db })
    }

    /// Returns true if and only if the regex matches the string given.
    ///
    /// It is recommended to use this method if all you need to do is test
    /// a match, since the underlying matching engine may be able to do less
    /// work.
    ///
    /// # Example
    ///
    /// Test if some text contains at least one word with exactly 13 ASCII word
    /// bytes:
    ///
    /// ```rust
    /// # extern crate hyperscan; use hyperscan::regex::Regex;
    /// # fn main() {
    /// let text = "I categorically deny having triskaidekaphobia.";
    /// assert!(Regex::new(r"\b\w{13}\b").unwrap().is_match(text));
    /// # }
    /// ```
    pub fn is_match(&self, text: &str) -> bool {
        self.find(text).is_some()
    }

    /// Returns the start and end byte range of the leftmost-first match in
    /// `text`. If no match exists, then `None` is returned.
    ///
    /// Note that this should only be used if you want to discover the position
    /// of the match. Testing the existence of a match is faster if you use
    /// `is_match`.
    ///
    /// # Example
    ///
    /// Find the start and end location of the first word with exactly 13
    /// ASCII word bytes:
    ///
    /// ```rust
    /// # extern crate hyperscan; use hyperscan::regex::Regex;
    /// # fn main() {
    /// let text = "I categorically deny having triskaidekaphobia.";
    /// let mat = Regex::new(r"\b\w{13}\b").unwrap().find(text).unwrap();
    /// assert_eq!((mat.start(), mat.end()), (2, 15));
    /// # }
    /// ```
    pub fn find<'t>(&self, text: &'t str) -> Option<Match<'t>> {
        self.db.alloc().ok().and_then(|mut s| {
            let m = RefCell::new(Match::new(text));

            self.db
                .scan(text, 0, &mut s, Some(Match::matched), Some(&m))
                .ok()
                .and_then(|_| if m.borrow().is_matched() {
                    Some(m.into_inner())
                } else {
                    None
                })
        })
    }

    /// Returns an iterator for each successive non-overlapping match in
    /// `text`, returning the start and end byte indices with respect to
    /// `text`.
    ///
    /// # Example
    ///
    /// Find the start and end location of every word with exactly 13 ASCII
    /// word bytes:
    ///
    /// ```rust
    /// # extern crate hyperscan; use hyperscan::regex::Regex;
    /// # fn main() {
    /// let text = "Retroactively relinquishing remunerations is reprehensible.";
    /// for mat in Regex::new(r"\b\w{13}\b").unwrap().find_iter(text) {
    ///     println!("{:?}", mat);
    /// }
    /// # }
    /// ```
    pub fn find_iter<'r, 't>(&'r self, text: &'t str) -> Matches<'r, 't> {
        Matches::new(&self.db, text)
    }
}

/// Match represents a single match of a regex in a haystack.
///
/// The lifetime parameter 't refers to the lifetime of the matched text.
#[derive(Clone, Debug)]
pub struct Match<'t> {
    text: &'t str,
    start: usize,
    end: usize,
}

impl<'t> Match<'t> {
    fn new(text: &'t str) -> Match<'t> {
        Match {
            text: text,
            start: 0,
            end: 0,
        }
    }

    fn is_matched(&self) -> bool {
        self.end > self.start
    }

    fn update(&mut self, from: u64, to: u64) {
        self.start = from as usize;
        self.end = to as usize;
    }

    extern "C" fn matched(id: u32, from: u64, to: u64, flags: u32, m: &RefCell<Match>) -> u32 {
        (*m.borrow_mut()).update(from, to);

        0
    }

    /// Returns the starting byte offset of the match in the haystack.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the ending byte offset of the match in the haystack.
    pub fn end(&self) -> usize {
        self.end
    }

    /// Returns the matched text.
    pub fn as_str(&self) -> &'t str {
        &self.text[self.start..self.end]
    }
}

#[derive(Debug)]
pub struct Matches<'r, 't> {
    db: &'r BlockDatabase,
    text: &'t str,
    m: RefCell<Option<Match<'t>>>,
}

impl<'r, 't> Iterator for Matches<'r, 't> {
    type Item = Match<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        let m = self.m.borrow_mut().take();

        m.map(|ref m| &self.text[m.end..]).and_then(|text| {
            self.db.alloc().ok().and_then(|mut s| {
                self.db
                    .scan(text, 0, &mut s, Some(Self::matched), Some(&self.m))
                    .ok()
                    .and_then(|_| {
                        (*self.m.borrow()).as_ref().and_then(|m| if m.is_matched() {
                            Some(m.clone())
                        } else {
                            None
                        })
                    })
            })
        })
    }
}

impl<'r, 't> Matches<'r, 't> {
    fn new(db: &'r BlockDatabase, text: &'t str) -> Matches<'r, 't> {
        Matches {
            db,
            text,
            m: RefCell::new(Some(Match::new(text))),
        }
    }
    extern "C" fn matched(id: u32, from: u64, to: u64, flags: u32, m: &RefCell<Option<Match<'t>>>) -> u32 {
        if let Some(ref mut m) = *m.borrow_mut() {
            m.update(from, to);
        }

        0
    }
}
