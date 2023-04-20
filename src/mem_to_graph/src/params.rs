use lazy_static::lazy_static;
use std::path::{PathBuf};
use dotenv::dotenv;
use std::sync::Once;

use crate::utils::Endianness;

pub const BLOCK_BYTE_SIZE: usize = 8; // 64-bit, ex: C0 03 7B 09 2A 56 00 00

/// WARN: SHOULD BE USED ONLY FOR NODE CONSTRUCTION (see utils::convert_block_to_pointer_if_possible)
pub const PTR_ENDIANNESS: Endianness = Endianness::Little;
pub const MALLOC_HEADER_ENDIANNESS: Endianness = Endianness::Little;

/// Initialize logger. 
/// WARN: Must be called before any logging is done.
fn init_logger() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or(LOGGER_MODE.as_str()));

    log::info!(" 🚀 starting mem to graph converter");
}

static INIT: Once = Once::new();

/// Initialize things that need to be initialized only once.
pub fn init() {
    INIT.call_once(|| {
        // initialization code here
        dotenv().ok();
        init_logger();
    });
}

// Get the path to files for the program, using the environment variables.
lazy_static! {
    static ref LOGGER_MODE: String = {
        let logger_mode = std::env::var("LOGGER_MODE");
        match logger_mode {
            Ok(mode) => mode,
            Err(_) => {
                println!("LOGGER_MODE environment variable not set. Defaulting to 'info'.");
                return "info".to_string();
            },
        }
    };

    static ref HOME_DIR: String = std::env::var("HOME")
        .expect("HOME environment variable must be set");

    static ref REPO_DIR: String = {
        let repo_dir = std::env::var("REPOSITORY_BASE_DIR")
            .expect("REPOSITORY_BASE_DIR environment variable must be set");
        HOME_DIR.clone() + &repo_dir
    };

    static ref DATA_DIR: String = {
        let data_dir = std::env::var("DATA_BASE_DIR")
            .expect("DATA_BASE_DIR environment variable must be set");
        HOME_DIR.clone() + &data_dir
    };
    
    pub static ref TEST_HEAP_DUMP_FILE_PATH: PathBuf = {
        let test_heap_dump_raw_file_path = std::env::var("TEST_HEAP_DUMP_RAW_FILE_PATH")
            .expect("TEST_HEAP_DUMP_RAW_FILE_PATH environment variable must be set").to_string();
        PathBuf::from(REPO_DIR.clone() + &test_heap_dump_raw_file_path)
    };

    pub static ref TEST_HEAP_JSON_FILE_PATH: PathBuf = {
        crate::utils::heap_dump_path_to_json_path(&TEST_HEAP_DUMP_FILE_PATH)
    };

    pub static ref COMPRESS_POINTER_CHAINS: bool = {
        let compress_pointer_chains = std::env::var("COMPRESS_POINTER_CHAINS");
        match compress_pointer_chains {
            Ok(mode) => mode == "true",
            Err(_) => {
                println!("COMPRESS_POINTER_CHAINS environment variable not set. Defaulting to 'false'.");
                return false;
            },
        }
    };

    pub static ref EMBEDDING_DEPTH: usize = {
        let base_embedding_depth = std::env::var("EMBEDDING_DEPTH");
        match base_embedding_depth {
            Ok(depth) => depth.parse::<usize>().unwrap(),
            Err(_) => {
                println!("EMBEDDING_DEPTH environment variable not set. Defaulting to '1'.");
                return 1;
            },
        }
    };

    pub static ref TEST_CSV_EMBEDDING_FILE_PATH: PathBuf = {
        let test_csv_embedding_file_path = std::env::var("TEST_CSV_EMBEDDING_FILE_PATH")
            .expect("TEST_CSV_EMBEDDING_FILE_PATH environment variable must be set").to_string();
        PathBuf::from(&test_csv_embedding_file_path)
    };

    pub static ref REMOVE_TRIVIAL_ZERO_SAMPLES: bool = {
        let remove_trivial_zero_samples = std::env::var("REMOVE_TRIVIAL_ZERO_SAMPLES");
        match remove_trivial_zero_samples {
            Ok(mode) => mode == "true",
            Err(_) => {
                println!("REMOVE_TRIVIAL_ZERO_SAMPLES environment variable not set. Defaulting to 'false'.");
                return false;
            },
        }
    };

    pub static ref TESTING_DATA_DIR_PATH: PathBuf = {
        let testing_data_dir_path = std::env::var("TESTING_DATA_DIR_PATH")
            .expect("TESTING_DATA_DIR_PATH environment variable must be set").to_string();
        PathBuf::from(&testing_data_dir_path)
    };

    pub static ref SAMPLES_AND_LABELS_DATA_DIR_PATH: PathBuf = {
        let samples_and_labels_data_dir_path = std::env::var("SAMPLES_AND_LABELS_DATA_DIR_PATH")
            .expect("SAMPLES_AND_LABELS_DATA_DIR_PATH environment variable must be set").to_string();
        PathBuf::from(&samples_and_labels_data_dir_path)
    };

}
