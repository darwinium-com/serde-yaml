use crate::libyaml::cstr::CStr;
use crate::libyaml::error::{Error, Mark, Result};
use crate::libyaml::tag::Tag;
use crate::libyaml::util::Owned;
use std::borrow::Cow;
use std::mem::MaybeUninit;
use std::ptr::{addr_of_mut, NonNull};
use std::slice;
use unsafe_libyaml as sys;

pub(crate) struct Parser<'input> {
    pin: Owned<ParserPinned<'input>>,
}

struct ParserPinned<'input> {
    sys: sys::yaml_parser_t,
    input: Cow<'input, [u8]>,
}

pub(crate) enum Event {
    StreamStart,
    StreamEnd,
    DocumentStart,
    DocumentEnd,
    Alias(Anchor),
    Scalar(Scalar),
    SequenceStart(SequenceStart),
    SequenceEnd,
    MappingStart(MappingStart),
    MappingEnd,
}

pub(crate) struct Scalar {
    pub anchor: Option<Anchor>,
    pub tag: Option<Tag>,
    pub value: Box<[u8]>,
    pub style: ScalarStyle,
}

pub(crate) struct SequenceStart {
    pub anchor: Option<Anchor>,
}

pub(crate) struct MappingStart {
    pub anchor: Option<Anchor>,
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct Anchor(Box<[u8]>);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum ScalarStyle {
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
}

impl<'input> Parser<'input> {
    pub fn new(input: Cow<'input, [u8]>) -> Parser<'input> {
        let owned = Owned::<ParserPinned>::new_uninit();
        let pin = unsafe {
            let parser = addr_of_mut!((*owned.ptr).sys);
            if sys::yaml_parser_initialize(parser) == 0 {
                panic!("malloc error: {}", Error::parse_error(parser));
            }
            sys::yaml_parser_set_encoding(parser, sys::YAML_UTF8_ENCODING);
            sys::yaml_parser_set_input_string(parser, input.as_ptr(), input.len() as u64);
            addr_of_mut!((*owned.ptr).input).write(input);
            Owned::assume_init(owned)
        };
        Parser { pin }
    }

    pub fn next(&mut self) -> Result<(Event, Mark)> {
        let mut event = MaybeUninit::<sys::yaml_event_t>::uninit();
        unsafe {
            let parser = addr_of_mut!((*self.pin.ptr).sys);
            let event = event.as_mut_ptr();
            if sys::yaml_parser_parse(parser, event) == 0 {
                return Err(Error::parse_error(parser));
            }
            let ret = convert_event(&*event);
            let mark = Mark {
                sys: (*event).start_mark,
            };
            sys::yaml_event_delete(event);
            Ok((ret, mark))
        }
    }
}

unsafe fn convert_event(sys: &sys::yaml_event_t) -> Event {
    match sys.type_ {
        sys::YAML_STREAM_START_EVENT => Event::StreamStart,
        sys::YAML_STREAM_END_EVENT => Event::StreamEnd,
        sys::YAML_DOCUMENT_START_EVENT => Event::DocumentStart,
        sys::YAML_DOCUMENT_END_EVENT => Event::DocumentEnd,
        sys::YAML_ALIAS_EVENT => Event::Alias(optional_anchor(sys.data.alias.anchor).unwrap()),
        sys::YAML_SCALAR_EVENT => Event::Scalar(Scalar {
            anchor: optional_anchor(sys.data.scalar.anchor),
            tag: optional_tag(sys.data.scalar.tag),
            value: Box::from(slice::from_raw_parts(
                sys.data.scalar.value,
                sys.data.scalar.length as usize,
            )),
            style: match sys.data.scalar.style {
                sys::YAML_PLAIN_SCALAR_STYLE => ScalarStyle::Plain,
                sys::YAML_SINGLE_QUOTED_SCALAR_STYLE => ScalarStyle::SingleQuoted,
                sys::YAML_DOUBLE_QUOTED_SCALAR_STYLE => ScalarStyle::DoubleQuoted,
                sys::YAML_LITERAL_SCALAR_STYLE => ScalarStyle::Literal,
                sys::YAML_FOLDED_SCALAR_STYLE => ScalarStyle::Folded,
                sys::YAML_ANY_SCALAR_STYLE | _ => unreachable!(),
            },
        }),
        sys::YAML_SEQUENCE_START_EVENT => Event::SequenceStart(SequenceStart {
            anchor: optional_anchor(sys.data.sequence_start.anchor),
        }),
        sys::YAML_SEQUENCE_END_EVENT => Event::SequenceEnd,
        sys::YAML_MAPPING_START_EVENT => Event::MappingStart(MappingStart {
            anchor: optional_anchor(sys.data.mapping_start.anchor),
        }),
        sys::YAML_MAPPING_END_EVENT => Event::MappingEnd,
        sys::YAML_NO_EVENT => unreachable!(),
        _ => unimplemented!(),
    }
}

unsafe fn optional_anchor(anchor: *const u8) -> Option<Anchor> {
    let ptr = NonNull::new(anchor as *mut i8)?;
    let cstr = CStr::from_ptr(ptr);
    Some(Anchor(Box::from(cstr.to_bytes())))
}

unsafe fn optional_tag(tag: *const u8) -> Option<Tag> {
    let ptr = NonNull::new(tag as *mut i8)?;
    let cstr = CStr::from_ptr(ptr);
    Some(Tag(Box::from(cstr.to_bytes())))
}

impl<'input> Drop for ParserPinned<'input> {
    fn drop(&mut self) {
        unsafe { sys::yaml_parser_delete(&mut self.sys) }
    }
}
