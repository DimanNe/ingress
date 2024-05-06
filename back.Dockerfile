FROM rust:1 as build-env
WORKDIR /app
COPY . /app

RUN --mount=type=cache,id=apt_common,sharing=locked,target=/var/cache/apt \
    --mount=type=cache,id=apt_common,sharing=locked,target=/var/lib/apt \
    --mount=type=cache,id=rust_build,sharing=locked,target=/app/target \
    apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y cmake protobuf-compiler && \
   cargo build --release && mkdir -p /out && cp /app/target/release/back /out

FROM gcr.io/distroless/cc-debian12
COPY --from=build-env /out/back /
ENTRYPOINT ["/back"]
