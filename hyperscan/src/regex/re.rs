use std::ops::Range;
use std::str::FromStr;
use std::sync::Arc;
use std::vec;

use crate::{
    common::BlockDatabase,
    compile::{Builder, Flags, Pattern},
    runtime::Matching,
    Error, Result,
};

/// Match represents a single match of a regex in a haystack.
///
/// The lifetime parameter `'t` refers to the lifetime of the matched text.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

    /// Returns the range over the starting and ending byte offsets of the
    /// match in the haystack.
    #[inline]
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
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
            start,
            end,
        }
    }
}

impl<'t> From<Match<'t>> for &'t str {
    fn from(m: Match<'t>) -> &'t str {
        m.as_str()
    }
}

impl<'t> From<Match<'t>> for Range<usize> {
    fn from(m: Match<'t>) -> Range<usize> {
        m.range()
    }
}

/// An iterator over all non-overlapping matches for a particular string.
///
/// The iterator yields a `Match` value. The iterator stops when no more
/// matches can be found.
///
/// `'r` is the lifetime of the compiled regular expression and `'t` is the
/// lifetime of the matched string.
pub struct Matches<'t>(&'t str, vec::IntoIter<Range<usize>>);

impl<'t> Matches<'t> {
    /// Return the text being searched.
    pub fn text(&self) -> &'t str {
        self.0
    }
}

impl<'t> Iterator for Matches<'t> {
    type Item = Match<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        self.1.next().map(|range| Match::new(self.0, range.start, range.end))
    }
}

impl<'t> DoubleEndedIterator for Matches<'t> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.1
            .next_back()
            .map(|range| Match::new(self.0, range.start, range.end))
    }
}

/// A compiled regular expression for matching Unicode strings.
#[derive(Clone)]
pub struct Regex(pub(crate) Arc<BlockDatabase>);

impl FromStr for Regex {
    type Err = Error;

    /// Attempts to parse a string into a regular expression
    fn from_str(s: &str) -> Result<Regex> {
        Regex::new(s)
    }
}

/// Core regular expression methods.
impl Regex {
    /// Compiles a regular expression.
    /// Once compiled, it can be used repeatedly to search, split or replace text in a string.
    ///
    /// If an invalid expression is given, then an error is returned.
    pub fn new<S: Into<String>>(re: S) -> Result<Regex> {
        Self::with_flags(re, Flags::empty())
    }

    pub(crate) fn with_flags<S: Into<String>>(re: S, flags: Flags) -> Result<Regex> {
        Pattern::with_flags(re, flags | Flags::SOM_LEFTMOST | Flags::UTF8)?
            .build()
            .map(|db| Regex(Arc::new(db)))
    }

    /// Returns true if and only if the regex matches the string given.
    ///
    /// It is recommended to use this method if all you need to do is test a match,
    /// since the underlying matching engine may be able to do less work.
    ///
    /// # Examples
    ///
    /// Test if some text contains at least one word with exactly 13 Unicode word characters:
    ///
    /// ```rust
    /// # use hyperscan::regex::Regex;
    /// let text = "I categorically deny having triskaidekaphobia.";
    /// assert!(Regex::new(r"\b\w{13}\b").unwrap().is_match(text));
    /// ```
    pub fn is_match(&self, text: &str) -> bool {
        let mut matched = false;

        let s = self.0.alloc_scratch().unwrap();
        let _ = self.0.scan(text, &s, |_, _, _, _| {
            matched = true;

            Matching::Terminate
        });

        matched
    }

    /// Returns the start and end byte range of the leftmost-first match in text. If no match exists, then None is returned.
    ///
    /// Note that this should only be used if you want to discover the position of the match. Testing the existence of a match is faster if you use is_match.
    ///
    /// # Examples
    ///
    /// Find the start and end location of the first word with exactly 13 Unicode word characters:
    ///
    /// ```rust
    /// # use hyperscan::regex::Regex;
    /// let text = "I categorically deny having triskaidekaphobia.";
    /// let mat = Regex::new(r"\b\w{13}\b").unwrap().find(text).unwrap();
    /// assert_eq!(mat.start(), 2);
    /// assert_eq!(mat.end(), 15);
    /// ```
    pub fn find<'t>(&self, text: &'t str) -> Option<Match<'t>> {
        let mut matched = vec![];

        let s = self.0.alloc_scratch().unwrap();
        let _ = self.0.scan(text, &s, |_, from, to, _| {
            matched.push((from as usize, to as usize));

            Matching::Terminate
        });

        matched
            .first()
            .map(|&(start, end)| Match::new(&text[start..end], start, end))
    }

    /// Returns an iterator for each successive non-overlapping match in
    /// `text`, returning the start and end byte indices with respect to
    /// `text`.
    ///
    /// # Examples
    ///
    /// Find the start and end location of every word with exactly 13 Unicode
    /// word characters:
    ///
    /// ```rust
    /// # use hyperscan::regex::Regex;
    /// let text = "Retroactively relinquishing remunerations is reprehensible.";
    /// for mat in Regex::new(r"\b\w{13}\b").unwrap().find_iter(text) {
    ///     println!("{:?}", mat);
    /// }
    /// ```
    pub fn find_iter<'t>(&self, text: &'t str) -> Matches<'t> {
        let mut matched = Vec::<Range<usize>>::new();

        let s = self.0.alloc_scratch().unwrap();
        let _ = self.0.scan(text, &s, |_, from, to, _| {
            let range = from as usize..to as usize;

            match matched.last() {
                Some(last) if last.start == range.start && last.end < range.end => {
                    // only the non-overlapping match should be return
                    *matched.last_mut().unwrap() = range;
                }
                _ => matched.push(range),
            }

            Matching::Continue
        });

        Matches(text, matched.into_iter())
    }

    /// Returns an iterator of substrings of `text` delimited by a match of the
    /// regular expression. Namely, each element of the iterator corresponds to
    /// text that *isn't* matched by the regular expression.
    ///
    /// This method will *not* copy the text given.
    ///
    /// # Examples
    ///
    /// To split a string delimited by arbitrary amounts of spaces or tabs:
    ///
    /// ```rust
    /// # use hyperscan::regex::Regex;
    /// let re = Regex::new(r"[ \t]+").unwrap();
    /// let fields: Vec<&str> = re.split("a b \t  c\td    e").collect();
    /// assert_eq!(fields, vec!["a", "b", "c", "d", "e"]);
    /// ```
    pub fn split<'t>(&self, text: &'t str) -> Split<'t> {
        Split {
            finder: self.find_iter(text),
            last: 0,
        }
    }

    /// Returns an iterator of at most `limit` substrings of `text` delimited
    /// by a match of the regular expression. (A `limit` of `0` will return no
    /// substrings.) Namely, each element of the iterator corresponds to text
    /// that *isn't* matched by the regular expression. The remainder of the
    /// string that is not split will be the last element in the iterator.
    ///
    /// This method will *not* copy the text given.
    ///
    /// # Examples
    ///
    /// Get the first two words in some text:
    ///
    /// ```rust
    /// # use hyperscan::regex::Regex;
    /// let re = Regex::new(r"\W+").unwrap();
    /// let fields: Vec<&str> = re.splitn("Hey! How are you?", 3).collect();
    /// assert_eq!(fields, vec!("Hey", "How", "are you?"));
    /// ```
    pub fn splitn<'t>(&self, text: &'t str, limit: usize) -> SplitN<'t> {
        SplitN {
            splits: self.split(text),
            n: limit,
        }
    }
}

/// Yields all substrings delimited by a regular expression match.
///
/// `'t` is the lifetime of the string being split.
pub struct Split<'t> {
    finder: Matches<'t>,
    last: usize,
}

impl<'t> Iterator for Split<'t> {
    type Item = &'t str;

    fn next(&mut self) -> Option<&'t str> {
        let text = self.finder.text();
        match self.finder.next() {
            None => {
                if self.last > text.len() {
                    None
                } else {
                    let s = &text[self.last..];
                    self.last = text.len() + 1; // Next call will return None
                    Some(s)
                }
            }
            Some(m) => {
                let matched = &text[self.last..m.start()];
                self.last = m.end();
                Some(matched)
            }
        }
    }
}

/// Yields at most `N` substrings delimited by a regular expression match.
///
/// The last substring will be whatever remains after splitting.
///
/// `'t` is the lifetime of the string being split.
pub struct SplitN<'t> {
    splits: Split<'t>,
    n: usize,
}

impl<'t> Iterator for SplitN<'t> {
    type Item = &'t str;

    fn next(&mut self) -> Option<&'t str> {
        if self.n == 0 {
            return None;
        }

        self.n -= 1;
        if self.n > 0 {
            return self.splits.next();
        }

        let text = self.splits.finder.text();
        if self.splits.last > text.len() {
            // We've already returned all substrings.
            None
        } else {
            // self.n == 0, so future calls will return None immediately
            Some(&text[self.splits.last..])
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_find_iter() {
        let regex = r"\b\w{13}\b";
        let text = "Retroactively relinquishing remunerations is reprehensible.";

        assert_eq!(
            regex::Regex::new(regex)
                .unwrap()
                .find_iter(text)
                .map(|m| m.range())
                .collect::<Vec<_>>(),
            super::Regex::new(regex)
                .unwrap()
                .find_iter(text)
                .map(|m| m.range())
                .collect::<Vec<_>>()
        );
    }
}
