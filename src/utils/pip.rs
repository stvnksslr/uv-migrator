use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn parse_pip_conf() -> Result<Vec<String>, String> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| "Unable to determine home directory".to_string())?;
    let pip_conf_path = home_dir.join(".pip").join("pip.conf");

    if !pip_conf_path.exists() {
        return Ok(vec![]);
    }

    let file = File::open(&pip_conf_path).map_err(|e| format!("Failed to open pip.conf: {}", e))?;
    let reader = BufReader::new(file);

    let mut extra_urls = vec![];
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Failed to read line from pip.conf: {}", e))?;
        let trimmed = line.trim();
        if trimmed.starts_with("extra-index-url") {
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() == 2 {
                extra_urls.push(parts[1].trim().to_string());
            }
        }
    }

    Ok(extra_urls)
}
