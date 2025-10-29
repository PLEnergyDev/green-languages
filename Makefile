M_SO_RELEASE := measurements/target/release/libmeasurements.so
M_A_RELEASE := measurements/target/release/libmeasurements.a
M_SO_DEBUG := measurements/target/debug/libmeasurements.so
M_A_DEBUG := measurements/target/debug/libmeasurements.a
M_HEADER := measurements/lib/measurements.h
M_JNI := measurements/lib/Measurements.java
GL_BIN_DIR := $(HOME)/.local/bin
GL_LIB_DIR := $(HOME)/.local/lib/green-languages
GL_BIN_ALIAS := gl
GL_BIN_DEBUG := target/debug/green-languages
GL_BIN_RELEASE := target/release/green-languages

all: debug

$(M_SO_DEBUG) $(M_A_DEBUG):
	cargo build --manifest-path measurements/Cargo.toml

$(M_SO_RELEASE) $(M_A_RELEASE):
	cargo build --release --manifest-path measurements/Cargo.toml

debug: $(M_SO_DEBUG) $(M_A_DEBUG)
	install -d -m 755 $(GL_BIN_DIR)
	install -d -m 755 $(GL_LIB_DIR)
	install -m 755 $(M_SO_DEBUG) $(GL_LIB_DIR)
	install -m 755 $(M_A_DEBUG) $(GL_LIB_DIR)
	install -m 644 $(M_HEADER) $(GL_LIB_DIR)
	install -m 644 $(M_JNI) $(GL_LIB_DIR)
	GL_LIB_DIR=$(GL_LIB_DIR) cargo build
	install -m 755 $(GL_BIN_DEBUG) $(GL_BIN_DIR)/green-languages
	sudo setcap cap_sys_rawio,cap_perfmon,cap_sys_nice=ep $(GL_BIN_DIR)/green-languages
	ln -sf green-languages $(GL_BIN_DIR)/$(GL_BIN_ALIAS)
	for script in script/*.py; do \
		if [ "$$(basename $$script)" = "__main__.py" ]; then \
			install -m 755 $$script $(GL_BIN_DIR)/gls; \
		else \
			install -m 644 $$script $(GL_LIB_DIR)/$$(basename $$script); \
		fi \
	done
	sudo sysctl kernel.perf_event_paranoid=2

release: $(M_SO_RELEASE) $(M_A_RELEASE)
	install -d -m 755 $(GL_BIN_DIR)
	install -d -m 755 $(GL_LIB_DIR)
	install -m 755 $(M_SO_RELEASE) $(GL_LIB_DIR)
	install -m 755 $(M_A_RELEASE) $(GL_LIB_DIR)
	install -m 644 $(M_HEADER) $(GL_LIB_DIR)
	install -m 644 $(M_JNI) $(GL_LIB_DIR)
	GL_LIB_DIR=$(GL_LIB_DIR) cargo build --release
	install -m 755 $(GL_BIN_RELEASE) $(GL_BIN_DIR)/green-languages
	sudo setcap cap_sys_rawio,cap_perfmon,cap_sys_nice=ep $(GL_BIN_DIR)/green-languages
	ln -sf green-languages $(GL_BIN_DIR)/$(GL_BIN_ALIAS)
	for script in script/*.py; do \
		if [ "$$(basename $$script)" = "__main__.py" ]; then \
			install -m 755 $$script $(GL_BIN_DIR)/gls; \
		else \
			install -m 644 $$script $(GL_LIB_DIR)/$$(basename $$script); \
		fi \
	done
	sudo sysctl kernel.perf_event_paranoid=2

uninstall:
	rm -f $(GL_BIN_DIR)/green-languages
	rm -f $(GL_BIN_DIR)/$(GL_BIN_ALIAS)
	rm -rf $(GL_LIB_DIR)
	cargo clean --manifest-path measurements/Cargo.toml
	cargo clean

.PHONY: all debug release uninstall
.SILENT:
