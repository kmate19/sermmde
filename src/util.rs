use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid UTF-16LE encoding")]
    Utf16LeError,
    #[error("Invalid UTF-16 encoding {0}")]
    FromUtf16Error(#[from] std::string::FromUtf16Error),
    #[error("Failed to decode UTF-16 character {0}")]
    DecodeUtf16Error(#[from] std::char::DecodeUtf16Error),
}

type Result<T> = std::result::Result<T, Error>;

/// Converts a UTF-16LE encoded byte slice to a Rust String.
///
/// This is taken from the nightly standard library with slight modifications.
///
/// Source: https://doc.rust-lang.org/stable/std/string/struct.String.html#method.from_utf16le
pub(crate) fn from_utf16le(v: &[u8]) -> Result<String> {
    // This means that the slice length must be even
    let (chunks, []) = v.as_chunks::<2>() else {
        return Err(Error::Utf16LeError);
    };

    let res = match (cfg!(target_endian = "little"), unsafe {
        v.align_to::<u16>()
    }) {
        (true, ([], v, [])) => String::from_utf16(v)?,
        _ => char::decode_utf16(chunks.iter().copied().map(u16::from_le_bytes))
            .map(|r| r.map_err(Into::into))
            .collect::<Result<_>>()?,
    };

    Ok(res)
}
