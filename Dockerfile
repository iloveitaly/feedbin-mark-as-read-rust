# Use the official Rust image as the base
FROM rust:1.68.2 AS builder

# Set the working directory
WORKDIR /usr/src/feedbin_cleaner

# Copy the project files into the working directory
COPY . .

# Build the project in release mode
RUN cargo build --release

# Start a new stage with a minimal image to reduce the final image size
FROM debian:buster-slim

# Install ca-certificates and libssl for HTTPS support
RUN apt-get update && apt-get install -y ca-certificates libssl1.1 && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/feedbin_cleaner/target/release/feedbin_cleaner /usr/local/bin/

# Set the entrypoint to run the compiled binary by default
ENTRYPOINT ["feedbin_cleaner"]