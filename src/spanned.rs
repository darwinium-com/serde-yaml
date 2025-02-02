#![allow(missing_docs)]

use serde::{
    de::{Error, MapAccess},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{
    borrow::{Borrow, BorrowMut},
    fmt::{self, Formatter},
    marker::PhantomData,
};

pub(crate) const NAME: &str = "$__serde_private_Spanned";
pub(crate) const START: &str = "$__serde_private_start";
pub(crate) const LENGTH: &str = "$__serde_private_length";
pub(crate) const PATH: &str = "$__serde_private_path";
pub(crate) const VALUE: &str = "$__serde_private_value";

pub(crate) const FIELDS: &[&str] = &[START, LENGTH, PATH, VALUE];

/// An wrapper which records the location of an item as byte indices into the
/// source text.
///
/// # Examples
///
/// Primitive values can be wrapped with [`Spanned<T>`] to get their location,
/// as you may expect.
///
/// ```rust
/// # use serde_yaml::Spanned;
/// #[derive(Debug, serde_derive::Deserialize)]
/// struct Document {
///     name: Spanned<String>,
/// }
/// # fn main() -> Result<(), serde_yaml::Error> {
///
/// let yaml = "name: Document";
///
/// let doc: Document = serde_yaml::from_str(yaml)?;
///
/// assert_eq!(doc.name.value, "Document");
/// assert_eq!(doc.name.start, 6);
/// assert_eq!(doc.name.len, "Document".len());
/// # Ok(())
/// # }
/// ```
///
/// More complex items like maps and arrays can also be used.
///
/// ```rust
/// # use serde_yaml::Spanned;
/// #[derive(Debug, serde_derive::Deserialize)]
/// struct Document {
///     words: Spanned<Vec<String>>,
/// }
/// # fn main() -> Result<(), serde_yaml::Error> {
///
/// let yaml = "words: [Hello, World]";
///
/// let doc: Document = serde_yaml::from_str(yaml)?;
///
/// assert_eq!(doc.words.value, &["Hello", "World"]);
/// assert_eq!(doc.words.start, yaml.find("[").unwrap());
/// assert_eq!(doc.words.len, "[Hello, World]".len());
/// assert_eq!(doc.words.end(), yaml.find("]").unwrap());
/// # Ok(())
/// # }
/// ```
///
/// Note that a map item starts after first key.
///
/// ```rust
/// # use serde_yaml::Spanned;
/// #[derive(Debug, serde_derive::Deserialize)]
/// struct Document {
///     nested: Spanned<Nested>,
/// }
/// #[derive(Debug, serde_derive::Deserialize)]
/// struct Nested {
///    first: u32,
///    second: u32,
/// }
/// # fn main() -> Result<(), serde_yaml::Error> {
///
/// let yaml = "nested:\n  first: 1\n  second: 2";
/// let doc: Document = serde_yaml::from_str(yaml)?;
///
/// let spanned_text = &yaml[doc.nested.span()];
/// assert_eq!(spanned_text, ": 1\n  second: 2");
/// # Ok(())
/// # }
/// ```
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Spanned<T> {
    pub value: T,
    pub start: usize,
    pub path: String,
    pub len: usize,
}

impl<T> Spanned<T> {
    pub const fn new(start: usize, len: usize, path: String, value: T) -> Self {
        Spanned { value, start, len, path }
    }

    /// The value's location in source as an inclusive range.
    pub const fn span(&self) -> std::ops::Range<usize> {
        self.start..(self.start + self.len)
    }

    pub const fn end(&self) -> usize {
        self.start + self.len - 1
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T, Q> AsRef<Q> for Spanned<T>
where
    T: AsRef<Q>,
{
    fn as_ref(&self) -> &Q {
        self.value.as_ref()
    }
}

impl<T, Q> AsMut<Q> for Spanned<T>
where
    T: AsMut<Q>,
{
    fn as_mut(&mut self) -> &mut Q {
        self.value.as_mut()
    }
}

impl<T> Borrow<T> for Spanned<T> {
    fn borrow(&self) -> &T {
        &self.value
    }
}

impl<T> BorrowMut<T> for Spanned<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T: Serialize> Serialize for Spanned<T> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(ser)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Spanned<T> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_struct(NAME, FIELDS, Visitor(PhantomData))
    }
}

struct Visitor<T>(PhantomData<T>);

impl<'de, T> serde::de::Visitor<'de> for Visitor<T>
where
    T: Deserialize<'de>,
{
    type Value = Spanned<T>;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "A spanned {}", core::any::type_name::<T>())
    }

    fn visit_map<A>(self, mut visitor: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        if visitor.next_key()? != Some(START) {
            return Err(Error::custom("spanned start key not found"));
        }

        let start: usize = visitor.next_value()?;

        if visitor.next_key()? != Some(VALUE) {
            return Err(Error::custom("spanned value key not found"));
        }

        let value: T = visitor.next_value()?;

        if visitor.next_key()? != Some(LENGTH) {
            return Err(Error::custom("spanned length key not found"));
        }

        let length: usize = visitor.next_value()?;

        if visitor.next_key()? != Some(PATH) {
            return Err(Error::custom("spanned length key not found"));
        }

        let path: String = visitor.next_value()?;

        Ok(Spanned::new(start, length, path, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializing_a_spanned_t_is_a_noop() {
        let value = Spanned::new(0, 0, String::from(".[0]"), "Hello, World!");
        let should_be = r#"---
"Hello, World!"
"#;

        let got = crate::to_string(&value).unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn deserialize_spanned_item() {
        let src = "42";
        let should_be = Spanned::new(0, src.len(), String::from("."), 42);

        let got: Spanned<i32> = crate::from_str(src).unwrap();

        assert_eq!(got, should_be);
    }

    #[test]
    fn deserialize_sequence() {
        let src = " [1, 22, 333]";
        let should_be = Spanned::new(
            1,
            src.len() - 1,
            String::from("."),
            vec![
                Spanned::new(2, 1, String::from(".[0]"), 1),
                Spanned::new(5, 2, String::from(".[1]"), 22),
                Spanned::new(9, 3, String::from(".[2]"), 333),
            ],
        );

        let got: Spanned<Vec<Spanned<i32>>> = crate::from_str(src).unwrap();

        println!("{:?}", &got.value[0]);

        assert_eq!("1", &src[got.value[0].span()]);
        assert_eq!("22", &src[got.value[1].span()]);
        assert_eq!("333", &src[got.value[2].span()]);
        assert_eq!("[1, 22, 333]", &src[got.span()]);
        assert_eq!(got, should_be);
    }

    #[test]
    fn deserialize_nested() {
        #[derive(Debug, PartialEq, serde_derive::Deserialize)]
        struct Item {
            name: Spanned<String>,
            values: Spanned<Vec<Spanned<i32>>>,
        }

        let src = r#"
- name: first
  values: [1, 2, 3]
- name: second
  values: [4, 5]"#;
        let items = vec![
            Spanned::new(
                7,
                59,
                String::from(".[0]"),
                Item {
                    name: Spanned::new(9, 5, String::from(".[0].name"), String::from("first")),
                    values: Spanned::new(
                        25,
                        9,
                        String::from(".[0].values"),
                        vec![
                            Spanned::new(26, 1, String::from(".[0].values[0]"), 1),
                            Spanned::new(29, 1, String::from(".[0].values[1]"), 2),
                            Spanned::new(32, 1, String::from(".[0].values[2]"), 3),
                        ],
                    ),
                },
            ),
            Spanned::new(
                41,
                23,
                String::from(".[1]"),
                Item {
                    name: Spanned::new(43, 6, String::from(".[1].name"), String::from("second")),
                    values: Spanned::new(
                        60,
                        6,
                        String::from(".[1].values"),
                        vec![Spanned::new(61, 1, String::from(".[1].values[0]"), 4), Spanned::new(64, 1, String::from(".[1].values[1]"), 5)],
                    ),
                },
            ),
        ];
        let should_be = Spanned::new(1, src.len(), String::from("."), items);

        let got: Spanned<Vec<Spanned<Item>>> = crate::from_str(src).unwrap();

        let five_from_second_value = &got.value[1].value.values.value[1];
        assert_eq!("5", &src[five_from_second_value.span()]);
        let second_values = &got.value[1].value.values;
        assert_eq!("[4, 5]", &src[second_values.span()]);

        assert_eq!(got, should_be);
    }

    #[test]
    fn deserialize_map_at_end_of_stream() {
        #[derive(Debug, PartialEq, serde_derive::Deserialize)]
        struct Document {
            nested: Spanned<Nested>,
        }
        #[derive(Debug, PartialEq, serde_derive::Deserialize)]
        struct Nested {
            value: String,
        }

        let src = "nested:\n  value: Hello, World!";
        let should_be = Document {
            nested: Spanned {
                start: src.rfind(":").unwrap(),
                len: ": Hello, World!".len(),
                value: Nested {
                    value: String::from("Hello, World!"),
                },
                path: String::from("nested"),
            },
        };

        let got: Document = crate::from_str(src).unwrap();

        assert_eq!(got.nested.end(), src.len() - 1);
        assert_eq!(got, should_be);
    }

    #[test]
    fn empty_map() {
        #[derive(Debug, PartialEq, serde_derive::Deserialize)]
        struct Document {
            nested: Spanned<Nested>,
        }
        #[derive(Debug, Default, PartialEq, serde_derive::Deserialize)]
        struct Nested {
            #[serde(default)]
            value: String,
        }

        let src = "nested: {}";
        let should_be = Document {
            nested: Spanned {
                start: 8,
                len: 2,
                value: Nested {
                    value: String::new(),
                },
                path: String::from("nested"),
            },
        };

        let got: Document = crate::from_str(src).unwrap();

        assert_eq!(got, should_be);
        let spanned_text = &src[got.nested.span()];
        assert_eq!(spanned_text, "{}");
    }
}
