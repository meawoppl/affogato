# Affogato: Unified ESP32-S2 + ICE40 FPGA Development Container
# Provides: Yosys, nextpnr-ice40, icestorm, iverilog, gtkwave, verilator, ESP-IDF
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# Build tools + FPGA toolchain dependencies
RUN apt-get update && apt-get install -y \
    # Core build tools
    git wget curl build-essential pkg-config \
    flex bison gperf ninja-build ccache \
    # Python
    python3 python3-pip python3-venv \
    # ESP-IDF dependencies
    libffi-dev libssl-dev dfu-util libusb-1.0-0 \
    # Yosys dependencies
    tcl-dev libreadline-dev gawk zlib1g-dev \
    # nextpnr dependencies
    libboost-all-dev libeigen3-dev \
    # icestorm dependencies
    libftdi-dev \
    # Clang for faster builds
    clang lld \
    # Visualization
    graphviz xdot \
    # Verilog simulation
    iverilog gtkwave verilator \
    && rm -rf /var/lib/apt/lists/*

# Modern CMake (nextpnr requires 3.25+)
RUN pip3 install --no-cache-dir cmake

# Yosys (synthesis) - pinned version for reproducibility
ARG YOSYS_VERSION=0.47
RUN git clone --depth=1 -b yosys-${YOSYS_VERSION} https://github.com/YosysHQ/yosys.git /tmp/yosys && \
    cd /tmp/yosys && \
    make config-clang && \
    make -j$(nproc) && \
    make install && \
    rm -rf /tmp/yosys

# icestorm (iCE40 tools - icepack, iceprog, timing data)
# Must be installed before nextpnr for chip timing database
RUN git clone --depth=1 https://github.com/YosysHQ/icestorm.git /tmp/icestorm && \
    cd /tmp/icestorm && \
    make -j$(nproc) && \
    make install && \
    rm -rf /tmp/icestorm

# nextpnr-ice40 (place & route)
RUN git clone --depth=1 https://github.com/YosysHQ/nextpnr.git /tmp/nextpnr && \
    cd /tmp/nextpnr && \
    cmake . -B build -DARCH=ice40 -DCMAKE_INSTALL_PREFIX=/usr/local && \
    cmake --build build -j$(nproc) && \
    cmake --install build && \
    rm -rf /tmp/nextpnr

# ESP-IDF - support ESP32-S2 and ESP32-S3
ENV IDF_PATH=/opt/esp-idf
ENV IDF_TOOLS_PATH=/opt/esp-tools
ARG IDF_VERSION=v5.3.2

RUN git clone -b ${IDF_VERSION} --recursive --depth=1 \
    https://github.com/espressif/esp-idf.git ${IDF_PATH} && \
    cd ${IDF_PATH} && \
    ./install.sh esp32s2,esp32s3 && \
    rm -rf ~/.cache/pip

# ESP-IDF activation script
RUN echo '#!/bin/bash\n\
. ${IDF_PATH}/export.sh\n\
exec "$@"' > /opt/esp-activate.sh && \
    chmod +x /opt/esp-activate.sh

# Add tools to PATH for non-interactive use
ENV PATH="${IDF_TOOLS_PATH}/tools:${PATH}"

# Enable ccache for faster rebuilds
ENV IDF_CCACHE_ENABLE=1

# Create workspace
WORKDIR /workspace

# Default entrypoint sources ESP-IDF environment
ENTRYPOINT ["/opt/esp-activate.sh"]
CMD ["/bin/bash"]
