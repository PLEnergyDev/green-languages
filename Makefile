M_SO_RELEASE := measurements/target/release/libmeasurements.so
M_A_RELEASE := measurements/target/release/libmeasurements.a
M_SO_DEBUG := measurements/target/debug/libmeasurements.so
M_A_DEBUG := measurements/target/debug/libmeasurements.a
M_HEADER := measurements/lib/measurements.h
M_JNI := measurements/lib/Measurements.java
GL_BIN_DIR := /usr/local/bin
GL_LIB_DIR := /usr/local/lib/green-languages
GL_BIN_DEBUG := target/debug/green-languages
GL_BIN_RELEASE := target/release/green-languages

all: debug

$(M_SO_DEBUG) $(M_A_DEBUG):
	cargo build --manifest-path measurements/Cargo.toml

$(M_SO_RELEASE) $(M_A_RELEASE):
	cargo build --release --manifest-path measurements/Cargo.toml

debug: $(M_SO_DEBUG) $(M_A_DEBUG)
	sudo install -d -m 755 $(GL_BIN_DIR)
	sudo install -d -m 755 $(GL_LIB_DIR)
	sudo install -m 755 $(M_SO_DEBUG) $(GL_LIB_DIR)
	sudo install -m 755 $(M_A_DEBUG) $(GL_LIB_DIR)
	sudo install -m 644 $(M_HEADER) $(GL_LIB_DIR)
	sudo install -m 644 $(M_JNI) $(GL_LIB_DIR)
	GL_LIB_DIR=$(GL_LIB_DIR) cargo build
	sudo install -m 755 $(GL_BIN_DEBUG) $(GL_BIN_DIR)/green-languages
	sudo setcap cap_sys_rawio,cap_perfmon,cap_sys_nice=ep $(GL_BIN_DIR)/green-languages
	sudo ln -sf green-languages $(GL_BIN_DIR)/gl
	sudo install -m 755 src/profiles.sh $(GL_BIN_DIR)/glp
	sudo install -d -m 755 $(GL_LIB_DIR)/glp.d
	sudo install -m 644 profiles/*.conf $(GL_LIB_DIR)/glp.d/ 2>/dev/null || true
	sudo sysctl kernel.perf_event_paranoid=2

release: $(M_SO_RELEASE) $(M_A_RELEASE)
	sudo install -d -m 755 $(GL_BIN_DIR)
	sudo install -d -m 755 $(GL_LIB_DIR)
	sudo install -m 755 $(M_SO_RELEASE) $(GL_LIB_DIR)
	sudo install -m 755 $(M_A_RELEASE) $(GL_LIB_DIR)
	sudo install -m 644 $(M_HEADER) $(GL_LIB_DIR)
	sudo install -m 644 $(M_JNI) $(GL_LIB_DIR)
	GL_LIB_DIR=$(GL_LIB_DIR) cargo build --release
	sudo install -m 755 $(GL_BIN_RELEASE) $(GL_BIN_DIR)/green-languages
	sudo setcap cap_sys_rawio,cap_perfmon,cap_sys_nice=ep $(GL_BIN_DIR)/green-languages
	sudo ln -sf green-languages $(GL_BIN_DIR)/gl
	sudo install -m 755 src/profiles.sh $(GL_BIN_DIR)/glp
	sudo install -d -m 755 $(GL_LIB_DIR)/glp.d
	sudo install -m 644 profiles/*.conf $(GL_LIB_DIR)/glp.d/ 2>/dev/null || true
	sudo sysctl kernel.perf_event_paranoid=2

uninstall:
	sudo rm -f $(GL_BIN_DIR)/green-languages
	sudo rm -f $(GL_BIN_DIR)/gl
	sudo rm -f $(GL_BIN_DIR)/glp
	sudo rm -rf $(GL_LIB_DIR)
	cargo clean --manifest-path measurements/Cargo.toml
	cargo clean

.PHONY: all debug release uninstall
.SILENT:
