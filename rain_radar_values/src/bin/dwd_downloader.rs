/// periodically crawls historical data in 15 minute intervals for later analysis
use anyhow::{anyhow, bail, ensure, Context, Result};
use chrono::prelude::*;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

fn main() -> Result<()> {
    let may_currently_exit = Arc::new(Mutex::new(true));
    let may_currently_exit_copy = may_currently_exit.clone();
    ctrlc::set_handler(move || {
        let may_currently_exit = may_currently_exit_copy.lock().expect("Failed locking may_currently_exit mutex");
        if *may_currently_exit {
            std::process::exit(0);
        } else {
            println!("Received Ctrl-C, but can't currently exit because there is a file copy ongoing. Please retry later.");
        }
    }).context("Failed setting handler")?;

    let target_directory: PathBuf = std::env::var("DWD_DOWNLOADER_TARGET_DIRECTORY")
        .context("Failed reading environment variable DWD_DOWNLOADER_TARGET_DIRECTORY")?
        .into();
    ensure!(
        target_directory
            .parent()
            .unwrap_or(&target_directory)
            .is_dir(),
        "{:?} is not a directory",
        target_directory
    );

    if !target_directory.is_dir() {
        std::fs::create_dir(&target_directory)
            .with_context(|| anyhow!("Failed creating directory {target_directory:?}"))?;
    }

    loop {
        eprintln!("Start crawling");
        let now = Utc::now();
        let begin_crawling_at_time = now - chrono::Duration::hours(48); // determined by gut feeling

        // round down to nearest 15 minutes
        let midnight = NaiveTime::from_hms(0, 0, 0);
        let time_since_day_began = begin_crawling_at_time
            .time()
            .signed_duration_since(midnight);
        let hours = time_since_day_began.num_hours() as u32;
        let minutes = (time_since_day_began.num_minutes() / 15 * 15) as u32 - hours * 60;
        let rounded_down_time = NaiveTime::from_hms(hours, minutes, 0);

        let begin_crawling_at_time_rounded = chrono::DateTime::<Utc>::from_utc(
            NaiveDateTime::new(begin_crawling_at_time.date().naive_utc(), rounded_down_time),
            *begin_crawling_at_time.offset(),
        );

        for date_time in std::iter::successors(Some(begin_crawling_at_time_rounded), |last_time| {
            let next_time = *last_time + chrono::Duration::minutes(15);
            if next_time < now {
                Some(next_time)
            } else {
                None
            }
        }) {
            // build path to file on disk
            let mut file_path_on_disk = PathBuf::from(&target_directory);
            file_path_on_disk.push(date_time.format("%Y%m%d").to_string());
            file_path_on_disk.push(date_time.format("%H%M%S.tar.bz2").to_string());

            fn file_exists_and_is_file<P: AsRef<Path>>(path: P) -> Result<bool> {
                let path = path.as_ref();
                match std::fs::metadata(path) {
                    Ok(metadata) => Ok(metadata.is_file()),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
                    Err(error) => {
                        Err(anyhow!(error).context(anyhow!("Failed getting metadata for {path:?}")))
                    }
                }
            }

            if !file_exists_and_is_file(&file_path_on_disk).with_context(|| {
                anyhow!("Failed checking whether {file_path_on_disk:?} exists and is a file")
            })? {
                eprintln!("File {file_path_on_disk:?} does not exist yet, retrieving...");
                // file does not exist yet -> download it
                let url = date_time.format("https://opendata.dwd.de/weather/radar/composit/rv/DE1200_RV%y%m%d%H%M.tar.bz2").to_string();
                let mut response = reqwest::blocking::get(&url)
                    .with_context(|| anyhow!("Failed retrieving radar file from {url}"))?;

                match response.status() {
                    reqwest::StatusCode::NOT_FOUND => {
                        eprintln!("File not found");
                        // server does not have that file
                    }
                    reqwest::StatusCode::OK => {
                        let parent_directory = file_path_on_disk
                            .parent()
                            .expect("Could not get parent directory of output file (this should be impossible)");
                        if !parent_directory.is_dir() {
                            std::fs::create_dir(&parent_directory).with_context(|| {
                                anyhow!("Failed creating directory {parent_directory:?}")
                            })?;
                        }

                        let mut file = std::fs::File::create(&file_path_on_disk)
                            .with_context(|| anyhow!("Failed creating {file_path_on_disk:?}"))?;
                        *may_currently_exit
                            .lock()
                            .expect("may_currently_exit mutex is poisoned") = false;

                        let copy_result = response.copy_to(&mut file);
                        *may_currently_exit
                            .lock()
                            .expect("may_currently_exit mutex is poisoned") = true;

                        match copy_result {
                            Ok(_) => {
                                eprintln!("Done");
                            }
                            Err(error) => {
                                drop(file);
                                std::fs::remove_file(&file_path_on_disk).with_context(|| {
                                    anyhow!("Failed removing file {file_path_on_disk:?}")
                                })?;
                                return Err(anyhow!(error).context(anyhow!("Failed copying result of retrieving {url} to {file_path_on_disk:?}")));
                            }
                        }
                    }
                    status_code => {
                        bail!("Received status code {status_code} while retrieving {url}")
                    }
                }
                // to not get banned by DWD servers
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
        }

        eprintln!("Done, going to sleep");
        std::thread::sleep(std::time::Duration::from_secs(600));
    }
}
