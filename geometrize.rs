use image::io::Reader as ImageReader;
use image::*;
use ordered_float::OrderedFloat;

type GrayBuffer = ImageBuffer<Luma<u16>, Vec<u16>>;

fn image_stats(source: &SubImage<&GrayBuffer>) -> (f64, f64) {
    let (width, height) = source.dimensions();
    let n = width as f64 * height as f64;
    let mean = source
        .pixels()
        .map(|(_, _, p)| p[0] as f64)
        .sum::<f64>() / n;
    let variance = source
        .pixels()
        .map(|(_, _, p)| {
            let p = p[0] as f64;
            let dp = p - mean;
            dp * dp
        })
        .sum::<f64>() / n;
    (mean, variance)
}

fn decontrast(img: &mut SubImage<&mut GrayBuffer>, mean: f64) {
    let (width, height) = img.dimensions();
    for x in 0..width {
        for y in 0..height {
            let p = img.get_pixel(x, y);
            let dp = p[0] as f64 - mean;
            img.put_pixel(x, y, [(mean + dp / 2.0) as u16].into());
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

fn best_vcut(img: &SubImage<&mut GrayBuffer>) -> Cut {
    let (width, height) = img.dimensions();
    (1..height - 1)
        .map(|y| {
            let top = img.view(0, 0, width, y);
            let (m_top, v_top) = image_stats(&top);
            let bottom = img.view(0, y, width, height - y);
            let (m_bottom, v_bottom) = image_stats(&bottom);
            let score = v_top * y as f64 + v_bottom * (height - y) as f64;
            Cut {
                coord: y,
                means: [m_top, m_bottom],
                score,
            }
        })
        .min()
        .unwrap()
}

fn geometrize(img: &mut SubImage<&mut GrayBuffer>) {
    let (width, height) = img.dimensions();
    let Cut { coord: y, means: [m1, m2], .. } = best_vcut(img);
    eprintln!("at {y} {m1}/{m2}");
    decontrast(&mut img.sub_image(0, 0, width, y), m1);
    decontrast(&mut img.sub_image(0, y, width, height - y), m2);
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let mut img = ImageReader::open(&argv[1]).unwrap().decode().unwrap().into_luma16();
    let (width, height) = img.dimensions();
    geometrize(&mut img.sub_image(0, 0, width, height));
    img.save(&argv[2]).unwrap();
}
