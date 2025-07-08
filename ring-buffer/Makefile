# Makefile for Chronicle Ring Buffer
# High-performance lock-free circular buffer for Arrow IPC messages

# Compiler and flags
CC = gcc
CFLAGS = -std=c11 -Wall -Wextra -Wpedantic -O3 -g
CFLAGS += -fPIC -D_GNU_SOURCE -D_POSIX_C_SOURCE=200809L
CFLAGS += -march=native -mtune=native

# Debug flags
DEBUG_CFLAGS = -std=c11 -Wall -Wextra -Wpedantic -O0 -g3
DEBUG_CFLAGS += -fPIC -D_GNU_SOURCE -D_POSIX_C_SOURCE=200809L
DEBUG_CFLAGS += -DDEBUG -fsanitize=address -fsanitize=thread

# Linker flags (macOS doesn't have librt)
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    LDFLAGS = -lpthread -lm
    DEBUG_LDFLAGS = -lpthread -lm -fsanitize=address -fsanitize=thread
else
    LDFLAGS = -lrt -lpthread -lm
    DEBUG_LDFLAGS = -lrt -lpthread -lm -fsanitize=address -fsanitize=thread
endif

# Source files
SOURCES = ring_buffer.c
HEADERS = ring_buffer.h
OBJECTS = $(SOURCES:.c=.o)

# Test files
TEST_SOURCES = test_ring_buffer.c
TEST_OBJECTS = $(TEST_SOURCES:.c=.o)
TEST_BINARY = test_ring_buffer

# Benchmark files
BENCH_SOURCES = bench_ring_buffer.c
BENCH_OBJECTS = $(BENCH_SOURCES:.c=.o)
BENCH_BINARY = bench_ring_buffer

# Library files
STATIC_LIB = libringbuffer.a
SHARED_LIB = libringbuffer.so

# Default target
.PHONY: all clean test bench debug install uninstall help

all: $(STATIC_LIB) $(SHARED_LIB) $(TEST_BINARY) $(BENCH_BINARY)

# Static library
$(STATIC_LIB): $(OBJECTS)
	@echo "Creating static library: $@"
	ar rcs $@ $^

# Shared library
$(SHARED_LIB): $(OBJECTS)
	@echo "Creating shared library: $@"
	$(CC) -shared -o $@ $^ $(LDFLAGS)

# Object files
%.o: %.c $(HEADERS)
	@echo "Compiling: $<"
	$(CC) $(CFLAGS) -c $< -o $@

# Test binary
$(TEST_BINARY): $(TEST_OBJECTS) $(STATIC_LIB)
	@echo "Linking test binary: $@"
	$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)

# Benchmark binary
$(BENCH_BINARY): $(BENCH_OBJECTS) $(STATIC_LIB)
	@echo "Linking benchmark binary: $@"
	$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)

# Debug builds
debug: CFLAGS = $(DEBUG_CFLAGS)
debug: LDFLAGS = $(DEBUG_LDFLAGS)
debug: clean $(STATIC_LIB) $(TEST_BINARY)
	@echo "Debug build complete"

# Run tests
test: $(TEST_BINARY)
	@echo "Running unit tests..."
	./$(TEST_BINARY)

# Run benchmarks
bench: $(BENCH_BINARY)
	@echo "Running benchmarks..."
	./$(BENCH_BINARY)

# Performance test with specific parameters
perf: $(BENCH_BINARY)
	@echo "Running performance tests..."
	./$(BENCH_BINARY) --messages 1000000 --threads 4 --buffer-size 67108864

# Memory test with valgrind
memtest: $(TEST_BINARY)
	@echo "Running memory tests with valgrind..."
	valgrind --tool=memcheck --leak-check=full --show-leak-kinds=all ./$(TEST_BINARY)

# Thread safety test with helgrind
threadtest: $(TEST_BINARY)
	@echo "Running thread safety tests with helgrind..."
	valgrind --tool=helgrind ./$(TEST_BINARY)

# Install library (requires root)
install: $(STATIC_LIB) $(SHARED_LIB)
	@echo "Installing ring buffer library..."
	install -d /usr/local/include
	install -m 644 $(HEADERS) /usr/local/include/
	install -d /usr/local/lib
	install -m 644 $(STATIC_LIB) /usr/local/lib/
	install -m 755 $(SHARED_LIB) /usr/local/lib/
	ldconfig

# Uninstall library (requires root)
uninstall:
	@echo "Uninstalling ring buffer library..."
	rm -f /usr/local/include/ring_buffer.h
	rm -f /usr/local/lib/$(STATIC_LIB)
	rm -f /usr/local/lib/$(SHARED_LIB)
	ldconfig

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	rm -f $(OBJECTS) $(TEST_OBJECTS) $(BENCH_OBJECTS)
	rm -f $(STATIC_LIB) $(SHARED_LIB)
	rm -f $(TEST_BINARY) $(BENCH_BINARY)
	rm -f *.gcov *.gcda *.gcno
	rm -f core core.*
	rm -f vgcore.*

# Code coverage
coverage: CFLAGS += --coverage
coverage: LDFLAGS += --coverage
coverage: clean $(TEST_BINARY)
	@echo "Running tests with coverage..."
	./$(TEST_BINARY)
	gcov $(SOURCES)
	@echo "Coverage report generated (*.gcov files)"

# Static analysis with cppcheck
static-analysis:
	@echo "Running static analysis..."
	cppcheck --enable=all --std=c11 --platform=unix64 $(SOURCES) $(TEST_SOURCES) $(BENCH_SOURCES)

# Format code
format:
	@echo "Formatting code..."
	clang-format -i $(SOURCES) $(HEADERS) $(TEST_SOURCES) $(BENCH_SOURCES)

# Profile with gprof
profile: CFLAGS += -pg
profile: LDFLAGS += -pg
profile: clean $(BENCH_BINARY)
	@echo "Running profiling..."
	./$(BENCH_BINARY)
	gprof $(BENCH_BINARY) gmon.out > profile.txt
	@echo "Profile report generated: profile.txt"

# Assembly output
asm: $(SOURCES)
	@echo "Generating assembly output..."
	$(CC) $(CFLAGS) -S -masm=intel $(SOURCES)

# Preprocessor output
preprocess: $(SOURCES)
	@echo "Generating preprocessor output..."
	$(CC) $(CFLAGS) -E $(SOURCES) > preprocessed.c

# Show compiler version and flags
info:
	@echo "Compiler: $(CC)"
	@echo "Version: $$($(CC) --version | head -1)"
	@echo "CFLAGS: $(CFLAGS)"
	@echo "LDFLAGS: $(LDFLAGS)"
	@echo "Sources: $(SOURCES)"
	@echo "Headers: $(HEADERS)"

# Help target
help:
	@echo "Chronicle Ring Buffer Makefile"
	@echo "==============================="
	@echo ""
	@echo "Targets:"
	@echo "  all             - Build static library, shared library, tests, and benchmarks"
	@echo "  $(STATIC_LIB)   - Build static library"
	@echo "  $(SHARED_LIB)   - Build shared library"
	@echo "  $(TEST_BINARY)  - Build test binary"
	@echo "  $(BENCH_BINARY) - Build benchmark binary"
	@echo "  debug           - Build debug version with sanitizers"
	@echo "  test            - Run unit tests"
	@echo "  bench           - Run benchmarks"
	@echo "  perf            - Run performance tests with specific parameters"
	@echo "  memtest         - Run memory tests with valgrind"
	@echo "  threadtest      - Run thread safety tests with helgrind"
	@echo "  coverage        - Build and run tests with coverage analysis"
	@echo "  static-analysis - Run static analysis with cppcheck"
	@echo "  format          - Format code with clang-format"
	@echo "  profile         - Build and run profiling with gprof"
	@echo "  asm             - Generate assembly output"
	@echo "  preprocess      - Generate preprocessor output"
	@echo "  install         - Install library system-wide (requires root)"
	@echo "  uninstall       - Uninstall library (requires root)"
	@echo "  clean           - Clean build artifacts"
	@echo "  info            - Show compiler and build information"
	@echo "  help            - Show this help message"
	@echo ""
	@echo "Examples:"
	@echo "  make clean && make all    # Clean build everything"
	@echo "  make test                 # Run unit tests"
	@echo "  make bench                # Run benchmarks"
	@echo "  make debug test           # Debug build and test"
	@echo "  make coverage             # Test with coverage analysis"

# Phony targets
.PHONY: all clean test bench debug install uninstall help coverage
.PHONY: static-analysis format profile asm preprocess info memtest threadtest perf