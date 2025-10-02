PREFIX := $(HOME)
BASE_DIR := .
BIN_DIR := $(PREFIX)/.local/bin
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

release: $(SIG_SO_RELEASE) $(SIG_A_RELEASE)
	install -d -m 755 $(BIN_DIR)
	install -d -m 755 $(SIG_LIB)
	install -m 755 $(SIG_SO_RELEASE) $(SIG_LIB)
	install -m 755 $(SIG_A_RELEASE) $(SIG_LIB)
	install -m 644 $(SIG_HEADER) $(SIG_LIB)
	install -m 644 $(SIG_JNI) $(SIG_LIB)
	cargo build --release

uninstall:
	rm -r $(SIG_LIB)
	cargo clean --manifest-path $(SIG_DIR)/Cargo.toml
	cargo clean

.PHONY: all debug release uninstall
.SILENT:
