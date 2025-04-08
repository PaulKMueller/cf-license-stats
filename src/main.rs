use serde_json::Value;

use rayon::prelude::*;
use spdx::Expression;
use spdx::ParseError;
use std::collections::HashMap;

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

#[derive(Serialize, Debug)]
struct LicenseValidities {
    arch: String,
    valid_licenses: u32,
    invalid_licenses: u32,
}

#[derive(serde::Serialize)]
struct LicenseCount {
    license: String,
    count: u64,
}

pub fn main() -> Result<()> {
    let platforms = vec!["linux-64", "osx-64", "win-64", "osx-arm64", "noarch"];

    let start = std::time::Instant::now();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut license_counter_map: HashMap<String, u64> = HashMap::new();

    let results: Vec<_> = platforms
        .par_iter()
        .map(|arch| {
            let arch = arch.to_string();

            let read_repodata_task = fetch_repodata_json(&arch, true);

            let json = match runtime.block_on(read_repodata_task) {
                Ok(config) => config,
                Err(_e) => Value::Null,
            };

            let plain_packages = json["packages"].as_object().unwrap();

            let packages_conda = json["packages.conda"].as_object().unwrap();

            let packages = plain_packages.iter().chain(packages_conda.iter());

            let mut package_map: HashMap<String, (String, Result<Expression, ParseError>)> =
                HashMap::new();

            for (_, package) in packages {
                if package["license"].is_null() {
                    continue;
                }

                if let (Some(name), Some(license)) =
                    (package["name"].as_str(), package["license"].as_str())
                {
                    let timestamp = if let Some(ts) = package["timestamp"].as_str() {
                        ts.to_string()
                    } else if let Some(ts) = package["timestamp"].as_i64() {
                        ts.to_string()
                    } else {
                        continue;
                    };

                    let parsed = Expression::parse(license);
                    package_map.insert(name.to_string(), (timestamp, parsed));
                }
            }

            let mut to_remove = vec![];
            for (a, (ts_a, _)) in &package_map {
                for (b, (ts_b, _)) in &package_map {
                    if a == b && ts_a < ts_b {
                        to_remove.push(a.clone());
                    }
                }
            }

            for name in to_remove {
                package_map.remove(&name);
            }

            let mut local_map = HashMap::new();
            let mut ok = 0;
            let mut bad = 0;

            for (_, (_, result)) in &package_map {
                if result.is_ok() {
                    ok += 1;
                } else {
                    bad += 1;
                }

                // License string is "INVALID" if Expression has not been parsed without error
                let license_str = if result.is_ok() {
                    result.as_ref().unwrap().to_string()
                } else {
                    "INVALID".to_string()
                };
                *local_map.entry(license_str).or_insert(0) += 1;
            }

            println!(
                "For platform {}, {} licenses are valid and {} are invalid",
                arch, ok, bad
            );

            (arch, ok, bad, local_map)
        })
        .collect();

    let mut license_summaries = Vec::new();

    for (arch, ok, bad, local_map) in results {
        license_summaries.push(LicenseValidities {
            arch,
            valid_licenses: ok,
            invalid_licenses: bad,
        });

        println!("{:?}", license_summaries);

        for (license_str, count) in local_map {
            let entry = license_counter_map.entry(license_str).or_insert(0);
            *entry += count;
        }
    }

    let output_data_json = serde_json::to_string_pretty(&license_summaries)?;
    let mut file = File::create("valid_licenses_data.json")?;
    file.write_all(output_data_json.as_bytes())?;

    let mut sorted_license_counter: Vec<_> = license_counter_map
        .iter()
        .map(|(license, count)| LicenseCount {
            license: license.clone(),
            count: *count,
        })
        .collect();

    sorted_license_counter.sort_by(|a, b| b.count.cmp(&a.count));

    let sorted_license_counter_json = serde_json::to_string_pretty(&sorted_license_counter)?;
    let mut file = File::create("sorted_license_counter.json")?;
    file.write_all(sorted_license_counter_json.as_bytes())?;

    let license_counter_json = serde_json::to_string_pretty(&license_counter_map)?;
    let mut file = File::create("license_counter.json")?;
    file.write_all(license_counter_json.as_bytes())?;

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
    let client = reqwest::Client::new();

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
    Ok(bytes.to_vec())
}

pub fn decompress_zst_to_string(compressed_data: &[u8]) -> Result<String, Box<dyn Error>> {
    let decompressed_data = match decode_all(compressed_data) {
        Ok(data) => data,
        Err(e) => panic!("Decode failed: {}", e),
    };
    let json_string = str::from_utf8(&decompressed_data)?.to_string();
    Ok(json_string)
}
