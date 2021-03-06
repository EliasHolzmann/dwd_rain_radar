use lazy_static::lazy_static;
#[cfg(feature = "rayon")]
use rayon::prelude::*;

lazy_static! {
    static ref ALL_FILES: Vec<std::path::PathBuf> = {
        let target_directory: std::path::PathBuf = std::env::var("DWD_DOWNLOADER_TARGET_DIRECTORY")
            .expect("Environment variable DWD_DOWNLOADER_TARGET_DIRECTORY could not be read")
            .into();

        std::fs::read_dir(&target_directory)
            .expect("Could not read directory")
            .map(|directory| directory.expect("Could not read directory"))
            .filter(|directory| directory.file_name() != "bitmaps")
            .flat_map(|directory| {
                std::fs::read_dir(directory.path())
                    .unwrap_or_else(|e| panic!("Could not read {:?}: {e:?}", directory.path()))
                    .map(|file| file.expect("Could not read directory").path())
            })
            .collect::<Vec<_>>()
    };
}

#[cfg(feature = "rayon")]
type SelectedFilesIter = impl IntoIterator<Item = &'static std::path::PathBuf>
    + rayon::iter::IntoParallelIterator<
        Iter = impl IndexedParallelIterator<Item = &'static std::path::PathBuf>,
        Item = &'static std::path::PathBuf,
    >;
#[cfg(not(feature = "rayon"))]
type SelectedFilesIter = impl IntoIterator<Item = &'static std::path::PathBuf>;
pub fn selected_files() -> SelectedFilesIter {
    use rand::prelude::*;
    let total_number_of_files = ALL_FILES.as_slice().len();
    if total_number_of_files == 0 {
        return std::vec::Vec::<&std::path::PathBuf>::new();
    }

    let number_of_files = 25;
    let random_file = rand::distributions::WeightedIndex::new(
        std::iter::repeat(1).take(total_number_of_files),
    )
    .expect("WeightedIndex::new failed");

    (0..number_of_files)
        .map(|_| &ALL_FILES.as_slice()[random_file.sample(&mut thread_rng())])
        .collect::<Vec<&std::path::PathBuf>>()
}

#[cfg(feature = "rayon")]
type AllFilesIter = impl IntoIterator<Item = &'static std::path::PathBuf>
    + rayon::iter::IntoParallelIterator<
        Iter = impl IndexedParallelIterator<Item = &'static std::path::PathBuf>,
        Item = &'static std::path::PathBuf,
    >;
#[cfg(not(feature = "rayon"))]
type AllFilesIter = impl IntoIterator<Item = &'static std::path::PathBuf>;
pub fn all_files() -> AllFilesIter {
    &ALL_FILES[..]
}
