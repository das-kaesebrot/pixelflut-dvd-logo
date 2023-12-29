pub(crate) mod args {
    pub use clap::Parser;

    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    pub struct PixelflutClientArgs {
        // Host to use
        #[arg(short = 'H', long)]
        pub host: String,
    
        // Port to use
        #[arg(short, long, default_value_t = 1337)]
        pub port: u16,

        // Resize width
        #[arg(long, default_value_t = 350)]
        pub resize: i16,

        // drift x
        #[arg(long, default_value_t = 12)]
        pub drift_x: u16,

        // drift y
        #[arg(long, default_value_t = 9)]
        pub drift_y: u16,

        // Image to draw
        #[arg(long)]
        pub image_path: String,

        // Rate to draw at in FPS
        #[arg(long, default_value_t = 60)]
        pub draw_rate: u16,

        // Stroke width
        #[arg(long, default_value_t = 4)]
        pub stroke: u32,

        // Stroke width
        #[arg(long, default_value_t = false)]
        pub jitter: bool,
    }
}
