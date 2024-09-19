//! Model for values in the WebAssembly syntax.

/// Names are sequences of characters, which are scalar values as defined by Unicode (Section 2.4).
/// Due to the limitations of the binary format,
/// the length of a name is bounded by the length of its UTF-8 encoding.
///
/// See <https://webassembly.github.io/spec/core/syntax/values.html#names>
///
/// # Examples
/// ```rust
/// use wasm_ast::Name;
///
/// let text = "test";
/// let name = Name::new(String::from(text));
///
/// assert_eq!(name, Name::from(text));
/// assert_eq!(name, Name::from(text.to_string()));
/// assert_eq!(name.as_bytes(), text.as_bytes());
/// assert_eq!(name.len(), text.len());
/// assert_eq!(name.is_empty(), false);
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Name {
    value: String,
}

impl Name {
    /// Creates a new name with the given Unicode text.
    pub fn new(value: String) -> Self {
        Name { value }
    }

    /// Returns a byte slice of this `Name`’s contents.
    pub fn as_bytes(&self) -> &[u8] {
        self.value.as_bytes()
    }

    /// Returns the length of this `Name`, in bytes, not chars or graphemes.
    /// In other words, it may not be what a human considers the length of the name.
    pub fn len(&self) -> usize {
        self.value.len()
    }

    /// Returns true if this `Name` has a length of zero, false otherwise.
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }
}

impl From<&str> for Name {
    fn from(name: &str) -> Self {
        Name {
            value: name.to_string(),
        }
    }
}

impl From<String> for Name {
    fn from(name: String) -> Self {
        Name { value: name }
    }
}
