# Dockerfile for creating a statically-linked Rust application using docker's
# multi-stage build feature. This also leverages the docker build cache to avoid
# re-downloading dependencies if they have not changed.
FROM rust:1.51.0 AS build
WORKDIR /usr/src

# Download the target for static linking.
#RUN rustup target add x86_64-unknown-linux-musl currently not an option due to ODPI-C runtime deps
RUN rustup target add x86_64-unknown-linux-gnu

# Create a dummy project and build the app's dependencies.
# If the Cargo.toml or Cargo.lock files have not changed,
# we can use the docker build cache and skip these (typically slow) steps.
RUN USER=root cargo new inquest
WORKDIR /usr/src/inquest
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

# Copy the source and build the application.
COPY src ./src
RUN cargo install --target x86_64-unknown-linux-gnu --path .

# Copy the statically-linked binary into a scratch container.
FROM registry.access.redhat.com/ubi8/ubi
RUN curl -o /tmp/oracle-client.rpm https://download.oracle.com/otn_software/linux/instantclient/211000/oracle-instantclient-basiclite-21.1.0.0.0-1.x86_64.rpm && \
    yum install -y /tmp/oracle-client.rpm && \
    rm -f oracle-client.rpm && \
    rm -rf /var/cache/yum

COPY --from=build /usr/local/cargo/bin/inquest .
USER 1000
ENTRYPOINT ["./inquest"]
CMD ["--help"]