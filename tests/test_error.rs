use indoc::indoc;
use serde_derive::Deserialize;
use serde_yaml::Deserializer;
use std::fmt::Debug;

fn test_error<T>(yaml: &str, expected: &str)
where
    T: serde::de::DeserializeOwned + Debug,
{
    let result = serde_yaml::from_str::<T>(yaml);
    assert_eq!(expected, result.unwrap_err().to_string());
}

#[test]
fn test_incorrect_type() {
    let yaml = indoc! {"
        ---
        str
    "};
    let expected = "invalid type: string \"str\", expected i16 at line 2 column 1";
    test_error::<i16>(yaml, expected);
}

#[test]
fn test_incorrect_nested_type() {
    #[derive(Deserialize, Debug)]
    struct A {
        #[allow(dead_code)]
        b: Vec<B>,
    }
    #[derive(Deserialize, Debug)]
    enum B {
        C(C),
    }
    #[derive(Deserialize, Debug)]
    struct C {
        #[allow(dead_code)]
        d: bool,
    }
    let yaml = indoc! {"
        b:
          - C:
              d: fase
    "};
    let expected =
        "b[0].C.d: invalid type: string \"fase\", expected a boolean at line 3 column 10";
    test_error::<A>(yaml, expected);
}

#[test]
fn test_empty() {
    let expected = "EOF while parsing a value";
    test_error::<String>("", expected);
}

#[test]
fn test_missing_field() {
    #[derive(Deserialize, Debug)]
    struct Basic {
        #[allow(dead_code)]
        v: bool,
        #[allow(dead_code)]
        w: bool,
    }
    let yaml = indoc! {"
        ---
        v: true
    "};
    let expected = "missing field `w` at line 2 column 1";
    test_error::<Basic>(yaml, expected);
}

#[test]
fn test_unknown_anchor() {
    let yaml = indoc! {"
        ---
        *some
    "};
    let expected = "unknown anchor at line 2 column 1";
    test_error::<String>(yaml, expected);
}

#[test]
fn test_ignored_unknown_anchor() {
    #[derive(Deserialize, Debug)]
    struct Wrapper {
        #[allow(dead_code)]
        c: (),
    }
    let yaml = indoc! {"
        b: [*a]
        c: ~
    "};
    let expected = "unknown anchor at line 1 column 5";
    test_error::<Wrapper>(yaml, expected);
}

#[test]
fn test_two_documents() {
    let yaml = indoc! {"
        ---
        0
        ---
        1
    "};
    let expected = "deserializing from YAML containing more than one document is not supported";
    test_error::<usize>(yaml, expected);
}

#[test]
fn test_second_document_syntax_error() {
    let yaml = indoc! {"
        ---
        0
        ---
        ]
    "};

    let mut de = Deserializer::from_str(yaml);
    let first_doc = de.next().unwrap();
    let result = <usize as serde::Deserialize>::deserialize(first_doc);
    assert_eq!(0, result.unwrap());

    let second_doc = de.next().unwrap();
    let result = <usize as serde::Deserialize>::deserialize(second_doc);
    let expected = "did not find expected node content at line 4 column 1, while parsing a block node at line 4 column 1";
    assert_eq!(expected, result.unwrap_err().to_string());
}

#[test]
fn test_variant_map_wrong_size() {
    #[derive(Deserialize, Debug)]
    enum E {
        V(usize),
    }
    let yaml = indoc! {r#"
        "V": 16
        "other": 32
    "#};
    let expected = "invalid length 2, expected map containing 1 entry";
    test_error::<E>(yaml, expected);
}

#[test]
fn test_variant_not_a_map() {
    #[derive(Deserialize, Debug)]
    enum E {
        V(usize),
    }
    let yaml = indoc! {r#"
        ---
        - "V"
    "#};
    let expected = "invalid type: sequence, expected string or singleton map at line 2 column 1";
    test_error::<E>(yaml, expected);
}

#[test]
fn test_variant_not_string() {
    #[derive(Deserialize, Debug)]
    enum E {
        V(bool),
    }
    let yaml = indoc! {r#"
        ---
        {}: true
    "#};
    let expected = "invalid type: map, expected variant of enum `E` at line 2 column 1";
    test_error::<E>(yaml, expected);
}

#[test]
fn test_bad_bool() {
    let yaml = indoc! {"
        ---
        !!bool str
    "};
    let expected = "invalid value: string \"str\", expected a boolean at line 2 column 1";
    test_error::<bool>(yaml, expected);
}

#[test]
fn test_bad_int() {
    let yaml = indoc! {"
        ---
        !!int str
    "};
    let expected = "invalid value: string \"str\", expected an integer at line 2 column 1";
    test_error::<i64>(yaml, expected);
}

#[test]
fn test_bad_float() {
    let yaml = indoc! {"
        ---
        !!float str
    "};
    let expected = "invalid value: string \"str\", expected a float at line 2 column 1";
    test_error::<f64>(yaml, expected);
}

#[test]
fn test_bad_null() {
    let yaml = indoc! {"
        ---
        !!null str
    "};
    let expected = "invalid value: string \"str\", expected null at line 2 column 1";
    test_error::<()>(yaml, expected);
}

#[test]
fn test_short_tuple() {
    let yaml = indoc! {"
        ---
        [0, 0]
    "};
    let expected = "invalid length 2, expected a tuple of size 3 at line 2 column 1";
    test_error::<(u8, u8, u8)>(yaml, expected);
}

#[test]
fn test_long_tuple() {
    let yaml = indoc! {"
        ---
        [0, 0, 0]
    "};
    let expected = "invalid length 3, expected sequence of 2 elements at line 2 column 1";
    test_error::<(u8, u8)>(yaml, expected);
}

#[test]
fn test_invalid_scalar_type() {
    #[derive(Deserialize, Debug)]
    struct S {
        #[allow(dead_code)]
        x: [(); 1],
    }

    let yaml = "x:\n";
    let expected = "x: invalid type: unit value, expected an array of length 1 at line 1 column 3";
    test_error::<S>(yaml, expected);
}

#[cfg(not(miri))]
#[test]
fn test_infinite_recursion_objects() {
    #[derive(Deserialize, Debug)]
    struct S {
        #[allow(dead_code)]
        x: Option<Box<S>>,
    }

    let yaml = "&a {'x': *a}";
    let expected = "recursion limit exceeded at position 0";
    test_error::<S>(yaml, expected);
}

#[cfg(not(miri))]
#[test]
fn test_infinite_recursion_arrays() {
    #[derive(Deserialize, Debug)]
    struct S {
        #[allow(dead_code)]
        x: Option<Box<S>>,
    }

    let yaml = "&a [*a]";
    let expected = "recursion limit exceeded at position 0";
    test_error::<S>(yaml, expected);
}

#[cfg(not(miri))]
#[test]
fn test_finite_recursion_objects() {
    #[derive(Deserialize, Debug)]
    struct S {
        #[allow(dead_code)]
        x: Option<Box<S>>,
    }

    let yaml = "{'x':".repeat(1_000) + &"}".repeat(1_000);
    let expected = "recursion limit exceeded at line 1 column 641";
    test_error::<S>(&yaml, expected);
}

#[cfg(not(miri))]
#[test]
fn test_finite_recursion_arrays() {
    #[derive(Deserialize, Debug)]
    struct S {
        #[allow(dead_code)]
        x: Option<Box<S>>,
    }

    let yaml = "[".repeat(1_000) + &"]".repeat(1_000);
    let expected = "recursion limit exceeded at line 1 column 129";
    test_error::<S>(&yaml, expected);
}
