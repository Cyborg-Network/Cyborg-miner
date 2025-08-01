use crate::models::ModelExtractor;
use futures::{stream::StreamExt, Future, Stream};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

pub struct TritonClient {
    client: Client,
    url: String,
    model_name: String,
    model_path: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub async fn new(
        triton_url: &str,
        model_name: &str,
        model_path: PathBuf,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Initialize the client
        let client = TritonClient {
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
    pub async fn unload_model(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/repository/models/{}/unload", self.url, self.model_name);
        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({}))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Failed to unload model '{}'. HTTP Status: {:?}",
                self.model_name,
                response.status()
            )
            .into())
        }
    }

    /// Fetches the metadata of a model from Triton Inference Server
    pub async fn get_model_metadata(
        &self,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/models/{}", self.url, self.model_name);

        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let metadata: Value = response.json().await?;
            Ok(metadata)
        } else {
            println!(
                "❌ Failed to fetch metadata. Status: {:?}",
                response.status()
            );
            Err(format!(
                "❌ Failed to fetch metadata for model '{}'. HTTP Status: {:?}",
                self.model_name,
                response.status()
            )
            .into())
        }
    }
    pub async fn align_inputs(
        &self,
        inputs: HashMap<String, TensorData>,
    ) -> Result<HashMap<String, (TensorData, Vec<usize>)>, Box<dyn std::error::Error + Send + Sync>>
    {
        // Fetch model metadata
        let metadata_url = format!("{}/models/{}", self.url, self.model_name);
        let metadata_response = self.client.get(&metadata_url).send().await?;

        if !metadata_response.status().is_success() {
            let error_message = metadata_response.text().await.unwrap_or_default();
            return Err(
                format!("❌ Failed to fetch model metadata: HTTP- {}", error_message).into(),
            );
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
        input_data: HashMap<&str, (TensorData, Vec<usize>)>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let model_inputs: Vec<_> = input_data
            .iter()
            .map(|(name, (tensor_data, shape))| {
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
            })
            .collect();

        let request_body = serde_json::json!({ "inputs": model_inputs });

        let url = format!("{}/models/{}/infer", self.url, self.model_name);
        let response = self.client.post(&url).json(&request_body).send().await?;

        if response.status().is_success() {
            let result = response.json::<serde_json::Value>().await?;
            Ok(result)
        } else {
            let error_message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(format!("❌ Inference failed: HTTP - {}", error_message).into())
        }
    }

    pub async fn run<S, C, CFut>(
        &self,
        mut request_stream: S,
        mut response_closure: C,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        S: Stream<Item = String> + Unpin + Send + 'static,
        C: FnMut(String) -> CFut + Send + 'static,
        CFut: Future<Output = ()> + Send + 'static,
    {
        while let Some(request) = request_stream.next().await {
            let parsed_inputs: Result<HashMap<String, TensorData>, _> =
                serde_json::from_str(&request);

            let result: Result<Value, Box<dyn std::error::Error + Send + Sync>> =
                match parsed_inputs {
                    Ok(inputs) => self.run_inference(inputs).await,
                    Err(e) => {
                        println!("❌ Failed to parse inputs: {}", e);
                        Err(format!("Invalid input format: {}", e).into())
                    }
                };

            let response = match result {
                Ok(json) => json.to_string(),
                Err(e) => format!("❌ Inference error: {}", e),
            };

            response_closure(response).await;
        }

        Ok(())
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
                return Err(e);
            }
        }

        // Run Inference
        let aligned_inputs_result = self.align_inputs(inputs).await;
        match aligned_inputs_result {
            Ok(aligned_inputs) => {
                let aligned_refs: HashMap<&str, (TensorData, Vec<usize>)> = aligned_inputs
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.clone()))
                    .collect();

                match self.infer(aligned_refs).await {
                    Ok(result) => {
                        self.unload_model().await?;
                        Ok(result)
                    }
                    Err(e) => {
                        self.unload_model().await?;
                        Err(format!("❌ Inference failed: {:?}", e).into())
                    }
                }
            }
            Err(e) => Err(format!("❌ Inference failed: {:?}", e).into()),
        }
    }
}
