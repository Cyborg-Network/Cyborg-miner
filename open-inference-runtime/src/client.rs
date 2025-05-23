
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use serde_json::json;
use crate::models::{ModelExtractor,verify_model_blob};

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
    pub async fn is_server_live(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/health/live", self.url);
        let response = self.client.get(&url).send().await?;
        
        if response.status().is_success() {
            Ok(true)
        } else {
            Err(format!("❌ Server is not live. HTTP Status: {:?}", response.status()).into())
        }
    }

    

    // Check if the server is ready
    pub async fn is_server_ready(&self ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/health/ready", self.url);
        let response = self.client.get(&url).send().await?;
        
        if response.status().is_success() {
            Ok(true)
        } else {
            Err(format!("❌ Server is not live. HTTP Status: {:?}", response.status()).into())
        }
    }
    // Load a model into Triton
    pub async fn load_model(&self, model_name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>  {
        let url = format!("{}/repository/models/{}/load", self.url, model_name);
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;
        if response.status().is_success() {
            println!("Successfully loaded model: {}", model_name);
            Ok(())
        } else {
            Err(format!("Failed to load model '{}'. HTTP Status: {:?}", model_name, response.status()).into())
        }
    }

    // Unload a model from Triton
    pub async fn unload_model(&self, model_name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/repository/models/{}/unload", self.url, model_name);
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;
        
        if response.status().is_success() {
            println!("Successfully unloaded model: {}", model_name);
            Ok(())
        } else {
            Err(format!("Failed to unload model '{}'. HTTP Status: {:?}", model_name, response.status()).into())
        }
    }

    /// Fetches the metadata of a model from Triton Inference Server
    pub async fn get_model_metadata(&self, model_name: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/models/{}", self.url, model_name);
    
        println!("Fetching metadata for model: {}", model_name);
    
        let response = self.client.get(&url).send().await?;
    
        if response.status().is_success() {
            let metadata: Value = response.json().await?;
            Ok(metadata)
        } else {
            println!("Failed to fetch metadata. Status: {:?}", response.status());
            Err(format!("Failed to fetch metadata for model '{}'. HTTP Status: {:?}", model_name, response.status()).into())
        }
    }
   pub async fn align_inputs(
        &self,
        model_name: &str,
        inputs: HashMap<String, TensorData>,
    ) -> Result<HashMap<String, (TensorData, Vec<usize>)>, Box<dyn std::error::Error + Send + Sync>> {
        // Fetch model metadata
        let metadata_url = format!("{}/models/{}", self.url, model_name);
        let metadata_response = self.client.get(&metadata_url).send().await?;
    
        if !metadata_response.status().is_success() {
            let error_message = metadata_response.text().await.unwrap_or_default();
            return Err(format!("❌ Failed to fetch model metadata: HTTP- {}", error_message).into());
        }
    
        let metadata: serde_json::Value = metadata_response.json().await?;
        let model_inputs = metadata["inputs"]
            .as_array()
            .ok_or("❌ Invalid model metadata format: 'inputs' not found")?;
    
        let mut aligned_inputs = HashMap::new();
    
        for input in model_inputs {
            let name = input["name"]
                .as_str()
                .ok_or("❌ Model metadata is missing 'name'")?;
            let expected_shape = input["shape"]
                .as_array()
                .ok_or("❌ Model metadata is missing 'shape'")?
                .iter()
                .map(|v| v.as_u64().unwrap() as usize)
                .collect::<Vec<usize>>();
    
            let expected_len = expected_shape.iter().product::<usize>();
    
            let tensor_data = inputs
                .get(name)
                .ok_or_else(|| format!("❌ Missing input data for '{}'", name))?;
    
            let data_len = match tensor_data {
                TensorData::F32(data) => data.len(),
                TensorData::I32(data) => data.len(),
                TensorData::I64(data) => data.len(),
                TensorData::U8(data) => data.len(),
                TensorData::Bool(data) => data.len(),
                TensorData::Str(data) => data.len(),
            };
    
            if data_len != expected_len {
                return Err(format!(
                    "❌ Shape mismatch for '{}'. Expected {:?}, got {}",
                    name, expected_shape, data_len
                )
                .into());
            }
    
            aligned_inputs.insert(name.to_string(), (tensor_data.clone(), expected_shape));
        }
    
        Ok(aligned_inputs)
    }
    
    
    

   pub async fn infer(
        &self,
        model_name: &str,
        input_data: HashMap<&str, (TensorData, Vec<usize>)>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let model_inputs: Vec<_> = input_data.iter().map(|(name, (tensor_data, shape))| {
            let datatype = match tensor_data {
                TensorData::F32(_) => "FP32",
                TensorData::I32(_) => "INT32",
                TensorData::I64(_) => "INT64",
                TensorData::U8(_) => "UINT8",
                TensorData::Bool(_) => "BOOL",
                TensorData::Str(_) => "BYTES",
            };
            serde_json::json!({
                "name": name,
                "shape": shape,
                "datatype": datatype,
                "data": tensor_data.to_serializable()
            })
        }).collect();
    
        let request_body = serde_json::json!({ "inputs": model_inputs });
    
        let url = format!("{}/models/{}/infer", self.url, model_name);
        let response = self.client.post(&url)
            .json(&request_body)
            .send()
            .await?;
    
        if response.status().is_success() {
            let result = response.json::<serde_json::Value>().await?;
            Ok(result)
        } else {
            let error_message = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(format!("❌ Inference failed: HTTP - {}", error_message).into())
        }
    }
     
   pub async fn run_inference(
    &self,
    model_name: &str,
    inputs: HashMap<String, TensorData>,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    // Check if the model is already extracted
    match ModelExtractor::new(model_name) {
        Ok(extractor) => {
            if let Err(e) = extractor.extract_model() {
                println!("❌ Extraction failed: {:?}", e);
            } else {
                println!("✅ Model '{}' successfully extracted!", model_name);
            }
        }
        Err(e) => {
            println!("❌ Initialization failed: {:?}", e);
        }
    }
    println!("-------------------------------------------");

    // Check if the Triton Server is live
    println!("Checking if the server is live...");
    if !self.is_server_live().await? {
        return Err("Server is not live".into());
    }
    println!("Server is live!");
    println!("-------------------------------------------");

    // Check if the Triton Server is ready
    println!("Checking if the server is ready...");
    if !self.is_server_ready().await? {
        return Err("Server is not ready".into());
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

    //verify Model hash after being loaded
    verify_model_blob(&model_name)?;

    // Fetch Model Metadata (just for confirmation and debugging)
    println!("Fetching model metadata...");
    match self.get_model_metadata(model_name).await {
        Ok(metadata) => println!("Model Metadata: {:#?}", metadata),
        Err(e) => {
            println!("Failed to fetch model metadata: {:?}", e);
            return Err(e);
        }
    }

	    // Run Inference
	    println!("Running inference...");
	    let aligned_inputs_result = self.align_inputs(model_name, inputs).await;
	    match aligned_inputs_result {
		Ok(aligned_inputs) => {
		    let aligned_refs: HashMap<&str, (TensorData, Vec<usize>)> = aligned_inputs
		        .iter()
		        .map(|(k, v)| (k.as_str(), v.clone()))
		        .collect();

		    match self.infer(model_name, aligned_refs).await {
		        Ok(result) => {
		            println!("Inference Successful: {:#?}", result);
		            println!("-------------------------------------------");
		            println!("-------------------------------------------");
		            self.unload_model(model_name).await?;
		            Ok(result)
		        }
		        Err(e) => {
		            println!("Inference failed: {:?}", e);
		            self.unload_model(model_name).await?;
		            Err(format!("Inference failed: {:?}", e).into())
		        }
		    }
		}
		Err(e) => Err(format!("Inference failed: {:?}", e).into()),
	    }
	}

    
    
}