FROM --platform=$BUILDPLATFORM rust:alpine as builder

WORKDIR /usr/src/zzdns

# This is a dummy build to get the dependencies cached.
RUN apk add musl-dev gcc g++ make libffi-dev openssl-dev libtool

# Now copy in the rest of the sources
COPY . .

# This is the actual application build.
RUN cargo build  --release

################
##### Runtime
FROM --platform=$TARGETPLATFORM alpine:3.16.0 AS runtime

WORKDIR /usr/local/zzdns/

# Copy application binary from builder image
COPY --from=builder /usr/src/zzdns/target/release/zzdns /usr/local/zzdns/

COPY config /usr/local/zzdns/config

# Run the application
CMD ["/usr/local/zzdns/zzdns"]