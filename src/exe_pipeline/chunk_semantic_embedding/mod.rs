use rayon::prelude::*;
use std::{time::Instant, path::PathBuf};

use crate::{graph_embedding::GraphEmbedding, exe_pipeline::progress_bar, params::argv::{SelectAnnotationLocation, EntropyFilter}, utils::truncate_path_to_last_n_dirs};

use super::get_raw_file_or_files_from_path;
/// Takes a directory or a file
/// If directory then list all files in that directory and its subdirectories
/// that are of type "-heap.raw", and their corresponding ".json" files.
/// Then do the sample and label generation for each of those files.
/// return: all samples and labels for all thoses files.
pub fn run_chunk_semantic_embedding(path: PathBuf, output_folder: PathBuf, annotation : SelectAnnotationLocation, no_value_node: bool, entropy_filter : EntropyFilter) {
    // start timer
    let start_time = Instant::now();

    // cut the path to just after "phdtrack_data"
    let dir_path_ = path.clone();
    let dir_path_end = truncate_path_to_last_n_dirs(&dir_path_, 5);
    let dir_path_end_str = dir_path_end.to_str().unwrap(); 

    let heap_dump_raw_file_paths: Vec<PathBuf> = get_raw_file_or_files_from_path(path.clone());

    let nb_files = heap_dump_raw_file_paths.len();
    let chunk_size = crate::params::NB_FILES_PER_CHUNK.clone();
    let mut chunck_index = 0;

    // test if there is at least one file
    if nb_files == 0 {
        panic!("The file doesn't exist or the directory doesn't contain any .raw file: {}", path.to_str().unwrap());
    }

    // run the sample and label generation for each file by chunks
    for chunk in heap_dump_raw_file_paths.chunks(chunk_size) {
        // chunk time
        let chunk_start_time = Instant::now();

        // check save
        let csv_file_name = format!("{}_chunck_idx-{}_samples.csv", dir_path_end_str.replace("/", "_"), chunck_index);
        let csv_path = output_folder.clone().join(csv_file_name.clone());
        if csv_path.exists() {
            log::info!(" 🔵 [N°{}-{} / {} files] [id: {}] already saved (csv: {}).", 
                chunck_index*chunk_size,
                chunck_index*chunk_size + chunk_size - 1,
                nb_files, 
                chunck_index,
                csv_file_name.as_str()
            );
            chunck_index += 1;
            continue;
        }

        // Create a thread pool with named threads
        let pool = rayon::ThreadPoolBuilder::new()
            .thread_name(|index| format!("worker-{}", index))
            .build()
            .unwrap();

        // generate samples and labels
        let results: Vec<_> = pool.install(|| {
            chunk.par_iter().enumerate().map(|(i, heap_dump_raw_file_path)| {
                let current_thread = std::thread::current();
                let thread_name = current_thread.name().unwrap_or("<unnamed>");
                let global_idx = i + chunk_size*chunck_index;

                let graph_embedding = GraphEmbedding::new(
                    heap_dump_raw_file_path.clone(),
                    crate::params::BLOCK_BYTE_SIZE,
                    *crate::params::EMBEDDING_DEPTH,
                    entropy_filter,
                    annotation,
                    no_value_node
                );

                match graph_embedding {
                    Ok(graph_embedding) => {
                        // generate samples and labels
                        let samples_ = graph_embedding.generate_semantic_samples_for_all_chunks();

                        let file_name_id = heap_dump_raw_file_path.file_name().unwrap().to_str().unwrap().replace("-heap.raw", "");
                        log::info!(" 🟢 [t: {}] [N°{} / {} files] [fid: {}]    (Nb samples: {})", thread_name, global_idx, nb_files, file_name_id, samples_.len());

                        (samples_, heap_dump_raw_file_path.as_os_str().to_str().unwrap().to_string())
                    },
                    Err(err) => match err {
                        crate::utils::ErrorKind::MissingJsonKeyError(key) => {
                            log::warn!(" 🔴 [t: {}] [N°{} / {} files] [fid: {}]    Missing JSON key: {}", thread_name, global_idx, nb_files, heap_dump_raw_file_path.file_name().unwrap().to_str().unwrap(), key);
                            (Vec::new(), "".to_string())
                        },
                        crate::utils::ErrorKind::JsonFileNotFound(json_file_path) => {
                            log::warn!(" 🟣 [t: {}] [N°{} / {} files] [fid: {}]    JSON file not found: {:?}", thread_name, global_idx, nb_files, heap_dump_raw_file_path.file_name().unwrap().to_str().unwrap(), json_file_path);
                            (Vec::new(), "".to_string())
                        },
                        _ => {
                            panic!("Other unexpected graph embedding error: {}", err);
                        }
                    }
                }
                
            }).collect()
        });

        // save to csv
        let mut samples = Vec::new();
        let mut paths = Vec::new();
        for (samples_, path) in results {
            for _ in 0..samples_.len() {
                paths.push(path.clone());
            }
            samples.extend(samples_);
        }
        save_chunk_semantic_embeding(samples, paths, csv_path, *crate::params::EMBEDDING_DEPTH);

        // log time
        let chunk_duration = chunk_start_time.elapsed();
        let total_duration = start_time.elapsed();
        let progress = progress_bar(chunck_index * chunk_size, nb_files, 20);
        log::info!(
            " ⏱️  [chunk: {:.2?} / total: {:.2?}] {}",
            chunk_duration,
            total_duration,
            progress
        );

        chunck_index += 1;
    }

}

/// NOTE: saving empty files allow so that we don't have to recompute the samples and labels
/// for broken files (missing JSON key, etc.)
fn save_chunk_semantic_embeding(samples: Vec<Vec<usize>>, paths : Vec<String>, csv_path: PathBuf, embedding_depth: usize) {
    assert!(samples.len() == paths.len());
    let csv_error_message = format!("Cannot create csv file: {:?}, no such file.", csv_path);
    let mut csv_writer = csv::Writer::from_path(csv_path).unwrap_or_else(
        |_| panic!("{}", csv_error_message)
    );
    // header of CSV
    let mut header = Vec::new();
    header.push("file_path".to_string());
    header.push("f_chn_addr".to_string());
    header.push("f_chunk_byte_size".to_string());
    header.push("f_chunk_ptrs".to_string());
    for i in 0..embedding_depth {
        header.push(format!("f_chns_ancestor_{}", i + 1));
        header.push(format!("f_ptrs_ancestor_{}", i + 1));
    }

    for i in 0..embedding_depth {
        header.push(format!("f_chns_children_{}", i + 1));
        header.push(format!("f_ptrs_children_{}", i + 1));
    }
    header.push("label".to_string());


    csv_writer.write_record(header).unwrap();

    // save samples and labels to CSV
    for (sample, path) in samples.iter().zip(paths.iter()) {
        let mut row: Vec<String> = Vec::new();
        row.push(path.to_string());
        row.extend(sample.iter().map(|value| value.to_string()));

        log::info!("row: {:?}", row);

        csv_writer.write_record(&row).unwrap();
    }

    csv_writer.flush().unwrap();
}