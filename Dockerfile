FROM rust:latest AS builder

WORKDIR /usr/src/app

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo,from=rust:latest,source=/usr/local/cargo \
    --mount=type=cache,target=target \
    cargo build --release && mv ./target/release/mahitm_vpn_node ./mahitm_vpn_node

FROM debian:bookworm-slim AS runtime

RUN apt update && apt install -y wireguard-tools iptables openssl ca-certificates iproute2

COPY --from=builder /usr/src/app/mahitm_vpn_node /app/mahitm_vpn_node

ENTRYPOINT ["/app/mahitm_vpn_node"]