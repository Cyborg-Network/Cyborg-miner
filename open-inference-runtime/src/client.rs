
// client.rs
use reqwest::Client;
use serde_json::Value;
// use std::collections::HashMap;
use serde_json::json;
use image::io::Reader as ImageReader;
use csv::ReaderBuilder;
use std::fs::File;
use std::path::Path;
use std::io::BufReader;

use crate::models::{Model, ModelStatus,ModelExtractor};
use crate::error::TritonError;

// const TRITON_URL: &str = "http://localhost:8000/v2";

pub struct TritonClient {
    client: Client,
    url: String,
}

#[derive(Clone, Debug)]
pub enum TensorData {
    F32(Vec<f32>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    U8(Vec<u8>),
    Bool(Vec<bool>),
    Str(Vec<String>),
}

impl TensorData {
    pub fn to_serializable(&self) -> Value {
        match self {
            TensorData::F32(data) => json!(data),
            TensorData::I32(data) => json!(data),
            TensorData::I64(data) => json!(data),
            TensorData::U8(data) => json!(data),
            TensorData::Bool(data) => json!(data),
            TensorData::Str(data) => json!(data),
        }
    }
}

impl TritonClient {
    pub fn new(TRITON_URL:String) -> Self {
        Self {
            client: Client::new(),
            url: triton_url.to_string(),
            model_name: model_name.to_string(),
            model_path: model_path.clone(),
        };

        match ModelExtractor::new(&client.model_name, model_path.clone()) {
            Ok(extractor) => {
                if let Err(e) = extractor.extract_model() {
                    println!("❌ Extraction failed: {:?}", e);
                } else {
                }
            }
            Err(e) => {
                println!("❌ Initialization of ModelExtractor failed: {:?}", e);
            }
        }

          println!("⏳ Checking if the server is live...");

        let mut url = format!("{}/health/live", &client.url);
        let mut response = client.client.get(&url).send().await?;
        if !response.status().is_success() {
            println!("✅ Server is not live: {}", response.status());
        }
        println!("✅ Server is live!");
        println!("⏳ Checking if the server is ready...");

        url = format!("{}/health/ready", &client.url);
        response = client.client.get(&url).send().await?;
        if !response.status().is_success() {
            println!("✅ Server is not ready: {}", response.status());
        }
        println!("✅ Server is ready!");

        println!("⏳ Loading model: {}", client.model_name);

        url = format!(
            "{}/repository/models/{}/load",
            &client.url, &client.model_name
        );
        response = client
            .client
            .post(&url)
            .json(&serde_json::json!({}))
            .send()
            .await?;
        if response.status().is_success() {
            println!("✅ Successfully loaded model: {}", &client.model_name);
        }


        // Check if the server is live
        // let mut url = format!("{}/health/ready", &client.url);
        // let mut response = client.client.get(&url).send().await?;
        // if !response.status().is_success() {
        //     println!("✅ Server is not live: {}", response.status());
        // }
        // // Check if the server is ready
        // url = format!("{}/health/ready", &client.url);
        // response = client.client.get(&url).send().await?;
        // if !response.status().is_success() {
        //     println!("✅ Server is not ready: {}", response.status());
        // }

        Ok(client)
    }

    // pub async fn load_model(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //     let url = format!("{}/repository/models/{}/load", self.url, self.model_name);
    //     let response = self
    //         .client
    //         .post(&url)
    //         .json(&serde_json::json!({}))
    //         .send()
    //         .await?;
    //     if response.status().is_success() {
    //         Ok(())
    //     } else {
    //         Err(format!(
    //             "Failed to load model '{}'. HTTP Status: {:?}",
    //             self.model_name,
    //             response.status()
    //         )
    //         .into())
    //     }
    // }

    // pub fn verify_model_blob(&self, expected_hash_hex: &str) -> io::Result<()> {
    //     let extracted_path = self.model_path.join(&self.model_name);
    //     let model_path = extracted_path.join("1").join("model.onnx");

    //     // Read model file into bytes
    //     let mut model_file = File::open(model_path)?;
    //     let mut model_data = Vec::new();
    //     model_file.read_to_end(&mut model_data)?;

    //     // Compute actual SHA-256 of model
    //     let model_sha256 = Sha256::digest(&model_data);
    //     let computed_hash_hex = hex::encode(&model_sha256);

    //     // Compare with provided hash
    //     if computed_hash_hex == expected_hash_hex.to_lowercase() {
    //         println!("✅ Hash verification passed");
    //         Ok(())
    //     } else {
    //         eprintln!("❌ Hash mismatch:");
    //         eprintln!("  Computed : {}", computed_hash_hex);
    //         eprintln!("  Expected : {}", expected_hash_hex.to_lowercase());
    //         std::process::exit(1);
    //     }
    // }

    // Unload a model from Triton
    pub async fn unload_model(&self, model_name: &str ) -> Result<(), TritonError> {
        let url = format!("{}/repository/models/{}/unload", self.url, model_name);
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;
        if response.status().is_success() {
            println!("Successfully unloaded model: {}", model_name);
            Ok(())
        } else {
            Err(TritonError::Http(response.status()))
        }
    }

    /// Fetches the metadata of a model from Triton Inference Server
      pub async fn get_model_metadata(&self, model_name: &str) -> Result<Value, TritonError> {
        let url = format!("{}/models/{}", self.url, model_name);
        
        println!("Fetching metadata for model: {}", model_name);
        
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let metadata: Value = response.json().await?;
            Ok(metadata)
        } else {
            println!("Failed to fetch metadata. Status: {:?}", response.status());
            Err(TritonError::Http(response.status()))
        }
    }

    // Get model status
    // pub async fn get_model_status(&self, model_name: &str) -> Result<ModelStatus, TritonError> {
    //     let url = format!("{}/models/{}/stats", TRITON_URL, model_name);
    //     let response = self.client.get(&url).send().await?;
    
    //     if response.status().is_success() {
    //         //Print the response to inspect
    //         let json_response: serde_json::Value = response.json().await?;
    //         println!("-------------------------------------------");
    //         println!("-------------------------------------------");
    //         println!("-------------------------------------------");
    //         println!("-------------------------------------------");
            
    //         println!("Triton Response: {:?}", json_response);

    //         println!("-------------------------------------------");
    //         println!("-------------------------------------------");
    //         println!("-------------------------------------------");
    //         println!("-------------------------------------------");
    
    //         // Navigate to `model_stats` array and get the first element
    //         if let Some(stats) = json_response["model_stats"].get(0) {
    //             // Deserialize the first element to ModelStatus
    //             let status: ModelStatus = serde_json::from_value(stats.clone())?;
    //             Ok(status)
    //         } else {
    //             Err(TritonError::InvalidResponse("No model stats found"))
    //         }
    //     } else {
    //         Err(TritonError::Http(response.status()))
    //     }
    // }
    
    pub fn load_data_from_file(&self, file_path: &str) -> Result<TensorData, Box<dyn std::error::Error>> {
        if file_path.ends_with(".jpg") || file_path.ends_with(".png") {
            let img = ImageReader::open(file_path)?.decode()?;
            let resized = img.resize_exact(224, 224, image::imageops::FilterType::Nearest);
            let rgb = resized.to_rgb8();
            let tensor_data: Vec<f32> = rgb
                .pixels()
                .flat_map(|p| vec![p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0])
                .collect();
            Ok(TensorData::F32(tensor_data))
        } else if file_path.ends_with(".csv") {
            let mut rdr = ReaderBuilder::new().from_path(file_path)?;
            let mut data_f32 = vec![];
            let mut data_i32 = vec![];
            let mut data_i64 = vec![];
            let mut data_u8 = vec![];
            let mut data_bool = vec![];
            let mut data_str = vec![];
    
            for result in rdr.records() {
                let record = result?;
                for field in record.iter() {
                    if let Ok(val) = field.parse::<f32>() {
                        data_f32.push(val);
                    } else if let Ok(val) = field.parse::<i32>() {
                        data_i32.push(val);
                    } else if let Ok(val) = field.parse::<i64>() {
                        data_i64.push(val);
                    } else if let Ok(val) = field.parse::<u8>() {
                        data_u8.push(val);
                    } else if let Ok(val) = field.parse::<bool>() {
                        data_bool.push(val);
                    } else {
                        data_str.push(field.to_string());
                    }
                }
            }
    
            // Choose the appropriate data type based on some logic, like content or file type
            if !data_f32.is_empty() {
                Ok(TensorData::F32(data_f32))
            } else if !data_i32.is_empty() {
                Ok(TensorData::I32(data_i32))
            } else if !data_i64.is_empty() {
                Ok(TensorData::I64(data_i64))
            } else if !data_u8.is_empty() {
                Ok(TensorData::U8(data_u8))
            } else if !data_bool.is_empty() {
                Ok(TensorData::Bool(data_bool))
            } else if !data_str.is_empty() {
                Ok(TensorData::Str(data_str))
            } else {
                Err("No valid data found in CSV".into())
            }
        } else if file_path.ends_with(".json") {
            let file = File::open(file_path)?;
            let reader = BufReader::new(file);
            let json_data: serde_json::Value = serde_json::from_reader(reader)?;
            
            // Extract the "data" field from the first object
            if let Some(array) = json_data.as_array() {
                if let Some(first_obj) = array.get(0) {
                    if let Some(data) = first_obj["data"].as_array() {
                        let data_f32: Vec<f32> = data
                            .iter()
                            .map(|v| v.as_f64().unwrap() as f32)
                            .collect();
                        
                        return Ok(TensorData::F32(data_f32));
                    }
                }
            }
            Err("Failed to parse data from JSON".into())
        } else {
            Err("Unsupported file type".into())
        }
    }
    

    pub async fn infer(
        &self,
        model_name: &str,
        file_path: &str,
    ) -> Result<Value, TritonError> {
        let metadata_url = format!("{}/models/{}", self.url, model_name);
        let metadata_response = self.client.get(&metadata_url).send().await?;
        if !metadata_response.status().is_success() {
            return Err(TritonError::Http(metadata_response.status()));
        }
    
        let metadata: Value = metadata_response.json().await?;
        let inputs = metadata["inputs"].as_array()
            .ok_or(TritonError::InvalidResponse("Invalid model metadata"))?;
    
        let expected_input = &inputs[0];
        let input_name = expected_input["name"].as_str().unwrap();
        let shape: Vec<usize> = expected_input["shape"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
    
        let tensor_data = self.load_data_from_file(file_path)
            .expect("Failed to load data from the specified file");
    
        let url = format!("{}/models/{}/infer", self.url, model_name);
        let response = self.client.post(&url)
            .json(&serde_json::json!({ "inputs": [json!({
                "name": input_name,
                "shape": shape,
                "datatype": "FP32",
                "data": tensor_data.to_serializable()
            })]}))
            .send()
            .await?;
        // println!("Payload sent to Triton: {:?}", serde_json::json!({ 
        //     "inputs": [json!({
        //         "name": input_name,
        //         "shape": shape,
        //         "datatype": "FP32",
        //         "data": tensor_data.to_serializable()
        //     })]
        // }));
    
    
        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(TritonError::Http(response.status()))
        }
    
    }
    
    pub async fn run_inference(
        &self,
        inputs: HashMap<String, TensorData>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        //  Load the Model
        // println!("⏳ Loading model: {}", self.model_name);
        // self.load_model().await.unwrap();
        match self.get_model_metadata().await {
            Ok(_) => println!(),
            Err(e) => {
                println!("Failed to fetch model metadata: {:?}", e);
                return Err(e);
            }
        }
        println!("-------------------------------------------");
    
        // Run Inference
        println!("Running inference...");
        match self.infer(model_name, file_path).await {
            Ok(result) => {
                println!("Inference Successful: {:#?}", result);
                println!("-------------------------------------------");
                println!("-------------------------------------------");
                self.unload_model(model_name).await?;
                Ok(result)
            },
            Err(e) => {
                println!("Inference failed: {:?}", e);
                self.unload_model(model_name).await?;
                Err(e)
            }
        }  
    }
    
    
}
