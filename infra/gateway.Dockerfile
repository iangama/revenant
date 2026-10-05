FROM rust:1.97-alpine AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY runtime/activities runtime/activities
COPY runtime/ai runtime/ai
COPY runtime/actors runtime/actors
COPY runtime/combat runtime/combat
COPY runtime/campaign runtime/campaign
COPY runtime/challenges runtime/challenges
COPY runtime/cooperation runtime/cooperation
COPY runtime/compatibility runtime/compatibility
COPY runtime/gateway runtime/gateway
COPY runtime/identity runtime/identity
COPY runtime/inventory runtime/inventory
COPY runtime/local-security-lab runtime/local-security-lab
COPY runtime/modules runtime/modules
COPY runtime/objectives runtime/objectives
COPY runtime/operations runtime/operations
COPY runtime/persistence runtime/persistence
COPY runtime/progression runtime/progression
COPY runtime/protocol runtime/protocol
COPY runtime/replay runtime/replay
COPY runtime/world runtime/world
COPY tools/fake-client tools/fake-client
COPY tools/reconstruction-server tools/reconstruction-server
COPY tools/revenant tools/revenant
COPY tools/activity-validator tools/activity-validator
COPY tools/combat-lab tools/combat-lab
COPY tools/cooperation-bot tools/cooperation-bot
COPY tools/cooperation-lab tools/cooperation-lab
COPY tools/module-lab tools/module-lab
COPY tools/operation-lab tools/operation-lab
COPY archive/clients/v1 archive/clients/v1
COPY scripts/activities scripts/activities
COPY client/game/world client/game/world
RUN cargo build --locked --release -p revenant-gateway

FROM alpine:3.20
RUN apk add --no-cache su-exec \
    && addgroup -S revenant \
    && adduser -S revenant -G revenant
COPY --from=builder /build/target/release/revenant-gateway /revenant-gateway
COPY scripts/activities /scripts/activities
COPY infra/gateway-entrypoint.sh /gateway-entrypoint.sh
EXPOSE 8080
EXPOSE 7000
ENTRYPOINT ["/gateway-entrypoint.sh"]
