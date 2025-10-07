PREFIX := $(HOME)
BASE_DIR := .
SIG_DIR := iterations
SIG_RELEASE :=$(SIG_DIR)/target/release
SIG_DEBUG :=$(SIG_DIR)/target/debug
SIG_LIB :=$(SIG_DIR)/lib
SIG_SO_RELEASE := $(SIG_RELEASE)/libiterations.so
SIG_A_RELEASE := $(SIG_RELEASE)/libiterations.a
SIG_SO_DEBUG := $(SIG_DEBUG)/libiterations.so
SIG_A_DEBUG := $(SIG_DEBUG)/libiterations.a
SIG_HEADER := $(SIG_DIR)/iterations.h
SIG_JNI := $(SIG_DIR)/Iterations.java
BIN_DIR := $(PREFIX)/.local/bin
BIN_NAME := green-languages
BIN_ALIAS := gl
BIN_DEBUG := target/debug/$(BIN_NAME)
BIN_RELEASE := target/release/$(BIN_NAME)

all: debug

$(SIG_SO_DEBUG) $(SIG_A_DEBUG):
	cargo build --manifest-path $(SIG_DIR)/Cargo.toml

$(SIG_SO_RELEASE) $(SIG_A_RELEASE):
	cargo build --release --manifest-path $(SIG_DIR)/Cargo.toml

debug: $(SIG_SO_DEBUG) $(SIG_A_DEBUG)
	install -d -m 755 $(BIN_DIR)
	install -d -m 755 $(SIG_LIB)
	install -m 755 $(SIG_SO_DEBUG) $(SIG_LIB)
	install -m 755 $(SIG_A_DEBUG) $(SIG_LIB)
	install -m 644 $(SIG_HEADER) $(SIG_LIB)
	install -m 644 $(SIG_JNI) $(SIG_LIB)
	cargo build
	install -m 755 $(BIN_DEBUG) $(BIN_DIR)/$(BIN_NAME)
	sudo setcap cap_sys_rawio,cap_perfmon,cap_sys_nice=ep $(BIN_DIR)/$(BIN_NAME)
	ln -sf $(BIN_NAME) $(BIN_DIR)/$(BIN_ALIAS)
	sudo sysctl kernel.perf_event_paranoid=2

release: $(SIG_SO_RELEASE) $(SIG_A_RELEASE)
	install -d -m 755 $(BIN_DIR)
	install -d -m 755 $(SIG_LIB)
	install -m 755 $(SIG_SO_RELEASE) $(SIG_LIB)
	install -m 755 $(SIG_A_RELEASE) $(SIG_LIB)
	install -m 644 $(SIG_HEADER) $(SIG_LIB)
	install -m 644 $(SIG_JNI) $(SIG_LIB)
	cargo build --release
	install -m 755 $(BIN_RELEASE) $(BIN_DIR)/$(BIN_NAME)
	sudo setcap cap_sys_rawio,cap_perfmon,cap_sys_nice=ep $(BIN_DIR)/$(BIN_NAME)
	ln -sf $(BIN_NAME) $(BIN_DIR)/$(BIN_ALIAS)
	sudo sysctl kernel.perf_event_paranoid=2

uninstall:
	rm -f $(BIN_DIR)/$(BIN_NAME)
	rm -rf $(SIG_LIB)
	cargo clean --manifest-path $(SIG_DIR)/Cargo.toml
	cargo clean

.PHONY: all debug release uninstall
.SILENT:
