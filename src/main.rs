mod api; 
mod discover;
use serde_json::{json, Value};
use std::io::{Write, Read};
use std::time::{Duration};
use std::fs::File;

pub async fn run_xbee_script() -> Result<bool, Box<dyn std::error::Error>> {
    // Open the file in read-only mode with buffer.
    let mut file = File::open("config.json").expect("Cannot open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Cannot read file");

    // Parse the string of data into serde_json::Value.
    let v: Value = serde_json::from_str(&contents).expect("Cannot parse JSON");
    
    let port = "/dev/ttyUSB0";
    let baud_rate = 9600;

    let mut xbee_device = match discover::DigiMeshDevice::new(port, baud_rate) {
        Ok(device) => device,
        Err(err) => {
            println!("Erreur lors de la création de l'appareil XBee : {}", err);
            return Ok(false);
        }
    };

    let addr_64bit = match xbee_device.get_64bit_addr() {
        Ok(addr) => addr,
        Err(err) => {
            println!("Erreur lors de la récupération de l'adresse 64 bits de l'appareil XBee : {}", err);
            return Ok(false);
        }
    };

    println!("Adresse 64 bits de l'appareil XBee : {:x}", addr_64bit);

    let node_id = match xbee_device.get_node_id() {
        Ok(id) => id,
        Err(err) => {
            println!("Erreur lors de la récupération de l'ID du noeud local : {}", err);
            return Ok(false);
        }
    };

    println!("ID du noeud local : {}", node_id);

    if v["instant_scan"].as_bool().unwrap_or(true) {
        println!("Exécution d'un scan instantané...");
        match xbee_device.discover_nodes(Some(Duration::from_secs(5))) {
            Ok(_) => {
                if let Some(nodes) = &xbee_device.nodes {
                    if nodes.is_empty() {
                        write_empty_json().unwrap();
                        return Ok(false);
                    } else {
                        write_nodes_to_json(nodes).unwrap();
                        return Ok(true);
                    }
                } else {
                    println!("Aucun noeud découvert.");
                    write_empty_json().unwrap();
                    return Ok(false);
                }
            }
            Err(err) => {
                println!("Erreur lors de la découverte des noeuds : {}", err);
                return Ok(false);
            }
        }
    } else {
        let start_after_duration = v.get("start_after_duration").unwrap_or(&Value::Number(serde_json::Number::from(0))).as_u64().unwrap();
        let scan_duration = Duration::from_secs(v.get("scan_duration").unwrap_or(&Value::Number(serde_json::Number::from(0))).as_u64().unwrap());

        for i in (1..=start_after_duration).rev() {
            println!("Scan starts in {} seconds", i);
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        
        println!("Début du scan de {}s...", scan_duration.as_secs());
        match xbee_device.scheduled_discover_nodes(Duration::from_secs(scan_duration.as_secs())) {
            Ok(_) => {
                if let Some(nodes) = &xbee_device.nodes {
                    if nodes.is_empty() {
                        write_empty_json().unwrap();
                        return Ok(false);
                    } else {
                        write_nodes_to_json(nodes)?; 
                        return Ok(true);
                    }
                } else {
                    println!("Aucun noeud découvert.");
                    write_empty_json().unwrap();
                    return Ok(false);
                }
            }
            Err(err) => {
                println!("Erreur lors de la découverte des noeuds : {}", err);
                return Ok(false);
            }
        }
    }
}

fn write_nodes_to_json(nodes: &[discover::RemoteDigiMeshDevice]) -> std::io::Result<()> {
    let mut data = serde_json::Map::new();

    for (index, node) in nodes.iter().enumerate() {
        let node_data = json!({
            "node_id": node.node_id,
            "node_address": format!("{:x}", node.addr_64bit),
        });
        data.insert((index + 1).to_string(), node_data);
    }

    let json_data = serde_json::to_string_pretty(&data)?;
    println!("{}", json_data);

    let mut file = File::create("zigbee_data.json")?;
    file.write_all(json_data.as_bytes())?;

    Ok(())
}

fn write_schedulednodes_to_json(nodes: &[discover::RemoteDigiMeshDevice]) -> std::io::Result<()> {
    let mut data = serde_json::Map::new();

    for (index, node) in nodes.iter().enumerate() {
        let durations_data: Vec<_> = node.durations.iter().map(|(start, end)| {
            json!({"start": start.elapsed().as_secs(), "end": end.elapsed().as_secs()})
        }).collect();

        let node_data = json!({
            "node_id": node.node_id,
            "node_address": format!("{:x}", node.addr_64bit),
            "zigbee_durations": durations_data,
        });
        data.insert(index.to_string(), node_data);
    }

    let json_data = serde_json::to_string_pretty(&data)?;
    println!("{}", json_data);

    let mut file = File::create("zigbee_scheduleddata.json")?;
    file.write_all(json_data.as_bytes())?;

    Ok(())
}


fn write_empty_json() -> std::io::Result<()> {
    let empty_data = serde_json::Map::new();
    let json_data = serde_json::to_string_pretty(&empty_data)?;
    println!("{}", json_data);

    let mut file = File::create("zigbee_data.json")?;
    file.write_all(json_data.as_bytes())?;

    Ok(())
}

#[tokio::main]
async fn main() {
    match run_xbee_script().await {
        Ok(success) => {
            if success {
                println!("Script executed successfully.");
            } else {
                println!("Script executed with errors.");
            }
        },
        Err(e) => println!("Failed to run script: {}", e),
    }
}