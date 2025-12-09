use core::fmt;
use std::io::Read;

use thiserror::Error;

use crate::util::from_utf16le;

// PMX Types
// Name	Size (bytes)	Structure	Notes
// vec2	8	float, float	XY vector
// vec3	12	float, float, float	XYZ vector
// vec4	16	float, float, float, float	XYZW vector
// text	4 + length	int, byte[]	Byte sequence encoding defined in Globals
// flag	1	byte	8 flags per byte. 0 = false, 1 = true
// index	1/2/4	byte/ubyte/short/ushort/int	Type defined in file header, sign depends on usage

/// Errors that can occur when dealing with PMX types.
#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Util(#[from] crate::util::Error),
    #[error("The length of the string was negative")]
    NegativeLength,
    #[error("Invalid text encoding")]
    InvalidTextEncoding,
    #[error(transparent)]
    FromUtf8(#[from] std::str::Utf8Error),
    #[error("Index size mismatch")]
    IndexSizeMismatch,
}

type Result<T> = std::result::Result<T, Error>;

/// A bitflag structure used in various parts of the PMX format.
/// 8 flags per byte. 0 = off, 1 = on.
// TODO(mate): consider using bitflags crate
#[derive(Debug)]
pub struct Flag {
    raw: u8,
}

impl Flag {
    /// Get the state of a specific bit in the flag.
    ///
    /// Note that this uses 0-based indexing and only supports bits 0-7.
    ///
    /// If `bit` is out of range, this will panic in debug builds, and wraps in release builds.
    /// You could try the `try_get_state` method if you want to be sure that the bit is valid.
    pub fn get_state(&self, bit: u8) -> bool {
        debug_assert!(bit < 8, "Bit index must be 0-7, got {}", bit);

        (self.raw & (1 << bit)) != 0
    }

    /// Get the state of a specific bit in the flag.
    ///
    /// Returns None if the bit is out of range (not 0-7).
    pub fn try_get_state(&self, bit: u8) -> Option<bool> {
        if bit >= 8 {
            return None;
        }

        Some((self.raw & (1 << bit)) != 0)
    }

    pub fn parse(reader: &mut impl Read) -> Result<Self> {
        let mut bytes = [0; 1];
        reader.read_exact(&mut bytes)?;
        Ok(Self { raw: bytes[0] })
    }
}

/// The text encoding used in the PMX file.
///
/// Defined in the PMX file header.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TextEncoding {
    UTF16LE,
    UTF8,
}

impl TryFrom<u8> for TextEncoding {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::UTF16LE),
            1 => Ok(Self::UTF8),
            _ => Err(Error::InvalidTextEncoding),
        }
    }
}

/// A PMX text string, encoded in either UTF18LE or UTF8, specified by the file's global variables.
pub struct PmxText {
    // TODO(mate): keep the rawy bytes for now, but maybe we can drop them later
    raw_bytes: Vec<u8>,
    // TODO(mate): this is also sort of useless as its in the file header and always the same for every text anyways
    encoding: TextEncoding,
    decoded: String,
}

impl fmt::Debug for PmxText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PmxText")
            .field("decoded", &self.decoded)
            .field("encoding", &self.encoding)
            .finish()
    }
}

impl fmt::Display for PmxText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.decoded)
    }
}

impl PmxText {
    /// Reads a PMX text string from the given reader and an encoding.
    ///
    /// Returns an error if the length is negative or if there was an IO error.
    ///
    /// Decoding the string is lazy and done when `try_into_string` is called.
    pub fn from_bytes(reader: &mut impl Read, encoding: TextEncoding) -> Result<Self> {
        let mut len = [0; 4];

        reader.read_exact(&mut len)?;

        // this is an i32 for some reason as per "spec"
        let len = i32::from_le_bytes(len);

        if len.is_negative() {
            Err(Error::NegativeLength)?
        }

        let len = len as usize;

        let mut raw_bytes = vec![0; len];

        reader.read_exact(&mut raw_bytes)?;

        let decoded = match encoding {
            TextEncoding::UTF8 => {
                // convert to &str first to validate UTF-8
                // so if it's invalid we have not cloned yet
                let str = str::from_utf8(&raw_bytes)?;
                str.to_string()
            }
            TextEncoding::UTF16LE => from_utf16le(&raw_bytes)?,
        };

        Ok(Self {
            raw_bytes,
            encoding,
            decoded,
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub enum IndexSize {
    Size1([u8; 1]),
    Size2([u8; 2]),
    Size4([u8; 4]),
}

impl TryFrom<u8> for IndexSize {
    type Error = Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Size1([0; 1])),
            2 => Ok(Self::Size2([0; 2])),
            4 => Ok(Self::Size4([0; 4])),
            _ => Err(Error::IndexSizeMismatch),
        }
    }
}

#[derive(Debug)]
pub struct Index {
    size: IndexSize,
    sign: bool,
    value: i32,
}

impl Index {
    pub fn parse(reader: &mut impl Read, mut size: IndexSize, sign: bool) -> Result<Self> {
        // read data into the index
        match &mut size {
            IndexSize::Size1(raw) => reader.read_exact(raw)?,
            IndexSize::Size2(raw) => reader.read_exact(raw)?,
            IndexSize::Size4(raw) => reader.read_exact(raw)?,
        };

        let value = match &size {
            IndexSize::Size1(raw) => {
                if sign {
                    i8::from_le_bytes(*raw) as i32
                } else {
                    u8::from_le_bytes(*raw) as i32
                }
            }
            IndexSize::Size2(raw) => {
                if sign {
                    i16::from_le_bytes(*raw) as i32
                } else {
                    u16::from_le_bytes(*raw) as i32
                }
            }
            IndexSize::Size4(raw) => i32::from_le_bytes(*raw),
        };

        Ok(Self { size, value, sign })
    }

    pub fn is_nil(&self) -> bool {
        self.value == -1
    }
}

#[cfg(not(feature = "math_glam"))]
pub type Vec2 = [f32; 2];
#[cfg(not(feature = "math_glam"))]
pub type Vec3 = [f32; 3];
#[cfg(not(feature = "math_glam"))]
pub type Vec4 = [f32; 4];

#[cfg(feature = "math_glam")]
pub use glam::{Vec2, Vec3, Vec4};

macro_rules! vec_from_bytes {
    ($t:ty,$reader:ident) => {{
        const SIZE: usize = std::mem::size_of::<$t>();
        const COUNT: usize = SIZE / 4;
        let mut bytes = [0; SIZE];

        $reader.read_exact(&mut bytes)?;

        let chunks = bytes.as_chunks::<4>().0;

        let floats: [f32; COUNT] = std::array::from_fn(|i| f32::from_le_bytes(chunks[i]));

        floats.into()
    }};
}
pub(super) use vec_from_bytes;
