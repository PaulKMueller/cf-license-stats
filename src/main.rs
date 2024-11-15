use serde_json::Value;

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use spdx::Expression;
use spdx::ParseError;
use std::collections::HashMap;
use std::env;

use serde::Deserialize;
use serde::Serialize;

use core::str;
use std::{error::Error, fs::File, io::Write};
use zstd::stream::decode_all;

use anyhow::Result;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RepodataPackage {
    pub name: String,
    pub version: String,
    pub license: String,
    pub sha256: String,
    pub build: String,
}

#[derive(Debug, Clone)]
pub struct RepodataPackages {
    pub packages: Vec<RepodataPackage>,
}

pub fn main() -> Result<()> {
    let platforms = vec!["linux-64", "osx-64", "win-64", "osx-arm64", "noarch"];

    // Use triple to keep track of ok_licenses and bad_licenses for each platform

    let mut output_data: Vec<(String, u32, u32)> = Vec::new();

    let start = std::time::Instant::now();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    for arch in platforms {
        let mut ok_licenses = 0;
        let mut bad_licenses = 0;

        let repodata_url = format!(
            "https://conda.anaconda.org/conda-forge/{}/repodata.json.zst",
            arch
        );
        let read_repodata_task = fetch_repodata_json(&arch, true);

        let json = match runtime.block_on(read_repodata_task) {
            Ok(config) => config,
            Err(_e) => Value::Null,
        };

        let plain_packages = json["packages"].as_object().unwrap();

        let packages_conda = json["packages.conda"].as_object().unwrap();

        let packages = plain_packages
            .iter()
            .chain(packages_conda.iter())
            .collect::<HashMap<_, _>>();

        let mut package_map: HashMap<String, (String, Result<Expression, ParseError>)> =
            HashMap::new();


        for (_, package) in packages {
            // If there is no license information, skip the package
            if package["license"].is_null() {
                continue;
            }

            // Extract "name" and "license" from the data
            if let Some(package_name) = package["name"].as_str() {
                if let Some(license_string) = package["license"].as_str() {
                    // Handle timestamp as either a string or a number
                    let timestamp = if let Some(timestamp_str) = package["timestamp"].as_str() {
                        timestamp_str.to_string()
                    } else if let Some(timestamp_num) = package["timestamp"].as_i64() {
                        timestamp_num.to_string()
                    } else {
                        // Skip the package if timestamp is neither a string nor a number
                        continue;
                    };
            
                    let parsed_license = Expression::parse(license_string);
            
                    package_map.insert(
                        package_name.to_string(),
                        (timestamp, parsed_license),
                    );
                }
            }
        }
        let mut packages_to_remove = Vec::new();

        for (package_name, (timestamp, _)) in &package_map {
            for (other_package_name, (other_timestamp, _)) in &package_map {
                if package_name == other_package_name && timestamp < other_timestamp {
                    packages_to_remove.push(package_name.clone());
                }
            }
        }

        for package_name in packages_to_remove {
            package_map.remove(&package_name);
        }

        for (package_name, (timestamp, parsed_license)) in package_map {
            if parsed_license.is_ok() {
                ok_licenses += 1;
            } else {
                bad_licenses += 1;
            }
        }

        println!(
            "For platform {}, {} licenses are valid and {} are invalid",
            arch, ok_licenses, bad_licenses
        );

        output_data.push((arch.to_string(), ok_licenses, bad_licenses));
    }

    // Write the output to data.json

    let output_data_json = serde_json::to_string(&output_data)?;

    let mut file = File::create("data.json")?;
    file.write_all(output_data_json.as_bytes())?;

    // Print the time taken to run this function
    let duration = start.elapsed();
    println!(
        "\n\nTime elapsed for get_repodata_licenses: {:?}\n\n",
        duration
    );

    Ok(())
}

pub async fn fetch_repodata_json(
    arch: &str,
    use_zst: bool,
) -> Result<Value, Box<dyn std::error::Error>> {
    // build the client.
    let client = reqwest::Client::new();

    // Make the request
    if !use_zst {
        let repodata_url = format!(
            "https://conda.anaconda.org/conda-forge/{}/repodata.json",
            arch
        );
        let repodata_json_str = client.get(repodata_url).send().await?.text().await?;
        let repodata_json: Value = serde_json::from_str(&repodata_json_str)?;
        Ok(repodata_json)
    } else {
        let repodata_url = format!(
            "https://conda.anaconda.org/conda-forge/{}/repodata.json.zst",
            arch
        );
        let repodata_json = download_zst(&repodata_url).await?;
        let repodata_json_str = decompress_zst_to_string(&repodata_json)?;
        let repodata_json: Value = serde_json::from_str(&repodata_json_str)?;
        Ok(repodata_json)
    }
}

pub async fn download_zst(url: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    // println!("Downloaded {} bytes", bytes.len());
    Ok(bytes.to_vec())
}

pub fn decompress_zst_to_string(compressed_data: &[u8]) -> Result<String, Box<dyn Error>> {
    let decompressed_data = match decode_all(compressed_data) {
        Ok(data) => data,
        Err(e) => panic!("Decode failed: {}", e),
    };
    let json_string = str::from_utf8(&decompressed_data)?.to_string();
    // println!("Decompressed {} bytes", json_string.len());
    Ok(json_string)
}
