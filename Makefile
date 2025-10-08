IT_SO_RELEASE := iterations/target/release/libiterations.so
IT_A_RELEASE := iterations/target/release/libiterations.a
IT_SO_DEBUG := iterations/target/debug/libiterations.so
IT_A_DEBUG := iterations/target/debug/libiterations.a
IT_HEADER := iterations/iterations.h
IT_JNI := iterations/Iterations.java
GL_BIN_DIR := $(HOME)/.local/bin
GL_LIB_DIR := $(HOME)/.local/lib/green-languages
GL_BIN_NAME := green-languages
GL_BIN_ALIAS := gl
GL_BIN_DEBUG := target/debug/$(GL_BIN_NAME)
GL_BIN_RELEASE := target/release/$(GL_BIN_NAME)

all: debug

$(IT_SO_DEBUG) $(IT_A_DEBUG):
	cargo build --manifest-path iterations/Cargo.toml

$(IT_SO_RELEASE) $(IT_A_RELEASE):
	cargo build --release --manifest-path iterations/Cargo.toml

debug: $(IT_SO_DEBUG) $(IT_A_DEBUG)
	install -d -m 755 $(GL_BIN_DIR)
	install -d -m 755 $(GL_LIB_DIR)
	install -m 755 $(IT_SO_DEBUG) $(GL_LIB_DIR)
	install -m 755 $(IT_A_DEBUG) $(GL_LIB_DIR)
	install -m 644 $(IT_HEADER) $(GL_LIB_DIR)
	install -m 644 $(IT_JNI) $(GL_LIB_DIR)
	GL_LIB_DIR=$(GL_LIB_DIR) cargo build
	install -m 755 $(GL_BIN_DEBUG) $(GL_BIN_DIR)/$(GL_BIN_NAME)
	sudo setcap cap_sys_rawio,cap_perfmon,cap_sys_nice=ep $(GL_BIN_DIR)/$(GL_BIN_NAME)
	ln -sf $(GL_BIN_NAME) $(GL_BIN_DIR)/$(GL_BIN_ALIAS)
	for script in script/*.py; do \
		if [ "$$(basename $$script)" = "__main__.py" ]; then \
			install -m 755 $$script $(GL_BIN_DIR)/gl-script; \
		else \
			install -m 644 $$script $(GL_LIB_DIR)/$$(basename $$script); \
		fi \
	done
	sudo sysctl kernel.perf_event_paranoid=2

release: $(IT_SO_RELEASE) $(IT_A_RELEASE)
	install -d -m 755 $(GL_BIN_DIR)
	install -d -m 755 $(GL_LIB_DIR)
	install -m 755 $(IT_SO_RELEASE) $(GL_LIB_DIR)
	install -m 755 $(IT_A_RELEASE) $(GL_LIB_DIR)
	install -m 644 $(IT_HEADER) $(GL_LIB_DIR)
	install -m 644 $(IT_JNI) $(GL_LIB_DIR)
	GL_LIB_DIR=$(GL_LIB_DIR) cargo build --release
	install -m 755 $(GL_BIN_RELEASE) $(GL_BIN_DIR)/$(GL_BIN_NAME)
	sudo setcap cap_sys_rawio,cap_perfmon,cap_sys_nice=ep $(GL_BIN_DIR)/$(GL_BIN_NAME)
	ln -sf $(GL_BIN_NAME) $(GL_BIN_DIR)/$(GL_BIN_ALIAS)
	for script in script/*.py; do \
		if [ "$$(basename $$script)" = "__main__.py" ]; then \
			install -m 755 $$script $(GL_BIN_DIR)/gl-script; \
		else \
			install -m 644 $$script $(GL_LIB_DIR)/$$(basename $$script); \
		fi \
	done
	sudo sysctl kernel.perf_event_paranoid=2

uninstall:
	rm -f $(GL_BIN_DIR)/$(GL_BIN_NAME)
	rm -f $(GL_BIN_DIR)/$(GL_BIN_ALIAS)
	rm -f $(GL_BIN_DIR)/gl-env
	rm -rf $(GL_LIB_DIR)
	cargo clean --manifest-path iterations/Cargo.toml
	cargo clean

.PHONY: all debug release uninstall
.SILENT:
