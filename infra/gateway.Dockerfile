FROM rust:1.97-alpine AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY runtime/activities runtime/activities
COPY runtime/ai runtime/ai
COPY runtime/actors runtime/actors
COPY runtime/combat runtime/combat
COPY runtime/compatibility runtime/compatibility
COPY runtime/gateway runtime/gateway
COPY runtime/identity runtime/identity
COPY runtime/inventory runtime/inventory
COPY runtime/objectives runtime/objectives
COPY runtime/persistence runtime/persistence
COPY runtime/progression runtime/progression
COPY runtime/protocol runtime/protocol
COPY runtime/replay runtime/replay
COPY runtime/world runtime/world
COPY tools/fake-client tools/fake-client
COPY tools/reconstruction-server tools/reconstruction-server
COPY tools/revenant tools/revenant
COPY archive/clients/v1 archive/clients/v1
RUN cargo build --locked --release -p revenant-gateway

FROM alpine:3.20
RUN addgroup -S revenant && adduser -S revenant -G revenant
COPY --from=builder /build/target/release/revenant-gateway /revenant-gateway
COPY scripts/activities /scripts/activities
USER revenant
EXPOSE 8080
EXPOSE 7000
ENTRYPOINT ["/revenant-gateway"]
