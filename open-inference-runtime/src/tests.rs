#[cfg(test)]

mod tests {
    use super::*;
    
    use crate::client::TritonClient;
    use crate::error::TritonError;
    use crate::models::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;
    use tokio;

    #[tokio::test]
    async fn test_server_live() {
        let client = TritonClient::new("http://localhost:8000/v2".to_string());
        let result = client.is_server_live().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_server_ready() {
        let client = TritonClient::new("http://localhost:8000/v2".to_string());
        let result = client.is_server_ready().await;
        assert!(result.is_ok());
    }

    // #[tokio::test]
    // async fn test_list_models() {
    //     let client = TritonClient::new("http://localhost:8000".to_string());
    //     let result = client.list_models().await;
    //     assert!(result.is_ok());
    // }

    // #[tokio::test]
    // async fn test_load_and_unload_model() {
    //     let client = TritonClient::new("http://localhost:8000".to_string());
    //     let model_name = "test_model";

    //     let load_result = client.load_model(model_name).await;
    //     assert!(load_result.is_ok());

    //     let unload_result = client.unload_model(model_name).await;
    //     assert!(unload_result.is_ok());
    // }

    // #[tokio::test]
    // async fn test_get_model_metadata() {
    //     let client = TritonClient::new("http://localhost:8000".to_string());
    //     let model_name = "test_model";
    //     let result = client.get_model_metadata(model_name).await;
    //     assert!(result.is_ok());
    // }

    // #[test]
    // fn test_load_data_from_image() {
    //     let client = TritonClient::new("http://localhost:8000".to_string());
    //     let image_path = "test_image.png";
    //     let mut file = File::create(image_path).unwrap();
    //     file.write_all(&[0u8; 224 * 224 * 3]).unwrap();

    //     let result = client.load_data_from_file(image_path);
    //     assert!(result.is_ok());
    // }

    #[test]
    fn test_load_data_from_csv() {
        let client = TritonClient::new("http://localhost:8000/v2".to_string());
        let csv_path = "test_data.csv";
        let mut file = File::create(csv_path).unwrap();
        file.write_all(b"1.0,2.0,3.0
4.0,5.0,6.0").unwrap();

        let result = client.load_data_from_file(csv_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_data_from_json() {
        let client = TritonClient::new("http://localhost:8000/v2".to_string());
        let json_path = "test_data.json";
        let mut file = File::create(json_path).unwrap();
        file.write_all(br#"[{"data": [1.0, 2.0, 3.0]}]"#).unwrap();

        let result = client.load_data_from_file(json_path);
        assert!(result.is_ok());
    }

//     #[tokio::test]
//     async fn test_infer() {
//         let client = TritonClient::new("http://localhost:8000".to_string());
//         let model_name = "test_model";
//         let file_path = "test_data.csv";

//         let mut file = File::create(file_path).unwrap();
//         file.write_all(b"1.0,2.0,3.0
// 4.0,5.0,6.0").unwrap();

//         let result = client.infer(model_name, file_path).await;
//         assert!(result.is_ok());
//     }

//     #[tokio::test]
//     async fn test_run_inference() {
//         let client = TritonClient::new("http://localhost:8000".to_string());
//         let model_name = "test_model";
//         let archive_path = "test_model.tar.gz";
//         let extract_to = "./models";
//         let file_path = "test_data.csv";

//         let mut file = File::create(file_path).unwrap();
//         file.write_all(b"1.0,2.0,3.0
// 4.0,5.0,6.0").unwrap();

//         let result = client.run_inference(model_name, archive_path, extract_to, file_path).await;
//         assert!(result.is_ok());
//     }
}