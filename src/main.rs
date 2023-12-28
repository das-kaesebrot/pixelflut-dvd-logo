use std::io::prelude::*;
use std::net::TcpStream;
use std::time::Duration;

use args::args::PixelflutClientArgs;
use clap::Parser;
use image::RgbaImage;
use simple_logger::SimpleLogger;

mod args;

fn main() -> std::io::Result<()> {
    SimpleLogger::new().init().unwrap();

    let args = PixelflutClientArgs::parse();

    let mut drift_x: i16 = args.drift_x as i16;
    let mut drift_y: i16 = args.drift_y as i16;

    let pixelflut_host = args.host;
    let pixelflut_port = args.port;

    let mut info_counter = 0;

    let mut image_path = args.image_path;
    if image_path.is_empty() {
        image_path = "assets/image.png".to_owned();
    }

    log::info!("Connecting to '{pixelflut_host}:{pixelflut_port}'");
    log::info!("Using image from path '{image_path}'");

    let im = image::open(image_path).unwrap();

    // Connect to Pixelflut server
    let mut stream = TcpStream::connect((pixelflut_host.clone(), pixelflut_port.clone())).unwrap();
    stream.set_nodelay(true)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .unwrap();

    log::info!("Successfully connected to server! Getting canvas size.");

    stream.write_all(b"SIZE\n")?;
    let mut size_buf:[u8; 1024] = [0; 1024];
    let mut size_str = "";
    let mut result: usize = 0;
    result = stream.read(&mut size_buf)?;

    let size_str_result = std::str::from_utf8(&size_buf);

    log::info!("Read data '{}'", size_str_result.unwrap_or("Empty"));
    log::info!("Read: {result} byte");

    size_str = size_str_result.unwrap();

    let size_split: Vec<&str> = size_str.trim().split(char::is_whitespace).collect();

    let canvas_width: i16 = size_split.get(1).unwrap().parse::<i16>().unwrap();
    let canvas_height: i16 = size_split.get(2).unwrap().parse::<i16>().unwrap();

    log::info!("Set canvas to: [{canvas_width}, {canvas_height}]");

    let size = args.resize_x;

    // start offset
    let mut offset_x: i16 = canvas_width / 2;
    let mut offset_y: i16 = canvas_height / 2;

    let im_resized = im.resize(
        size as u32,
        size as u32,
        image::imageops::FilterType::Gaussian,
    );
    let mut im_rgb = im_resized.to_rgba8();

    change_color(&mut im_rgb);

    let im_half_width = im_rgb.width() as i16 / 2;
    let im_half_height = im_rgb.height() as i16 / 2;

    // Draw the image on the Pixelflut canvas
    loop {
        // Every 1000 iterations, display some stats
        if info_counter > 10000 {
            log::info!("Offset: [{offset_x}, {offset_y}] - Drift: [{drift_x}, {drift_y}]");
            info_counter = 0;
        }

        if (offset_x + im_half_width) > canvas_width as i16 || (offset_x + im_half_width) < 0 {
            drift_x = -drift_x; // invert drift so that the image seems to bounce at the edge
            change_color(&mut im_rgb);

            drift_x = jitter_drift(&mut drift_x);
            drift_y = jitter_drift(&mut drift_y);

            log::info!("Detected bounce");
            log::info!("Offset: [{offset_x}, {offset_y}] - Drift: [{drift_x}, {drift_y}]");
        }

        if (offset_y + im_half_height) > canvas_height as i16 || (offset_y + im_half_height) < 0 {
            drift_y = -drift_y;
            change_color(&mut im_rgb);

            drift_x = jitter_drift(&mut drift_x);
            drift_y = jitter_drift(&mut drift_y);

            log::info!("Detected bounce");
            log::info!("Offset: [{offset_x}, {offset_y}] - Drift: [{drift_x}, {drift_y}]");
        }

        draw_image(
            &mut stream,
            &im_rgb,
            (canvas_width, canvas_height),
            (offset_x, offset_y),
        )?;

        offset_x += drift_x;
        offset_y += drift_y;

        info_counter += 1;
    }
}

fn jitter_drift(drift: &mut i16) -> i16 {
    let drift_rng = rand::thread_rng().gen_range(0..9);

    if (drift_rng == 0) {
        *drift += 1;
    }

    return *drift;
}

fn change_color(image: &mut RgbaImage) {
    let color_r = rand::thread_rng().gen_range(0..255);
    let color_g = rand::thread_rng().gen_range(0..255);
    let color_b = rand::thread_rng().gen_range(0..255);

    log::info!("Changing colors to [{color_r}, {color_g}, {color_b}]");

    for (pixel) in image.pixels_mut() {
        // starting to become transparent --> don't draw, skip pixel
        if pixel.0[3] <= 240 {
            continue;
        }

        pixel.0[0] = color_r;
        pixel.0[1] = color_g;
        pixel.0[2] = color_b;
    }
}

fn draw_image(
    stream: &mut TcpStream,
    image: &RgbaImage,
    canvas_size: (i16, i16),
    offset: (i16, i16),
) -> std::io::Result<()> {
    for (pixel_x, pixel_y, rgb_values) in image.enumerate_pixels() {
        // starting to become transparent --> don't draw, skip pixel
        if rgb_values[3] <= 240 {
            continue;
        }

        let x: i16 = pixel_x as i16 + offset.0 + (image.width() as i16 / 2);
        let y: i16 = pixel_y as i16 + offset.1 + (image.height() as i16 / 2);

        // skip if we're outside of canvas bounds
        if x > canvas_size.0 as i16 {
            continue;
        }

        if y > canvas_size.1 as i16 {
            continue;
        }

        if x < 0 || y < 0 {
            continue;
        }

        let command = format!(
            "PX {} {} {:02X}{:02X}{:02X}\n",
            x, y, rgb_values[0], rgb_values[1], rgb_values[2]
        );
        stream.write_all(command.as_bytes())?;
    }
    Ok(())
}
