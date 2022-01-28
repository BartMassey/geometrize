use image::io::Reader as ImageReader;
use image::*;

type GrayBuffer = ImageBuffer<Luma<u16>, Vec<u16>>;

fn geometrize(
    width: u32,
    height: u32,
    source: &GrayBuffer,
    dest: &mut GrayBuffer,
) {
    let scale = 1.0 / (width as f64 * height as f64);
    for (x, y, p) in source.enumerate_pixels() {
        let scale = scale * x as f64 * y as f64;
        dest.put_pixel(x, y, [(p[0] as f64 * scale) as u16].into());
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let source = ImageReader::open(&argv[1]).unwrap().decode().unwrap().into_luma16();
    let (width, height) = source.dimensions();
    let mut dest: GrayBuffer = ImageBuffer::new(width, height);
    geometrize(width, height, &source, &mut dest);
    dest.save(&argv[2]).unwrap();
}
