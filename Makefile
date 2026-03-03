LG_SO := target/release/libgreen.so
LG_A := target/release/libgreen.a
LG_HEADER := libgreen/include/green.h
LIB_DIR := /usr/lib
INCLUDE_DIR := /usr/include
BIN_DIR := /usr/bin
SETUPS_DIR := /usr/share/green-languages/setups
EXE := target/release/green-languages

all: release

release:
	cargo build --release --workspace
	sudo install -m755 $(LG_SO) $(LIB_DIR)
	sudo install -m755 $(LG_A) $(LIB_DIR)
	sudo install -m644 $(LG_HEADER) $(INCLUDE_DIR)
	sudo ldconfig
	sudo install -m755 $(EXE) $(BIN_DIR)/green-languages
	sudo install -m755 scripts/setups.sh $(BIN_DIR)/green-languages-setups
	sudo install -d -m755 $(SETUPS_DIR)
	sudo install -m644 scripts/setups/*.conf $(SETUPS_DIR)/
	sudo sed -i -E '/^SETUPS_DIR=.*setups/ s|^SETUPS_DIR=.*|SETUPS_DIR="$(SETUPS_DIR)/"|' $(BIN_DIR)/green-languages-setups
	sudo setcap cap_sys_rawio,cap_perfmon,cap_sys_nice=ep $(BIN_DIR)/green-languages
	sudo sysctl -w kernel.perf_event_paranoid=-1

uninstall:
	sudo rm -f $(BIN_DIR)/green-languages
	sudo rm -f $(BIN_DIR)/green-languages-setups
	sudo rm -rf $(SETUPS_DIR)
	sudo rm -f $(LIB_DIR)/libgreen.so
	sudo rm -f $(LIB_DIR)/libgreen.a
	sudo rm -f $(INCLUDE_DIR)/green.h
	sudo ldconfig
	cargo clean

.PHONY: all release uninstall
.SILENT:
