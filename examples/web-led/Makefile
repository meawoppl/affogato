# Web LED - ESP32 + ICE40 WiFi RGB LED Control
# Use 'affogato' CLI for better experience

DOCKER_IMAGE ?= ghcr.io/meawoppl/affogato:latest
DOCKER_RUN = docker run --rm -v $(CURDIR):/workspace -v $(CURDIR)/../../components:/workspace/components -w /workspace $(DOCKER_IMAGE)
DOCKER_RUN_USB = docker run --rm -v $(CURDIR):/workspace -v $(CURDIR)/../../components:/workspace/components -w /workspace --device /dev/ttyACM0 --privileged $(DOCKER_IMAGE)
PORT ?= /dev/ttyACM0

.PHONY: build-fpga build flash monitor run clean

build-fpga:
	$(MAKE) -C fpga

build: build-fpga
	$(DOCKER_RUN) bash -c "cd firmware && idf.py build"

flash:
	$(DOCKER_RUN_USB) bash -c "cd firmware && idf.py -p $(PORT) flash"

monitor:
	$(DOCKER_RUN_USB) bash -c "cd firmware && idf.py -p $(PORT) monitor"

run:
	$(DOCKER_RUN_USB) bash -c "cd firmware && idf.py -p $(PORT) flash monitor"

clean:
	$(MAKE) -C fpga clean
	$(DOCKER_RUN) bash -c "cd firmware && idf.py fullclean" || true
