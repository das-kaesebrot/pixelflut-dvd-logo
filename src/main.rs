use std::io::prelude::*;
use std::net::TcpStream;
use std::time::{Duration, Instant};

use args::args::PixelflutClientArgs;
use clap::Parser;
use image::RgbaImage;
use rand::Rng;
use simple_logger::SimpleLogger;

mod args;

fn main() -> std::io::Result<()> {
    SimpleLogger::new().init().unwrap();

    let args = PixelflutClientArgs::parse();

    let mut drift_x: i16 = args.drift_x as i16;
    let mut drift_y: i16 = args.drift_y as i16;

    let pixelflut_host = args.host;
    let pixelflut_port = args.port;

    let draw_duration = Duration::from_secs_f64(1 as f64 / args.draw_rate as f64);

    let mut info_counter = 0;

    let mut streams: Vec<TcpStream> = Vec::new();

    let mut image_path = args.image_path;
    if image_path.is_empty() {
        image_path = "assets/image.png".to_owned();
    }

    log::info!("Connecting to '{pixelflut_host}:{pixelflut_port}'");
    log::info!("Using image from path '{image_path}'");

    let im = image::open(image_path).unwrap();

    // Connect to Pixelflut server
    streams.push(TcpStream::connect((pixelflut_host.clone(), pixelflut_port.clone())).unwrap());

    let mut stream = &streams[0];
    stream.set_nodelay(true)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .unwrap();

    log::info!("Successfully connected to server! Getting canvas size.");

    stream.write_all(b"SIZE\n")?;
    let mut size_buf: [u8; 1024] = [0; 1024];
    let result = stream.read(&mut size_buf)?;

    let size_str_result = std::str::from_utf8(&size_buf);

    log::info!("Read data '{}'", size_str_result.unwrap_or("Empty"));
    log::info!("Read: {result} byte");

    let size_str = size_str_result.unwrap();

    let size_split: Vec<&str> = size_str.trim().split(char::is_whitespace).collect();

    let canvas_width: i16 = size_split.get(1).unwrap().parse::<i16>().unwrap();
    let canvas_height: i16 = size_split.get(2).unwrap().parse::<i16>().unwrap();

    log::info!("Set canvas to [{canvas_width}, {canvas_height}]");

    let size = args.resize;

    // start offset
    let mut offset_x: i16 = canvas_width / 2;
    let mut offset_y: i16 = canvas_height / 2;

    let im_resized = im.resize(
        size as u32,
        size as u32,
        image::imageops::FilterType::Gaussian,
    );
    let mut im_rgb = im_resized.to_rgba8();

    while streams.len() < size as usize {
        streams.push(TcpStream::connect((pixelflut_host.clone(), pixelflut_port.clone())).unwrap());
    }

    log::info!("Opened {} server connections", streams.len());

    change_color(&mut im_rgb);

    let mut bounce = false;

    // Draw the image on the Pixelflut canvas
    loop {
        let start = Instant::now();

        // Every 1000 iterations, display some stats
        if info_counter > 1000 {
            log::info!("Offset: [{offset_x}, {offset_y}] - Drift: [{drift_x}, {drift_y}]");
            info_counter = 0;
        }

        let bound_left = offset_x;
        let bound_right = offset_x + im_rgb.width() as i16;
        let bound_upper = offset_y;
        let bound_lower = offset_y + im_rgb.height() as i16;

        if bound_left < 0 || bound_right > canvas_width as i16 {
            drift_x = -drift_x; // invert drift so that the image seems to bounce at the edge
            bounce = true;
        }

        if bound_upper < 0 || bound_lower > canvas_height {
            drift_y = -drift_y;
            bounce = true;
        }

        if bounce {
            change_color(&mut im_rgb);
            add_stroke(&mut im_rgb, 5);

            drift_x = jitter_drift(&mut drift_x);
            drift_y = jitter_drift(&mut drift_y);

            log::info!("Detected bounce");
            log::info!("Offset: [{offset_x}, {offset_y}] - Drift: [{drift_x}, {drift_y}]");
            bounce = false;
        }

        let mut duration = Duration::ZERO;
        while duration < draw_duration {
            draw_image(
                &mut streams,
                &im_rgb,
                (canvas_width, canvas_height),
                (offset_x, offset_y),
            )?;

            duration = start.elapsed();
        }

        offset_x += drift_x;
        offset_y += drift_y;

        info_counter += 1;

        if duration > Duration::from_secs(1) {
            log::warn!("Slow drawing ({:.2}s)", duration.as_secs_f32());
        }
    }
}

fn jitter_drift(drift: &mut i16) -> i16 {
    let drift_rng = rand::thread_rng().gen_range(0..9);

    if drift_rng == 0 {
        *drift += 1;
    }

    return *drift;
}

fn change_color(image: &mut RgbaImage) {
    let color_r = rand::thread_rng().gen_range(0..255);
    let color_g = rand::thread_rng().gen_range(0..255);
    let color_b = rand::thread_rng().gen_range(0..255);

    log::info!("Changing colors to [{color_r}, {color_g}, {color_b}]");

    for pixel in image.pixels_mut() {
        // starting to become transparent --> don't draw, skip pixel
        if pixel.0[3] <= 240 {
            continue;
        }

        pixel.0[0] = color_r;
        pixel.0[1] = color_g;
        pixel.0[2] = color_b;
    }
}

fn add_stroke(image: &mut RgbaImage, width: u32) {
    let img_clone = image.clone();
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        // skip if pixel is not transparent
        if pixel.0[3] < 240 {
            continue;
        }

        let mut set_black: bool = false;

        for neighbor_offset in 0..width {
            set_black = pixel_is_transparent(x - neighbor_offset, y, &img_clone)
                || pixel_is_transparent(x + neighbor_offset, y, &img_clone)
                || pixel_is_transparent(x, y - neighbor_offset, &img_clone)
                || pixel_is_transparent(x, y + neighbor_offset, &img_clone);
        }

        if set_black {
            pixel.0[0] = 0;
            pixel.0[1] = 0;
            pixel.0[2] = 0;
        }
    }
}

fn pixel_is_transparent(x: u32, y: u32, image: &RgbaImage) -> bool {
    let pixel: Option<&image::Rgba<u8>> = image.get_pixel_checked(x, y);
    if pixel.is_some() {
        if pixel.unwrap().0[3] < 240 {
            return true;
        }
    }
    return false;
}

fn draw_image(
    stream: &mut Vec<TcpStream>,
    image: &RgbaImage,
    canvas_size: (i16, i16),
    offset: (i16, i16),
) -> std::io::Result<()> {
    let mut conn_index = 0;

    for (pixel_x, pixel_y, rgb_values) in image.enumerate_pixels() {
        // starting to become transparent --> don't draw, skip pixel
        if rgb_values[3] <= 240 {
            continue;
        }

        let x: i16 = pixel_x as i16 + offset.0;
        let y: i16 = pixel_y as i16 + offset.1;

        // skip if we're outside of canvas bounds
        if x + image.width() as i16 > canvas_size.0 as i16 {
            continue;
        }

        if y + image.height() as i16 > canvas_size.1 as i16 {
            continue;
        }

        if x < 0 || y < 0 {
            continue;
        }

        let command = format!(
            "PX {} {} {:02X}{:02X}{:02X}\n",
            x, y, rgb_values[0], rgb_values[1], rgb_values[2]
        );
        stream[conn_index].write_all(command.as_bytes())?;

        conn_index += 1;
        if conn_index >= stream.len() {
            conn_index = 0;
        }
    }

    Ok(())
}
