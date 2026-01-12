M_SO := measurements/target/release/libmeasurements.so
M_A := measurements/target/release/libmeasurements.a
M_HEADER := measurements/include/measurements.h
M_JNI := measurements/include/Measurements.java
GL_BIN_DIR := /usr/local/bin
GL_LIB_DIR := /usr/local/lib/green-languages
GL_EXE := target/release/green-languages

all: release

release: $(M_SO) $(M_A)
	sudo install -d -m755 $(GL_BIN_DIR)
	sudo install -d -m755 $(GL_LIB_DIR)
	sudo install -m755 $(M_SO) $(GL_LIB_DIR)
	sudo install -m755 $(M_A) $(GL_LIB_DIR)
	sudo install -m644 $(M_HEADER) $(GL_LIB_DIR)
	sudo install -m644 $(M_JNI) $(GL_LIB_DIR)
	GL_LIB_DIR=$(GL_LIB_DIR) cargo build --release --workspace
	sudo install -m755 $(GL_EXE) $(GL_BIN_DIR)/green-languages
	sudo setcap cap_sys_rawio,cap_perfmon,cap_sys_nice=ep $(GL_BIN_DIR)/green-languages
	sudo install -m755 scripts/profiles.sh $(GL_BIN_DIR)/green-languages-profiles
	sudo install -d -m 755 $(GL_LIB_DIR)/profiles
	sudo install -m644 profiles/*.conf $(GL_LIB_DIR)/profiles/
	sudo sysctl kernel.perf_event_paranoid=2

uninstall:
	sudo rm -f $(GL_BIN_DIR)/green-languages
	sudo rm -f $(GL_BIN_DIR)/green-languages-profiles
	sudo rm -rf $(GL_LIB_DIR)
	cargo clean --manifest-path measurements/Cargo.toml
	cargo clean

.PHONY: all release uninstall
.SILENT:
