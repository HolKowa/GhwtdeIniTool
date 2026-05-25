FROM ghcr.io/prulloac/base:bookworm

# Install dependencies for building Tauri
RUN apt-get update && apt-get install -y \
    pkg-config \
    libwebkit2gtk-4.1-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    && rm -rf /var/lib/apt/lists/*