FROM rust:1.89 AS rust-builder
RUN rustup target add x86_64-unknown-linux-musl
RUN rustup target add wasm32-unknown-unknown
# RUN cargo install wasm-pack  # It was very slow.
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

FROM rust-builder AS vis
COPY Cargo.toml /app/Cargo.toml
COPY vis/Cargo.toml /app/vis/Cargo.toml
WORKDIR /app
RUN mkdir -p ./src ./vis/src \
    && touch ./src/lib.rs ./vis/src/lib.rs \
    && cd ./vis \
    && cargo vendor \
    && { wasm-pack build --target web || true; } \
    && rm -rf ./src ./vis/src
COPY src /app/src
COPY vis/src /app/vis/src
COPY vis/index.html /www/visualizer.html
RUN touch ./src/lib.rs ./vis/src/lib.rs \
    && cd ./vis \
    && wasm-pack build --target web \
    && cp ./pkg/*.js ./pkg/*.wasm /www/

FROM rust-builder AS service
COPY Cargo.toml /app/Cargo.toml
WORKDIR /app
RUN mkdir -p ./src \
    && touch ./src/lib.rs \
    && cargo vendor \
    && cargo build --release \
    && rm -rf ./src
COPY src /app/src
RUN touch ./src/lib.rs \
    && cargo build --release --bin www \
    && cp ./target/release/www /app/

FROM rust-builder AS server

ARG UNAGI_PASSWORD
ENV UNAGI_PASSWORD ${UNAGI_PASSWORD}

RUN sed -i.bak -e "s%http://archive.ubuntu.com/ubuntu/%http://asia-northeast1.gce.archive.ubuntu.com/ubuntu/%g" \
    /etc/apt/sources.list
RUN apt-get update \
    && apt-get install -y nginx apache2-utils supervisor \
    && rm -rf /var/lib/apt/lists/*

RUN htpasswd -b -c /etc/nginx/.htpasswd unagi ${UNAGI_PASSWORD}

RUN rm /etc/nginx/sites-enabled/default
COPY configs/nginx.conf /etc/nginx/sites-enabled/
COPY configs/supervisord.conf /etc/supervisor/conf.d/supervisord.conf

COPY --from=vis /www /www
COPY --from=service /app/www /usr/local/bin/app
COPY static /www/static
# COPY 3d/web/www /www/3d/web/www
WORKDIR /app
ENV RUST_BACKTRACE 1

EXPOSE 80
CMD ["/usr/bin/supervisord"]
