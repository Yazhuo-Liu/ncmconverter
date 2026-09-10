use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum Type {
    Png,
    Jpeg,
    Gif,
    Bmp,
    Webp,
    Unknown,
}

#[derive(Clone)]
pub struct Image(Type, Vec<u8>);

impl Image {
    pub fn ext(&self) -> &'static str {
        match &self.0 {
            Type::Png => "png",
            Type::Jpeg => "jpeg",
            Type::Gif => "gif",
            Type::Bmp => "bmp",
            Type::Webp => "webp",
            Type::Unknown => "image",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match &self.0 {
            Type::Png => "image/png",
            Type::Jpeg => "image/jpeg",
            Type::Gif => "image/gif",
            Type::Bmp => "image/bmp",
            Type::Webp => "image/webp",
            Type::Unknown => "image/*",
        }
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.1
    }

    pub fn into_data(self) -> Vec<u8> {
        self.1
    }
}

impl From<Vec<u8>> for Image {
    fn from(value: Vec<u8>) -> Self {
        let image_type = if value.starts_with(b"\x89PNG\r\n\x1A\n") {
            Type::Png
        } else if matches!(
            value.as_slice(),
            [0xFF, 0xD8, 0xFF, 0xE0 | 0xE1 | 0xE2 | 0xE3 | 0xE8, ..]
        ) {
            Type::Jpeg
        } else if value.starts_with(b"RIFF") && value.get(8..12) == Some(b"WEBP") {
            Type::Webp
        } else if value.starts_with(b"GIF8") {
            Type::Gif
        } else if value.starts_with(b"BM") {
            Type::Bmp
        } else {
            Type::Unknown
        };

        Image(image_type, value)
    }
}

impl Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image").field("type", &self.0).field("size", &self.1.len()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::Image;

    #[test]
    fn test_image() {
        let mut data = vec![0; 32];
        data[..4].copy_from_slice(b"GIF8");
        let image = Image::from(data);
        assert_eq!(image.ext(), "gif");

        let mut data = vec![0; 32];
        data[..4].copy_from_slice(&[0xFF, 0xD8, 0xFF, 0xE0]);
        let image = Image::from(data);
        assert_eq!(image.ext(), "jpeg");
    }

    #[test]
    fn short_images_are_unknown_without_panicking() {
        for len in 0..12 {
            let image = Image::from(vec![0; len]);
            assert_eq!(image.ext(), "image");
        }
    }
}
