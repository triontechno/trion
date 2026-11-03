# Explicit TRION-H processor verification commands.
# These targets are not run automatically. No operating-system image targets
# are provided here.

ROOT := ../..
BUILD_DIR := $(ROOT)/.build/processor
RTL := design.v
TB := testbench.v
IVERILOG ?= iverilog
VVP ?= vvp
VERILATOR ?= verilator

.PHONY: help test test-icarus lint-verilator test-verilator clean

help:
	@echo "test-icarus      Compile and run the self-checking testbench with Icarus"
	@echo "lint-verilator  Lint the synthesizable core with Verilator"
	@echo "test-verilator  Build and run the testbench with Verilator"
	@echo "clean           Remove only .build/processor generated files"

test: test-icarus

test-icarus:
	mkdir -p $(BUILD_DIR)
	$(IVERILOG) -g2005 -Wall -s testbench -o $(BUILD_DIR)/trion_h_tb $(RTL) $(TB)
	$(VVP) $(BUILD_DIR)/trion_h_tb

lint-verilator:
	$(VERILATOR) --lint-only --Wall -Wno-fatal --top-module trion_h_core $(RTL)

test-verilator:
	mkdir -p $(BUILD_DIR)/verilator
	$(VERILATOR) --binary --timing -Wno-fatal --top-module testbench \
		-Mdir $(BUILD_DIR)/verilator $(RTL) $(TB)
	$(BUILD_DIR)/verilator/Vtestbench

clean:
	rm -rf $(BUILD_DIR)
