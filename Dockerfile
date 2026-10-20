FROM rust:1.99.1-bookworm@sha256:e12566d5abc17a0da741628782cdb504b68b8961f5bfcf677936bf3e8b32de29 AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY vendor/legacy-chart/ vendor/legacy-chart/
COPY src/ src/
COPY scripts/collect-dependency-licenses.sh scripts/
RUN cargo build --locked --release --bin hwp-engine \
    && sh scripts/collect-dependency-licenses.sh "$CARGO_HOME" /dependency-licenses

FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587 AS runtime
ARG VCS_REF=unknown
ARG IMAGE_VERSION=0.1.0-container.3
LABEL org.opencontainers.image.title="hwp-engine" \
      org.opencontainers.image.source="https://github.com/Taek0/hwp-is-pdf" \
      org.opencontainers.image.revision="${VCS_REF}" \
      org.opencontainers.image.version="${IMAGE_VERSION}"
COPY --from=build /build/target/release/hwp-engine /usr/local/bin/hwp-engine
COPY --from=build /dependency-licenses/ /usr/share/licenses/hwp-engine/dependencies/
COPY LICENSE THIRD_PARTY_NOTICES.md /usr/share/licenses/hwp-engine/
COPY vendor/legacy-chart/LICENSE vendor/legacy-chart/UPSTREAM.md /usr/share/licenses/hwp-engine/legacy-chart/
WORKDIR /work
USER 10001:10001
ENTRYPOINT ["/usr/local/bin/hwp-engine"]
