//! Object names as game characteristics (CR 201, 111.4): a literal name, or the ordered subtype words an unnamed
//! token was created with. Creating text is frozen here; later subtype changes do not rename that token.

use std::{borrow::Cow, fmt, hash::{Hash, Hasher}};

use crate::{Subtype, Words};

#[derive(Clone, Copy, Debug)]
pub enum ObjectName {
    Literal(&'static str),
    Token { subtypes: &'static [Subtype], words: Words },
}

impl ObjectName {
    pub fn text(self) -> Cow<'static, str> {
        match self {
            Self::Literal(name) => Cow::Borrowed(name),
            Self::Token { .. } => Cow::Owned(self.to_string()),
        }
    }

    pub fn eq_ignore_ascii_case(self, name: &str) -> bool {
        self.text().eq_ignore_ascii_case(name)
    }
}

impl fmt::Display for ObjectName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Literal(name) => f.write_str(name),
            Self::Token { subtypes, words } => {
                for subtype in subtypes { write!(f, "{} ", words.subtype(*subtype).name())?; }
                f.write_str("Token")
            }
        }
    }
}

impl PartialEq for ObjectName {
    fn eq(&self, other: &Self) -> bool { self.text() == other.text() }
}
impl Eq for ObjectName {}
impl Hash for ObjectName {
    fn hash<H: Hasher>(&self, state: &mut H) { self.text().hash(state); }
}
impl PartialEq<&str> for ObjectName {
    fn eq(&self, other: &&str) -> bool { self.text() == *other }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Word;
    use std::collections::hash_map::DefaultHasher;

    #[test]
    fn ordered_words_suffix_and_literal_names_have_the_same_identity() {
        let name = ObjectName::Token {
            subtypes: &[Subtype::Forest, Subtype::Tentacle],
            words: Words::NONE.changed(Word::Subtype(Subtype::Tentacle), Word::Subtype(Subtype::Goblin)),
        };
        assert_eq!(name.to_string(), "Forest Goblin Token");
        let literal = ObjectName::Literal("Forest Goblin Token");
        assert_eq!(name, literal);
        assert_eq!(name, "Forest Goblin Token");
        let hash = |name: ObjectName| { let mut h = DefaultHasher::new(); name.hash(&mut h); h.finish() };
        assert_eq!(hash(name), hash(literal));
        assert!(name.eq_ignore_ascii_case("forest goblin token"));
    }

    #[test]
    fn literal_names_are_borrowed_and_an_unnamed_subtypeless_token_is_token() {
        assert!(matches!(ObjectName::Literal("Forest Tentacle Token").text(), Cow::Borrowed(_)));
        assert_eq!(ObjectName::Token { subtypes: &[], words: Words::NONE }, "Token");
    }
}
