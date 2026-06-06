//! 1-bpp framebuffer for the 800 × 480 e-paper panel.

use core::convert::Infallible;

use embedded_graphics::{
    geometry::{OriginDimensions, Size},
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Pixel, Point},
};

/// Native panel width in pixels.
pub const WIDTH: u32 = 800;
/// Native panel height in pixels.
pub const HEIGHT: u32 = 480;
/// Bytes in one panel row at 1 bit per pixel.
pub const ROW_BYTES: usize = WIDTH as usize / 8;
/// Total bytes required by a monochrome frame.
pub const FRAMEBUFFER_SIZE: usize = ROW_BYTES * HEIGHT as usize;

/// Heap-backed 1-bpp image buffer.
///
/// The panel uses `1 = white` and `0 = black`. `BinaryColor::On` is mapped to
/// black ink so embedded-graphics primitives read naturally as "drawn" pixels.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameBuffer {
    bytes: Box<[u8]>,
}

impl FrameBuffer {
    /// Allocate a white frame without placing a 48 KB temporary array on the
    /// Rust main-task stack.
    #[must_use]
    pub fn new_white() -> Self {
        Self {
            bytes: vec![0xFF; FRAMEBUFFER_SIZE].into_boxed_slice(),
        }
    }

    /// Build a native panel framebuffer from exactly 48,000 packed bytes.
    ///
    /// Sleep-image decoding uses this narrow boundary after validating a BMP
    /// payload. Product UI rendering continues to use [`Self::new_white`].
    pub fn from_native_bytes(bytes: Vec<u8>) -> Result<Self, &'static str> {
        if bytes.len() != FRAMEBUFFER_SIZE {
            return Err("native panel framebuffer requires exactly 48,000 bytes");
        }
        Ok(Self {
            bytes: bytes.into_boxed_slice(),
        })
    }

    /// Return the packed bytes expected by the panel driver.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Reset the full image to white.
    pub fn clear_white(&mut self) {
        self.bytes.fill(0xFF);
    }

    /// Read a packed panel pixel. Out-of-range coordinates return `None`.
    #[must_use]
    pub fn is_black(&self, point: Point) -> Option<bool> {
        let (byte_index, mask) = pixel_address(point)?;
        Some(self.bytes[byte_index] & mask == 0)
    }

    /// Set a native panel pixel. Orientation wrappers use this narrow method
    /// after transforming logical product coordinates.
    pub(crate) fn set_native_black(&mut self, point: Point, black: bool) {
        let Some((byte_index, mask)) = pixel_address(point) else {
            return;
        };

        if black {
            self.bytes[byte_index] &= !mask;
        } else {
            self.bytes[byte_index] |= mask;
        }
    }
}

impl OriginDimensions for FrameBuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for FrameBuffer {
    type Color = BinaryColor;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            self.set_native_black(point, color == BinaryColor::On);
        }

        Ok(())
    }
}

fn pixel_address(point: Point) -> Option<(usize, u8)> {
    if point.x < 0 || point.y < 0 || point.x >= WIDTH as i32 || point.y >= HEIGHT as i32 {
        return None;
    }

    let x = point.x as usize;
    let y = point.y as usize;
    let byte_index = y * ROW_BYTES + x / 8;
    let mask = 0x80 >> (x % 8);
    Some((byte_index, mask))
}

#[cfg(test)]
mod tests {
    use embedded_graphics::{pixelcolor::BinaryColor, prelude::Pixel, prelude::Point};

    use super::{FrameBuffer, FRAMEBUFFER_SIZE, ROW_BYTES};
    use embedded_graphics::prelude::DrawTarget;

    #[test]
    fn allocates_expected_panel_buffer() {
        let frame = FrameBuffer::new_white();
        assert_eq!(frame.as_bytes().len(), FRAMEBUFFER_SIZE);
        assert!(frame.as_bytes().iter().all(|byte| *byte == 0xFF));
    }

    #[test]
    fn accepts_exact_native_byte_payload() {
        let bytes = vec![0xA5; FRAMEBUFFER_SIZE];
        let frame = FrameBuffer::from_native_bytes(bytes).unwrap();
        assert_eq!(frame.as_bytes().len(), FRAMEBUFFER_SIZE);
        assert_eq!(frame.as_bytes()[0], 0xA5);
        assert!(FrameBuffer::from_native_bytes(vec![0xFF; 1]).is_err());
    }

    #[test]
    fn packs_msb_first_black_pixels() {
        let mut frame = FrameBuffer::new_white();
        frame
            .draw_iter([
                Pixel(Point::new(0, 0), BinaryColor::On),
                Pixel(Point::new(7, 0), BinaryColor::On),
                Pixel(Point::new(8, 1), BinaryColor::On),
            ])
            .unwrap();

        assert_eq!(frame.as_bytes()[0], 0b0111_1110);
        assert_eq!(frame.as_bytes()[ROW_BYTES + 1], 0b0111_1111);
    }

    #[test]
    fn ignores_out_of_bounds_pixels() {
        let mut frame = FrameBuffer::new_white();
        frame
            .draw_iter([Pixel(Point::new(-1, 0), BinaryColor::On)])
            .unwrap();
        assert!(frame.as_bytes().iter().all(|byte| *byte == 0xFF));
    }
}
