use std::env::current_dir;
use std::ffi::OsString;
use std::process::Command;
use std::thread;
use std::time::Duration;

use eframe::App;
use eframe::egui::{SidePanel, RichText, Button, Layout, Spinner, widgets, TextureOptions, Sense};
use eframe::egui;
use eframe::epaint::{TextureHandle, ColorImage};
use futures::channel::mpsc::{Receiver, Sender};
use futures::channel::oneshot;
use image::{ImageBuffer, Rgba};
use image_transfer::image_mode::ImageMode;

const PY_SCRIPT_FLUSH_TIME : Duration = Duration::from_secs(1); 
const NORMAL_SCRIPT_FLUSH_TIME : Duration = Duration::from_secs(1); 
const TIME_SLICE : Duration = Duration::from_millis(100); 

pub fn main() {
    println!("Hello, world!"); 
    // Python scripts checking 
    let (py_script_updates_tx, py_script_updates_rx) = futures::channel::mpsc::channel(1); 
    let (py_script_checker_tx, py_script_checker_rx) = futures::channel::mpsc::channel(1); 
    std::thread::spawn(move || {
        let mut py_script_updates_tx = py_script_updates_tx; 
        let mut py_script_checker_rx = py_script_checker_rx; 
        let mut clock : Duration = Duration::from_secs(0); 
        let mut is_flush; 
        loop {
            is_flush = false; 
            match py_script_checker_rx.try_next() {
                Ok(Some(())) => {
                    is_flush = true;  
                }
                Ok(None) => break, 
                _ => (),
            } 
            clock += TIME_SLICE; 
            if clock >= PY_SCRIPT_FLUSH_TIME || is_flush {
                clock = Duration::from_secs(0); 
            } else {
                std::thread::sleep(TIME_SLICE); 
                continue; 
            } 
            let read_dir = std::fs::read_dir("./pyscripts"); 
            let s; 
            match read_dir {
                Ok(dir) => {
                    let v : Vec<_> = 
                        dir.into_iter().flat_map(|f| f.ok().map(|f| f.path())).filter(|f| f.extension() == Some("py".as_ref()))
                            .map(OsString::from)
                            .map(|f| f.to_string_lossy().into_owned())
                            .collect(); 
                    s = py_script_updates_tx.try_send(v); 
                }
                Err(_) => {
                    s = py_script_updates_tx.try_send(Vec::new()); 
                }
            }
            let _ = s; 
        }
        dbg!("python scripts checking thread exit.");
    }); 
    // Native scripts checking 
    let (native_script_updates_tx, native_script_updates_rx) = futures::channel::mpsc::channel(1); 
    let (native_script_checker_tx, native_script_checker_rx) = futures::channel::mpsc::channel(1); 
    std::thread::spawn(move || {
        let mut native_script_updates_tx = native_script_updates_tx; 
        let mut native_script_checker_rx = native_script_checker_rx; 
        let mut clock : Duration = Duration::from_secs(0); 
        let mut is_flush; 
        loop {
            is_flush = false; 
            match native_script_checker_rx.try_next() {
                Ok(Some(())) => {
                    is_flush = true;  
                }
                Err(_) => break, 
                _ => (),
            } 
            clock += TIME_SLICE; 
            if clock >= NORMAL_SCRIPT_FLUSH_TIME || is_flush {
                clock = Duration::from_secs(0); 
            } else {
                std::thread::sleep(TIME_SLICE); 
                continue; 
            } 
            let read_dir = std::fs::read_dir("./nativescripts"); 
            let s; 
            match read_dir {
                Ok(dir) => {
                    let v : Vec<_> = 
                        dir.into_iter().flat_map(|f| f.ok().map(|f| f.path())).filter(|f| f.extension() == Some("rs".as_ref()))
                            .map(OsString::from)
                            .map(|f| f.to_string_lossy().into_owned())
                            .collect(); 
                    s = native_script_updates_tx.try_send(v); 
                }
                Err(_) => {
                    s = native_script_updates_tx.try_send(Vec::new()); 
                }
            }
            if s.is_err() {
                break; 
            }
        }
    }); 
    let app = MyApp {
        py_script_updates: (py_script_updates_rx, py_script_checker_tx), 
        native_script_updates: (native_script_updates_rx, native_script_checker_tx), 
        active_py_script: None, 
        active_native_script: None, 
        py_executor: None, 
        is_native_mode: false,
        py_lists: Vec::new(), 
        native_lists: Vec::new(),
        image_mode: ImageMode::BiImage,
        input_image_single: None,
        input_image_bi: (None, None), 
        output_image_none: None,
        output_image_single: None,
        output_image_bi: None,
        input_image_singal_rx: None, 
        output_image_singal_rx: None,
        output_image_none_rx: None,
        input_image_bi1_rx: None, 
        input_image_bi2_rx: None, 
        output_image_bi_rx: None,
        movable_image_display: false,
        extra_arguments: String::new(), 
    }; 
    let mut native_options = eframe::NativeOptions::default(); 
    native_options.initial_window_size = Some(egui::Vec2::new(1024.0, 768.0)); 
    eframe::run_native("Image Transfer", native_options, Box::new( |_| Box::new(app))).unwrap(); 
}

pub struct MyApp {
    /// Python 脚本更新线程
    pub py_script_updates: (Receiver<Vec<String>>, Sender<()>), 
    /// Native 脚本更新线程 
    pub native_script_updates: (Receiver<Vec<String>>, Sender<()>), 
    /// 当前激活的 Python 脚本 
    pub active_py_script: Option<String>, 
    /// 当前激活的 Native 脚本 
    pub active_native_script: Option<String>, 
    /// 当前使用的 Python 解释器路径；None 则尝试本路径下的 python 程序 / python.exe (in windows)
    pub py_executor : Option<String>, 
    /// 当前模式：Python 或 Native 
    pub is_native_mode: bool, 
    /// 当前 Python 脚本列表
    pub py_lists: Vec<String>, 
    /// 当前 native 脚本列表
    pub native_lists: Vec<String>, 
    /// 当前图像模式
    pub image_mode: ImageMode, 
    /// 当前的输入图像 
    pub input_image_single: Option<(TextureHandle, String)>, 
    /// 当前的输出图像 模式 2 
    pub input_image_bi: (Option<(TextureHandle, String)>, Option<(TextureHandle, String)>), 
    /// 当前的输出图像 None 
    pub output_image_none: Option<(TextureHandle, String)>, 
    /// 当前的输出图像 模式 1 
    pub output_image_single: Option<(TextureHandle, String)>, 
    /// 当前的输出图像 模式 2 
    pub output_image_bi: Option<(TextureHandle, String)>, 
    /// single 模式输入图像通道 
    pub input_image_singal_rx: Option<oneshot::Receiver<(ImageBuffer<Rgba<u8>, Vec<u8>>, String)>>, 
    /// single 模式输出图像通道
    pub output_image_singal_rx: Option<oneshot::Receiver<(ImageBuffer<Rgba<u8>, Vec<u8>>, String)>>, 
    /// None 模式输出图像通道
    pub output_image_none_rx: Option<oneshot::Receiver<(ImageBuffer<Rgba<u8>, Vec<u8>>, String)>>, 
    /// bi 模式输入图像通道 1 
    pub input_image_bi1_rx: Option<oneshot::Receiver<(ImageBuffer<Rgba<u8>, Vec<u8>>, String)>>, 
    /// bi 模式输入图像通道 2 
    pub input_image_bi2_rx: Option<oneshot::Receiver<(ImageBuffer<Rgba<u8>, Vec<u8>>, String)>>, 
    /// bi 模式输出图像通道 
    pub output_image_bi_rx: Option<oneshot::Receiver<(ImageBuffer<Rgba<u8>, Vec<u8>>, String)>>, 
    /// 可移除已经装载的任务
    pub movable_image_display: bool, 
    /// 额外参数
    pub extra_arguments: String, 
}

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 检查 Python 脚本更新 
        match self.py_script_updates.0.try_next() {
            Ok(Some(v)) => {
                self.py_lists = v; 
            }
            _ => (),
        } 
        // 检查 Native 脚本更新 
        match self.native_script_updates.0.try_next() {
            Ok(Some(v)) => {
                self.native_lists = v; 
            }
            _ => (),
        } 
        match self.input_image_singal_rx {
            Some(ref mut rx) => {
                match rx.try_recv() {
                    Ok(None) => (), 
                    Ok(Some((ib, n))) => {
                        let ci = ColorImage::from_rgba_unmultiplied([ib.width() as usize, ib.height() as usize], &ib); 
                        let tex = ctx.load_texture(n.clone(), ci, TextureOptions::LINEAR); 
                        self.input_image_single = Some((tex, n)); 
                    }
                    Err(_) => {
                        self.input_image_singal_rx = None; 
                        if self.movable_image_display {
                            self.input_image_single = None; 
                        }
                    } 
                } 
            },
            None => {},
        }
        match self.input_image_bi1_rx {
            Some(ref mut rx) => {
                match rx.try_recv() {
                    Ok(None) => (), 
                    Ok(Some((ib, n))) => {
                        let ci = ColorImage::from_rgba_unmultiplied([ib.width() as usize, ib.height() as usize], &ib); 
                        let tex = ctx.load_texture(n.clone(), ci, TextureOptions::LINEAR); 
                        self.input_image_bi.0 = Some((tex, n)); 
                    }
                    Err(_) => {
                        self.input_image_bi1_rx = None; 
                        if self.movable_image_display {
                            self.input_image_bi.0 = None; 
                        }
                    } 
                } 
            },
            None => {}, 
        }
        match self.input_image_bi2_rx {
            Some(ref mut rx) => {
                match rx.try_recv() {
                    Ok(None) => (), 
                    Ok(Some((ib, n))) => {
                        let ci = ColorImage::from_rgba_unmultiplied([ib.width() as usize, ib.height() as usize], &ib); 
                        let tex = ctx.load_texture(n.clone(), ci, TextureOptions::LINEAR); 
                        self.input_image_bi.1 = Some((tex, n)); 
                    }
                    Err(_) => {
                        self.input_image_bi2_rx = None; 
                        if self.movable_image_display {
                            self.input_image_bi.1 = None; 
                        }
                    } 
                } 
            },
            None => {},  
        }
        match self.output_image_singal_rx {
            Some(ref mut rx) => {
                match rx.try_recv() {
                    Ok(None) => (),
                    Ok(Some((ib, n))) => {
                        let ci = ColorImage::from_rgba_unmultiplied([ib.width() as usize, ib.height() as usize], &ib); 
                        let tex = ctx.load_texture(n.clone(), ci, TextureOptions::LINEAR); 
                        self.output_image_single = Some((tex, n)); 
                    },
                    Err(_) => {
                        self.output_image_singal_rx = None; 
                        if self.movable_image_display {
                            self.output_image_single = None; 
                        } 
                    },
                } 
            },
            None => {},
        }
        match self.output_image_none_rx {
            Some(ref mut rx) => {
                match rx.try_recv() {
                    Ok(None) => (), 
                    Ok(Some((ib, n))) => {
                        let ci = ColorImage::from_rgba_unmultiplied([ib.width() as usize, ib.height() as usize], &ib); 
                        let tex = ctx.load_texture(n.clone(), ci, TextureOptions::LINEAR); 
                        self.output_image_none = Some((tex, n)); 
                    },
                    Err(_) => {
                        self.output_image_none_rx = None; 
                        if self.movable_image_display {
                            self.output_image_none = None; 
                        } 
                    },
                } 
            },
            None => {}, 
        }
        SidePanel::left("script_panel").show(ctx, |ui| {
            let display_python = !self.is_native_mode; 
            egui::ScrollArea::vertical().show(ui, |ui| {
                if display_python {
                    for py in self.py_lists.iter() {
                        let select = self.active_py_script.as_ref().map(|s| s == py).unwrap_or(false); 
                        let select = ui.selectable_label(select, py); 
                        if select.clicked() {
                            self.active_py_script = Some(py.clone());  
                        }
                    }
                } else {
                    for native in self.native_lists.iter() {
                        let select = self.active_native_script.as_ref().map(|s| s == native).unwrap_or(false); 
                        let select = ui.selectable_label(select, native); 
                        if select.clicked() {
                            self.active_native_script = Some(native.clone());  
                        }
                    }  
                }
            });
        });
        SidePanel::right("options_panel").min_width(110.).default_width(110.).show(ctx, |ui| {
            let text: RichText; 
            if self.is_native_mode {
                text = "Native Mode".into(); 
            } else {
                text = "Python Mode".into();  
            }
            ui.add_space(10.);
            ui.label("Mode: ");
            let selected = ui.add(egui::Button::new(text).min_size([90.0, 25.0].into())); 
            if selected.clicked() {
                self.is_native_mode = !self.is_native_mode; 
            }
            ui.separator(); 
            ui.add_space(10.); 
            let flush = ui.add(Button::new("Flush Scripts").min_size([90.0, 25.0].into())
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 0, 0))));
            if flush.clicked() { 
                if self.is_native_mode {
                    let _ = self.native_script_updates.1.try_send(()); 
                } else {
                    let _ = self.py_script_updates.1.try_send(()); 
                } 
            } 
            ui.add_space(40.); 
            ui.label("Image Input Mode: "); 
            ui.separator(); 
            ui.radio_value(&mut self.image_mode, ImageMode::None, "None Image Mode");  
            ui.radio_value(&mut self.image_mode, ImageMode::SingleImage, "Single Image Mode"); 
            ui.radio_value(&mut self.image_mode, ImageMode::BiImage, "Bi-Image Mode");  
            ui.add_space(30.); 
            ui.horizontal(|ui| {
                ui.label("Image Unload Allowed: ");
                let c = ui.selectable_label(self.movable_image_display, "Yes");
                if c.clicked() {
                    self.movable_image_display = !self.movable_image_display; 
                }
            }); 
            ui.separator();
            ui.add_space(30.); 
            let can_execute = false; 
            let r = ui.add_enabled(can_execute, Button::new("Execute"));
            #[cfg(target_os = "windows")]
            const DEFAULT_PYTHON_EXECUTOR : &str = "python.exe"; 
            #[cfg(not(target_os = "windows"))]
            const DEFAULT_PYTHON_EXECUTOR : &str = "python"; 
            if r.clicked() {
                || -> () {
                    if let Some(ref s) = self.active_py_script {
                        let mut cmd; 
                        if self.is_native_mode {
                            match self.active_native_script {
                                Some(ref n) => {
                                    cmd = Command::new(n); 
                                },
                                None => {
                                    return ; 
                                },
                            }
                        } else {
                            cmd = Command::new(self.py_executor.as_ref().map(|s| s.as_str()).unwrap_or(DEFAULT_PYTHON_EXECUTOR)); 
                            cmd.arg(s); 
                        }
                        cmd.arg("./outcome/result.jpg");
                        match self.image_mode {
                            ImageMode::None => (), 
                            ImageMode::SingleImage => {
                                if let Some((_, ref n)) = self.input_image_single {
                                    cmd.arg(n); 
                                } else {
                                    return ; 
                                }
                            }
                            ImageMode::BiImage => {
                                if let Some((_, ref n)) = self.input_image_bi.0 {
                                    cmd.arg(n); 
                                } else {
                                    return ; 
                                }
                                if let Some((_, ref n)) = self.input_image_bi.1 {
                                    cmd.arg(n); 
                                } else {
                                    return ; 
                                } 
                            }
                        }
                        if !self.extra_arguments.is_empty() {
                            cmd.arg(self.extra_arguments.as_str()); 
                        }
                        let (tx, rx) = oneshot::channel(); 
                        match self.image_mode {
                            ImageMode::None => {
                                self.output_image_none_rx = Some(rx);  
                            }
                            ImageMode::SingleImage => {
                                self.output_image_singal_rx = Some(rx);  
                            }
                            ImageMode::BiImage => {
                                self.output_image_bi_rx = Some(rx);   
                            }
                        }
                        thread::spawn(move || {
                            let mut cmd = cmd; 
                            let tx = tx; 
                            let cmd = cmd.status();  
                            match cmd {
                                Ok(e) => {
                                    if e.success() {
                                        let image = image::open("./outcome/result.jpg"); 
                                        if let Ok(image) = image {
                                            let _ = tx.send((image.to_rgba8(), "./outcome/result.jpg".to_string())); 
                                        } else {
                                            eprintln!("Error: {:?}", image.err()); 
                                        } 
                                    } else {
                                        eprintln!("Error: {:?}", e);  
                                    }
                                },
                                Err(_) => {},
                            } 
                        });
                    }
                }(); 
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello World!"); 
            ui.with_layout(Layout::top_down_justified(eframe::emath::Align::Center), |ui| {
                match self.image_mode {
                    ImageMode::None => {
                        ui.label("[Mode] No Image Selected. ");
                    }
                    ImageMode::SingleImage => {
                        let click; 
                        match self.input_image_single {
                            Some((ref t, _)) => {
                                // ui.image(t, [300., 300.]);
                                let k = ui.add_sized([300., 300.], widgets::ImageButton::new(t, [300., 300.])); 
                                click = k.clicked(); 
                            },
                            None => {
                                let u = ui.allocate_response([300., 300.].into(), Sense::click()); 
                                ui.put(u.rect, Spinner::new()); 
                                click = u.clicked(); 
                                // click = ui.add_sized([300., 300.], Spinner::new()); 
                            },
                        }
                        if click {
                            let (tx, rx) = oneshot::channel(); 
                            self.input_image_singal_rx = Some(rx); 
                            std::thread::spawn(move || {
                                let task = rfd::AsyncFileDialog::new()
                                    .set_directory(current_dir().unwrap_or("~".into()))
                                    .add_filter("Images", &["jpg", "jpeg", "png"])
                                    .pick_files(); 
                                let task = futures::executor::block_on(task); 
                                // dbg!(&task);
                                if let Some(path) = task {
                                    if path.len() != 1 {
                                        return ; 
                                    }
                                    if let Some(path) = path.into_iter().nth(0) {
                                        let path_str = path.path().to_string_lossy().into_owned(); 
                                        let image = image::open(path.path()); 
                                        if let Ok(image) = image {
                                            let _ = tx.send((image.to_rgba8(), path_str)); 
                                        } else {
                                            eprintln!("Error: {:?}", image.err()); 
                                        } 
                                        return ; 
                                    }
                                } 
                            }); 
                        }
                    }
                    ImageMode::BiImage => {
                        ui.allocate_ui_with_layout([700., 350.].into(), Layout::left_to_right(eframe::emath::Align::Center), |ui| {
                        // ui.with_layout(Layout::left_to_right(eframe::emath::Align::Center), |ui| {
                            // add two spinners, and handle the click event for select images 
                            let click1; 
                            match self.input_image_bi.0 {
                                Some((ref t, _)) => {
                                    let k = ui.add_sized([300., 300.], widgets::ImageButton::new(t, [300., 300.])); 
                                    click1 = k.clicked(); 
                                },
                                None => {
                                    let u = ui.allocate_response([300., 300.].into(), Sense::click()); 
                                    ui.put(u.rect, Spinner::new()); 
                                    click1 = u.clicked(); 
                                },
                            } 
                            let click2; 
                            match self.input_image_bi.1 {
                                Some((ref t, _)) => {
                                    let k = ui.add_sized([300., 300.], widgets::ImageButton::new(t, [300., 300.])); 
                                    click2 = k.clicked(); 
                                },
                                None => {
                                    let u = ui.allocate_response([300., 300.].into(), Sense::click()); 
                                    ui.put(u.rect, Spinner::new()); 
                                    click2 = u.clicked(); 
                                },
                            } 
                            if click1 {
                                let (tx, rx) = oneshot::channel(); 
                                self.input_image_bi1_rx = Some(rx); 
                                std::thread::spawn(move || {
                                    let task = rfd::AsyncFileDialog::new()
                                        .set_directory(current_dir().unwrap_or("~".into()))
                                        .add_filter("Images", &["jpg", "jpeg", "png"])
                                        .pick_files(); 
                                    let task = futures::executor::block_on(task); 
                                    if let Some(path) = task {
                                        if path.len() != 1 {
                                            return ; 
                                        }
                                        if let Some(path) = path.into_iter().nth(0) {
                                            let path_str = path.path().to_string_lossy().into_owned(); 
                                            let image = image::open(path.path()); 
                                            if let Ok(image) = image {
                                                let _ = tx.send((image.to_rgba8(), path_str)); 
                                            } else {
                                                eprintln!("Error: {:?}", image.err()); 
                                            } 
                                            return ; 
                                        }
                                    } 
                                });  
                            } 
                            if click2 {
                                let (tx, rx) = oneshot::channel(); 
                                self.input_image_bi2_rx = Some(rx); 
                                std::thread::spawn(move || {
                                    let task = rfd::AsyncFileDialog::new()
                                        .set_directory(current_dir().unwrap_or("~".into()))
                                        .add_filter("Images", &["jpg", "jpeg", "png"])
                                        .pick_files(); 
                                    let task = futures::executor::block_on(task); 
                                    if let Some(path) = task {
                                        if path.len() != 1 {
                                            return ; 
                                        }
                                        if let Some(path) = path.into_iter().nth(0) {
                                            let path_str = path.path().to_string_lossy().into_owned(); 
                                            let image = image::open(path.path()); 
                                            if let Ok(image) = image {
                                                let _ = tx.send((image.to_rgba8(), path_str)); 
                                            } else {
                                                eprintln!("Error: {:?}", image.err()); 
                                            } 
                                            return ; 
                                        }
                                    } 
                                }); 
                            }
                        });
                    }
                }
            }); 
        }); 
    }
}