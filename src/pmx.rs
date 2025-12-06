use core::fmt;
use std::{
    io::{BufReader, Read},
    path::Path,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("File had an invalid tag")]
    InvalidTag,
    #[error("Unknown encoding")]
    InvalidEncoding,
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Failed to parse text as utf16")]
    Utf16Error,
    #[error("Failed to decode utf16 char")]
    DecodeUtf16(#[from] std::char::DecodeUtf16Error),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Pmx {
    header: Header,
}

impl Pmx {
    pub fn open(path: &Path) -> Result<Self> {
        let fh = std::fs::File::open(path)?;

        let mut reader = BufReader::new(fh);

        let header = Header::parse(&mut reader)?;

        Ok(Pmx { header })
    }
}

pub struct PmdText {
    len: i32,
    raw_bytes: Vec<u8>,
}

impl fmt::Debug for PmdText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = self
            .try_into_string()
            .unwrap_or("<Unable to parse string>".into());

        f.debug_struct("PmdText")
            .field("len", &self.len)
            .field("contents", &string)
            .finish()
    }
}

impl PmdText {
    pub fn from_bytes(reader: &mut impl Read) -> Result<Self> {
        let mut len = [0; 4];

        reader.read_exact(&mut len)?;

        // this is an i32 for some reason as per "spec"
        let len = i32::from_le_bytes(len);

        if len.is_negative() {
            // TODO(mate): make better error types
            Err(Error::InvalidEncoding)?
        }

        let mut raw_bytes = vec![0; len as _];

        reader.read_exact(&mut raw_bytes)?;

        Ok(Self { len, raw_bytes })
    }

    pub fn try_into_string(&self) -> Result<String> {
        from_utf16le(&self.raw_bytes)
    }
}

fn from_utf16le(v: &[u8]) -> Result<String> {
    let (chunks, []) = v.as_chunks::<2>() else {
        return Err(Error::Utf16Error);
    };

    let res = match (cfg!(target_endian = "little"), unsafe {
        v.align_to::<u16>()
    }) {
        (true, ([], v, [])) => String::from_utf16(v).map_err(|_| Error::Utf16Error)?,
        // TODO(mate): don't discard error info here
        _ => char::decode_utf16(chunks.iter().copied().map(u16::from_le_bytes))
            .map(|u| u.map_err(|_| Error::Utf16Error))
            .collect::<Result<_>>()?,
    };

    Ok(res)
}

#[derive(Debug)]
pub struct Header {
    version: f32,
    globals: Globals,
    name: ModelName,
    comment: Comment,
}

#[derive(Debug)]
pub struct ModelName {
    pub local: PmdText,
    pub universal: PmdText,
}

#[derive(Debug)]
pub struct Comment {
    pub local: PmdText,
    pub universal: PmdText,
}

impl Header {
    pub fn parse(r: &mut impl Read) -> Result<Self> {
        // 4 bytes since there's a space after
        let mut tag = [0; 4];

        r.read_exact(&mut tag)?;

        if &tag[..3] != b"PMX" {
            Err(Error::InvalidTag)?
        }

        let mut ver = [0; 4];

        r.read_exact(&mut ver)?;

        let version = f32::from_le_bytes(ver);

        let globals = Globals::parse(r)?;

        let local_name = PmdText::from_bytes(r)?;

        let universal_name = PmdText::from_bytes(r)?;

        let name = ModelName {
            local: local_name,
            universal: universal_name,
        };

        let local_comment = PmdText::from_bytes(r)?;

        let universal_comment = PmdText::from_bytes(r)?;

        let comment = Comment {
            local: local_comment,
            universal: universal_comment,
        };

        Ok(Self {
            version,
            globals,
            name,
            comment,
        })
    }
}

#[derive(Debug)]
pub enum Encoding {
    UTF16LE,
    UTF8,
}

impl TryFrom<u8> for Encoding {
    type Error = Error;

    fn try_from(value: u8) -> Result<Encoding> {
        match value {
            0 => Ok(Encoding::UTF16LE),
            1 => Ok(Encoding::UTF8),
            _ => Err(Error::InvalidEncoding),
        }
    }
}

#[derive(Debug)]
pub struct Globals {
    encoding: Encoding,
    vec4_additional: u8,
    vert_idx_size: u8,
    tex_idx_size: u8,
    material_idx_size: u8,
    bone_idx_size: u8,
    morph_idx_size: u8,
    rb_idx_size: u8,
    /// Store additional fields here that we don't know the specific purpose of right now.
    additional: Option<Vec<u8>>,
}

impl Globals {
    pub fn parse(r: &mut impl Read) -> Result<Self> {
        let mut global_count = [0; 1];

        r.read_exact(&mut global_count)?;

        if global_count[0] < 8 {
            // TODO(mate): make better error types
            Err(Error::InvalidEncoding)?
        }

        let mut globals = vec![0; global_count[0] as _];

        r.read_exact(&mut globals)?;

        let additional = if global_count[0] > 8 {
            Some(globals.split_off(8))
        } else {
            None
        };

        Ok(Self {
            encoding: globals[0].try_into()?,
            vec4_additional: globals[1],
            vert_idx_size: globals[2],
            tex_idx_size: globals[3],
            material_idx_size: globals[4],
            bone_idx_size: globals[5],
            morph_idx_size: globals[6],
            rb_idx_size: globals[7],
            additional,
        })
    }
}
