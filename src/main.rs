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

    let canvas_width: i16 = args.canvas_x;
    let canvas_height: i16 = args.canvas_y;

    let size = args.resize_x;

    // start offset
    let mut offset_x: i16 = 960;
    let mut offset_y: i16 = 540;

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
    let im_resized = im.resize(
        size as u32,
        size as u32,
        image::imageops::FilterType::Gaussian,
    );
    let im_rgb = im_resized.to_rgba8();

    // Connect to Pixelflut server
    let mut stream = TcpStream::connect((pixelflut_host.clone(), pixelflut_port.clone())).unwrap();
    let mut _discard = Vec::new();
    stream.set_nodelay(true)?;
    stream.set_nonblocking(true)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream.peek(&mut _discard)?;

    log::info!("Successfully connected to server!");

    // Draw the image on the Pixelflut canvas
    loop {
        // Every 1000 iterations, display some stats
        if info_counter > 10000 {
            log::info!("Offset: [{offset_x}, {offset_y}] - Drift: [{drift_x}, {drift_y}]");
            info_counter = 0;
        }

        if offset_x > canvas_width.try_into().unwrap() || offset_x < 0 {
            drift_x = -drift_x; // invert drift so that the image seems to bounce at the edge
        }

        if offset_y > canvas_height.try_into().unwrap() || offset_y < 0 {
            drift_y = -drift_y;
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

        let x: i16 = pixel_x as i16 + offset.0 + image.height() as i16 / 2;
        let y: i16 = pixel_y as i16 + offset.1 + image.width() as i16 / 2;

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
        // println!("Sending {command}");
        stream.write_all(command.as_bytes())?;
    }
    Ok(())
}
