use std::path::PathBuf;
use walkdir::WalkDir;
use rayon::prelude::*;

use crate::graph_embedding::GraphEmbedding;

/// Takes a directory, then list all files in that directory and its subdirectories
/// that are of type "-heap.raw", and their corresponding ".json" files.
/// Then do the sample and label generation for each of those files.
/// return: all samples and labels for all thoses files.
pub fn run(dir_path: PathBuf) {
    // cut the path to just after "phdtrack_data"
    let dir_path_ = dir_path.clone();
    let dir_path_end_str = dir_path_.to_str().unwrap().split("phdtrack_data/").collect::<Vec<&str>>()[1];

    let mut heap_dump_raw_file_paths: Vec<PathBuf> = Vec::new();

    // list all files in the directory and its subdirectories
    for entry in WalkDir::new(dir_path) {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_file() {
            if path.extension().map_or(false, |ext| ext == "raw") {
                heap_dump_raw_file_paths.push(path.to_path_buf());
            }
        }
    }

    let nb_files = heap_dump_raw_file_paths.len();
    let chunk_size = 10;
    let mut chunck_index = 0;

    for chunk in heap_dump_raw_file_paths.chunks(chunk_size) {
        // check save
        let csv_file_name = format!("{}_chunck_idx-{}_samples.csv", dir_path_end_str.replace("/", "_"), chunck_index);
        let csv_path = crate::params::SAMPLES_AND_LABELS_DATA_DIR_PATH.clone().join(csv_file_name.clone());
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

        // generate samples and labels
        let results: Vec<_> = chunk.par_iter().enumerate().map(|(i, heap_dump_raw_file_path)| {
            let global_idx = i + chunk_size*chunck_index;

            let graph_embedding = GraphEmbedding::new(
                heap_dump_raw_file_path.clone(),
                crate::params::BLOCK_BYTE_SIZE,
                *crate::params::EMBEDDING_DEPTH
            );

            let (samples_, labels_) = graph_embedding.generate_samples_and_labels();

            let file_name_id = heap_dump_raw_file_path.file_name().unwrap().to_str().unwrap().replace("-heap.raw", "");
            log::info!(" 🟢 [N°{} / {} files] [id: {}]    (Nb samples: {})", global_idx, nb_files, file_name_id, samples_.len());

            (samples_, labels_)
        }).collect();

        // save to csv
        let mut samples = Vec::new();
        let mut labels = Vec::new();
        for (samples_, labels_) in results {
            samples.extend(samples_);
            labels.extend(labels_);
            
        }
        save(samples, labels, csv_path);

        chunck_index += 1;
    }

}

pub fn save(samples: Vec<Vec<usize>>, labels: Vec<usize>, csv_path: PathBuf) {
    let mut csv_writer = csv::Writer::from_path(csv_path).unwrap();

    // header of CSV
    let mut header = Vec::new();
    header.push("f_dtn_byte_size".to_string());
    header.push("f_position_in_dtn".to_string());
    header.push("f_dtn_ptrs".to_string());
    header.push("f_dtn_vns".to_string());
    // start at 1 since 0 is a ValueNode (so always [0, 0])
    for i in 1..*crate::params::EMBEDDING_DEPTH {
        header.push(format!("f_dtns_ancestor_{}", i));
        header.push(format!("f_ptrs_ancestor_{}", i));
    }
    header.push("label".to_string());
    csv_writer.write_record(header).unwrap();

    // save samples and labels to CSV
    for (sample, label) in samples.iter().zip(labels.iter()) {
        let mut row: Vec<String> = Vec::new();
        row.extend(sample.iter().map(|value| value.to_string()));
        row.push(label.to_string());

        csv_writer.write_record(&row).unwrap();
    }

    csv_writer.flush().unwrap();
}