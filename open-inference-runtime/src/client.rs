use crate::models::ModelExtractor;
use futures::{stream::StreamExt, Future, Stream};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use rand::distr::weighted::WeightedIndex;
use rand::distr::Distribution;
use rand::thread_rng;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct TritonClient {
    client: Client,
    url: String,
    model_path: PathBuf,
    model_name: String, // ✅ now part of the struct
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
        let client = Client::new();

        println!("⏳ Checking if the server is live...");
        let live_url = format!("{}/health/live", triton_url);
        let response = client.get(&live_url).send().await?;
        if !response.status().is_success() {
            println!("Server is not live: {}", response.status());
        } else {
            println!("Server is live!");
        }

        println!("⏳ Checking if the server is ready...");
        let ready_url = format!("{}/health/ready", triton_url);
        let response = client.get(&ready_url).send().await?;
        if !response.status().is_success() {
            println!("Server is not ready: {}", response.status());
        } else {
            println!("Server is ready!");
        }

        println!("⏳ Fetching model list...");
        let repo_url = format!("{}/repository/index", triton_url);
        let repo_resp = client.post(&repo_url).send().await?;
        let models: Vec<serde_json::Value> = repo_resp.json().await?;

        let model_name = models
            .get(0)
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str())
            .ok_or("No model found in repository")?
            .to_string();

        println!("✅ Detected model: {}", model_name);
        println!("✅ Triton Client created.");

        Ok(Self {
            client,
            url: triton_url.to_string(),
            model_path,
            model_name,
        })
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
    pub async fn load_model(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let status_url = format!("{}/models/{}", self.url, self.model_name);
        let status_response = self.client.get(&status_url).send().await?;

        if status_response.status() == reqwest::StatusCode::OK {
            println!("Model '{}' is already loaded on Triton.", self.model_name);
            return Ok(());
        } else if status_response.status() != reqwest::StatusCode::NOT_FOUND {
            return Err(format!(
                "Failed to check model status '{}'. HTTP: {:?}",
                self.model_name,
                status_response.status()
            )
            .into());
        }

        match ModelExtractor::new(&self.model_name, self.model_path.clone()) {
            Ok(extractor) => {
                extractor.extract_model()?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                println!(
                    "✅ Model '{}' already extracted, continuing.",
                    self.model_name
                );
            }
            Err(e) => {
                return Err(Box::new(e));
            }
        }

        // 3. Load the model
        let url = format!("{}/repository/models/{}/load", self.url, self.model_name);
        let response = self.client.post(&url).json(&json!({})).send().await?;

        if response.status().is_success() {
            println!("✅ Model '{}' loaded successfully.", self.model_name);
            Ok(())
        } else {
            Err(format!(
                "Failed to load model '{}'. HTTP: {:?}",
                self.model_name,
                response.status()
            )
            .into())
        }
    }

    // Unload a model from Triton
    pub async fn unload_model(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let status_url = format!("{}/models/{}", self.url, self.model_name);
        let status_response = self.client.get(&status_url).send().await?;

        if status_response.status() == reqwest::StatusCode::NOT_FOUND {
            println!("⚠️ Model '{}' is not loaded on Triton.", self.model_name);
            return Ok(false);
        } else if !status_response.status().is_success() {
            return Err(format!(
                "Failed to check model status '{}'. HTTP: {:?}",
                self.model_name,
                status_response.status()
            )
            .into());
        }

        let url = format!("{}/repository/models/{}/unload", self.url, self.model_name);
        let response = self.client.post(&url).json(&json!({})).send().await?;

        if response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            if text.to_lowercase().contains("error") {
                return Err(format!("Unload failed with message: {}", text).into());
            }
            println!("✅ Model '{}' unloaded successfully.", self.model_name);
            Ok(true)
        } else {
            Err(format!(
                "Failed to unload model '{}'. HTTP: {:?}",
                self.model_name,
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
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        self.load_model()
            .await
            .map_err(|e| format!("Failed to load model: {}", e))?;

        let url = format!("{}/models/{}", self.url, self.model_name);

        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let metadata: Value = response.json().await?;
            Ok(metadata)
        } else {
            println!("Failed to fetch metadata. Status: {:?}", response.status());
            Err(format!(
                "Failed to fetch metadata for model '{}'. HTTP Status: {:?}",
                self.model_name,
                response.status()
            )
            .into())
        }
    }

    pub async fn get_model_stats(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        self.load_model()
            .await
            .map_err(|e| format!("Failed to load model: {}", e))?;
        let url = format!("{}/models/{}/stats", self.url, self.model_name);
        let response = Client::new().get(&url).send().await?;
        let json: Value = response.json().await?;
        Ok(json)
    }

    pub async fn generate_inputs(
        &self,
    ) -> Result<HashMap<String, (TensorData, Vec<usize>)>, Box<dyn std::error::Error + Send + Sync>>
    {
        let metadata = self.get_model_metadata().await?;
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
            let (command, inputs_opt) =
                if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&request) {
                    let cmd = map
                        .get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("infer")
                        .to_string();
                    let inputs = map.get("inputs").cloned();
                    (cmd, inputs)
                } else {
                    let parts: Vec<&str> = request.trim().split_whitespace().collect();
                    let cmd = parts.get(0).unwrap_or(&"").to_string();
                    (cmd, None)
                };

            let response_json = match command.as_str() {
                // Example > infer
                "infer" => {
                    self.load_model()
                        .await
                        .map_err(|e| format!("Failed to unload model: {}", e))?;

                    let inputs = if let Some(inputs_val) = inputs_opt {
                        match serde_json::from_value::<HashMap<String, (TensorData, Vec<usize>)>>(
                            inputs_val,
                        ) {
                            Ok(user_inputs) => user_inputs,
                            Err(e) => {
                                println!("⚠️ Failed to parse user inputs: {}", e);
                                match self.generate_inputs().await {
                                    Ok(dummy) => dummy,
                                    Err(e) => {
                                        println!(
                                            "{}",
                                            json!({ "error": format!("Failed to generate inputs: {}", e) })
                                        );
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    } else {
                        match self.generate_inputs().await {
                            Ok(dummy) => dummy,
                            Err(e) => {
                                println!(
                                    "{}",
                                    json!({ "error": format!("Failed to generate inputs: {}", e) })
                                );
                                return Ok(());
                            }
                        }
                    };

                    match self.run_inference(inputs).await {
                        Ok(output) => {
                            println!(
                                "🧠 Inference Output:\n{}",
                                serde_json::to_string_pretty(&output)?
                            );
                            self.unload_model()
                                .await
                                .map_err(|e| format!("Failed to unload model: {}", e))?;

                            output
                        }
                        Err(e) => json!({ "error": format!("Inference error: {}", e) }),
                    }
                }

                "metadata" => match self.get_model_metadata().await {
                    Ok(meta) => meta,
                    Err(e) => json!({ "error": format!("Failed to get metadata: {}", e) }),
                },

                "load" => match self.load_model().await {
                    Ok(_) => json!({ "load": "Success" }),
                    Err(e) => json!({ "error": format!("Failed to load model: {}", e) }),
                },

                "stats" => match self.get_model_stats().await {
                    Ok(status) => status,
                    Err(e) => json!({ "error": format!("Failed to get status: {}", e) }),
                },

                "unload" => match self.unload_model().await {
                    Ok(unloaded) => {
                        if unloaded {
                            json!({ "unload": "Success" })
                        } else {
                            json!({ "unload": "Model was not loaded" })
                        }
                    }
                    Err(e) => json!({ "error": format!("Failed to unload model: {}", e) }),
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

                //Example : {"command":"infertext","input":"Hello"}
                "infertext" => {
                    use serde_json::json;
                    println!("⏳ Loading model: {}", self.model_name);
                    self.load_model()
                        .await
                        .map_err(|e| format!("Failed to load model: {}", e))?;
                    let (prompt, max_len) = if let Some(inputs_val) = inputs_opt {
                        let p = inputs_val
                            .get("input")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let m = inputs_val
                            .get("max_len")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as usize)
                            .unwrap_or(128);

                        let wrapped = format!("<s>[INST] {} [/INST]", p);
                        (wrapped, m)
                    } else {
                        ("<s>[INST] Hello! [/INST]".to_string(), 128)
                    };

                    // ----------------- Load tokenizer -----------------
                    let tok_path = self.model_path.join(self.model_name.clone());
                    let tok = match crate::tokenizer::TextTokenizer::from_repo(tok_path) {
                        Ok(t) => t,
                        Err(e) => {
                            println!(
                                "{}",
                                json!({ "error": format!("Tokenizer load failed: {e}") })
                            );
                            return Ok(());
                        }
                    };

                    // -------- Helpers: logits & past extraction --------
                    fn extract_last_logits(raw_out: &serde_json::Value) -> Option<Vec<f32>> {
                        // Expect outputs[0] to be the logits tensor
                        let first = raw_out.get("outputs")?.as_array()?.get(0)?;
                        let shape = first.get("shape")?.as_array()?;

                        // Accept common shapes: [B, S, V], [S, V], or [V]
                        let (seq_len, vocab) = match shape.len() {
                            3 => {
                                let s = shape.get(1)?.as_u64()? as usize;
                                let v = shape.get(2)?.as_u64()? as usize;
                                (s, v)
                            }
                            2 => {
                                let s = shape.get(0)?.as_u64()? as usize;
                                let v = shape.get(1)?.as_u64()? as usize;
                                (s, v)
                            }
                            1 => {
                                let v = shape.get(0)?.as_u64()? as usize;
                                (1, v)
                            }
                            _ => return None,
                        };

                        let data = first.get("data")?.as_array()?;
                        if seq_len == 0 || vocab == 0 || data.len() < vocab {
                            return None;
                        }
                        let start = data.len().saturating_sub(vocab);
                        let mut out = Vec::with_capacity(vocab);
                        for v in &data[start..] {
                            out.push(v.as_f64().unwrap_or(0.0) as f32);
                        }
                        Some(out)
                    }

                    fn build_past_from(
                        raw_out: &serde_json::Value,
                    ) -> std::collections::HashMap<String, (crate::client::TensorData, Vec<usize>)>
                    {
                        use crate::client::TensorData;
                        let mut new_past = std::collections::HashMap::new();
                        for i in 0..22 {
                            let kname = format!("present.{i}.key");
                            let vname = format!("present.{i}.value");

                            if let Some(pkey) = raw_out.get(&kname) {
                                if let Some(arr) = pkey.get("data").and_then(|d| d.as_array()) {
                                    let f32_data: Vec<f32> = arr
                                        .iter()
                                        .filter_map(|v| v.as_f64().map(|x| x as f32))
                                        .collect();
                                    let shape = pkey
                                        .get("shape")
                                        .and_then(|s| s.as_array())
                                        .map(|sarr| {
                                            sarr.iter()
                                                .filter_map(|v| v.as_u64().map(|x| x as usize))
                                                .collect()
                                        })
                                        .unwrap_or_else(|| vec![]);
                                    new_past.insert(
                                        format!("past_key_values.{i}.key"),
                                        (TensorData::F32(f32_data), shape),
                                    );
                                }
                            }
                            if let Some(pval) = raw_out.get(&vname) {
                                if let Some(arr) = pval.get("data").and_then(|d| d.as_array()) {
                                    let f32_data: Vec<f32> = arr
                                        .iter()
                                        .filter_map(|v| v.as_f64().map(|x| x as f32))
                                        .collect();
                                    let shape = pval
                                        .get("shape")
                                        .and_then(|s| s.as_array())
                                        .map(|sarr| {
                                            sarr.iter()
                                                .filter_map(|v| v.as_u64().map(|x| x as usize))
                                                .collect()
                                        })
                                        .unwrap_or_else(|| vec![]);
                                    new_past.insert(
                                        format!("past_key_values.{i}.value"),
                                        (TensorData::F32(f32_data), shape),
                                    );
                                }
                            }
                        }
                        new_past
                    }

                    // ----------------- PREFILL (full prompt) -----------------
                    let mut all_tokens = match tok.encode_ids(&prompt, true, false, None) {
                        Ok(ids) => ids,
                        Err(e) => {
                            println!(
                                "{}",
                                json!({ "error": format!("Tokenization failed: {e}") })
                            );
                            return Ok(());
                        }
                    };

                    let prefill_inputs =
                        crate::tokenizer::make_llm_inputs_with_past(all_tokens.clone(), None);
                    let prefill_out = match self.run_inference(prefill_inputs).await {
                        Ok(o) => o,
                        Err(e) => {
                            println!(
                                "{}",
                                json!({ "error": format!("Inference (prefill) failed: {e}") })
                            );
                            return Ok(());
                        }
                    };

                    // Peek at shapes for sanity
                    if let Some(arr) = prefill_out
                        .get("outputs")
                        .and_then(|o| o.as_array())
                        .and_then(|a| a.get(0))
                    {
                        let shp = arr.get("shape").unwrap_or(&serde_json::Value::Null);
                        eprintln!("[prefill] logits shape = {shp}");
                    }
                    if let Some(p0k) = prefill_out.get("present.0.key") {
                        let shp = p0k.get("shape").unwrap_or(&serde_json::Value::Null);
                        eprintln!("[prefill] present.0.key shape = {shp}");
                    }

                    let mut logits = if let Some(l) = extract_last_logits(&prefill_out) {
                        l
                    } else {
                        println!("{}", json!({ "error": "No logits in prefill output" }));
                        return Ok(());
                    };

                    let mut past = Some(build_past_from(&prefill_out));

                    // ----------------- DECODE LOOP -----------------
                    let mut text = String::new();
                    let mut generated: Vec<i64> = Vec::new();
                    for step in 0..max_len {
                        // Greedy argmax
                        let next_token_id = if logits.is_empty() {
                            0
                        } else {
                            sample_from_logits(&logits, 0.7)
                        };

                        // Debug: show token and fragment
                        let piece = tok.decode_ids(&[next_token_id]).unwrap_or_default();
                        eprintln!("step {step}: id={next_token_id}, piece={piece:?}");

                        // Stop on EOS
                        if tok.eos_id().is_some() && Some(next_token_id) == tok.eos_id() {
                            eprintln!("EOS reached at step {step}");
                            break;
                        }

                        // Append and run one-token decode with KV cache
                        generated.push(next_token_id);
                        all_tokens.push(next_token_id);

                        let decode_inputs = crate::tokenizer::make_llm_inputs_with_past(
                            vec![next_token_id],
                            past.clone(),
                        );
                        let step_out = match self.run_inference(decode_inputs).await {
                            Ok(o) => o,
                            Err(e) => {
                                println!(
                                    "{}",
                                    json!({ "error": format!("Inference (decode step {step}) failed: {e}") })
                                );
                                break;
                            }
                        };

                        // Optional: watch the cache grow
                        if let Some(p0k) = step_out.get("present.0.key") {
                            let shp = p0k.get("shape").unwrap_or(&serde_json::Value::Null);
                            eprintln!("[step {step}] present.0.key shape = {shp}");
                        }

                        if let Some(l) = extract_last_logits(&step_out) {
                            logits = l;
                        } else {
                            println!("{}", json!({ "error": "No logits in decode output" }));
                            break;
                        }
                        past = Some(build_past_from(&step_out));
                    }

                    // ----------------- FINALIZE -----------------
                    let raw_text = tok.decode_ids(&all_tokens).unwrap_or_default();

                    // Clean up unwanted tokens and markers
                    let clean_text = raw_text
                        .replace("<|assistant|>", "")
                        .replace("<|user|>", "")
                        .replace("<|>", "")
                        .replace("[INST]", "")
                        .replace("[/INST]", "")
                        .replace("\n", " ")
                        .replace("\r", " ") 
                        .split_whitespace() 
                        .collect::<Vec<_>>()
                        .join(" ") 
                        .trim()
                        .to_string();

                    self.unload_model()
                        .await
                        .map_err(|e| format!("Failed to unload model: {}", e))?;
                    json!({ "text": clean_text })
                }

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
        inputs: HashMap<String, (TensorData, Vec<usize>)>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let aligned_refs: HashMap<&str, (TensorData, Vec<usize>)> = inputs
            .iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();

        match self.infer(aligned_refs).await {
            Ok(result) => Ok(result),
            Err(e) => Err(format!("Inference failed: {:?}", e).into()),
        }
    }
}

fn get_help_message() -> &'static str {
    r#"Available commands:
    infer                    - Run inference. Requires 'inputs' field in JSON format.
    infertext                - Run inference. Example :  
    metadata                 - Get model metadata.
    load                     - Load the model into memory.
    unload                   - Unload the model from memory.
    stats                    - Get statistics for a loaded model.
    list                     - List all available models in the repository.
    ping                     - Check basic connection (returns pong).
    live                     - Check if the Triton server is live.
    ready                    - Check if the Triton server is ready.

    Usage note:
    Use plain text like: 'load my_model' or use JSON for 'infer' with inputs.
    Example : {"command":"infertext","input":"Hello"} 
    Example : infer 
    "#
}


fn sample_from_logits(logits: &[f32], temperature: f32) -> i64 {
    let mut scaled: Vec<f32> = logits.iter().map(|&x| x / temperature).collect();
    let max = scaled.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exp: Vec<f32> = scaled.iter().map(|&x| (x - max).exp()).collect();
    let sum: f32 = exp.iter().sum();
    let probs: Vec<f32> = exp.iter().map(|&x| x / sum).collect();
    let dist = WeightedIndex::new(&probs).unwrap();
    let mut rng = thread_rng();
    dist.sample(&mut rng) as i64
}
