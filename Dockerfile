FROM rust:latest as builder
WORKDIR /usr/src/euroka3d

COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to compile dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release

COPY . .

RUN touch src/main.rs && \
    cargo build --release

FROM gcr.io/distroless/cc-debian12

COPY --from=builder /usr/src/euroka3d/target/release/euroka3d /usr/local/bin/euroka3d
COPY --from=builder /usr/src/euroka3d/static /usr/local/bin/static
COPY --from=builder /usr/src/euroka3d/templates /usr/local/bin/templates
COPY --from=builder /usr/src/euroka3d/settings.toml /usr/local/bin/settings.toml

# -e REST_LOG=debug if need be
ENV RUST_LOG=info

# `-p 80:8080` to talk to this app from the outside with port 3000
EXPOSE 8080

WORKDIR /usr/local/bin
# Run the binary
CMD ["./euroka3d"]

