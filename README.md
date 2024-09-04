# MahitM VPN Node

# Build Docker Image
1. sudo docker build . -t mahitm_vpn_node 

# Run Docker Image
1. sudo docker run -v ./config.toml:/app/config.toml -e CONFIG_DIR=/app/ -e RUST_LOG=debug  --cap-add=NET_ADMIN --cap-add=SYS_MODULE --sysctl net.ipv4.conf.all.src_valid_mark=1 --sysctl net.ipv4.ip_forward=1 -p 8552:8552/udp -t mahitm_vpn_node