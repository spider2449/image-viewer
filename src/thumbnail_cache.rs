use crate::disk_cache::DiskCache;
use egui::ColorImage;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub struct ThumbnailRequest {
    pub path: PathBuf,
    pub max_size: u32,
}

struct StoreRequest {
    path: PathBuf,
    max_size: u32,
    image: ColorImage,
}

pub struct ThumbnailResult {
    pub path: PathBuf,
    pub image: Option<ColorImage>,
    #[allow(dead_code)]
    pub full_width: u32,
    #[allow(dead_code)]
    pub full_height: u32,
    #[allow(dead_code)]
    pub load_time: Duration,
}

pub struct ThumbnailCache {
    cache: Arc<Mutex<LruCache<PathBuf, (ColorImage, u32, u32)>>>,
    pending: Arc<Mutex<Vec<PathBuf>>>,
    sender: Sender<ThumbnailRequest>,
    receiver: Receiver<ThumbnailResult>,
    disk_cache: Option<Arc<DiskCache>>,
}

impl ThumbnailCache {
    pub fn new(capacity: usize, worker_count: usize, disk_cache: Option<DiskCache>) -> Self {
        let (req_tx, req_rx) = mpsc::channel::<ThumbnailRequest>();
        let (res_tx, res_rx) = mpsc::channel::<ThumbnailResult>();
        let (store_tx, store_rx) = mpsc::channel::<StoreRequest>();

        let cache = Arc::new(Mutex::new(
            LruCache::new(NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(256).unwrap())),
        ));
        let pending: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));

        let disk_cache = disk_cache.map(Arc::new);
        let req_rx = Arc::new(Mutex::new(req_rx));

        // Dedicated store thread — sequential disk writes to avoid contention
        if let Some(ref dc) = disk_cache {
            let dc = dc.clone();
            let store_rx = Arc::new(Mutex::new(store_rx));
            thread::spawn(move || {
                loop {
                    let req = {
                        let rx = store_rx.lock().unwrap();
                        match rx.try_recv() {
                            Ok(r) => Some(r),
                            Err(TryRecvError::Empty) => None,
                            Err(TryRecvError::Disconnected) => break,
                        }
                    };
                    match req {
                        Some(req) => {
                            dc.store(&req.path, req.max_size, &req.image);
                        }
                        None => {
                            thread::sleep(Duration::from_millis(10));
                        }
                    }
                }
            });
        }

        // Decoder threads — decode only, no disk I/O for store
        for _ in 0..worker_count.max(1) {
            let cache_clone = cache.clone();
            let res_tx = res_tx.clone();
            let req_rx = req_rx.clone();
            let dc = disk_cache.clone();
            let store_tx = store_tx.clone();

            thread::spawn(move || {
                loop {
                    let req = {
                        let rx = req_rx.lock().unwrap();
                        match rx.try_recv() {
                            Ok(r) => Some(r),
                            Err(TryRecvError::Empty) => None,
                            Err(TryRecvError::Disconnected) => break,
                        }
                    };
                    match req {
                        Some(req) => {
                            let start = Instant::now();

                            // Check disk cache first
                            let from_disk = dc.as_ref()
                                .and_then(|d| d.lookup(&req.path, req.max_size));

                            let result = match from_disk {
                                Some(ci) => {
                                    let w = ci.size[0] as u32;
                                    let h = ci.size[1] as u32;
                                    {
                                        let mut c = cache_clone.lock().unwrap();
                                        c.put(req.path.clone(), (ci.clone(), w, h));
                                    }
                                    ThumbnailResult {
                                        path: req.path,
                                        image: Some(ci),
                                        full_width: w,
                                        full_height: h,
                                        load_time: start.elapsed(),
                                    }
                                }
                                None => {
                                    let result = crate::image_loader::load_thumbnail(&req.path, req.max_size);
                                    match result {
                                        Ok((ci, w, h)) => {
                                            // Enqueue disk write — non-blocking, store thread handles it
                                            store_tx.send(StoreRequest {
                                                path: req.path.clone(),
                                                max_size: req.max_size,
                                                image: ci.clone(),
                                            }).ok();
                                            {
                                                let mut c = cache_clone.lock().unwrap();
                                                c.put(req.path.clone(), (ci.clone(), w, h));
                                            }
                                            ThumbnailResult {
                                                path: req.path,
                                                image: Some(ci),
                                                full_width: w,
                                                full_height: h,
                                                load_time: start.elapsed(),
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("[thumbnail] decode failed for {:?}: {e}", req.path);
                                            ThumbnailResult {
                                                path: req.path,
                                                image: None,
                                                full_width: 0,
                                                full_height: 0,
                                                load_time: start.elapsed(),
                                            }
                                        }
                                    }
                                }
                            };
                            res_tx.send(result).ok();
                        }
                        None => {
                            thread::sleep(Duration::from_millis(10));
                        }
                    }
                }
            });
        }

        Self {
            cache,
            pending,
            sender: req_tx,
            receiver: res_rx,
            disk_cache,
        }
    }

    pub fn request(&self, path: PathBuf, max_size: u32) {
        {
            let mut p = self.pending.lock().unwrap();
            if p.contains(&path) {
                return;
            }
            p.push(path.clone());
        }
        self.sender.send(ThumbnailRequest { path, max_size }).ok();
    }

    pub fn poll(&self) -> Option<ThumbnailResult> {
        let result = self.receiver.try_recv().ok();
        if let Some(ref r) = result {
            let mut p = self.pending.lock().unwrap();
            p.retain(|x| x != &r.path);
        }
        result
    }

    #[allow(dead_code)]
    pub fn get_cached(&self, path: &PathBuf) -> Option<(ColorImage, u32, u32)> {
        self.cache.lock().unwrap().get(path).cloned()
    }

    pub fn clear_disk_cache(&self) {
        if let Some(ref dc) = self.disk_cache {
            dc.clear_all();
        }
    }
}
