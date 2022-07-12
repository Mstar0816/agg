use anyhow::Result;
use clap::{ArgEnum, Parser};
use log::info;
use std::{fs::File, thread, time::Instant};
use vt::VT;
mod asciicast;
mod frames;
mod renderer;
use renderer::Renderer;

// TODO:
// switch to vt from git
// theme selection
// zoom selection
// additional font dirs
// time window (from/to)
// fps cap override

#[derive(Clone, ArgEnum)]
enum RendererBackend {
    Fontdue,
    Resvg,
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// asciicast path/filename
    input_filename: String,

    /// GIF path/filename
    output_filename: String,

    /// Frame rendering backend
    #[clap(long, arg_enum, default_value_t = RendererBackend::Fontdue)]
    renderer: RendererBackend,

    /// Font family
    #[clap(long, default_value_t = String::from("JetBrains Mono,Fira Code,SF Mono,Menlo,Consolas,DejaVu Sans Mono,Liberation Mono"))]
    font_family: String,

    /// Playback speed
    #[clap(long, default_value_t = 1.0)]
    speed: f64,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let zoom = 2.0;
    let fps_cap = 30.0;

    // =========== asciicast

    let (cols, rows, events) = {
        let (header, events) = asciicast::open(&cli.input_filename)?;

        (
            header.width,
            header.height,
            frames::stdout(events, cli.speed, fps_cap),
        )
    };

    // ============ VT

    let vt = VT::new(cols, rows);

    // ============ font database

    let mut font_db = fontdb::Database::new();
    font_db.load_system_fonts();
    font_db.load_fonts_dir("fonts");

    let families = cli
        .font_family
        .split(',')
        .map(fontdb::Family::Name)
        .collect::<Vec<_>>();

    let query = fontdb::Query {
        families: &families,
        weight: fontdb::Weight::NORMAL,
        stretch: fontdb::Stretch::Normal,
        style: fontdb::Style::Normal,
    };

    let face_id = font_db
        .query(&query)
        .ok_or_else(|| anyhow::anyhow!("no faces matching font family {}", cli.font_family))?;

    let face_info = font_db.face(face_id).unwrap();
    let font_family = face_info.family.clone();

    info!("selected font family: {}", &font_family);

    // =========== renderer

    let mut renderer: Box<dyn Renderer> = match cli.renderer {
        RendererBackend::Fontdue => {
            Box::new(renderer::fontdue(cols, rows, font_db, &font_family, zoom))
        }
        RendererBackend::Resvg => {
            Box::new(renderer::resvg(cols, rows, font_db, &font_family, zoom))
        }
    };

    // ============ GIF writer

    let settings = gifski::Settings {
        width: Some(renderer.pixel_width() as u32),
        height: Some(renderer.pixel_height() as u32),
        quality: 100,
        fast: true,
        ..gifski::Settings::default()
    };

    let (mut collector, writer) = gifski::new(settings)?;

    // ============= iterator

    let count = events.len() as u64;

    let images = events
        .iter()
        .scan(vt, |vt, (t, d)| {
            vt.feed_str(&d);
            let cursor = vt.get_cursor();
            let lines = vt.lines();
            Some((t, lines, cursor))
        })
        .map(move |(time, lines, cursor)| (renderer.render(lines, cursor), time));

    // ======== goooooooooooooo

    let start_time = Instant::now();

    let file = File::create(cli.output_filename)?;

    let writer_handle = thread::spawn(move || {
        let mut pr = gifski::progress::ProgressBar::new(count);
        writer.write(file, &mut pr)
    });

    for (i, (image, time)) in images.enumerate() {
        collector.add_frame_rgba(i, image, *time)?;
    }

    drop(collector);

    writer_handle.join().unwrap()?;

    info!("finished in {}s", start_time.elapsed().as_secs_f32());

    Ok(())
}
