use image::io::Reader as ImageReader;
use image::*;
use ordered_float::OrderedFloat;

type GrayBuffer = ImageBuffer<Luma<u16>, Vec<u16>>;

fn image_stats(img: &SubImage<&GrayBuffer>) -> (f64, f64) {
    let (width, height) = img.dimensions();
    let n = width as f64 * height as f64;
    let mean = img
        .pixels()
        .map(|(_, _, p)| p[0] as f64)
        .sum::<f64>() / n;
    let variance = img
        .pixels()
        .map(|(_, _, p)| {
            let p = p[0] as f64;
            let dp = p - mean;
            dp * dp
        })
        .sum::<f64>() / n;
    (mean, variance)
}

fn decontrast(img: &mut SubImage<&mut GrayBuffer>, mean: f64, contrast: f64) {
    let (width, height) = img.dimensions();
    for x in 0..width {
        for y in 0..height {
            let p = img.get_pixel(x, y);
            let dp = p[0] as f64 - mean;
            img.put_pixel(x, y, [(mean + dp * contrast) as u16].into());
        }
    }
}

struct Cut {
    coord: u32,
    means: [f64; 2],
    score: f64,
}

impl std::cmp::PartialEq<Cut> for Cut {
    fn eq(&self, other: &Cut) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl std::cmp::Eq for Cut {}

impl std::cmp::PartialOrd for Cut {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        OrderedFloat(self.score).partial_cmp(&OrderedFloat(other.score))
    }
}

impl std::cmp::Ord for Cut {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

fn best_hcut(img: &SubImage<&mut GrayBuffer>) -> Cut {
    let (width, height) = img.dimensions();
    (1..height - 1)
        .map(|y| {
            let top = img.view(0, 0, width, y);
            let (m_top, v_top) = image_stats(&top);
            let bottom = img.view(0, y, width, height - y);
            let (m_bottom, v_bottom) = image_stats(&bottom);
            let score = (v_top * y as f64 + v_bottom * (height - y) as f64) / height as f64;
            Cut {
                coord: y,
                means: [m_top, m_bottom],
                score,
            }
        })
        .min()
        .unwrap()
}

// XXX So much copy-paste.
fn best_vcut(img: &SubImage<&mut GrayBuffer>) -> Cut {
    let (width, height) = img.dimensions();
    (1..width - 1)
        .map(|x| {
            let left = img.view(0, 0, x, height);
            let (m_left, v_left) = image_stats(&left);
            let right = img.view(x, 0, width - x, height);
            let (m_right, v_right) = image_stats(&right);
            let score = (v_left * x as f64 + v_right * (width - x) as f64) / width as f64;
            Cut {
                coord: x,
                means: [m_left, m_right],
                score,
            }
        })
        .min()
        .unwrap()
}

fn geometrize(img: &mut SubImage<&mut GrayBuffer>, depth: usize, contrast: f64) {
    if depth == 0 {
        return;
    }
    let (width, height) = img.dimensions();
    if width < 5 || height < 5 {
        return;
    }
    let hcut = best_hcut(img);
    let vcut = best_vcut(img);
    if hcut < vcut {
        let Cut { coord: y, means: [m1, m2], .. } = hcut;
        let mut top = img.sub_image(0, 0, width, y);
        decontrast(&mut top, m1, contrast);
        geometrize(&mut top, depth - 1, contrast);
        let mut bottom = img.sub_image(0, y, width, height - y);
        decontrast(&mut bottom, m2, contrast);
        geometrize(&mut bottom, depth - 1, contrast);
    } else {
        let Cut { coord: x, means: [m1, m2], .. } = vcut;
        let mut left = img.sub_image(0, 0, x, height);
        decontrast(&mut left, m1, contrast);
        geometrize(&mut left, depth - 1, contrast);
        let mut right = img.sub_image(x, 0, width - x, height);
        decontrast(&mut right, m2, contrast);
        geometrize(&mut right, depth - 1, contrast);
    }
}


fn image_expand_luma(img: &mut GrayBuffer) {
    // XXX Two passes here is gross.
    let min = img
        .pixels()
        .map(|p| p[0])
        .min()
        .unwrap();
    let max = img
        .pixels()
        .map(|p| p[0])
        .max()
        .unwrap();
    let scale = 65536.0 / (max as f64 - min as f64);
    let offset = 65536.0 / min as f64;
    for p in img.pixels_mut() {
        p[0] = ((p[0] - min) as f64 * scale + offset) as u16;
    }
}

fn main() {
    let args = argwerk::args! {
        /// Image geometrizer.
        "geometrize" {
            depth: usize = 1,
            contrast: f64 = 0.5,
            source: String,
            dest: String,
        }
        /// Partition depth. (default: 1).
        ["-d" | "--depth", int] => {
            depth = str::parse(&int)?;
        }
        /// Contrast factor. (default: 0.5).
        ["-c" | "--contrast", float] => {
            contrast = str::parse(&float)?;
        }
        /// Input and output image.
        [src, dst] => {
            source = src;
            dest = dst;
        }
    }.unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    });

    let mut img = ImageReader::open(&args.source).unwrap().decode().unwrap().into_luma16();
    let (width, height) = img.dimensions();
    let sub_image = &mut img.sub_image(0, 0, width, height);
    geometrize(sub_image, args.depth, args.contrast);
    image_expand_luma(&mut img);
    img.save(&args.dest).unwrap();
}
