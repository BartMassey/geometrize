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

fn geometrize(img: &mut GrayBuffer) {
    let (width, height) = img.dimensions();
    let (y, m_top, m_bottom, _) = (1..height - 1)
        .map(|y| {
            let top = img.view(0, 0, width, y);
            let (m_top, v_top) = image_stats(&top);
            let bottom = img.view(0, y, width, height - y);
            let (m_bottom, v_bottom) = image_stats(&bottom);
            let score = v_top * y as f64 + v_bottom * (height - y) as f64;
            (y, m_top, m_bottom, score)
        })
        .min_by_key(|&(_, _, _, v)| OrderedFloat(v))
        .unwrap();
    eprintln!("at {y} {m_top}/{m_bottom}");
    decontrast(&mut img.sub_image(0, 0, width, y), m_top);
    decontrast(&mut img.sub_image(0, y, width, height - y), m_bottom);
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let mut img = ImageReader::open(&argv[1]).unwrap().decode().unwrap().into_luma16();
    geometrize(&mut img);
    img.save(&argv[2]).unwrap();
}
