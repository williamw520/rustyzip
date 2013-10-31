
# Targets
RUSTYZIP_LIB = rustyzip
RGZIP = rgzip

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
all:  $(BUILD_DIR)/$(RGZIP)

$(BUILD_DIR)/$(BUILD_DIR).stamp:
	@echo "Building $@..."
	@mkdir -p $(BUILD_DIR)
	@touch $@

$(BUILD_DIR)/$(RUSTYZIP_LIB).stamp: $(SRC_DIR)/lib.rs  $(wildcard $(SRC_DIR)/rustyzip_lib/*)  $(wildcard $(SRC_DIR)/common/*)  $(wildcard $(SRC_DIR)/common/**/*)  $(BUILD_DIR)/$(BUILD_DIR).stamp
	@echo "Building $<..."
	@$(RUSTC) $(ROPTS)  $<
	@touch $@

$(BUILD_DIR)/$(RUSTYZIP_LIB): $(BUILD_DIR)/$(RUSTYZIP_LIB).stamp  $(BUILD_DIR)/$(BUILD_DIR).stamp

$(BUILD_DIR)/$(RGZIP): $(SRC_DIR)/$(RGZIP).rs $(BUILD_DIR)/$(RUSTYZIP_LIB)
	@echo "Building $@..."
	@$(RUSTC) $(ROPTS)  $<


clean:
	rm -R -f $(BUILD_DIR)
	rm -f $(SRC_DIR)/*~
	rm -f *~


test-lib:
	@$(RUSTC) --out-dir=$(BUILD_DIR) -L $(LIBRARY_DIRS) --test $(SRC_DIR)/lib.rs
	@$(BUILD_DIR)/lib

