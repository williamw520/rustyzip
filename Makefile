
# Targets
RUSTYZIP_LIB = rustyzip
RZIP = rzip

# Dir setup
ROOT_DIR        = .
SRC_DIR         = $(ROOT_DIR)/src
TEST_DIR        = $(ROOT_DIR)/src/test
BUILD_DIR       = $(ROOT_DIR)/bin
LIBRARY_DIRS    = $(BUILD_DIR)
#ROPTS           = --out-dir=$(BUILD_DIR) -L $(LIBRARY_DIRS)
ROPTS           = --out-dir=$(BUILD_DIR) -L $(LIBRARY_DIRS) --cfg debug

# Compile command, for general and for Windows
#RUSTC           = rustc
RUSTC           = rustc.exe


#all:  $(BUILD_DIR)/$(RUSTYZIP_LIB)
all:  $(BUILD_DIR)/$(RZIP)

$(BUILD_DIR)/$(BUILD_DIR).stamp:
	@echo "Building $@..."
	@mkdir -p $(BUILD_DIR)
	@touch $@

$(BUILD_DIR)/$(RUSTYZIP_LIB).stamp: $(SRC_DIR)/lib.rs  $(wildcard $(SRC_DIR)/rustyzip_lib/*)  $(wildcard $(SRC_DIR)/common/*)  $(wildcard $(SRC_DIR)/common/**/*)  $(BUILD_DIR)/$(BUILD_DIR).stamp
	@echo "Building $<..."
	@$(RUSTC) $(ROPTS)  $<
	@touch $@

$(BUILD_DIR)/$(RUSTYZIP_LIB): $(BUILD_DIR)/$(RUSTYZIP_LIB).stamp  $(BUILD_DIR)/$(BUILD_DIR).stamp

$(BUILD_DIR)/$(RZIP): $(SRC_DIR)/$(RZIP).rs $(BUILD_DIR)/$(RUSTYZIP_LIB)
	@echo "Building $@..."
	@$(RUSTC) $(ROPTS)  $<


clean:
	rm -R -f $(BUILD_DIR)
	rm -f $(SRC_DIR)/*~
	rm -f *~


scratch: $(TEST_DIR)/scratch.rs
	@$(RUSTC) --out-dir=$(BUILD_DIR) -L $(LIBRARY_DIRS)  $(TEST_DIR)/scratch.rs
	@$(BUILD_DIR)/scratch

test-rustyzip:
	@$(RUSTC) --out-dir=$(BUILD_DIR) -L $(LIBRARY_DIRS) --test $(SRC_DIR)/rustyzip.rs
	@$(BUILD_DIR)/rustyzip

test-bitstream:
	@$(RUSTC) --out-dir=$(BUILD_DIR) -L $(LIBRARY_DIRS) --test $(SRC_DIR)/common/bitstream.rs
	@$(BUILD_DIR)/bitstream

test-strutil:
	@$(RUSTC) --out-dir=$(BUILD_DIR) -L $(LIBRARY_DIRS) --test $(SRC_DIR)/common/strutil.rs
	@$(BUILD_DIR)/strutil

bench:
	@$(RUSTC) --out-dir=$(BUILD_DIR) -L $(LIBRARY_DIRS) --test $(SRC_DIR)/test/bench.rs
	@$(BUILD_DIR)/bench --bench

