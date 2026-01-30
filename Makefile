S_SO := target/release/libsignals.so
S_A := target/release/libsignals.a
S_HEADER := signals/include/signals.h
S_JNI := signals/include/Signals.java
BIN_DIR := /usr/local/bin
LIB_DIR := /usr/local/lib/green-languages
EXE := target/release/green-languages

all: release

release:
	sudo install -d -m755 $(BIN_DIR)
	sudo install -d -m755 $(LIB_DIR)
	GL_LIB_DIR=$(LIB_DIR) cargo build --release --workspace
	sudo install -m755 $(S_SO) $(LIB_DIR)
	sudo install -m755 $(S_A) $(LIB_DIR)
	sudo install -m644 $(S_HEADER) $(LIB_DIR)
	sudo install -m644 $(S_JNI) $(LIB_DIR)
	sudo install -m755 $(EXE) $(BIN_DIR)/green-languages
	sudo install -m755 scripts/setups.sh $(BIN_DIR)/green-languages-setups
	sudo install -d -m 755 $(LIB_DIR)/setups
	sudo install -m644 scripts/setups/*.conf $(LIB_DIR)/setups/
	sudo sed -i -E '/^SETUPS_DIR=.*setups/ s|^SETUPS_DIR=.*|SETUPS_DIR="'"$(LIB_DIR)"'/setups/"|' $(BIN_DIR)/green-languages-setups
	sudo setcap cap_sys_rawio,cap_perfmon,cap_sys_nice=ep $(BIN_DIR)/green-languages
	sudo sysctl kernel.perf_event_paranoid=2

uninstall:
	sudo rm -f $(BIN_DIR)/green-languages
	sudo rm -f $(BIN_DIR)/green-languages-setups
	sudo rm -rf $(LIB_DIR)
	cargo clean

.PHONY: all release uninstall
.SILENT:

