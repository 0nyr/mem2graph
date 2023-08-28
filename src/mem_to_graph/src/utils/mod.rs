use std::collections::HashMap;
use std::convert::TryInto;
use std::path::PathBuf;
use error_chain::error_chain;
use serde_json::Value;

use crate::params::PTR_ENDIANNESS;
use crate::graph_structs::{Node, PointerNode, ValueNode, BasePointerNode, BaseValueNode};

/// convert an address to an index
/// NOTE: addresses are represented as u64
#[cfg(test)]
pub fn addr_to_index(addr: u64, min_addr: u64, block_size: usize) -> usize {
    ((addr - min_addr) / block_size as u64) as usize
}

/// convert an index to an address
/// NOTE: indexes are represented as usize
pub fn index_to_addr(index: usize, min_addr: u64, block_size: usize) -> u64 {
    (index * block_size) as u64 + min_addr
}

/// convert a block of bytes to a u64 address
pub fn block_bytes_to_addr(block_bytes: &[u8; crate::params::BLOCK_BYTE_SIZE], endianness: Endianness) -> u64 {
    let mut addr = 0u64;
    for (i, byte) in block_bytes.iter().enumerate() {
        match endianness {
            Endianness::Big => addr += (*byte as u64) << (8 * (7 - i)),
            Endianness::Little => addr += (*byte as u64) << (8 * i),
        }
    }
    addr
}

/// convert a json value to an address represented as a u64 (intended from a hex string)
/// WARN: all addresses in the json file are big endian
pub fn json_value_to_addr(json_value: &Value) -> u64 {
    // check whether the json value is a string or an integer
    if json_value.is_string() {
        return hex_str_to_addr(json_value.as_str().unwrap(), Endianness::Big).unwrap();
    } else if json_value.is_number() {
        return json_value.as_u64().unwrap();
    } else {
        panic!("Invalid json value: {}", json_value);
    }
}

/// convert a json value to a usize (intented from a decimal string)
pub fn json_value_to_usize(json_value: &Value) -> usize {
    // check whether the json value is a string or an integer
    if json_value.is_string() {
        return json_value.as_str().unwrap().parse::<usize>().unwrap();
    } else if json_value.is_number() {
        return json_value.as_u64().unwrap() as usize;
    } else {
        panic!("Invalid json value: {}", json_value);
    }
}

error_chain! {
    foreign_links {
        Io(std::io::Error);
        Json(serde_json::Error);
    }
    errors {
        MissingJsonKeyError(json_annotation: String) {
            description("Invalid json annotation")
            display("Invalid json annotation: {}", json_annotation)
        }
        JsonFileNotFound(json_file_path: PathBuf) {
            description("Json file not found")
            display("Json file not found: {:?}", json_file_path)
        }
    }
}

/// check if a json value is a null value
/// in that case, return custom error
pub fn json_value_for_key(json: &Value, key: String) -> Result<&Value> {
    json.get(&key)
        .ok_or_else(|| Error::from_kind(ErrorKind::MissingJsonKeyError(key)))
}

/// convert a hex string to an address represented as a u64
/// WARN: necessary to specify the string endianness for the conversion
/// WARN: Due to little endian needing to have a fixed length of 16 characters, 
///       the hex string will be padded with 0s to the right if it is less than 16 characters
/// NOTE: always returns a big endian address as a u64
pub fn hex_str_to_addr(hex_str: &str, endianness: Endianness) -> std::result::Result<u64, std::num::ParseIntError> {
    match endianness {
        Endianness::Big => Ok(u64::from_str_radix(hex_str, 16)?),
        Endianness::Little => {
            //assert_eq(hex_str.len(), 16, "Little endian hex string ({}) must be 16 characters long", hex_str);
            // append 0s to the right if the hex string is less than 16 characters
            let mut padded_hex_str = hex_str.to_string();
            while padded_hex_str.len() < 16 {
                padded_hex_str.push('0');
            }
            let addr = u64::from_str_radix(padded_hex_str.as_str(), 16)?;
            //log::debug!("Little endian padded hex string {}", padded_hex_str);
            Ok(addr.swap_bytes())
        },
    }
}

/// convert a hex string to a block of bytes
pub fn hex_str_to_block_bytes(hex_str: &str) -> [u8; crate::params::BLOCK_BYTE_SIZE] {
    assert_eq!(hex_str.len(), crate::params::BLOCK_BYTE_SIZE * 2, "Hex string ({}) must be {} characters long", hex_str, crate::params::BLOCK_BYTE_SIZE * 2);
    let padded_hex_str = hex_str.to_string();
    let mut block_bytes = [0u8; crate::params::BLOCK_BYTE_SIZE];
    for (i, byte) in padded_hex_str.as_bytes().chunks(2).enumerate() {
        block_bytes[i] = u8::from_str_radix(std::str::from_utf8(byte).unwrap(), 16).unwrap();
    }
    block_bytes
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Endianness {
    Big,
    Little,
}

/// convert a block of bytes to a pointer if it is a valid pointer
/// NOTE: A valid pointer is a pointer that is within the heap dump range
/// NOTE: remember that our heap dump vectors are in the format given as a program argument
pub fn convert_block_to_pointer_if_possible(data: &[u8], min_addr: u64, max_addr: u64) -> Option<u64> {
    // WARN: THIS IS THE ONLY PLACE WHERE THE POINTER ENDIANNESS IS USED
    

    let potential_ptr_int = match PTR_ENDIANNESS {
        Endianness::Big => u64::from_be_bytes(data.try_into().unwrap()),
        Endianness::Little => u64::from_le_bytes(data.try_into().unwrap()),
    };

    // check if the potential pointer is within the heap dump range
    if potential_ptr_int >= min_addr && potential_ptr_int <= max_addr {
        Some(potential_ptr_int)
    } else {
        None
    }
}

/// create a node from a block of bytes, following the specified endianness
/// NOTE: no need to provide endianess for the pointer conversion directly, 
/// it uses the global pointer endianness
pub fn create_node_from_bytes(
    block: &[u8; crate::params::BLOCK_BYTE_SIZE],
    addr: u64,
    dtn_addr: u64,
    min_addr: u64,
    max_addr: u64,
) -> Node {
    let potential_ptr = convert_block_to_pointer_if_possible(
        block, min_addr, max_addr
    );
    if potential_ptr.is_some() {
        Node::PointerNode(
            PointerNode::BasePointerNode(
                BasePointerNode {
                    addr,
                    points_to: potential_ptr.unwrap(),
                    dtn_addr,
                }
            )
        )
    } else {
        Node::ValueNode(
            ValueNode::BaseValueNode(
                BaseValueNode {
                    addr,
                    value: *block,
                    dtn_addr,
                }
            )
        )
    }
}

/// Convert a path to a heap dump file to a path to a associated json file
pub fn heap_dump_path_to_json_path(heap_dump_raw_file_path: &PathBuf) -> PathBuf {
    let original_heap_path_str = heap_dump_raw_file_path.to_str().unwrap().to_string();
    let json_path = PathBuf::from(
        original_heap_path_str.replace("-heap.raw", ".json")
    );

    if !json_path.exists() {
        log::error!("File doesn't exist: {:?}", json_path);
    }
    return json_path;
}

/// compute division on 2 integers and round up if necessary
// For example, let's say we have a numerator of 11 and a denominator of 4:
//     Original division: 11 / 4 = 2.75 (rounded down to 2 using integer division)
//     New numerator: 11 + 4 - 1 = 14
//     New division: 14 / 4 = 3.5 (rounded down to 3 using integer division)
pub fn div_round_up(numerator: usize, denominator: usize) -> usize {
    (numerator + denominator - 1) / denominator
}

/// generate all possible bit combinations of size n
/// bitwise order
pub fn generate_bit_combinations(n: usize) -> Vec<String> {
    let mut result = Vec::new();
    let max = 1 << n; // 2^n

    for i in 0..max {
        let binary = format!("{:0width$b}", i, width = n);
        result.push(binary);
    }

    result
}

/// convert u64 into binary of length n
pub fn to_n_bits_binary(value: u64, n: usize) -> String {
    format!("{:0width$b}", value, width = n)
}

/// convert u64 into 8 bytes
pub fn u64_to_bytes(value: u64) -> [u8; 8] {
    let mut bytes = [0u8; 8];
    for i in 0..8 {
        bytes[i] = ((value >> ((8 - i - 1) * 8)) & 0xFF) as u8;
    }
    bytes
}

/// Computes various statistical measures for a given dataset of bytes.
///
/// This function calculates the following statistics:
/// 1. Mean Byte Value
/// 2. Mean Absolute Deviation (MAD)
/// 3. Standard Deviation
/// 4. Skewness
/// 5. Kurtosis
///
/// # Arguments
///
/// * `data` - A reference to a vector of bytes (`Vec<u8>`) for which the statistics are to be computed.
///
/// # Returns
///
/// A tuple containing five `f64` values representing the mean byte value, MAD, standard deviation, skewness, and kurtosis, respectively.
pub fn compute_statistics(data: &Vec<u8>) -> (f64, f64, f64, f64, f64) {
    let mean = {
        let sum: u64 = data.iter().map(|&x| u64::from(x)).sum();
        sum as f64 / data.len() as f64
    };

    let mad = {
        let sum: f64 = data.iter().map(|&x| (x as f64 - mean).abs()).sum();
        sum / data.len() as f64
    };

    let std_dev = {
        let variance: f64 = data.iter().map(|&x| (x as f64 - mean).powi(2)).sum::<f64>() / data.len() as f64;
        variance.sqrt()
    };
    // WARN : at least 4 byte needed
    let skew = {
        if data.len() < 4 {
            f64::NAN
        }else{
            let n = data.len() as f64;
            let sum: f64 = data.iter().map(|&x| ((x as f64 - mean) / std_dev).powi(3)).sum();
            (n / ((n - 1.0) * (n - 2.0))) * sum
        }
    };
    // WARN : at least 4 byte needed
    let kurt = {
        if data.len() < 4 {
            f64::NAN
        }else{
            let n = data.len() as f64;
            let sum: f64 = data.iter().map(|&x| ((x as f64 - mean) / std_dev).powi(4)).sum();
            (n * (n + 1.0) / ((n - 1.0) * (n - 2.0) * (n - 3.0))) * sum - (3.0 * (n - 1.0).powi(2) / ((n - 2.0) * (n - 3.0)))
        }
    };

    (mean, mad, std_dev, skew, kurt)
}

/// compute the shannon entropy
pub fn shannon_entropy(data: &Vec<u8>) -> f64 {
    let mut frequency = HashMap::new();
    for &byte in data.iter() {
        *frequency.entry(byte).or_insert(0 as u64) += 1;
    }

    let len = data.len() as f64;
    let mut entropy = 0.0;

    for &count in frequency.values() {
        let probability = count as f64 / len;
        entropy -= probability * probability.log2();
    }

    entropy
}