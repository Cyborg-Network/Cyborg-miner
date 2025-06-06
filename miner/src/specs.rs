use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::{Command, Stdio};
use std::{env, str};
use sysinfo::{MemoryRefreshKind, RefreshKind, System};

use crate::{
    error::Result,
    substrate_interface::api::runtime_types::bounded_collections::bounded_vec::BoundedVec,
    types::{IpResponse, MinerConfig},
};

#[derive(Deserialize, Debug)]
struct IpLocation {
    loc: Option<String>,
}

type Coordinates = (i32, i32);

#[derive(Serialize, Deserialize)]
pub struct Location {
    coordinates: Coordinates,
}

pub async fn gather_worker_spec() -> Result<MinerConfig> {

    println!("Using IP: {}", env::var("CYBORG_WORKER_NODE_TEST_IP").unwrap_or("".to_string()));

    let response = match env::var("CYBORG_WORKER_NODE_TEST_IP") {
        Ok(val) => val,
        Err(_) => {
            reqwest::get("https://api.ipify.org?format=json")
                .await?
                .json::<IpResponse>()
                .await?
                .ip
        }
    };

    //let response = worker::IpResponse { ip: String::from("127.0.0.1") };

    let location = Location::get_location().await;

    let ram = return_total_memory();

    let cpu = get_cpu_cores();

    let storage = return_total_storage();

    Ok(MinerConfig {
        domain: BoundedVec::from(BoundedVec(response.as_bytes().to_vec())),
        latitude: location.coordinates.0,
        longitude: location.coordinates.1,
        ram,
        storage,
        cpu,
    })
}

fn get_cpu_cores() -> u16 {
    let mut sys = System::new_all();
    sys.refresh_all();

    sys.cpus().len() as u16
}

impl Location {
    pub async fn get_location() -> Location {
        // Try getting GPS location first
        if let Ok((lat, lon)) = get_gps_location() {
            return Location {
                coordinates: f64_to_i32_coordinates(lat, lon),
            };
        }
        
        match get_ip_location().await {
            Ok((lat, lon)) => {
                println!("Failed to get GPS location. Falling back to IP-based geolocation.");
                return Location {
                    coordinates: f64_to_i32_coordinates(lat, lon),
                };
            }
            Err(e) => {
                panic!("Failed to get the location: {}", e);
            }
        }
    }
}

fn f64_to_i32_coordinates(lat: f64, lon: f64) -> Coordinates {
    let lat_i32 = (lat * 1_000_000.00).round() as i32;
    let lon_i32 = (lon * 1_000_000.00).round() as i32;

    (lat_i32, lon_i32)
}

fn get_gps_location() -> Result<(f64, f64)> {
    // Use gpspipe to get single GPS datum
    let output = Command::new("gpspipe")
        .arg("-w")
        .arg("-n")
        .arg("1")
        .output()?;

    if !output.status.success() {
        return Err("Failed to execute gpspipe".into());
    }

    // Convert GPS data to string
    let gps_data = String::from_utf8_lossy(&output.stdout);
    println!("GPS data: {}", gps_data); // Debugging purposes

    let json: Value = serde_json::from_str(&gps_data)?;

    // Extract latitude and longitude (adjust based on the actual JSON structure)
    if let Some(lat) = json["lat"].as_f64() {
        if let Some(lon) = json["lon"].as_f64() {
            return Ok((lat, lon));
        }
    }

    Err("Failed to extract latitude and longitude from GPS data".into())
}

async fn get_ip_location() -> Result<(f64, f64)> {
    let url = "https://ipinfo.io/json";
    let response = reqwest::get(url).await?;

    if response.status().is_success() {
        let ip_info: IpLocation = response.json().await?;

        let loc = ip_info
            .loc
            .ok_or_else(|| "Failed to get location via IP.")?;

        let loc_parts: Vec<&str> = loc.split(',').collect();

        if loc_parts.len() == 2 {
            let lat = loc_parts[0]
                .parse::<f64>()
                .map_err(|_| "Failed to parse latitude")?;
            let lon = loc_parts[1]
                .parse::<f64>()
                .map_err(|_| "Failed to parse longitude")?;

            return Ok((lat, lon));
        }

        Err("Failed to get location via IP.".into())
    } else {
        Err("Failed to get location via IP.".into())
    }
}

#[allow(dead_code)]
pub async fn get_memory() -> Result<String> {
    let ps_child = Command::new("free") // `ps` command...
        .arg("-h") // with argument `axww`...
        .stdout(Stdio::piped()) // of which we will pipe the output.
        .spawn() // Once configured, we actually spawn the command...
        .unwrap(); // and assert everything went right.
    let grep_child_one = Command::new("grep")
        .arg("-i")
        .arg("Mem")
        .stdin(Stdio::from(ps_child.stdout.unwrap())) // Pipe through.
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let output = grep_child_one.wait_with_output().unwrap();
    let result = str::from_utf8(&output.stdout).unwrap();

    let res = &result.to_string()[14..19];

    Ok(res.to_string())
}

pub fn return_total_memory() -> u64 {
    let mut system =
        System::new_with_specifics(RefreshKind::new().with_memory(MemoryRefreshKind::new()));
    system.refresh_memory();

    system.total_memory()
}

//disk space needs to be retrieved like this, because the rust crates will return total diskspaces
//and are not filterable by mountpoint like this is, which causes issues on VPS
//this should only be a workaround until we have a better solution
pub fn return_total_storage() -> u64 {
    let output = Command::new("df")
        .arg("--block-size=1")
        .arg("--total")
        .arg("-B1")
        .output()
        .expect("Failed to execute command");

    // Print the raw command output for debugging
    println!(
        "Command output: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8");

    let mut total_space: u64 = 0;

    for line in stdout.lines().skip(1) {
        // Skip the header line
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Check if the first column (filesystem) starts with "/dev/"
        if let Some(filesystem) = parts.get(0) {
            if filesystem.starts_with("/dev/") {
                if let Some(space) = parts.get(1) {
                    total_space += space.parse::<u64>().unwrap_or(0);
                }
            }
        }
    }

    // Print the total disk space found
    println!("Total disk space from /dev/: {}", total_space);
    total_space
}
