mod music_meta;

use anyhow::{Context, Ok, Result};
use id3::TagLike;
use miniserde::json;
use ncm_core::{audio::Type as AudioType, decoder::Decoder};
use std::{
    io::{Cursor, Read, Write},
    vec,
};

use crate::music_meta::MusicMeta;

const TOOL_INFO: &str = include_str!("tool_info");

pub struct Encoder {
    pub data: Vec<u8>,
    pub meta: String,
}

impl Encoder {
    pub fn encode<R>(decoder: Decoder<R>) -> Result<Self>
    where
        R: Read,
    {
        let mut buffer = vec![];
        let audio_type = decoder.audio_type();

        let Decoder { comment, meta, image, mut audio, .. } = decoder;

        audio.read_to_end(&mut buffer)?;

        if meta.is_empty() {
            return Ok(Self { data: buffer, meta: "meta not found".into() });
        }

        let meta = String::from_utf8_lossy(&meta);
        let music_meta: MusicMeta =
            json::from_str(&meta).with_context(|| format!("failed to unpack: {meta}"))?;

        match audio_type {
            AudioType::Flac => {
                let mut tag = metaflac::Tag::read_from(&mut Cursor::new(&buffer))?;
                let data = metaflac::Tag::skip_metadata(&mut Cursor::new(&buffer));

                let vorbis_comment = tag.vorbis_comments_mut();
                vorbis_comment.set_title(vec![music_meta.music_name]);
                vorbis_comment.set_album(vec![music_meta.album]);
                vorbis_comment
                    .set_artist(music_meta.artist.into_iter().map(|ar| ar.0).collect::<Vec<_>>());
                vorbis_comment
                    .set("DESCRIPTION", vec![String::from_utf8_lossy(&comment), TOOL_INFO.into()]);
                vorbis_comment.set("TOOL", vec![TOOL_INFO]);

                if let Some(image) = image {
                    tag.add_picture(
                        image.mime_type(),
                        metaflac::block::PictureType::CoverFront,
                        image.into_data(),
                    );
                }
                buffer.clear();
                tag.remove_blocks(metaflac::BlockType::Padding);
                tag.write_to(&mut buffer)?;
                buffer.write_all(&data)?;
            }
            AudioType::Mp3 => {
                buffer = Self::encode_mp3(buffer, music_meta, &comment, image)?;
            }
            _ => {}
        }

        Ok(Self { data: buffer, meta: meta.into() })
    }

    fn encode_mp3(
        buffer: Vec<u8>,
        music_meta: MusicMeta,
        comment: &[u8],
        image: Option<ncm_core::image::Image>,
    ) -> Result<Vec<u8>> {
        let (mut tag, had_existing_tag) =
            match id3::no_tag_ok(id3::Tag::read_from2(&mut Cursor::new(&buffer)))? {
                Some(tag) => (tag, true),
                None => (id3::Tag::new(), false),
            };
        let mut data_reader = Cursor::new(&buffer);
        if had_existing_tag {
            id3::Tag::skip(&mut data_reader)?;
        }

        tag.set_title(music_meta.music_name);
        tag.set_album(music_meta.album);
        tag.set_artist(music_meta.artist.into_iter().map(|ar| ar.0).collect::<Vec<_>>().join("/"));
        tag.add_frame(id3::frame::Comment {
            lang: "eng".into(),
            description: "".into(),
            text: String::from_utf8_lossy(comment).into(),
        });
        tag.set_text("TSSE", TOOL_INFO);
        tag.set_text("TENC", TOOL_INFO);
        if let Some(image) = image {
            tag.add_frame(id3::frame::Picture {
                mime_type: image.mime_type().into(),
                picture_type: id3::frame::PictureType::CoverFront,
                description: "Cover".into(),
                data: image.into_data(),
            });
        }

        let mut result = vec![];
        tag.write_to(&mut result, id3::Version::Id3v24)?;
        data_reader.read_to_end(&mut result)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::{music_meta::MusicId, music_meta::MusicMeta, Encoder};
    use id3::TagLike;
    use std::io::Cursor;

    fn music_meta() -> MusicMeta {
        MusicMeta {
            music_id: MusicId::Num(1),
            music_name: "Title".into(),
            artist: vec![("Artist".into(), MusicId::Num(2))],
            album: "Album".into(),
            album_pic: "".into(),
            format: "mp3".into(),
        }
    }

    #[test]
    fn writes_a_new_id3_tag_for_a_bare_mp3() {
        let audio = vec![0xFF, 0xFB, 0x90, 0, 1, 2, 3, 4];
        let encoded = Encoder::encode_mp3(audio.clone(), music_meta(), b"comment", None).unwrap();

        let tag = id3::Tag::read_from2(&mut Cursor::new(&encoded)).unwrap();
        assert_eq!(tag.title(), Some("Title"));
        assert_eq!(tag.album(), Some("Album"));
        assert_eq!(tag.artist(), Some("Artist"));

        let mut reader = Cursor::new(&encoded);
        id3::Tag::skip(&mut reader).unwrap();
        assert_eq!(&encoded[reader.position() as usize..], audio);
    }
}
