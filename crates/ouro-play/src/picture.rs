//! An RGB picture and its PNG: the snapshots' and previews' pictures are
//! kept as PNG (game pictures pack well), made smaller when asked.

/// Rows of RGB pixels.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Picture {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
}

impl Picture {
    pub fn new(width: usize, height: usize) -> Picture {
        Picture { width, height, rgb: vec![0; width * height * 3] }
    }

    /// Half the size: each pixel the mean of four.
    pub fn half(&self) -> Picture {
        let (w, h) = (self.width / 2, self.height / 2);
        let mut out = Picture::new(w, h);
        for y in 0..h {
            for x in 0..w {
                for c in 0..3 {
                    let at = |dx: usize, dy: usize| u32::from(self.rgb[((2 * y + dy) * self.width + 2 * x + dx) * 3 + c]);
                    out.rgb[(y * w + x) * 3 + c] = ((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1)) / 4) as u8;
                }
            }
        }
        out
    }

    pub fn png(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut e = png::Encoder::new(&mut out, self.width as u32, self.height as u32);
        e.set_color(png::ColorType::Rgb);
        e.set_depth(png::BitDepth::Eight);
        let mut w = e.write_header().expect("PNG into memory");
        w.write_image_data(&self.rgb).expect("PNG into memory");
        drop(w);
        out
    }

    /// A PNG (8-bit RGB or RGBA; alpha dropped).
    pub fn from_png(data: &[u8]) -> Option<Picture> {
        let mut d = png::Decoder::new(std::io::Cursor::new(data));
        d.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut r = d.read_info().ok()?;
        let mut buf = vec![0; r.output_buffer_size()?];
        let info = r.next_frame(&mut buf).ok()?;
        let n = info.color_type.samples();
        let rgb = buf[..info.buffer_size()]
            .chunks_exact(n)
            .flat_map(|p| if n < 3 { [p[0], p[0], p[0]] } else { [p[0], p[1], p[2]] })
            .collect();
        Some(Picture { width: info.width as usize, height: info.height as usize, rgb })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_round_trip_and_half() {
        let mut p = Picture::new(4, 2);
        p.rgb[..6].copy_from_slice(&[200, 0, 0, 100, 0, 0]);
        p.rgb[12..18].copy_from_slice(&[0, 0, 0, 100, 40, 0]);
        assert_eq!(Picture::from_png(&p.png()), Some(p.clone()));
        let h = p.half();
        assert_eq!((h.width, h.height), (2, 1));
        assert_eq!(&h.rgb[..3], &[100, 10, 0]);
    }
}
