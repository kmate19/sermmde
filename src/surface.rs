use std::io::Read;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Negative size encountered where positive expected")]
    NegativeSize,
}

type Result<T> = std::result::Result<T, Error>;

pub struct Surfaces {
    inner: Vec<Surface>,
}

impl Surfaces {
    pub fn parse(reader: &mut impl Read) -> Result<Self> {
        let mut size_bytes = [0; 4];

        reader.read_exact(&mut size_bytes);

        let size = i32::from_le_bytes(size_bytes);

        if size.is_negative() {
            Err(Error::NegativeSize)?
        }

        let size = size as usize;

        let mut inner_vec = Vec::with_capacity(size);

        for _ in 0..size {
            let surf = Surface::parse(reader)?;
            inner_vec.push(surf);
        }

        Ok(Self { inner: inner_vec })
    }
}

pub struct Surface {}

impl Surface {
    pub fn parse(reader: &mut impl Read) -> Result<Self> {
        Ok(Self {})
    }
}
