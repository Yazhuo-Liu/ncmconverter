use crate::ncm_rc4::NcmRc4;
use anyhow::Result;
use std::{
    fmt::{Debug, Display},
    io::{Chain, Cursor, Read},
    iter::Cycle,
};

#[derive(Debug, Clone, Copy)]
pub enum Type {
    Flac,
    Mp3,
    M4a,
    Ogg,
    Unknown,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ext = match &self {
            Type::Flac => "flac",
            Type::Mp3 => "mp3",
            Type::M4a => "m4a",
            Type::Ogg => "ogg",
            Type::Unknown => "audio",
        };

        write!(f, "{}", ext)
    }
}

impl From<[u8; 12]> for Type {
    fn from(value: [u8; 12]) -> Self {
        match &value[..4] {
            b"fLaC" => Type::Flac,
            b"OggS" => Type::Ogg,
            header if is_mpeg_layer_iii_header(header) => Type::Mp3,
            [b'I', b'D', b'3', ..] => Type::Mp3,
            _ => {
                if &value[4..12] == b"ftypM4A " {
                    Type::M4a
                } else {
                    Type::Unknown
                }
            }
        }
    }
}

fn is_mpeg_layer_iii_header(header: &[u8]) -> bool {
    header.len() >= 4
        && header[0] == 0xFF
        && header[1] & 0xE0 == 0xE0
        // MPEG version 01 is reserved.
        && header[1] & 0x18 != 0x08
        // Layer bits 01 mean Layer III.
        && header[1] & 0x06 == 0x02
        // Free format and bad bitrate values cannot identify a normal audio frame.
        && header[2] & 0xF0 != 0
        && header[2] & 0xF0 != 0xF0
        // Sample rate index 11 is reserved.
        && header[2] & 0x0C != 0x0C
}

type Rc4Iter = Cycle<std::array::IntoIter<u8, 256_usize>>;

pub struct Audio<R>
where
    R: Read,
{
    r#type: Type,
    rc4_iter: Rc4Iter,
    reader: Chain<Cursor<[u8; 12]>, R>,
}

impl<R> Audio<R>
where
    R: Read,
{
    pub fn try_new(mut input: R, key: &[u8]) -> Result<Self> {
        let rc4_iter = NcmRc4::new(key).into_iter().cycle();

        let mut buf = [0; 12];
        input.read_exact(&mut buf)?;

        let reader = Cursor::new(buf).chain(input);

        let r#type = {
            let mut tmp_iter = rc4_iter.clone();
            Self::decrypt(&mut tmp_iter, &mut buf);
            buf.into()
        };

        Ok(Self { r#type, rc4_iter, reader })
    }

    pub fn r#type(&self) -> Type {
        self.r#type
    }

    pub fn ext(&self) -> String {
        self.r#type.to_string()
    }

    fn decrypt(rc4_iter: &mut Rc4Iter, buf: &mut [u8]) {
        buf.iter_mut().zip(rc4_iter).for_each(|(byte, x)| *byte ^= x);
    }
}

impl<R> Read for Audio<R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let size = self.reader.read(buf)?;
        Self::decrypt(&mut self.rc4_iter, &mut buf[..size]);
        Ok(size)
    }
}

impl<R> Debug for Audio<R>
where
    R: Read,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Audio").field(&format!("{}", self.r#type)).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::Type;

    #[test]
    fn detects_mpeg_layer_iii_variants() {
        assert!(matches!(Type::from([0xFF, 0xFB, 0x90, 0, 0, 0, 0, 0, 0, 0, 0, 0]), Type::Mp3));
        assert!(matches!(Type::from([0xFF, 0xF2, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0]), Type::Mp3));
        assert!(matches!(Type::from([0xFF, 0xE2, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0]), Type::Mp3));
        assert!(matches!(Type::from([0xFF, 0xFB, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]), Type::Unknown));
    }
}
