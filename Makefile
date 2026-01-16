# Affogato - ESP32-S2 + ICE40 Development Tool
# https://github.com/meawoppl/affogato

.PHONY: help docker-build docker-push docker-shell new-project lint clean

# Docker configuration
DOCKER_REGISTRY ?= ghcr.io
DOCKER_USER ?= meawoppl
DOCKER_IMAGE = $(DOCKER_REGISTRY)/$(DOCKER_USER)/affogato
DOCKER_TAG ?= latest

# Default target
default: help

help:  ## Show this help message
	@echo "Affogato - ESP32-S2 + ICE40 Development Tool"
	@echo ""
	@echo "Docker Commands:"
	@grep -E '^docker-[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Project Commands:"
	@grep -E '^(new-project|lint|clean):.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "For project-specific commands, see your project's Makefile"

# =============================================================================
# Docker Management
# =============================================================================

docker-build:  ## Build the Affogato Docker container locally
	docker build -t $(DOCKER_IMAGE):$(DOCKER_TAG) docker/

docker-push:  ## Push container to GitHub Container Registry
	docker push $(DOCKER_IMAGE):$(DOCKER_TAG)
	@echo "Pushed to: $(DOCKER_IMAGE):$(DOCKER_TAG)"

docker-pull:  ## Pull latest container from GitHub Container Registry
	docker pull $(DOCKER_IMAGE):$(DOCKER_TAG)

docker-shell:  ## Start an interactive shell in the container
	docker run --rm -it \
		-v $(PWD):/workspace \
		-w /workspace \
		$(DOCKER_IMAGE):$(DOCKER_TAG) \
		/bin/bash

docker-shell-usb:  ## Start a shell with USB device access (for flashing)
	docker run --rm -it \
		-v $(PWD):/workspace \
		-w /workspace \
		--device /dev/ttyACM0 \
		--privileged \
		$(DOCKER_IMAGE):$(DOCKER_TAG) \
		/bin/bash

# =============================================================================
# Project Scaffolding
# =============================================================================

new-project:  ## Create a new Affogato project (usage: make new-project NAME=myproject)
ifndef NAME
	$(error NAME is required. Usage: make new-project NAME=myproject)
endif
	@echo "Creating new project: $(NAME)"
	@mkdir -p $(NAME)/firmware/main $(NAME)/fpga/rtl
	@# Copy firmware templates
	@sed 's/{{PROJECT_NAME}}/$(NAME)/g' templates/firmware/CMakeLists.txt > $(NAME)/firmware/CMakeLists.txt
	@cp templates/firmware/main/CMakeLists.txt $(NAME)/firmware/main/
	@sed 's/{{PROJECT_NAME}}/$(NAME)/g' templates/firmware/main/main.c > $(NAME)/firmware/main/main.c
	@cp templates/firmware/sdkconfig.defaults $(NAME)/firmware/
	@# Copy FPGA templates
	@cp templates/fpga/Makefile $(NAME)/fpga/
	@cp templates/fpga/project.pcf $(NAME)/fpga/
	@sed 's/{{PROJECT_NAME}}/$(NAME)/g' templates/fpga/rtl/top.v > $(NAME)/fpga/rtl/top.v
	@# Copy reusable Verilog modules
	@cp fpga/rtl/spi_slave_bulk.v $(NAME)/fpga/rtl/
	@# Create project Makefile
	@echo "Creating project Makefile..."
	@$(MAKE) -s _create_project_makefile NAME=$(NAME)
	@echo ""
	@echo "Project created: $(NAME)/"
	@echo ""
	@echo "Next steps:"
	@echo "  cd $(NAME)"
	@echo "  make build-fpga   # Build FPGA bitstream"
	@echo "  make build        # Build ESP32 firmware"
	@echo "  make flash        # Flash to device"

_create_project_makefile:
	@cat > $(NAME)/Makefile << 'MAKEFILE_EOF'
# $(NAME) - Affogato Project Makefile

.PHONY: help build-fpga build flash monitor clean

# Affogato path (adjust if needed)
export AFFOGATO_PATH ?= $(shell realpath ..)

# Docker image
DOCKER_IMAGE ?= ghcr.io/meawoppl/affogato:latest
DOCKER_RUN = docker run --rm -v $(CURDIR):/workspace -w /workspace $(DOCKER_IMAGE)
DOCKER_RUN_USB = docker run --rm -v $(CURDIR):/workspace -w /workspace --device /dev/ttyACM0 --privileged $(DOCKER_IMAGE)

# Serial port
PORT ?= /dev/ttyACM0

default: help

help:  ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

build-fpga:  ## Build FPGA bitstream
	$(MAKE) -C fpga

build: build-fpga  ## Build ESP32 firmware (includes FPGA)
	$(DOCKER_RUN) bash -c "cd firmware && idf.py build"

flash:  ## Flash firmware to device
	$(DOCKER_RUN_USB) bash -c "cd firmware && idf.py -p $(PORT) flash"

monitor:  ## Monitor serial output (Ctrl+] to exit)
	$(DOCKER_RUN_USB) bash -c "cd firmware && idf.py -p $(PORT) monitor"

flash-monitor: flash  ## Flash and immediately monitor
	$(DOCKER_RUN_USB) bash -c "cd firmware && idf.py -p $(PORT) monitor"

menuconfig:  ## Open ESP-IDF configuration menu
	$(DOCKER_RUN) bash -c "cd firmware && idf.py menuconfig"

clean:  ## Clean build artifacts
	$(MAKE) -C fpga clean
	$(DOCKER_RUN) bash -c "cd firmware && idf.py clean"

fullclean:  ## Full clean including CMake cache
	$(MAKE) -C fpga clean
	$(DOCKER_RUN) bash -c "cd firmware && idf.py fullclean"
	rm -rf firmware/sdkconfig firmware/sdkconfig.old
MAKEFILE_EOF

# =============================================================================
# Development
# =============================================================================

lint:  ## Lint all Verilog files
	docker run --rm -v $(PWD):/workspace -w /workspace \
		$(DOCKER_IMAGE):$(DOCKER_TAG) \
		verilator --lint-only -Wall fpga/rtl/*.v

clean:  ## Clean Affogato build artifacts
	rm -rf docker/*.log

# =============================================================================
# GitHub Actions (for CI/CD)
# =============================================================================

.PHONY: ci-docker-build ci-docker-push

ci-docker-build:  ## Build container (for CI)
	docker build -t $(DOCKER_IMAGE):$(DOCKER_TAG) docker/
	docker tag $(DOCKER_IMAGE):$(DOCKER_TAG) $(DOCKER_IMAGE):$(shell git rev-parse --short HEAD)

ci-docker-push:  ## Push container with git SHA tag (for CI)
	docker push $(DOCKER_IMAGE):$(DOCKER_TAG)
	docker push $(DOCKER_IMAGE):$(shell git rev-parse --short HEAD)
