FROM rust:1.89 AS rust-builder
RUN rustup target add x86_64-unknown-linux-musl
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        clang \
        build-essential \
        pkg-config \
        libssl-dev \
        cmake \
    && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /work/src
WORKDIR /work
COPY Cargo.lock /work/Cargo.lock
COPY Cargo.toml /work/Cargo.toml
RUN touch ./src/lib.rs && cargo vendor && cargo build --release && rm -rf ./src
COPY src/ /work/src/
RUN find /work/src -print -exec touch "{}" \; \
    && cargo build --release --bins
COPY scripts/copy_binaries.sh /work/scripts/copy_binaries.sh
RUN bash /work/scripts/copy_binaries.sh

FROM ubuntu:24.04
RUN set -eux; \
    MIRROR="http://asia-northeast1.gce.archive.ubuntu.com/ubuntu/"; \
    for f in /etc/apt/sources.list.d/ubuntu.sources /etc/apt/sources.list; do \
      if [ -f "$f" ]; then \
        sed -i.bak -e "s|http://archive.ubuntu.com/ubuntu/|$MIRROR|g" "$f"; \
      fi; \
    done
RUN apt-get update -qy && apt-get install -qy apt-transport-https ca-certificates gnupg curl
RUN echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] http://packages.cloud.google.com/apt cloud-sdk main" \
    | tee -a /etc/apt/sources.list.d/google-cloud-sdk.list \
    && curl https://packages.cloud.google.com/apt/doc/apt-key.gpg \
    | apt-key --keyring /usr/share/keyrings/cloud.google.gpg  add - \
    && apt-get update -y && apt-get install google-cloud-sdk -y
COPY ./secrets/service_account.json /service_account.json
RUN gcloud auth activate-service-account icfpc2025@icfpc-primary.iam.gserviceaccount.com \
        --key-file=/service_account.json \
    && gcloud config set project icfpc-primary

COPY --from=rust-builder /usr/local/bin/* /usr/local/bin/
COPY scripts/deploy_binaries.sh /work/scripts/deploy_binaries.sh
