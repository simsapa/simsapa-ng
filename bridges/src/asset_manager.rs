use std::thread;
use std::fs::{File, create_dir_all, remove_file, remove_dir_all};
use std::io::{self, Read, Write, BufReader};
use std::path::Path;
use std::error::Error;

use core::pin::Pin;
use cxx_qt_lib::{QString, QStringList, QUrl};
use cxx_qt::{CxxQtThread, Threading};
use reqwest::blocking::Client;
use bzip2::read::BzDecoder;
use tar::Archive;

use simsapa_backend::{move_folder_contents, AppGlobalPaths};
use simsapa_backend::asset_helpers::import_suttas_lang_to_userdata;
use simsapa_backend::logger::{info, error};
use simsapa_backend::lookup::LANG_CODE_TO_NAME;

#[cxx_qt::bridge]
pub mod qobject {

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;

        include!("wake_lock.h");
        fn acquire_wake_lock();
        fn release_wake_lock();
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[namespace = "asset_manager"]
        type AssetManager = super::AssetManagerRust;
    }

    impl cxx_qt::Threading for AssetManager{}

    extern "RustQt" {
        #[qinvokable]
        fn download_urls_and_extract(self: Pin<&mut AssetManager>, urls: QStringList);

        #[qinvokable]
        fn get_available_languages(self: Pin<&mut AssetManager>) -> QStringList;

        #[qinvokable]
        fn get_init_languages(self: Pin<&mut AssetManager>) -> QString;

        #[qinvokable]
        fn acquire_wake_lock_rust(self: Pin<&mut AssetManager>);

        #[qinvokable]
        fn release_wake_lock_rust(self: Pin<&mut AssetManager>);

        #[qsignal]
        #[cxx_name = "downloadProgressChanged"]
        fn download_progress_changed(self: Pin<&mut AssetManager>,
                                     op_msg: QString,
                                     downloaded_bytes: usize,
                                     total_bytes: usize);

        #[qsignal]
        #[cxx_name = "downloadShowMsg"]
        fn download_show_msg(self: Pin<&mut AssetManager>, message: QString);

        #[qsignal]
        #[cxx_name = "downloadsCompleted"]
        fn downloads_completed(self: Pin<&mut AssetManager>, value: bool);
    }
}

#[derive(Default, Copy, Clone)]
pub struct AssetManagerRust;

impl qobject::AssetManager {
    fn acquire_wake_lock_rust(self: Pin<&mut Self>) {
        qobject::acquire_wake_lock();
    }

    fn release_wake_lock_rust(self: Pin<&mut Self>) {
        qobject::release_wake_lock();
    }

    /// Get list of available language codes that can be downloaded
    /// Returns format: "code1|Name1,code2|Name2,..."
    fn get_available_languages(self: Pin<&mut Self>) -> QStringList {
        let mut langs = QStringList::default();

        // Filter out base languages (en, pli, san) which are always included
        let mut lang_list: Vec<_> = LANG_CODE_TO_NAME.iter()
            .filter(|(code, _)| !["en", "pli", "san"].contains(code))
            .map(|(code, name)| format!("{}|{}", code, name))
            .collect();

        lang_list.sort();

        for lang in lang_list {
            langs.append(QString::from(&lang));
        }

        langs
    }

    /// Read download_languages.txt if it exists in app_assets_dir
    /// Returns comma-separated language codes (e.g. "hu, pt, it")
    fn get_init_languages(self: Pin<&mut Self>) -> QString {
        let paths = AppGlobalPaths::new();
        let download_languages_path = paths.app_assets_dir.join("download_languages.txt");

        if download_languages_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&download_languages_path) {
                // Remove the file after reading
                let _ = std::fs::remove_file(&download_languages_path);
                return QString::from(contents.trim());
            }
        }

        QString::from("")
    }

    fn download_urls_and_extract(self: Pin<&mut Self>, urls: QStringList) {
        info(&format!("download_urls_and_extract(): {} urls", urls.len()));

        // AppGlobals was initialized before the storage path selection.
        // Get new paths to apply storage path selection.
        let paths = AppGlobalPaths::new();

        // Save to a temp folder (not in app-assets).
        // Only replace the assets after downloading and extracting succeeds.
        let download_temp_folder = paths.download_temp_folder.clone();
        let extract_temp_folder = paths.extract_temp_folder.clone();
        let app_assets_folder = paths.app_assets_dir.clone();

        let qt_thread = self.qt_thread();

        // Spawn a thread so Qt event loop is not blocked
        thread::spawn(move || {
            for url_qstr in urls.iter() {

                let url_str = url_qstr.to_string();
                let url = QUrl::from(&url_str);

                let download_file_name = url.file_name().to_string();
                let download_temp_file_path = download_temp_folder.join(&download_file_name);

                match extract_temp_folder.try_exists() {
                    Ok(exists) => {
                        if exists {
                            // If it exists, remove and re-create it to make sure we extract to an empty folder
                            let _ = remove_dir_all(&extract_temp_folder);
                        }
                        match create_dir_all(&extract_temp_folder) {
                            Ok(_) => {},
                            Err(e) => {
                                error(&format!("{}", e));
                                return;
                            },
                        };
                    }

                    Err(e) => {
                        error(&format!("{}", e));
                        return;
                    }
                }

                let client = Client::new();
                // Start blocking GET
                let resp = match client.get(&url_str).send() {
                    Ok(r) => r,
                    Err(e) => {
                        // Emit finished signal with the error message
                        qt_thread.queue(move |mut qo| {
                            let msg = QString::from(&format!("Error: {}", e));
                            qo.as_mut().download_show_msg(msg);
                        }).unwrap();
                        return;
                    }
                };

                let mut file = match File::create(&download_temp_file_path) {
                    Ok(f) => f,
                    Err(e) => {
                        qt_thread.queue(move |mut qo| {
                            let msg = QString::from(&format!("Error creating the file: {}", e));
                            qo.as_mut().download_show_msg(msg);
                        }).unwrap();
                        return;
                    }
                };

                let total = match resp.content_length() {
                    Some(n) => n as usize,
                    None => {
                        qt_thread.queue(move |mut qo| {
                            let msg = QString::from("Error: can't read download content length.");
                            qo.as_mut().download_show_msg(msg);
                        }).unwrap();
                        // The download file may have already been created with 0 length.
                        let _ = remove_file(download_temp_file_path);
                        return;
                    }
                };

                let mut reader = resp;
                let mut buf = [0u8; 8192]; // 8 KB buffer
                let mut downloaded = 0 as usize;

                loop {
                    let n = reader.read(&mut buf).unwrap(); // read up to buf.len()
                    if n == 0 { break; } // EOF
                    file.write_all(&buf[..n]).unwrap();
                    downloaded += n;
                    let op_msg = QString::from(format!("Downloading {}", &download_file_name));
                    qt_thread.queue(move |mut qo| {
                        qo.as_mut().download_progress_changed(op_msg, downloaded, total);
                    }).unwrap();
                }

                let op_msg = QString::from(format!("Extracting {}", &download_file_name));
                qt_thread.queue(move |mut qo| {
                    qo.as_mut().download_progress_changed(op_msg, 0, 0);
                }).unwrap();

                // Extract contents to a temp folder and move contents on success
                let msg = match extract_tar_bz2_with_progress(&download_temp_file_path,
                                                              &extract_temp_folder,
                                                              &qt_thread) {
                    Ok(_) => QString::from(format!("Completed extracting {}", &download_file_name)),
                    Err(e) => QString::from(format!("{}", e)),
                };

                // Remove the downloaded tar.bz2 whether the extraction was successful or not.
                let _ = remove_file(download_temp_file_path);

                // Import language databases before moving files
                if download_file_name.starts_with("suttas_lang_") && download_file_name.ends_with(".tar.bz2") {
                    let import_msg = QString::from(format!("Importing {}", &download_file_name));
                    qt_thread.queue(move |mut qo| {
                        qo.as_mut().download_show_msg(import_msg);
                    }).unwrap();

                    match import_suttas_lang_to_userdata(&extract_temp_folder, &paths.userdata_database_url) {
                        Ok(_) => {
                            info(&format!("Successfully imported {}", &download_file_name));
                        }
                        Err(e) => {
                            error(&format!("Failed to import {}: {}", &download_file_name, e));
                        }
                    }
                }

                // Move extracted contents to assets
                match move_folder_contents(&extract_temp_folder, &app_assets_folder) {
                    Ok(_) => {}
                    Err(e) => error(&format!("{}", e))
                }

                qt_thread.queue(move |mut qo| {
                    qo.as_mut().download_show_msg(msg);
                }).unwrap();
            } // end of for loop

            // Clean-up. All downloads are completed and extracted, remove the
            // download temp folder.
            let _ = remove_dir_all(&download_temp_folder);

            info("download_urls_and_extract(): all downloads completed");
            qt_thread.queue(move |mut qo| {
                qo.as_mut().downloads_completed(true);
            }).unwrap();

        }); // end of thread
    }
}

/// A wrapper around a Read trait object that tracks progress and sends messages to Qt.
struct ProgressReader<'a, R: Read> {
    /// The underlying reader.
    inner: R,
    /// Total bytes read so far from the inner reader. The self.bytes_read value
    /// can exceed self.total_size if BzDecoder reads a bit more for buffering.
    bytes_read: usize,
    /// Total size of the stream from the inner reader.
    total_size: usize,
    /// Last sent bytes_read value, for only sending increasing values.
    last_bytes_read: usize,
    /// The archive file name for formatting messages.
    file_name: String,
    /// CxxQtThread for sending signals to Qt.
    qt_thread: &'a CxxQtThread<qobject::AssetManager>,
}

impl<'a, R: Read> ProgressReader<'a, R> {
    /// Arguments:
    /// - `inner` - The reader to wrap.
    /// - `total_size` - The total number of bytes expected from the inner reader.
    /// - `qt_thread` - CxxQtThread for sending signals to Qt.
    fn new(inner: R,
           total_size: usize,
           file_name: &str,
           qt_thread: &'a CxxQtThread<qobject::AssetManager>)
           -> ProgressReader<'a, R> {
        ProgressReader {
            inner,
            bytes_read: 0,
            total_size,
            last_bytes_read: 0,
            file_name: file_name.to_string(),
            qt_thread,
        }
    }
}

impl<R: Read> Read for ProgressReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Read data from the inner reader
        let num_bytes = self.inner.read(buf)?;

        self.bytes_read += num_bytes;
        // Don't report greater than total size.
        if self.bytes_read > self.total_size {
            self.bytes_read = self.total_size;
        }

        if self.total_size > 0 {
            // Send only increased values.
            // The self.bytes_read can exceed self.total_size if BzDecoder reads a bit more
            // for its internal buffering before signaling EOF on the decompressed stream.
            if self.bytes_read > self.last_bytes_read {
                let op_msg = QString::from(format!("Extracting {}", &self.file_name));
                let bytes_read = self.bytes_read;
                let total_size = self.total_size;
                self.qt_thread.queue(move |mut qo| {
                    qo.as_mut().download_progress_changed(op_msg, bytes_read, total_size);
                }).unwrap();

                self.last_bytes_read = bytes_read;
            }
        }
        Ok(num_bytes)
    }
}

/// Extracts a .tar.bz2 archive to a specified output folder, printing progress.
///
/// Arguments:
/// - `archive_path` - Path to the .tar.bz2 archive file.
/// - `output_folder` - Path to the directory where contents will be extracted.
/// - `qt_thread` - CxxQtThread for sending signals to Qt.
pub fn extract_tar_bz2_with_progress(
    archive_path: &Path,
    output_folder: &Path,
    qt_thread: &CxxQtThread<qobject::AssetManager>,
) -> Result<(), Box<dyn Error>> {
    // 1. Create the output directory if it doesn't exist.
    create_dir_all(output_folder)
        .map_err(|e| format!("Failed to create output directory '{}': {}", output_folder.display(), e))?;

    // 2. Open the input .tar.bz2 file.
    let input_file = File::open(archive_path)
        .map_err(|e| format!("Failed to open archive file '{}': {}", archive_path.display(), e))?;

    // Get the total size of the compressed file for progress calculation.
    let total_size = input_file.metadata()?.len() as usize;

    if total_size == 0 {
        return Err(format!("Archive '{}' is empty. Nothing to extract.", archive_path.display()).into());
    }

    // 3. Wrap the file reader: File -> BufReader -> ProgressReader
    // BufReader adds buffering for potentially better I/O performance.
    let buffered_reader = BufReader::new(input_file);

    let a = archive_path.file_name().unwrap_or_default();
    let file_name = format!("{}", a.to_str().unwrap_or_default());

    // ProgressReader tracks bytes read from buffered_reader (the compressed stream).
    let progress_reader = ProgressReader::new(buffered_reader, total_size, &file_name, qt_thread);

    // 4. Set up the bzip2 decompressor.
    // BzDecoder will read from our ProgressReader.
    let bz_decoder = BzDecoder::new(progress_reader);

    // 5. Set up the tar archive reader.
    // Archive will read decompressed data from BzDecoder.
    let mut archive = Archive::new(bz_decoder);

    // Send initial progress status.
    let file_name_b = file_name.clone();
    qt_thread.queue(move |mut qo| {
        let op_msg = QString::from(format!("Extracting {}", &file_name_b));
        qo.as_mut().download_progress_changed(op_msg, 0, total_size);
    }).unwrap();

    // 6. Iterate through the entries in the tar archive and unpack them.
    // Progress signals are sent during read.
    for (i, entry_result) in archive.entries()?.enumerate() {
        let mut entry = entry_result
            .map_err(|e| format!("Failed to read entry {} from tar archive: {}", i, e))?;

        // Unpack the entry into the output folder.
        // This preserves the directory structure from the archive.
        entry.unpack_in(output_folder).map_err(|e| {
            format!(
                "Failed to unpack entry {} ('{}') into '{}': {}",
                i,
                entry.path().unwrap_or_default().display(),
                output_folder.display(),
                e
            )
        })?;
    }

    // 7. Send final progress status.
    qt_thread.queue(move |mut qo| {
        let op_msg = QString::from(format!("Completed extracting {}", &file_name));
        qo.as_mut().download_progress_changed(op_msg, total_size, total_size);
    }).unwrap();

    Ok(())
}
