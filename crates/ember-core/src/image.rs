//! RGBA8 image buffer. Pure data — no backend dependency.

/// An uncompressed 32-bit RGBA image.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Image {
    width: u32,
    height: u32,
    /// Length is `width * height * 4`, channel order R, G, B, A.
    data: Vec<u8>,
}

impl Image {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0; (width * height * 4) as usize],
        }
    }

    pub fn from_rgba8(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        if data.len() != (width * height * 4) as usize {
            return None;
        }
        Some(Self {
            width,
            height,
            data,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: crate::Color) {
        if x >= self.width || y >= self.height {
            return;
        }
        let i = ((y * self.width + x) * 4) as usize;
        self.data[i] = (color.r.clamp(0.0, 1.0) * 255.0) as u8;
        self.data[i + 1] = (color.g.clamp(0.0, 1.0) * 255.0) as u8;
        self.data[i + 2] = (color.b.clamp(0.0, 1.0) * 255.0) as u8;
        self.data[i + 3] = (color.a.clamp(0.0, 1.0) * 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_image_is_all_transparent() {
        let img = Image::new(2, 2);
        assert_eq!(img.as_bytes().len(), 16);
        assert!(img.as_bytes().iter().all(|&b| b == 0));
    }

    #[test]
    fn from_rgba8_rejects_wrong_length() {
        assert!(Image::from_rgba8(2, 2, vec![0; 15]).is_none());
    }

    #[test]
    fn set_pixel_writes_rgba() {
        let mut img = Image::new(1, 1);
        img.set_pixel(0, 0, crate::Color::rgb(1.0, 0.5, 0.0));
        assert_eq!(img.as_bytes(), &[255, 127, 0, 255]);
    }

    #[test]
    fn set_pixel_out_of_bounds_is_noop() {
        let mut img = Image::new(2, 2);
        img.set_pixel(5, 5, crate::Color::WHITE);
        assert!(img.as_bytes().iter().all(|&b| b == 0));
    }
}
