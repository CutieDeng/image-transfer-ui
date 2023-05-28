use std::{path::PathBuf, sync::mpsc, process, borrow::Cow, env::current_dir};

use eframe::{egui::{CentralPanel, widgets, TextureOptions, Spinner, SidePanel, ScrollArea}, epaint::{TextureId, ColorImage, TextureHandle}};
use image::{ImageBuffer, Rgba};

use arboard::Clipboard; 

fn main() {
    let mut native_options = eframe::NativeOptions::default(); 
    native_options.initial_window_size = Some(eframe::egui::Vec2::new(850.0, 700.0)); 
    eframe::run_native( "Image Style Transfer", native_options, Box::new( 
        |_| Box::new( App {
            content_texture_id: None, 
            style_texture_id: None, 
            result_texture_id: None, 
            size : 300.0, 
            result_channel: None,
            content_path: None, 
            style_path: None,
            content_channel: None, 
            style_channel: None,
            selected: None,
            files: Vec::new(), 
            tick: 0, 
        } ) 
    )).unwrap(); 
}

pub struct App {
    content_channel: Option<mpsc::Receiver<(ImageBuffer<Rgba<u8>, Vec<u8>>, PathBuf)>>, 
    style_channel: Option<mpsc::Receiver<(ImageBuffer<Rgba<u8>, Vec<u8>>, PathBuf)>>, 
    result_channel: Option<mpsc::Receiver<ImageBuffer<Rgba<u8>, Vec<u8>>>>, 
    content_texture_id: Option<TextureHandle>,
    style_texture_id: Option<TextureHandle>, 
    result_texture_id : Option<TextureHandle>,
    size: f32, 
    content_path: Option<PathBuf>,
    style_path: Option<PathBuf>, 
    selected: Option<PathBuf>, 
    files: Vec<PathBuf>,
    tick: u128, 
}

pub enum ImagePath {
    Content, 
    Style,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        if let Some(path_channel) = &mut self.content_channel {
            if let Ok((t, p)) = path_channel.try_recv() {
                // load the image 
                let ci = ColorImage::from_rgba_unmultiplied([t.width() as usize, t.height() as usize], &t); 
                let texture_id = ctx.load_texture("content", ci, TextureOptions::LINEAR); 
                self.content_texture_id = Some(texture_id);
                self.content_path = Some(p);

            }  
        }
        if let Some(path_channel) = &mut self.style_channel {
            if let Ok((t, p)) = path_channel.try_recv() {
                // load the image 
                let ci = ColorImage::from_rgba_unmultiplied([t.width() as usize, t.height() as usize], &t); 
                let texture_id = ctx.load_texture("style", ci, TextureOptions::LINEAR); 
                self.style_texture_id = Some(texture_id);
                self.style_path = Some(p);
            }  
        }
        if let Some(result_channel) = &mut self.result_channel {
            if let Ok(t) = result_channel.try_recv() {
                let image = t; 
                let ci = ColorImage::from_rgba_unmultiplied([image.width() as usize, image.height() as usize], &image);
                let texture_id = ctx.load_texture("result", ci, TextureOptions::LINEAR);
                self.result_texture_id = Some(texture_id); 
            }
        }
        if self.tick % 300 == 0 {
            let files = std::fs::read_dir("./src"); 
            if let Ok(files) = files {
                let mut files = files.map(|x| x.unwrap().path())
                    .filter(|x| x.extension() == Some(std::ffi::OsStr::new("py")))
                    .collect::<Vec<_>>(); 
                files.sort(); 
                self.files = files;  
            }
        } 
        SidePanel::left("script_active").show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                for file in &self.files {
                    let is_eq = self.selected.as_ref().map(|x| x == file).unwrap_or(false); 
                    let button = ui.add(widgets::SelectableLabel::new(is_eq, file.to_string_lossy())); 
                    if button.clicked() {
                        self.selected = Some(file.clone()); 
                    }
                }
            });
        });
        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| ui.heading("Image Style Transfer"));
            // ui.heading("Image Style Transfer"); 
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    // add a box, with a middle button (if non image is loaded), or clickable image (if image is loaded) 
                    let click = ui.add(widgets::ImageButton::new(
                        if let Some(id) = &self.content_texture_id {
                            id.id()
                        } else {
                            TextureId::default()
                        }, (self.size, self.size))
                    ); 
                    if click.clicked() {
                        // open a file dialog, and load the image 
                        let task = rfd::AsyncFileDialog::new()
                            .set_directory(current_dir().unwrap_or(".".into()))
                            .add_filter("Images", &["jpg", "jpeg", "png"])
                            .pick_files(); 
                        let (tx, rx) = mpsc::channel(); 
                        self.content_channel = Some(rx); 
                        std::thread::spawn(move || {
                            let task = futures::executor::block_on(task); 
                            if let Some(path) = task {
                                dbg!(&path);
                                if let Some(path) = path.into_iter().nth(0) {
                                    let image = image::open(path.path()); 
                                    if let Ok(image) = image {
                                        let _ = tx.send((image.to_rgba8(), PathBuf::from(path.path()))); 
                                    } else {
                                        eprintln!("Error: {:?}", image.err()); 
                                    } 
                                    return ; 
                                }
                            } 
                        }); 
                    } 
                    
                    // add a box, with a middle button (if non image is loaded), or clickable image (if image is loaded) 
                    let click = ui.add(widgets::ImageButton::new(
                        if let Some(id) = &self.style_texture_id {
                            id.id()
                        } else {
                            TextureId::default()
                        }, (self.size, self.size))
                    ); 
                    if click.clicked() {
                        // open a file dialog, and load the image 
                        let task = rfd::AsyncFileDialog::new()
                            .set_directory(current_dir().unwrap_or(".".into()))
                            .add_filter("Images", &["jpg", "jpeg", "png"])
                            .pick_files(); 
                        let (tx, rx) = mpsc::channel(); 
                        self.style_channel = Some(rx); 
                        std::thread::spawn(move || {
                            let task = futures::executor::block_on(task); 
                            if let Some(path) = task {
                                if let Some(path) = path.into_iter().nth(0) {
                                    let image = image::open(path.path()); 
                                    if let Ok(image) = image {
                                        let _ = tx.send((image.to_rgba8(), PathBuf::from(path.path()))); 
                                    } else {
                                        eprintln!("Error: {:?}", image.err()); 
                                    } 
                                    return ; 
                                }
                            } 
                        }); 
                    } 
                }); 
            });
            let p = ui.button("Transfer").on_hover_text("Transfer the style of the style image to the content image");
            if p.clicked() {
                let (tx, rx) = mpsc::channel(); 
                self.result_channel = Some(rx); 
                let p; 
                match (&self.content_path, &self.style_path) {
                    (Some(content_path), Some(style_path)) => {
                        p = Some((content_path.clone(), style_path.clone())); 
                    },
                    (Some(content_path), None) => {
                        p = Some((content_path.clone(), content_path.clone())); 
                    },
                    _ => {
                        p = None;  
                    }
                } 
                'big_if: {
                    if let Some((p1, p2)) = p {
                        let ps1; 
                        let ps2; 
                        if let Some(p1) = p1.to_str() {
                            ps1 = p1; 
                        } else {
                            break 'big_if;         
                        }
                        if let Some(p2) = p2.to_str() {
                            ps2 = p2; 
                        } else {
                            break 'big_if; 
                        } 
                        let ps1 = ps1.to_string(); 
                        let ps2 = ps2.to_string(); 
                        dbg!(&ps1); 
                        dbg!(&ps2);
                        let select = self.selected.clone(); 
                        dbg!(&select);
                        std::thread::spawn(move || {
                            println!("executing python script");
                            // execute! 
                            #[cfg(target_os = "windows")]
                            const EXECUTABLE: &str = "python.exe"; 
                            #[cfg(not(target_os = "windows"))]
                            const EXECUTABLE: &str = "python"; 
                            let s; 
                            if let Some(select) = select {
                                s = select; 
                            } else {
                                return ; 
                            }
                            let file_name = s.file_name().unwrap_or_default().to_string_lossy().to_string(); 
                            let file_name = format!("src/{}", file_name);
                            let progress = process::Command::new(EXECUTABLE)
                                .arg(&file_name)
                                .arg("res/output.jpg")
                                .arg(format!("file://{}", ps1))
                                .arg(format!("file://{}", ps2))
                                .spawn(); 
                            match progress {
                                Ok(mut c) => {
                                    let r = c.wait();
                                    if let Ok(r) = r {
                                        dbg!(&r); 
                                        if !r.success() {
                                            return 
                                        }
                                        let image = image::open("res/output.jpg"); 
                                        if let Ok(image) = image {
                                            let _ = tx.send(image.to_rgba8()); 
                                        } 
                                    }
                                },
                                Err(err) => {
                                    dbg!(err);  
                                },
                            }
                        });
                    }

                }
            }
            if let Some(id) = &self.result_texture_id {
                let click = ui.add(widgets::ImageButton::new(id.id(), (self.size, self.size))); 
                if click.clicked() {
                    std::thread::spawn(|| {
                        let clipboard = Clipboard::new(); 
                        if let Ok(mut clip) = clipboard {
                            let image = image::open("res/output.jpg");
                            if let Ok(image) = image {
                                let image = image.to_rgba8(); 
                                let _ = clip.set_image(arboard::ImageData {
                                    width: image.width() as usize,
                                    height: image.height() as usize, 
                                    bytes: {
                                        let i: Vec<_> = image.into_raw().into_iter().map(|x| x as u8).collect(); 
                                        Cow::Owned(i)
                                    }
                                }); 
                            } 
                        }
                    }); 
                } 
            } else {
                // waiting scrolling animations 
                ui.add_sized([self.size, self.size], Spinner::new());
            }
        }); 
    }
}