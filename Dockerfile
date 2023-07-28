####################################################################################################
## Builder
####################################################################################################
FROM rust:latest AS builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates
RUN apt-get -y install libssl-dev pkg-config

# Create appuser
ENV USER=rust
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"


WORKDIR /proxy

COPY ./ .

# We no longer need to use the x86_64-unknown-linux-musl target
RUN cargo build --target x86_64-unknown-linux-musl --release

####################################################################################################
## Final image
####################################################################################################
FROM alpine

# Install ca-certificates so that HTTPS works consistently
RUN apk update && apk add ca-certificates && rm -rf /var/cache/apk/*

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /proxy

# Copy our build
COPY --from=builder /proxy/target/x86_64-unknown-linux-musl/release/rust-reverse-proxy ./

# Use an unprivileged user.
USER rust:rust

ENTRYPOINT ["./rust-reverse-proxy"]
