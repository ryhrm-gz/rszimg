use clap::Parser;
use image::imageops::FilterType;
use mozjpeg::{ColorSpace, Compress, ScanMode};
use std::path::PathBuf;
use std::{fs, process};

#[derive(Parser)]
#[command(version, about)]
struct Args {
    // The path to the image or directory to resize
    target: PathBuf,

    #[arg(
        short,
        long,
        default_value = "./resized",
        help = "The directory to store the output"
    )]
    directory: PathBuf,

    #[arg(short,long, default_values_t = vec![1280, 640], help = "The sizes to resize the image to")]
    sizes: Vec<usize>,
}

fn main() {
    let args = Args::parse();
    let Args {
        target,
        directory,
        sizes,
    } = args;

    if !target.exists() {
        eprintln!("The target {:?} does not exist", target);
        process::exit(1);
    }

    if directory.exists() && !directory.is_dir() {
        eprintln!("The directory {:?} is not directory", directory);
        process::exit(1);
    }

    let image_paths: Vec<PathBuf> = if target.is_dir() {
        target
            .read_dir()
            .expect("Failed to read directory")
            .map(|entry| entry.expect("Failed to read entry").path())
            .filter(|path| path.is_file())
            .filter(|path| {
                let ext = path.extension().unwrap_or_default();
                ext == "jpg" || ext == "jpeg" || ext == "png"
            })
            .collect()
    } else {
        let ext = target.extension().unwrap_or_default();
        if ext == "jpg" || ext == "jpeg" || ext == "png" {
            vec![target]
        } else {
            vec![]
        }
    };

    if !image_paths.is_empty() {
        if !directory.exists() {
            fs::create_dir(&directory).expect("Failed to create directory");
        }
        for image_path in image_paths.iter() {
            for target_size in sizes.iter() {
                let (resized_image_data, resized_width, resized_height) =
                    match resize(image_path, *target_size) {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("Failed to resize image {:?}: {}", image_path, e);
                            process::exit(1);
                        }
                    };
                let compressed = match compress(resized_image_data, resized_width, resized_height) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Failed to compress image {:?}: {}", image_path, e);
                        process::exit(1);
                    }
                };

                let filename = format!(
                    "{}_{}.jpg",
                    image_path.file_stem().unwrap().to_string_lossy(),
                    target_size,
                );
                let output_path = directory.join(&filename);
                let _ = fs::write(output_path, compressed);
                println!("Resized {:?}", &filename);
            }
        }
    }
}

fn resize(path: &PathBuf, target_size: usize) -> Result<(Vec<u8>, usize, usize), String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let width = target_size as u32;
    let height = target_size as u32;
    let resized = img.resize(width, height, FilterType::Lanczos3);
    Ok((
        resized.to_rgb8().to_vec(),
        resized.width() as usize,
        resized.height() as usize,
    ))
}

fn compress(image_data: Vec<u8>, width: usize, height: usize) -> Result<Vec<u8>, String> {
    let mut comp = Compress::new(ColorSpace::JCS_RGB);
    comp.set_scan_optimization_mode(ScanMode::AllComponentsTogether);
    comp.set_quality(70.0);
    comp.set_size(width, height);

    let mut comp = comp.start_compress(Vec::new()).map_err(|e| e.to_string())?;

    let mut line = 0;
    loop {
        if line >= height {
            break;
        }
        let buf = unsafe { image_data.get_unchecked(line * width * 3..(line + 1) * width * 3) };
        let _ = comp.write_scanlines(buf);
        line += 1;
    }

    let writer = comp.finish().map_err(|e| e.to_string())?;
    Ok(writer)
}
