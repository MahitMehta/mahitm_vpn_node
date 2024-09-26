# MahitM VPN Node

# Simply Get Started 

1. Create a `config.toml` file (checkout the [config schema](#Config-Schema) for more information)

2. `sudo docker pull mahitm/mahitm_vpn_node:<arch>-latest` (arch = arm64 | x86_64)

3. `sudo docker run -d -v /etc/wireguard:/etc/wireguard -v ./config.toml:/app/config.toml -e CONFIG_DIR=/app/ -e RUST_LOG=debug  --cap-add=NET_ADMIN --cap-add=SYS_MODULE --sysctl net.ipv4.conf.all.src_valid_mark=1 --sysctl net.ipv4.ip_forward=1 -p 8552:8552/udp --restart always --name mahitm_vpn_node mahitm_vpn_node`

# Development

## Build Docker Image

- `sudo docker build . -t mahitm_vpn_node`

## Build & Run Outside of Docker

- `cargo build && sudo RUST_LOG=debug ./target/debug/mahitm_vpn_node`

# Config Schema
```toml
[node]
id = "<group>_<region>_node_<#>" # ex. mahitm_ash_node_0
ipv4 = "<0-255>.<0-255>.<0-255>.<0-255>" # ex. 150.136.127.166
src_port = 8552
dst_port = 8552
network_adapter = "<string>" # ex. eth0
wg_interface = "<string>" # ex. wg0
conf_dir = "/etc/wireguard" 

[control_plane]
host = "cp0-vpn.mahitm.com" # change if using to use a self-hosted CP
secure = true # secure ? wss : ws

[mesh]
id = "<string>_mesh" # ex. mahitm_mesh
ipv4 = "<0-255>.<0-255>.<0-255>.<0-255>" # ex. 192.168.2.1 (LAN IP)

[[user]]
# either exact email or the domain of an email
# note: multiple exact emails can be provided by adding more user tables
# ex. mahit.py@gmail.com | @mahitm.com
pattern = "<string>"
```