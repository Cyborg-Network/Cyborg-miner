
// client.rs
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use serde_json::json;
use image::io::Reader as ImageReader;
use csv::ReaderBuilder;
use std::fs::File;
use std::path::Path;
use std::io::Cursor;
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
            url:TRITON_URL
        }
    }

    // Check if the server is live
    pub async fn is_server_live(&self,) -> Result<bool, TritonError> {
        let url = format!("{}/health/live", self.url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }

    

    // Check if the server is ready
    pub async fn is_server_ready(&self ) -> Result<bool, TritonError> {
        let url = format!("{}/health/ready", self.url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }

    // List all models currently loaded
    pub async fn list_models(&self) -> Result<(), TritonError> {
        let url = format!("{}/repository/index", self.url);
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;

        if response.status().is_success() {
            let models: Value = response.json().await?;
            
            println!("+---------------------------------+---------+---------+");
            println!("| Model                           | Version |");
            println!("+---------------------------------+---------+---------+");

            if let Some(model_list) = models.as_array() {
                for model in model_list {
                    let name = model["name"].as_str().unwrap_or("N/A");
                    let version = model["version"].as_str().unwrap_or("N/A");
                    // let status = model["state"].as_str().unwrap_or("UNKNOWN");

                    println!(
                        "| {:<31} | {:<7} | ",
                        name,
                        version,
                        // status
                    );
                }
            }

            println!("+---------------------------------+---------+---------+");
            Ok(())
        } else {
            Err(TritonError::Http(response.status()))
        }
    }

    // Load a model into Triton
    pub async fn load_model(&self, model_name: &str) -> Result<(), TritonError> {
        let url = format!("{}/repository/models/{}/load", self.url, model_name);
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;
        if response.status().is_success() {
            println!("Successfully loaded model: {}", model_name);
            Ok(())
        } else {
            Err(TritonError::Http(response.status()))
        }
    }

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
    //     let url = format!("{}/models/{}/stats", self.url, model_name);
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
    
    pub async fn infer(
        &self,
        model_name: &str,
        input_data: HashMap<&str, (TensorData, Vec<usize>)>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // Step 1: Get model metadata
        let metadata_url = format!("{}/models/{}", self.url, model_name);
        let metadata_response = self.client.get(&metadata_url).send().await?;
    
        if !metadata_response.status().is_success() {
            let error_message = metadata_response.text().await?;
            return Err(format!("❌ Failed to fetch model metadata: HTTP- {}", error_message).into());
        }
    
        let metadata: serde_json::Value = metadata_response.json().await?;
    
        // Step 2: Extract input names and shapes
        let inputs = metadata["inputs"]
            .as_array()
            .ok_or("❌ Invalid model metadata format: 'inputs' not found")?;
    
        // Step 3: Map the inputs to Triton's expected format
        let mut model_inputs = vec![];
        for input in inputs {
            let name = input["name"].as_str().unwrap();
            let expected_shape = input["shape"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as usize)
                .collect::<Vec<usize>>();
    
            if let Some((tensor_data, shape)) = input_data.get(name) {
                // Auto-fix if batch dimension is there
                let mut corrected_shape = shape.clone();
                if shape.len() == expected_shape.len() + 1 && shape[0] == 1 {
                    corrected_shape.remove(0);
                }
    
                // Shape check
                if corrected_shape == expected_shape {
                    // Get the datatype from the enum
                    let datatype = match tensor_data {
                        TensorData::F32(_) => "FP32",
                        TensorData::I32(_) => "INT32",
                        TensorData::I64(_) => "INT64",
                        TensorData::U8(_) => "UINT8",
                        TensorData::Bool(_) => "BOOL",
                        TensorData::Str(_) => "BYTES",
                    };
    
                    // Add to the model inputs with serialization
                    model_inputs.push(serde_json::json!({
                        "name": name,
                        "shape": corrected_shape,
                        "datatype": datatype,
                        "data": tensor_data.to_serializable()
                    }));
                } else {
                    println!("⚠️  Shape mismatch for '{}'. Expected {:?}, but got {:?}", name, expected_shape, corrected_shape);
                    return Err(format!("❌ Shape mismatch for '{}'. Expected {:?}, but got {:?}", name, expected_shape, corrected_shape).into());
                }
            } else {
                println!("⚠️  Missing data for input: '{}'", name);
                return Err(format!("❌ Missing data for input: '{}'", name).into());
            }
        }
    
        // Step 4: Build the request body
        let request_body = serde_json::json!({
            "inputs": model_inputs
        });
    
        // Step 5: Send the inference request
        let url = format!("{}/models/{}/infer", self.url, model_name);
        let response = self.client.post(&url)
            .json(&request_body)
            .send()
            .await?;
    
        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            Ok(result)
        } else {
            let error_message = response.text().await?;
            println!("❌ Inference failed. Response: {}", error_message);
            Err(format!("❌ Inference failed: HTTP - {}", error_message).into())
        }
    }
    
   
   
    
    pub async fn run_inference(
        &self,
        model_name: &str,   // Give me exact name of the model
        input_data: HashMap<&str, (TensorData, Vec<usize>)>, // Input file path
    ) -> Result<Value, TritonError> {
        // Check if the model is already extracted
       
        match ModelExtractor::new(model_name) {
            Ok(extractor) => {
                if let Err(e) = extractor.extract_model() {
                    println!("❌ Extraction failed: {:?}", e);
                } else {
                    println!("✅ Model '{}' successfully extracted!", model_name);
                }
            },
            Err(e) => {
                println!("❌ Initialization failed: {:?}", e);
            }
        }
        println!("-------------------------------------------");


        // Check if the Triton Server is live
        println!("Checking if the server is live...");
        if !self.is_server_live().await? {
            return Err(TritonError::InvalidResponse("Server is not live"));
        }
        println!("Server is live!");
        println!("-------------------------------------------");
    
        // Check if the Triton Server is ready
        println!("Checking if the server is ready...");
        if !self.is_server_ready().await? {
            return Err(TritonError::InvalidResponse("Server is not ready"));
        }
        println!("Server is ready!");
        println!("-------------------------------------------");
    
        // Load the Model
        println!("Loading model: {}", model_name);
        match self.load_model(model_name).await {
            Ok(_) => println!("Model loaded successfully!"),
            Err(e) => {
                println!("Failed to load model: {:?}", e);
                return Err(e);
            }
        }
        println!("-------------------------------------------");
        println!("-------------------------------------------");
    
        // Fetch Model Metadata (just for confirmation and debugging)
        println!("Fetching model metadata...");
        match self.get_model_metadata(model_name).await {
            Ok(metadata) => println!("Model Metadata: {:#?}", metadata),
            Err(e) => {
                println!("Failed to fetch model metadata: {:?}", e);
                return Err(e);
            }
        }
        // println!("-------------------------------------------");
    
        // Run Inference
        println!("Running inference...");
        match self.infer(model_name,input_data).await {
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
                Err(TritonError::Unknown(format!("Inference failed: {:?}", e)))

            }
        }  
    }
    
    
}
