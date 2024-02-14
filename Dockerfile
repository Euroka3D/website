FROM rust:latest as builder

WORKDIR /usr/src/euroka3d

COPY . .

RUN cargo build --release

FROM gcr.io/distroless/cc

COPY --from=builder /usr/src/euroka3d/target/release/euroka3d /usr/local/bin/euroka3d
COPY --from=builder /usr/src/euroka3d/static /usr/local/bin/static
COPY --from=builder /usr/src/euroka3d/templates /usr/local/bin/templates

# -e REST_LOG=debug if need be
ENV RUST_LOG=info

# `-p 3000:8080` to talk to this app from the outside with port 3000
EXPOSE 8080

WORKDIR /usr/local/bin
# Run the binary
CMD ["./euroka3d"]

