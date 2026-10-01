//! Background, streaming downloads with no size cap or overall timeout.
//! Bytes are streamed straight to a `.part` file (never buffered in memory)
//! and renamed into place when complete, so arbitrarily large files work.

use std::{
    fs::{self, File},
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, TryRecvError},
        Arc,
    },
    thread,
    time::Duration,
};

const CHUNK: usize = 256 * 1024;

#[derive(Default)]
struct Progress {
    received: AtomicU64,
    /// 0 when the server didn't announce a length.
    total: AtomicU64,
    cancelled: AtomicBool,
}

/// A download (or local file copy) running on a worker thread.
pub struct Download {
    /// The URL, or the source path for a copy.
    pub url: String,
    /// `true` for a local file being copied into an assets folder.
    pub is_copy: bool,
    progress: Arc<Progress>,
    result: Receiver<Result<PathBuf, String>>,
}

impl Download {
    /// Starts downloading `url` into `directory`.
    pub fn start(url: &str, directory: &Path) -> Self {
        let worker_url = url.to_owned();
        Self::spawn(
            url.to_owned(),
            false,
            directory,
            move |directory, progress| fetch(&worker_url, directory, progress),
        )
    }

    /// Starts copying the local file `source` into `directory`, streamed in
    /// chunks so files of any size copy without blocking the UI.
    pub fn copy(source: &Path, directory: &Path) -> Self {
        let worker_source = source.to_path_buf();
        Self::spawn(
            source.display().to_string(),
            true,
            directory,
            move |directory, progress| copy_into(&worker_source, directory, progress),
        )
    }

    fn spawn<F>(url: String, is_copy: bool, directory: &Path, work: F) -> Self
    where
        F: FnOnce(&Path, &Progress) -> Result<PathBuf, String> + Send + 'static,
    {
        let progress = Arc::new(Progress::default());
        let (sender, result) = mpsc::channel();
        let worker_progress = Arc::clone(&progress);
        let directory = directory.to_path_buf();
        thread::spawn(move || {
            let _ = sender.send(work(&directory, &worker_progress));
        });
        Self {
            url,
            is_copy,
            progress,
            result,
        }
    }

    /// `Some` once the download has finished (successfully or not).
    pub fn poll(&self) -> Option<Result<PathBuf, String>> {
        match self.result.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(format!(
                "{} worker for {} stopped",
                if self.is_copy { "copy" } else { "download" },
                self.url
            ))),
        }
    }

    /// `(bytes received, total bytes if known)`.
    pub fn progress(&self) -> (u64, Option<u64>) {
        let total = self.progress.total.load(Ordering::Relaxed);
        (
            self.progress.received.load(Ordering::Relaxed),
            (total > 0).then_some(total),
        )
    }

    pub fn cancel(&self) {
        self.progress.cancelled.store(true, Ordering::Relaxed);
    }

    /// Fraction complete in `0..=1` when the size is known.
    pub fn fraction(&self) -> Option<f32> {
        let (received, total) = self.progress();
        total.map(|total| (received as f64 / total as f64).clamp(0.0, 1.0) as f32)
    }

    pub fn describe(&self) -> String {
        let (received, total) = self.progress();
        let name = if self.is_copy {
            let name = Path::new(&self.url)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| self.url.clone());
            format!("Copying {}", shorten(&name))
        } else {
            display_name(&self.url)
        };
        match total {
            Some(total) => format!(
                "{name} {} / {} ({:.0}%)",
                human_bytes(received),
                human_bytes(total),
                received as f64 * 100.0 / total.max(1) as f64
            ),
            None => format!("{name} {}", human_bytes(received)),
        }
    }
}

pub fn is_url(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Converts `file://` URLs (as produced by file managers) to local paths.
pub fn file_url_to_path(text: &str) -> Option<PathBuf> {
    let rest = text.strip_prefix("file://")?;
    // Skip an optional host component ("file://localhost/...").
    let path = match rest.find('/') {
        Some(0) => rest,
        Some(index) => &rest[index..],
        None => return None,
    };
    Some(PathBuf::from(percent_decode(path)))
}

/// Splits clipboard text into individual URL / path entries.
pub fn clipboard_entries(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.trim_matches(|c| c == '"' || c == '\'' || c == '<' || c == '>'))
        .map(str::to_owned)
        .collect()
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn display_name(url: &str) -> String {
    shorten(&url_file_name(url).unwrap_or_else(|| url.to_owned()))
}

fn shorten(name: &str) -> String {
    if name.chars().count() > 40 {
        format!("{}…", name.chars().take(39).collect::<String>())
    } else {
        name.to_owned()
    }
}

/// `true` when `path` is a regular file outside `directory`, i.e. adding it
/// to a grid whose assets folder is `directory` should copy it in.
pub fn needs_copy(path: &Path, directory: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let file = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let directory = fs::canonicalize(directory).unwrap_or_else(|_| directory.to_path_buf());
    !file.starts_with(directory)
}

/// Copies `source` into `directory` right away (for scripts and tests),
/// returning the new file's path. Files already inside `directory` are
/// returned unchanged.
#[cfg_attr(not(feature = "scripting"), allow(dead_code))]
pub fn import_file(source: &Path, directory: &Path) -> Result<PathBuf, String> {
    if !source.is_file() {
        return Err(format!("{} is not a file", source.display()));
    }
    if !needs_copy(source, directory) {
        return Ok(source.to_path_buf());
    }
    copy_into(source, directory, &Progress::default())
}

/// Streams `source` into `directory` via a `.part` file, keeping its name
/// (made unique if taken).
fn copy_into(source: &Path, directory: &Path, progress: &Progress) -> Result<PathBuf, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    let mut input = File::open(source)
        .map_err(|error| format!("could not open {}: {error}", source.display()))?;
    if let Ok(meta) = input.metadata() {
        progress.total.store(meta.len(), Ordering::Relaxed);
    }
    let name = source
        .file_name()
        .and_then(|name| sanitize_file_name(&name.to_string_lossy()))
        .unwrap_or_else(|| format!("file-{}", crate::timestamp()));
    let (final_path, part_path, file) = loop {
        let final_path = numbered_path(directory, &name);
        let part_path = final_path.with_file_name(format!(
            "{}.part",
            final_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("copy")
        ));
        // `create_new` so two copies racing for one name never share a file.
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&part_path)
        {
            Ok(file) => break (final_path, part_path, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("could not create {}: {error}", part_path.display())),
        }
    };
    let mut writer = BufWriter::with_capacity(CHUNK, file);
    let mut buffer = vec![0u8; CHUNK];
    let outcome = loop {
        if progress.cancelled.load(Ordering::Relaxed) {
            break Err(format!("copy of {} cancelled", source.display()));
        }
        match input.read(&mut buffer) {
            Ok(0) => break writer.flush().map_err(|error| error.to_string()),
            Ok(count) => {
                if let Err(error) = writer.write_all(&buffer[..count]) {
                    break Err(format!("could not write {}: {error}", part_path.display()));
                }
                progress.received.fetch_add(count as u64, Ordering::Relaxed);
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => break Err(format!("could not read {}: {error}", source.display())),
        }
    };
    drop(writer);
    if let Err(error) = outcome {
        let _ = fs::remove_file(&part_path);
        return Err(error);
    }
    fs::rename(&part_path, &final_path)
        .map_err(|error| format!("could not finish {}: {error}", final_path.display()))?;
    Ok(final_path)
}

/// `directory/name`, or `name (2).ext`, `name (3).ext`… if taken (including
/// by a copy still in progress).
fn numbered_path(directory: &Path, name: &str) -> PathBuf {
    let free = |candidate: &Path| {
        let mut part = candidate.as_os_str().to_owned();
        part.push(".part");
        !candidate.exists() && !Path::new(&part).exists()
    };
    let candidate = directory.join(name);
    if free(&candidate) {
        return candidate;
    }
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_owned());
    let ext = path
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default();
    (2..)
        .map(|n| directory.join(format!("{stem} ({n}){ext}")))
        .find(|candidate| free(candidate))
        .expect("an unused name exists")
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).ok();
            if let Some(value) = hex.and_then(|hex| u8::from_str_radix(hex, 16).ok()) {
                out.push(value);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Strips anything that isn't safe in a file name on every platform.
fn sanitize_file_name(name: &str) -> Option<String> {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.').to_owned();
    if cleaned.is_empty() {
        return None;
    }
    // Keep names to a sane length while preserving the extension.
    if cleaned.chars().count() > 120 {
        let path = Path::new(&cleaned);
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .filter(|ext| ext.len() <= 10)
            .map(|ext| format!(".{ext}"))
            .unwrap_or_default();
        let stem: String = cleaned.chars().take(100).collect();
        return Some(format!("{stem}{ext}"));
    }
    Some(cleaned)
}

fn url_file_name(url: &str) -> Option<String> {
    let without_query = url.split(['?', '#']).next()?;
    let after_scheme = without_query
        .split_once("://")
        .map_or(without_query, |(_, rest)| rest);
    let (_, path) = after_scheme.split_once('/')?;
    let last = path.rsplit('/').find(|segment| !segment.is_empty())?;
    sanitize_file_name(&percent_decode(last))
}

fn content_disposition_name(value: &str) -> Option<String> {
    let mut plain = None;
    for part in value.split(';').map(str::trim) {
        let lower = part.to_ascii_lowercase();
        if let Some(encoded) = lower
            .starts_with("filename*=")
            .then(|| &part["filename*=".len()..])
        {
            // RFC 5987: charset'lang'percent-encoded
            let encoded = encoded.trim_matches('"');
            let value = encoded.rsplit('\'').next().unwrap_or(encoded);
            return sanitize_file_name(&percent_decode(value));
        }
        if lower.starts_with("filename=") {
            plain = sanitize_file_name(part["filename=".len()..].trim_matches('"'));
        }
    }
    plain
}

fn unique_path(directory: &Path, name: &str) -> PathBuf {
    let candidate = directory.join(name);
    if !candidate.exists() {
        return candidate;
    }
    directory.join(format!("{}-{name}", crate::timestamp()))
}

fn fetch(url: &str, directory: &Path, progress: &Progress) -> Result<PathBuf, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    let client = reqwest::blocking::Client::builder()
        .timeout(None::<Duration>)
        .connect_timeout(Duration::from_secs(30))
        .user_agent(concat!("Selenite/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("could not create HTTP client: {error}"))?;
    let mut response = client
        .get(url)
        .send()
        .map_err(|error| format!("could not download {url}: {error}"))?
        .error_for_status()
        .map_err(|error| format!("download failed for {url}: {error}"))?;

    if let Some(length) = response.content_length() {
        progress.total.store(length, Ordering::Relaxed);
    }
    let header = |name| {
        response
            .headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    };
    let content_type = header(reqwest::header::CONTENT_TYPE).unwrap_or_default();
    let mut name = header(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|value| content_disposition_name(&value))
        .or_else(|| url_file_name(response.url().as_str()))
        .or_else(|| url_file_name(url))
        .unwrap_or_else(|| format!("download-{}", crate::timestamp()));
    if Path::new(&name).extension().is_none() {
        name.push_str(crate::extension_for_content_type(&content_type));
    }

    let final_path = unique_path(directory, &name);
    let part_path = final_path.with_file_name(format!(
        "{}.part",
        final_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("download")
    ));
    let file = File::create(&part_path)
        .map_err(|error| format!("could not create {}: {error}", part_path.display()))?;
    let mut writer = BufWriter::with_capacity(CHUNK, file);
    let mut buffer = vec![0u8; CHUNK];
    let outcome = loop {
        if progress.cancelled.load(Ordering::Relaxed) {
            break Err(format!("download of {url} cancelled"));
        }
        match response.read(&mut buffer) {
            Ok(0) => break writer.flush().map_err(|error| error.to_string()),
            Ok(count) => {
                if let Err(error) = writer.write_all(&buffer[..count]) {
                    break Err(format!("could not write {}: {error}", part_path.display()));
                }
                progress.received.fetch_add(count as u64, Ordering::Relaxed);
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => break Err(format!("download of {url} interrupted: {error}")),
        }
    };
    drop(writer);
    if let Err(error) = outcome {
        let _ = fs::remove_file(&part_path);
        return Err(error);
    }
    fs::rename(&part_path, &final_path)
        .map_err(|error| format!("could not finish {}: {error}", final_path.display()))?;
    Ok(final_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_urls_and_entries() {
        assert_eq!(
            url_file_name("https://x.org/a/b/My%20Song.mp3?x=1#t").as_deref(),
            Some("My Song.mp3")
        );
        assert_eq!(url_file_name("https://x.org/"), None);
        assert_eq!(url_file_name("https://x.org"), None);
        assert_eq!(
            content_disposition_name("attachment; filename=\"clip.mp4\"").as_deref(),
            Some("clip.mp4")
        );
        assert_eq!(
            content_disposition_name("attachment; filename*=UTF-8''na%C3%AFve.png").as_deref(),
            Some("naïve.png")
        );
        assert_eq!(
            content_disposition_name("attachment; filename=\"../../etc/passwd\"").as_deref(),
            Some("_.._etc_passwd")
        );
        assert_eq!(
            file_url_to_path("file:///home/me/My%20File.txt"),
            Some(PathBuf::from("/home/me/My File.txt"))
        );
        assert_eq!(
            file_url_to_path("file://localhost/tmp/a.png"),
            Some(PathBuf::from("/tmp/a.png"))
        );
        assert!(is_url("HTTPS://example.com/x"));
        assert!(!is_url("ftp://example.com/x"));
        assert_eq!(
            clipboard_entries("  https://a/b.png \n\n# comment\n\"/tmp/x y.txt\"\n"),
            vec!["https://a/b.png".to_owned(), "/tmp/x y.txt".to_owned()]
        );
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1536), "1.5 KB");
        assert_eq!(human_bytes(5 * 1024 * 1024 * 1024), "5.0 GB");
    }

    #[test]
    fn streams_a_local_http_download_to_disk() {
        use std::{io::BufRead, net::TcpListener};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let body: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
        let served = body.clone();
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap() > 2 {
                line.clear();
            }
            let mut stream = stream;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: attachment; filename=\"big.bin\"\r\nConnection: close\r\n\r\n",
                served.len()
            )
            .unwrap();
            stream.write_all(&served).unwrap();
        });

        let directory = std::env::temp_dir().join(format!(
            "selenite-download-test-{}-{}",
            std::process::id(),
            crate::timestamp()
        ));
        let download = Download::start(&format!("http://127.0.0.1:{port}/files/x"), &directory);
        let path = loop {
            if let Some(result) = download.poll() {
                break result.expect("download should succeed");
            }
            thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(path.file_name().unwrap(), "big.bin");
        assert_eq!(fs::read(&path).unwrap(), body);
        assert_eq!(
            download.progress(),
            (body.len() as u64, Some(body.len() as u64))
        );
        assert_eq!(download.fraction(), Some(1.0));
        assert!(!path.with_file_name("big.bin.part").exists());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn copies_local_files_into_an_assets_folder() {
        let base = std::env::temp_dir().join(format!(
            "selenite-copy-test-{}-{}",
            std::process::id(),
            crate::timestamp()
        ));
        let assets = base.join("assets");
        fs::create_dir_all(&base).unwrap();
        let source = base.join("My Song.mp3");
        let body: Vec<u8> = (0..1_000_000u32).map(|i| (i % 253) as u8).collect();
        fs::write(&source, &body).unwrap();

        assert!(needs_copy(&source, &assets));
        assert!(!needs_copy(&base, &assets), "directories stay references");
        assert!(!needs_copy(&base.join("missing.txt"), &assets));

        let job = Download::copy(&source, &assets);
        assert!(job.is_copy);
        let copied = loop {
            if let Some(result) = job.poll() {
                break result.expect("copy should succeed");
            }
            thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(copied, assets.join("My Song.mp3"));
        assert_eq!(fs::read(&copied).unwrap(), body);
        assert_eq!(job.fraction(), Some(1.0));
        assert!(job.describe().starts_with("Copying My Song.mp3"));
        assert!(source.exists(), "the original is left alone");
        assert!(
            !needs_copy(&copied, &assets),
            "already in the assets folder"
        );

        let second = import_file(&source, &assets).unwrap();
        assert_eq!(second, assets.join("My Song (2).mp3"));
        assert_eq!(import_file(&copied, &assets).unwrap(), copied);
        assert!(import_file(&base, &assets).is_err());
        let _ = fs::remove_dir_all(base);
    }
}
