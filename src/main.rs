#![allow(non_snake_case)]
use indexmap::IndexMap;
use std::fs;
use std::fs::File;
use std::io::stdout;
use std::io::stdin;
use std::io::BufWriter;
use std::io::Write;
use std::num::ParseIntError;
use colorize::AnsiColor;
use reqwest::header::HeaderName;
use reqwest::Client;
use reqwest::Response;
use serde_derive::{Serialize, Deserialize};

static MODLOADER_NAMES: [&str; 7] = ["Any", "Forge", "Cauldron", "LiteLoader", "Fabric", "Quilt", "NeoForge"];

#[derive(Serialize, Deserialize)]
struct OutputFile {
    data: IndexMap<String, IndexMap<String, i64>> 
}

async fn get_mod_count(client: Client, cf_api_key: String, game_version: String, modloader: String) -> Option<i64> {
    let mut modloader_type: String = String::new();
    if modloader != "0" {
        modloader_type = modloader.clone();
    }

    let query: Vec<(String, String)> = vec![
        ("gameId".to_string(), "432".to_string()),
        ("gameVersion".to_string(), game_version),
        ("modLoaderType".to_string(), modloader_type),
        ("pageSize".to_string(), "50".to_string()),
        ("sortField".to_string(), "1".to_string()),
        ("sortOrder".to_string(), "desc".to_string()),
        ("index".to_string(), "0".to_string())
    ];

    let res: Response = client.get("https://api.curseforge.com/v1/mods/search")
        .header(HeaderName::from_static("x-api-key"), cf_api_key)
        .query(&query)
        .send()
        .await.unwrap();

    let total_count: Option<i64> = match &res.status().as_u16() {
        200..=299 => {
            let body: &String = &res
                .text()
                .await.unwrap();

            let json_res: serde_json::Value = serde_json::from_str(body.as_str()).unwrap();

            Some(json_res["pagination"]["totalCount"].as_i64().unwrap())
        }   
        400..=599 => {
            let status: &reqwest::StatusCode = &res.status();
            let url: reqwest::Url = res.url().clone();
            let error_message: &String = &res
                .text()
                .await.unwrap();
            println!("Error {}: {} (URL: {})", status, error_message, url);
            None
        }
        _ => {
            println!("Unexpected status code: {}", &res.status());
            None
        }
    };

    total_count
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    println!("Minecraft Mod Counter v1.0.0 - made by Chace Pratt\n");

    let cf_api_key: String = get_cf_api_key();

    if cf_api_key == String::new() {
        println!("\n=== Press any key to close ===");
        stdin().read_line(&mut String::new()).unwrap();
        return Ok(());
    }

    print!("{}", "Please enter a list of Minecraft version numbers seperated by commas: ".b_cyan());

    let _ = stdout().flush();
    let mut version_list: String = String::new();
    stdin().read_line(&mut version_list).expect("Invalid string entered");

    if let Some('\n')=version_list.chars().next_back() {
        version_list.pop();
    }
    if let Some('\r')=version_list.chars().next_back() {
        version_list.pop();
    }

    version_list = version_list.replace(" ", "");
    let versions: Vec<String> = version_list.split(",").map(String::from).collect();

    println!("{}", "CurseForge mod loaders as of May 30th, 2024: 0=Any, 1=Forge, 2=Cauldron, 3=LiteLoader, 4=Fabric, 5=Quilt, 6=NeoForge".cyan());
    println!("{}", "Latest mod loader IDs can be found here: https://docs.curseforge.com/#tocS_ModLoaderType".cyan());
    print!("{}", "Please enter a list of Minecraft mod loader IDs seperated by commas: ".b_cyan());

    let _ = stdout().flush();
    let mut modloader_list: String = String::new();
    stdin().read_line(&mut modloader_list).expect("Invalid string entered");

    if let Some('\n')=modloader_list.chars().next_back() {
        modloader_list.pop();
    }
    if let Some('\r')=modloader_list.chars().next_back() {
        modloader_list.pop();
    }

    modloader_list = modloader_list.replace(" ", "");
    let modloaders: Vec<String> = modloader_list.split(",").map(String::from).collect();

    print!("{}", "Save to JSON file? (y/N): ".b_cyan());

    let _ = stdout().flush();
    let mut y_or_n: String = String::new();
    stdin().read_line(&mut y_or_n).expect("Invalid string entered");

    if let Some('\n')=y_or_n.chars().next_back() {
        y_or_n.pop();
    }
    if let Some('\r')=y_or_n.chars().next_back() {
        y_or_n.pop();
    }

    let mut save_to_file: bool = false;

    if y_or_n == "y" || y_or_n == "Y" {
        save_to_file = true;
    } else if y_or_n != "n" && y_or_n != "N" && y_or_n != String::new() {
        println!("{}", "Invalid character entered, will default to not saving data to a JSON file.".b_red())
    }

    let mut file_data: OutputFile = OutputFile { data: IndexMap::new() };

    let client: Client = reqwest::ClientBuilder::new()
        .build()
        .unwrap();

    for version in versions {
        print!("\n{}{}{}", "Initializing search for mod count(s) of version ".cyan(), version.clone().b_cyan(), "...".cyan());
        let mut version_data: IndexMap<String, i64> = IndexMap::new();

        for modloader in modloaders.clone() {
            let count: i64 = get_mod_count(client.clone(), cf_api_key.clone(), version.clone(), modloader.clone()).await.unwrap_or(-1);

            if count == -1 {
                println!("{}", "Failed to receive data from the CurseForge API. Is your API key valid?".b_red());

                println!("\n=== Press any key to close ===");
                stdin().read_line(&mut String::new()).unwrap();
                return Ok(());
            }

            let mut displayed_modloader: String = "Unknown mod loader".to_string();
            let modloader_id: Result<usize, ParseIntError> = modloader.parse::<usize>();

            if modloader_id.is_ok() {
                if MODLOADER_NAMES.get(modloader_id.clone().unwrap()).is_some() {
                    displayed_modloader = MODLOADER_NAMES.get(modloader_id.unwrap()).unwrap().to_string();
                }

                version_data.insert(modloader.clone(), count);
                print!("\n{}{}{}{}{}{} {}", "    \\\\--- ", displayed_modloader, " (", modloader.clone(), ")", ":", count.to_string().b_green());
            } else {
                println!("{}", "\nERROR: Mod loader ID is not a number.".b_red());
            }
        }

        file_data.data.insert(version, version_data);
    }

    if save_to_file {
        let file_result: Result<File, std::io::Error> = File::create("data.json");
        if file_result.is_ok() {
            let file: File = file_result.unwrap();
            let mut writer = BufWriter::new(file);
            serde_json::to_writer_pretty(&mut writer, &file_data).unwrap();
            writer.flush().unwrap();
        } else {
            println!("{}", "Failed to write to/create data.json file.".b_red())
        }
    }
    
    println!("\n\n=== Press any key to close ===");
    stdin().read_line(&mut String::new()).unwrap();

    Ok(())
}

fn get_cf_api_key() -> String {
    let contents: Result<String, std::io::Error> = fs::read_to_string("apikey.txt");
    if contents.is_ok() {
        return contents.unwrap();
    } else {
        println!("{}", "Failed to find/read apikey.txt".b_red());
    }
    String::new()
}