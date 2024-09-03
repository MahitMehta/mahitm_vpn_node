use log::error;
use std::{
    error::Error,
    io::Write,
    process::{Command, Stdio},
};

pub fn generate_private_key() -> Result<String, Box<dyn Error>> {
    let private_key = Command::new("wg")
        .arg("genkey")
        .stdout(Stdio::piped())
        .output()?;

    let private_key_str = String::from_utf8(private_key.stdout)?;
    Ok(String::from(private_key_str.trim()))
}

pub fn generate_public_key(wg_private_key: &String) -> Result<String, Box<dyn Error>> {
    let mut public_key = Command::new("wg")
        .arg("pubkey")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    if let Some(stdin) = public_key.stdin.as_mut() {
        stdin.write_all(wg_private_key.as_bytes())?;

        let public_key_output = public_key.wait_with_output()?;
        let public_key_output_str = String::from_utf8(public_key_output.stdout)?;
        return Ok(String::from(public_key_output_str.trim()));
    }

    Err("Failed to open to stdin".into())
}

pub fn add_peer_to_conf(
    wg_inteface: &String,
    ipv4: &String,
    public_key: &String,
) -> Result<(), Box<dyn Error>> {
    let output = Command::new("wg")
        .args([
            "set",
            wg_inteface,
            "peer",
            public_key,
            "allowed-ips",
            format!("{}/32", ipv4).as_str(),
        ])
        .stdout(Stdio::piped())
        .output()?;

    let stderr = String::from_utf8(output.stderr).unwrap();
    if stderr.len() > 0 {
        error!("STDERR: {}", stderr);
        return Err(format!("Failed to add peer to {}", wg_inteface).into());
    }

    let output = Command::new("ip")
        .args([
            "-4",
            "route",
            "add",
            format!("{}/32", ipv4).as_str(),
            "dev",
            wg_inteface,
        ])
        .stdout(Stdio::piped())
        .output()?;

    let stderr = String::from_utf8(output.stderr).unwrap();
    if stderr.len() > 0 {
        error!("STDERR: {}", stderr);
        return Err("Failed to add peer route".into());
    }

    Ok(())
}

pub(crate) fn remove_peer_from_conf(
    wg_inteface: &String,
    peer_ipv4: &String,
    peer_public_key: &String,
) -> Result<(), Box<dyn Error>> {
    let output = Command::new("wg")
        .args(["set", wg_inteface, "peer", peer_public_key, "remove"])
        .stdout(Stdio::piped())
        .output()?;

    let stderr = String::from_utf8(output.stderr).unwrap();
    if stderr.len() > 0 {
        error!("STDERR: {}", stderr);
        return Err(format!("Failed to remove peer from {}", wg_inteface).into());
    }

    let output = Command::new("ip")
        .args([
            "-4",
            "route",
            "delete",
            format!("{}/32", peer_ipv4).as_str(),
            "dev",
            wg_inteface,
        ])
        .stdout(Stdio::piped())
        .output()?;

    let stderr = String::from_utf8(output.stderr).unwrap();
    if stderr.len() > 0 {
        error!("STDERR: {}", stderr);
        return Err("Failed to remove peer route".into());
    }

    Ok(())
}
