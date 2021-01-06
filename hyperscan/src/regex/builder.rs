use crate::{compile::Flags, regex::Regex, Result};

/// A configurable builder for a regular expression.
///
/// A builder can be used to configure how the regex is built,
/// for example, by setting the default flags
/// (which can be overridden in the expression itself).
pub type RegexBuilder = Builder<String>;

/// A configurable builder for a set of regular expressions.
///
/// A builder can be used to configure how the regexes are built,
/// for example, by setting the default flags
/// (which can be overridden in the expression itself).
pub type RegexSetBuilder = Builder<Vec<String>>;

/// A configurable builder for a regular expression.
pub struct Builder<T> {
    expr: T,
    flags: Flags,
}

impl Builder<String> {
    /// Create a new regular expression builder with the given pattern.
    ///
    /// If the pattern is invalid, then an error will be returned when build is called.
    pub fn new<S: Into<String>>(pattern: S) -> Self {
        Builder {
            expr: pattern.into(),
            flags: Flags::empty(),
        }
    }

    /// Consume the builder and compile the regular expression.
    ///
    /// Note that calling `as_str` on the resulting Regex will produce the pattern given to new verbatim.
    /// Notably, it will not incorporate any of the flags set on this builder.
    pub fn build(&self) -> Result<Regex> {
        Regex::with_flags(&self.expr, self.flags)
    }
}

impl<T> Builder<T> {
    fn toggle(&mut self, flag: Flags, yes: bool) -> &mut Self {
        if yes {
            self.flags.insert(flag)
        } else {
            self.flags.remove(flag)
        }
        self
    }

    /// Set the value for the case insensitive (`i`) flag.
    ///
    /// When enabled, letters in the pattern will match both upper case and lower case variants.
    pub fn case_insensitive(&mut self, yes: bool) -> &mut Self {
        self.toggle(Flags::CASELESS, yes)
    }

    /// Set the value for the multi-line matching (`m`) flag.
    ///
    /// When enabled, ^ matches the beginning of lines and $ matches the end of lines.
    ///
    /// By default, they match beginning/end of the input.
    pub fn multi_line(&mut self, yes: bool) -> &mut Self {
        self.toggle(Flags::MULTILINE, yes)
    }

    /// Set the value for the any character (`s`) flag,
    /// where in . matches anything when s is set and matches anything
    /// except for new line when it is not set (the default).
    ///
    /// N.B. "matches anything" means "any byte" when Unicode is disabled
    /// and means "any valid UTF-8 encoding of any Unicode scalar value" when Unicode is enabled.
    pub fn dot_matches_new_line(&mut self, yes: bool) -> &mut Self {
        self.toggle(Flags::DOTALL, yes)
    }

    /// Set the value for the Unicode (u) flag.
    ///
    /// Enabled by default. When disabled, character classes such as `\w` only match ASCII word characters
    /// instead of all Unicode word characters.
    pub fn unicode(&mut self, yes: bool) -> &mut Self {
        self.toggle(Flags::UCP, yes)
    }
}
