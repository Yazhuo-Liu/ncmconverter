use anyhow::{bail, Context, Result};
use js_sys::Uint8Array;
use ncm_core::{audio::Type as AudioType, decoder::Decoder};
use ncm_meta::{track_metadata, Encoder};

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct ConvertResult {
    audio: Uint8Array,
    format: String,
    extension: String,
    mime_type: String,
    title: String,
    artist: String,
    album: String,
}

#[wasm_bindgen]
impl ConvertResult {
    #[wasm_bindgen(getter)]
    pub fn audio(&self) -> Uint8Array {
        self.audio.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn format(&self) -> String {
        self.format.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn extension(&self) -> String {
        self.extension.clone()
    }

    #[wasm_bindgen(getter, js_name = mimeType)]
    pub fn mime_type(&self) -> String {
        self.mime_type.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn title(&self) -> String {
        self.title.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn artist(&self) -> String {
        self.artist.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn album(&self) -> String {
        self.album.clone()
    }
}

struct Conversion {
    audio: Vec<u8>,
    format: &'static str,
    extension: &'static str,
    mime_type: &'static str,
    title: String,
    artist: String,
    album: String,
}

#[wasm_bindgen]
pub fn convert(input: &[u8]) -> Result<ConvertResult, JsValue> {
    let result = convert_inner(input).map_err(|error| JsValue::from_str(&error.to_string()))?;

    Ok(ConvertResult {
        audio: Uint8Array::from(result.audio.as_slice()),
        format: result.format.into(),
        extension: result.extension.into(),
        mime_type: result.mime_type.into(),
        title: result.title,
        artist: result.artist,
        album: result.album,
    })
}

fn convert_inner(input: &[u8]) -> Result<Conversion> {
    let reader = std::io::Cursor::new(input);
    let decoder = Decoder::decode(reader).context("could not decode the NCM file")?;
    let (format, extension, mime_type) = format_details(decoder.audio_type())?;
    let metadata = track_metadata(&decoder.meta).context("could not read the track metadata")?;
    let Encoder { data, .. } =
        Encoder::encode(decoder).context("could not build the output audio file")?;

    let (title, artist, album) = metadata.map_or_else(
        || (String::new(), String::new(), String::new()),
        |metadata| (metadata.title, metadata.artists.join(" / "), metadata.album),
    );

    Ok(Conversion { audio: data, format, extension, mime_type, title, artist, album })
}

fn format_details(audio_type: AudioType) -> Result<(&'static str, &'static str, &'static str)> {
    match audio_type {
        AudioType::Mp3 => Ok(("MP3", "mp3", "audio/mpeg")),
        AudioType::Flac => Ok(("FLAC", "flac", "audio/flac")),
        AudioType::M4a => Ok(("M4A", "m4a", "audio/mp4")),
        AudioType::Ogg => Ok(("OGG", "ogg", "audio/ogg")),
        AudioType::Unknown => bail!(
            "unsupported output format: the decrypted audio header is not MP3, FLAC, M4A, or OGG"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::convert_inner;

    #[test]
    fn converts_mp3_with_format_and_metadata() {
        let input = include_bytes!("../../ncmc/tests/input/1356233945.ncm");
        let result = convert_inner(input).unwrap();

        assert_eq!(result.format, "MP3");
        assert_eq!(result.extension, "mp3");
        assert_eq!(result.mime_type, "audio/mpeg");
        assert_eq!(result.title, "Eye Catch");
        assert_eq!(result.artist, "伊賀拓郎");
        assert_eq!(result.album, "TVアニメ「私に天使が舞い降りた!」サウンドコレクション");
        assert!(result.audio.starts_with(b"ID3"));
    }

    #[test]
    fn converts_flac_and_reports_its_decrypted_header() {
        let input = include_bytes!("../../ncmc/tests/input/1431343706.ncm");
        let result = convert_inner(input).unwrap();

        assert_eq!(result.format, "FLAC");
        assert_eq!(result.extension, "flac");
        assert_eq!(result.mime_type, "audio/flac");
        assert!(result.audio.starts_with(b"fLaC"));
    }
}
