binary := "sideband"
target_pi := "aarch64-unknown-linux-gnu"
remote := "shack"

# --- Raspberry Pi Commands ---

# Full deploy: Build (cross), Upload, and Run
deploy args="": upload
    ssh {{remote}} "~/{{binary}} {{args}}"

# Build specifically for the Pi using cross
build-pi:
    cross build -p {{binary}} --target {{target_pi}} --release

# Transfer the Pi binary and ensure it is executable
upload: build-pi
    scp target/{{target_pi}}/release/{{binary}} {{remote}}:~/
    ssh {{remote}} "chmod +x ~/{{binary}}"

# Just run the binary that is already on the Pi
run-pi args="":
    ssh {{remote}} "~/{{binary}} {{args}}"

# --- Local Machine Commands ---

# Build for the local machine using standard cargo
build:
    cargo build -p {{binary}} --release

# Run locally (e.g., to test device listing)
run args="": build
    ./target/release/{{binary}} {{args}}
