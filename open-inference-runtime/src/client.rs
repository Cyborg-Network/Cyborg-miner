use crate::models::ModelExtractor;
use futures::{stream::StreamExt, Future, Stream};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct TritonClient {
    client: Client,
    url: String,
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
        model_path: PathBuf,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let client = TritonClient {
            client: Client::new(),
            url: triton_url.to_string(),
            model_path,
        };

        println!("⏳ Checking if the server is live...");
        let url = format!("{}/health/live", &client.url);
        let response = client.client.get(&url).send().await?;
        if !response.status().is_success() {
            println!("Server is not live: {}", response.status());
        } else {
            println!("Server is live!");
        }

        println!("⏳ Checking if the server is ready...");
        let url = format!("{}/health/ready", &client.url);
        let response = client.client.get(&url).send().await?;
        if !response.status().is_success() {
            println!("Server is not ready: {}", response.status());
        } else {
            println!("Server is ready!");
        }

        Ok(client)
    }

    // Check if the server is live
    pub async fn is_server_live(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/health/live", self.url);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            Ok(true)
        } else {
            Err(format!("Server is not live. HTTP Status: {:?}", response.status()).into())
        }
    }

    // Check if the server is ready
    pub async fn is_server_ready(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/health/ready", self.url);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            Ok(true)
        } else {
            Err(format!("Server is not live. HTTP Status: {:?}", response.status()).into())
        }
    }
    // Load the model 
    pub async fn load_model(
    &self,
    model_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let status_url = format!("{}/models/{}", self.url, model_name);
    let status_response = self.client.get(&status_url).send().await?;

    if status_response.status() == reqwest::StatusCode::OK {
        println!("⚠️ Model '{}' is already loaded on Triton.", model_name);
        return Ok(()); // Model is already loaded, no need to load again
    } else if status_response.status() != reqwest::StatusCode::NOT_FOUND {
        return Err(format!(
            "Failed to check model status '{}'. HTTP: {:?}",
            model_name,
            status_response.status()
        )
        .into());
    }

    match ModelExtractor::new(model_name, self.model_path.clone()) {
        Ok(extractor) => {
            extractor.extract_model()?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            println!("✅ Model '{}' already extracted, continuing.", model_name);
        }
        Err(e) => {
            return Err(Box::new(e));
        }
    }

    // 3. Load the model
    let url = format!("{}/repository/models/{}/load", self.url, model_name);
    let response = self.client.post(&url).json(&json!({})).send().await?;

    if response.status().is_success() {
        println!("✅ Model '{}' loaded successfully.", model_name);
        Ok(())
    } else {
        Err(format!(
            "Failed to load model '{}'. HTTP: {:?}",
            model_name,
            response.status()
        )
        .into())
    }
}


    // Unload a model from Triton
    pub async fn unload_model(
    &self,
    model_name: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let status_url = format!("{}/models/{}", self.url, model_name);
    let status_response = self.client.get(&status_url).send().await?;

    if status_response.status() == reqwest::StatusCode::NOT_FOUND {
        println!("⚠️ Model '{}' is not loaded on Triton.", model_name);
        return Ok(false);  
    } else if !status_response.status().is_success() {
        return Err(format!(
            "Failed to check model status '{}'. HTTP: {:?}",
            model_name,
            status_response.status()
        ).into());
    }

    let url = format!("{}/repository/models/{}/unload", self.url, model_name);
    let response = self.client.post(&url).json(&json!({})).send().await?;

    if response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        if text.to_lowercase().contains("error") {
            return Err(format!("Unload failed with message: {}", text).into());
        }
        println!("✅ Model '{}' unloaded successfully.", model_name);
        Ok(true)
    } else {
        Err(format!(
            "Failed to unload model '{}'. HTTP: {:?}",
            model_name,
            response.status()
        )
        .into())
    }
}


    // Fetch the list of availabe models in repository
    pub async fn list_models(
        &self,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/repository/index", self.url);
        let response = self.client.post(&url).send().await?;

        if response.status().is_success() {
            let models = response.json::<Vec<serde_json::Value>>().await?;
            let names = models
                .iter()
                .filter_map(|entry| entry.get("name").and_then(|v| v.as_str()))
                .map(String::from)
                .collect::<Vec<_>>();
            Ok(names)
        } else {
            Err(format!("Failed to list models: HTTP {}", response.status()).into())
        }
    }

    /// Fetches the metadata of a model from Triton Inference Server
    pub async fn get_model_metadata(
        &self,
        model_name: &str,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        self.load_model(model_name)
            .await
            .map_err(|e| format!("Failed to load model: {}", e))?;

        match ModelExtractor::new(model_name, self.model_path.clone()) {
            Ok(extractor) => {
                extractor.extract_model()?;
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                println!("✅ Model '{}' already extracted, continuing.", model_name);
            }
            Err(e) => {
                return Err(Box::new(e));
            }
        }
        let url = format!("{}/models/{}", self.url, model_name);

        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let metadata: Value = response.json().await?;
            Ok(metadata)
        } else {
            println!("Failed to fetch metadata. Status: {:?}", response.status());
            Err(format!(
                "Failed to fetch metadata for model '{}'. HTTP Status: {:?}",
                model_name,
                response.status()
            )
            .into())
        }
    }

    pub async fn get_model_stats(
        &self,
        model_name: &str,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        self.load_model(model_name)
            .await
            .map_err(|e| format!("Failed to load model: {}", e))?;

        match ModelExtractor::new(model_name, self.model_path.clone()) {
            Ok(extractor) => {
                extractor.extract_model()?;
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                println!("✅ Model '{}' already extracted, continuing.", model_name);
            }
            Err(e) => {
                return Err(Box::new(e));
            }
        }
        let url = format!("{}/models/{}/stats", self.url, model_name);
        let response = Client::new().get(&url).send().await?;
        let json: Value = response.json().await?;
        Ok(json)
    }

    pub async fn generate_inputs(
    &self,
    model_name: &str,
) -> Result<HashMap<String, (TensorData, Vec<usize>)>, Box<dyn std::error::Error + Send + Sync>> {
    let metadata = self.get_model_metadata(model_name).await?;
    let model_inputs = metadata["inputs"]
        .as_array()
        .ok_or("Invalid model metadata format: 'inputs' not found")?;

    let mut inputs = HashMap::new();

    for input in model_inputs {
        let name = input["name"]
            .as_str()
            .ok_or("Input name missing")?
            .to_string();

        let shape = input["shape"]
            .as_array()
            .ok_or("Shape missing")?
            .iter()
            .map(|v| {
                if let Some(signed) = v.as_i64() {
                    if signed < 0 {
                        Ok(1usize)
                    } else {
                        Ok(signed as usize)
                    }
                } else {
                    Err("Invalid shape element: expected i64")
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let total_elements = shape.iter().product::<usize>();
        let datatype = input["datatype"].as_str().ok_or("Datatype missing")?;

        let tensor_data = match datatype {
            "FP32" => TensorData::F32(vec![0.0; total_elements]),
            "INT32" => TensorData::I32(vec![0; total_elements]),
            "INT64" => TensorData::I64(vec![0; total_elements]),
            "UINT8" => TensorData::U8(vec![0; total_elements]),
            "BOOL" => TensorData::Bool(vec![false; total_elements]),
            "BYTES" => TensorData::Str(vec!["".to_string(); total_elements]),
            _ => {
                return Err(format!("Unsupported datatype: {}", datatype).into());
            }
        };

        inputs.insert(name, (tensor_data, shape));
    }

    Ok(inputs)
}

    pub async fn infer(
        &self,
        model_name: &str,
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

        let url = format!("{}/models/{}/infer", self.url, model_name);
        let response = self.client.post(&url).json(&request_body).send().await?;

        if response.status().is_success() {
            let result = response.json::<serde_json::Value>().await?;
            Ok(result)
        } else {
            let error_message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(format!("Inference failed: HTTP - {}", error_message).into())
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
        let (command, model_name_opt, inputs_opt) = if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&request) {
        let cmd = map.get("command").and_then(|v| v.as_str()).unwrap_or("infer").to_string();
        let model_name = map.get("model_name").and_then(|v| v.as_str()).map(|s| s.to_string());
        let inputs = map.get("inputs").cloned(); 
        (cmd, model_name, inputs)
    } else {
        let parts: Vec<&str> = request.trim().split_whitespace().collect();
        let cmd = parts.get(0).unwrap_or(&"").to_string();
        let model_name = parts.get(1).map(|s| s.to_string());
        (cmd, model_name, None)
    };

            let response_json = match command.as_str() {
               "infer" => {
                        match model_name_opt {
                            Some(ref model_name) => {
                                let inputs = if let Some(inputs_val) = inputs_opt {
                                    match serde_json::from_value::<HashMap<String, (TensorData, Vec<usize>)>>(inputs_val) {
                                        Ok(user_inputs) => user_inputs,
                                        Err(e) => {
                                            println!("⚠️ Failed to parse user inputs: {}", e);
                                            match self.generate_inputs(model_name).await {
                                                Ok(dummy) => dummy,
                                                Err(e) => {
                                                    println!("{}", json!({ "error": format!("Failed to generate inputs: {}", e) }));
                                                    return Ok(()); 
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    
                                    match self.generate_inputs(model_name).await {
                                        Ok(dummy) => dummy,
                                        Err(e) => {
                                            println!("{}", json!({ "error": format!("Failed to generate inputs: {}", e) }));
                                            return Ok(()); 
                                        }
                                    }
                                };

                                match self.run_inference(model_name, inputs).await {
                                    Ok(output) => {
                                        println!("🧠 Inference Output:\n{}", serde_json::to_string_pretty(&output)?);
                                        output
                                    }
                                    Err(e) => json!({ "error": format!("Inference error: {}", e) }),
                                }
                            }
                            None => json!({ "error": "'model_name' is required for inference." }),
                        }
                    }

                "metadata" => match model_name_opt {
                    Some(model_name) => match self.get_model_metadata(&model_name).await {
                        Ok(meta) => meta,
                        Err(e) => json!({ "error": format!("Failed to get metadata: {}", e) }),
                    },
                    None => json!({ "error": "'model_name' is required for metadata command." }),
                },

                "load" => match model_name_opt {
                    Some(model_name) => match self.load_model(&model_name).await {
                        Ok(_) => json!({ "load": "Success" }),
                        Err(e) => json!({ "error": format!("Failed to load model: {}", e) }),
                    },
                    None => json!({ "error": "'model_name' is required for load command." }),
                },
                "stats" => match model_name_opt {
                    Some(model_name) => match self.get_model_stats(&model_name).await {
                        Ok(status) => status,
                        Err(e) => json!({ "error": format!("Failed to get status: {}", e) }),
                    },
                    None => json!({ "error": "'model_name' is required for status command." }),
                },
                "unload" => match model_name_opt {
                    Some(model_name) => match self.unload_model(&model_name).await {
                        Ok(unloaded) => {
                            if unloaded {
                                json!({ "unload": "Success" })
                            } else {
                                json!({ "unload": "Model was not loaded" })
                            }
                        }
                        Err(e) => json!({ "error": format!("Failed to unload model: {}", e) }),
                    },
                    None => json!({ "error": "'model_name' is required for unload command." }),
                },

                "ping" => json!({ "response": "pong" }),

                "live" => match self.is_server_live().await {
                    Ok(true) => json!({ "live": true }),
                    Ok(false) => json!({ "live": false }),
                    Err(e) => json!({ "error": format!("Live check failed: {}", e) }),
                },

                "ready" => match self.is_server_ready().await {
                    Ok(true) => json!({ "ready": true }),
                    Ok(false) => json!({ "ready": false }),
                    Err(e) => json!({ "error": format!("Ready check failed: {}", e) }),
                },
                "list" => match self.list_models().await {
                    Ok(models) => json!({ "models": models }),
                    Err(e) => json!({ "error": format!("Failed to list models: {}", e) }),
                },

                _ => {
                    let help_msg = get_help_message();
                    let formatted_msg =
                        format!("❓ Unknown command: '{}'\n\n{}", command, help_msg);
                    json!({ "message": formatted_msg })
                }
            };
            if let Some(msg) = response_json.get("message").and_then(|v| v.as_str()) {
                println!("{msg}");
            } else {
                println!("{}", response_json);
            }

            response_closure(response_json.to_string()).await;
        }

        Ok(())
    }


    pub async fn run_inference(
        &self,
        model_name: &str,
        inputs: HashMap<String, (TensorData, Vec<usize>)>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        println!("⏳ Loading model: {}", model_name);
        self.load_model(model_name)
            .await
            .map_err(|e| format!("Failed to load model: {}", e))?;

        let aligned_refs: HashMap<&str, (TensorData, Vec<usize>)> = inputs
            .iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();

        match self.infer(model_name, aligned_refs).await {
            Ok(result) => {
                self.unload_model(model_name).await?;
                Ok(result)
            }
            Err(e) => Err(format!("Inference failed: {:?}", e).into()),
        }
    }
}

fn get_help_message() -> &'static str {
    r#"Available commands:
    infer <model_name>       - Run inference. Requires 'inputs' field in JSON format. 
    metadata <model_name>    - Get model metadata.
    load <model_name>        - Load the model into memory.
    unload <model_name>      - Unload the model from memory.
    stats <model_name>       - Get statistics for a loaded model.
    list                     - List all available models in the repository.
    ping                     - Check basic connection (returns pong).
    live                     - Check if the Triton server is live.
    ready                    - Check if the Triton server is ready.

    Usage note:
    Use plain text like: 'load my_model' or use JSON for 'infer' with inputs.
    Example : {"command":"infer","model_name":"densenet_onnx","inputs":{"INPUT0":[0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8],"INPUT1":[1,2,3,4]}}
    "#
}
