use std::fs::{OpenOptions, File};
use chrono::Local;
use crate::types::Miner;
use fs2::FileExt;
use std::io::Write;

pub fn write_log(miner: &Miner, message: &str) {
    println!("Log: {}", message);
    if let Ok(mut file) = OpenOptions::new().append(true).create(true).open(&miner.log_path) {
    
        if let Err(e) = file.lock_exclusive() {
            println!("Failed to lock file: {}", e);
            return;
        }
    
        let now = Local::now();
        let formatted_message = format!("{} - {}\n", now.format("%Y-%m-%d %H:%M:%S"), message);
    
        if let Err(e) = file.write_all(formatted_message.as_bytes()) {
            println!("Failed to write to file: {}", e);
            return;
        }
    
        if let Err(e) = file.unlock() {
            println!("Failed to unlock file: {}", e);
            return;
        }
    } else {
        println!("Failed to open file");
        return;
    }
}

pub fn reset_log(miner: &Miner) {
    if let Ok(file) = File::create(&miner.log_path){
        if let Err(e) = file.lock_exclusive() {
            println!("Failed reset log file: {}", e);
            return;
        }
    
        if let Err(e) = file.set_len(0) {
            println!("Failed to reset log file: {}", e);
            return;
        }
    
        if let Err(e) = file.unlock() {
            println!("Failed to reset log file: {}", e);
            return;
        }
    }
}