use crate::{
    audio::{Audio, Type as AudioType},
    image::Image,
    key::{decrypt_key, decrypt_meta},
};
use anyhow::{ensure, Ok, Result};
use base64::{engine::general_purpose::STANDARD as base64, Engine};
use std::{io::Read, vec};

const MAX_KEY_FRAME_LEN: usize = 1024 * 1024;
const MAX_COMMENT_FRAME_LEN: usize = 4 * 1024 * 1024;
const MAX_IMAGE_FRAME_LEN: usize = 32 * 1024 * 1024;
const MAX_COVER_OFFSET: usize = 64 * 1024 * 1024;
const SKIP_BUFFER_LEN: usize = 16 * 1024;

#[derive(Debug)]
pub struct Decoder<R>
where
    R: Read,
{
    pub key: Vec<u8>,
    pub comment: Vec<u8>,
    pub meta: Vec<u8>,
    pub image: Option<Image>,
    pub audio: Audio<R>,
}

impl<R> Decoder<R>
where
    R: Read,
{
    pub fn decode(mut input: R) -> Result<Self> {
        let mut buffer = [0; 10];
        input.read_exact(&mut buffer)?;

        ensure!(buffer.starts_with(b"CTENFDAM"), "CTENFDAM file header mismatch");

        let key = {
            let (mut key, _) = Self::read_frame(&mut input, MAX_KEY_FRAME_LEN, "key")?;

            let key = decrypt_key(&mut key)?;

            let key = key
                .strip_prefix(b"neteasecloudmusic")
                .ok_or_else(|| anyhow::anyhow!("invalid ncm key prefix"))?;
            ensure!(!key.is_empty(), "invalid empty ncm key");

            key.to_vec()
        };

        let comment = {
            let (mut comment, _) = Self::read_frame(&mut input, MAX_COMMENT_FRAME_LEN, "comment")?;
            if !comment.is_empty() {
                comment.iter_mut().for_each(|byte| *byte ^= 99);

                ensure!(comment.starts_with(b"163 key(Don't modify):"), "invalid comment prefix");
            }

            comment
        };

        let meta = if !comment.is_empty() {
            let meta = &comment[22..];
            let mut meta = base64.decode(meta)?;

            let meta = decrypt_meta(&mut meta)?;

            Self::parse_meta(meta)?
        } else {
            vec![]
        };

        Self::skip(&mut input, 5)?;

        let image = {
            let offset = usize::try_from(Self::read_len(&mut input)?)?;
            ensure!(offset <= MAX_COVER_OFFSET, "cover offset exceeds limit");

            let (image, img_len) = Self::read_frame(&mut input, MAX_IMAGE_FRAME_LEN, "image")?;
            let img_len = usize::try_from(img_len)?;

            ensure!(offset >= img_len, "invalid cover offset smaller than image length");

            if offset > img_len {
                Self::skip(&mut input, offset - img_len)?;
            }

            if img_len > 0 {
                ensure!(image.len() >= 12, "invalid image frame shorter than 12 bytes");
                Some(image.into())
            } else {
                None
            }
        };

        let audio = Audio::try_new(input, &key)?;

        Ok(Self { key, comment, meta, image, audio })
    }

    pub fn audio_type(&self) -> AudioType {
        self.audio.r#type()
    }

    pub fn ext(&self) -> String {
        self.audio.ext()
    }

    fn read_frame(input: &mut R, max_len: usize, frame_name: &str) -> Result<(Vec<u8>, u32)> {
        let len = Self::read_len(input)?;
        let len_usize = usize::try_from(len)?;
        ensure!(len_usize <= max_len, "{frame_name} frame exceeds {max_len} byte limit");

        if len > 0 {
            let mut data = vec![0; len_usize];
            input.read_exact(&mut data)?;
            Ok((data, len))
        } else {
            Ok((vec![], 0))
        }
    }

    fn read_len(input: &mut R) -> Result<u32> {
        let mut buffer = [0; 4];
        input.read_exact(&mut buffer)?;
        Ok(u32::from_le_bytes(buffer))
    }

    fn skip(input: &mut R, mut remaining: usize) -> Result<()> {
        let mut buffer = [0; SKIP_BUFFER_LEN];

        while remaining > 0 {
            let len = remaining.min(buffer.len());
            input.read_exact(&mut buffer[..len])?;
            remaining -= len;
        }

        Ok(())
    }

    fn parse_meta(meta: &[u8]) -> Result<Vec<u8>> {
        let meta =
            meta.strip_prefix(b"music:").ok_or_else(|| anyhow::anyhow!("invalid meta prefix"))?;
        Ok(meta.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::{Decoder, MAX_KEY_FRAME_LEN};
    use std::{io::Cursor, panic::AssertUnwindSafe};

    const FIXTURE: &[u8] = include_bytes!("../../ncmc/tests/input/1356233945.ncm");

    fn assert_decode_error_without_panic(data: Vec<u8>) {
        let result =
            std::panic::catch_unwind(AssertUnwindSafe(|| Decoder::decode(Cursor::new(data))));
        assert!(result.is_ok(), "decoder panicked");
        assert!(result.unwrap().is_err(), "malformed input decoded successfully");
    }

    #[test]
    fn rejects_an_oversized_frame_before_allocating() {
        let mut data = b"CTENFDAM\0\0".to_vec();
        data.extend_from_slice(&u32::try_from(MAX_KEY_FRAME_LEN + 1).unwrap().to_le_bytes());

        assert_decode_error_without_panic(data);
    }

    #[test]
    fn short_key_and_comment_frames_return_errors() {
        let mut short_key = FIXTURE.to_vec();
        short_key[10..14].copy_from_slice(&1_u32.to_le_bytes());
        assert_decode_error_without_panic(short_key);

        let key_len = u32::from_le_bytes(FIXTURE[10..14].try_into().unwrap()) as usize;
        let comment_len_start = 14 + key_len;
        let mut short_comment = FIXTURE.to_vec();
        short_comment[comment_len_start..comment_len_start + 4]
            .copy_from_slice(&1_u32.to_le_bytes());
        assert_decode_error_without_panic(short_comment);
    }

    #[test]
    fn short_decrypted_meta_returns_an_error_without_panicking() {
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            Decoder::<Cursor<Vec<u8>>>::parse_meta(b"tiny")
        }));
        assert!(result.is_ok(), "meta parser panicked");
        assert!(result.unwrap().is_err(), "short meta was accepted");
    }

    #[test]
    fn invalid_cover_framing_returns_an_error() {
        let key_len = u32::from_le_bytes(FIXTURE[10..14].try_into().unwrap()) as usize;
        let comment_len_start = 14 + key_len;
        let comment_len = u32::from_le_bytes(
            FIXTURE[comment_len_start..comment_len_start + 4].try_into().unwrap(),
        ) as usize;
        let offset_start = comment_len_start + 4 + comment_len + 5;

        let mut invalid_offset = FIXTURE.to_vec();
        invalid_offset[offset_start..offset_start + 4].copy_from_slice(&0_u32.to_le_bytes());
        assert_decode_error_without_panic(invalid_offset);

        let image_len_start = offset_start + 4;
        let mut short_image = FIXTURE.to_vec();
        short_image[image_len_start..image_len_start + 4].copy_from_slice(&1_u32.to_le_bytes());
        assert_decode_error_without_panic(short_image);
    }

    #[test]
    fn large_skip_uses_a_bounded_buffer() {
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            Decoder::<Cursor<Vec<u8>>>::skip(&mut Cursor::new(Vec::new()), usize::MAX)
        }));
        assert!(result.is_ok(), "skip panicked");
        assert!(result.unwrap().is_err(), "skip unexpectedly succeeded");
    }
}
