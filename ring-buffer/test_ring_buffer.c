/**
 * @file test_ring_buffer.c
 * @brief Comprehensive unit tests for the ring buffer implementation
 * 
 * Tests cover basic operations, concurrent access, error conditions,
 * and performance characteristics of the lock-free ring buffer.
 */

#include "ring_buffer.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <pthread.h>
#include <unistd.h>
#include <time.h>
#include <sys/time.h>

/* Test configuration */
#define TEST_BUFFER_SIZE (1024 * 1024)  /* 1MB for tests */
#define TEST_MESSAGE_COUNT 10000
#define TEST_THREAD_COUNT 4
#define TEST_LARGE_MESSAGE_SIZE (512 * 1024)  /* 512KB */

/* Test statistics */
typedef struct {
    int tests_run;
    int tests_passed;
    int tests_failed;
} test_stats_t;

static test_stats_t g_test_stats = {0, 0, 0};

/* Test utilities */
#define TEST_ASSERT(condition, message) do { \
    g_test_stats.tests_run++; \
    if (!(condition)) { \
        printf("FAIL: %s - %s\n", __func__, message); \
        g_test_stats.tests_failed++; \
        return false; \
    } \
    g_test_stats.tests_passed++; \
} while(0)

#define RUN_TEST(test_func) do { \
    printf("Running %s...\n", #test_func); \
    if (test_func()) { \
        printf("PASS: %s\n", #test_func); \
    } else { \
        printf("FAIL: %s\n", #test_func); \
    } \
} while(0)

/* Thread test data */
typedef struct {
    ring_buffer_t *rb;
    int thread_id;
    int message_count;
    int messages_written;
    int messages_read;
    int write_errors;
    int read_errors;
    double start_time;
    double end_time;
} thread_test_data_t;

/* Get current time in seconds */
static double get_time(void) {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (double)tv.tv_sec + (double)tv.tv_usec / 1000000.0;
}

/* Generate test data */
static void generate_test_data(void *data, size_t size, int pattern) {
    uint8_t *bytes = (uint8_t *)data;
    for (size_t i = 0; i < size; i++) {
        bytes[i] = (uint8_t)((pattern + i) & 0xFF);
    }
}

/* Verify test data */
static bool verify_test_data(const void *data, size_t size, int pattern) {
    const uint8_t *bytes = (const uint8_t *)data;
    for (size_t i = 0; i < size; i++) {
        if (bytes[i] != (uint8_t)((pattern + i) & 0xFF)) {
            return false;
        }
    }
    return true;
}

/* Test basic ring buffer creation and destruction */
static bool test_create_destroy(void) {
    ring_buffer_t *rb = ring_buffer_create(TEST_BUFFER_SIZE);
    TEST_ASSERT(rb != NULL, "Failed to create ring buffer");
    TEST_ASSERT(ring_buffer_validate(rb), "Invalid ring buffer");
    
    ring_buffer_destroy(rb);
    return true;
}

/* Test invalid parameters */
static bool test_invalid_params(void) {
    ring_buffer_t *rb = ring_buffer_create(TEST_BUFFER_SIZE);
    TEST_ASSERT(rb != NULL, "Failed to create ring buffer");
    
    char data[100];
    ring_buffer_message_t msg;
    
    /* Test NULL parameters */
    TEST_ASSERT(ring_buffer_write(NULL, data, sizeof(data)) == RING_BUFFER_ERROR_INVALID_PARAM,
                "Should reject NULL buffer");
    TEST_ASSERT(ring_buffer_write(rb, NULL, sizeof(data)) == RING_BUFFER_ERROR_INVALID_PARAM,
                "Should reject NULL data");
    TEST_ASSERT(ring_buffer_write(rb, data, 0) == RING_BUFFER_ERROR_INVALID_PARAM,
                "Should reject zero size");
    
    TEST_ASSERT(ring_buffer_read(NULL, &msg) == RING_BUFFER_ERROR_INVALID_PARAM,
                "Should reject NULL buffer");
    TEST_ASSERT(ring_buffer_read(rb, NULL) == RING_BUFFER_ERROR_INVALID_PARAM,
                "Should reject NULL message");
    
    ring_buffer_destroy(rb);
    return true;
}

/* Test basic read/write operations */
static bool test_basic_read_write(void) {
    ring_buffer_t *rb = ring_buffer_create(TEST_BUFFER_SIZE);
    TEST_ASSERT(rb != NULL, "Failed to create ring buffer");
    
    /* Test writing and reading a single message */
    const char *test_data = "Hello, World!";
    size_t test_size = strlen(test_data);
    
    ring_buffer_error_t result = ring_buffer_write(rb, test_data, test_size);
    TEST_ASSERT(result == RING_BUFFER_SUCCESS, "Failed to write message");
    
    ring_buffer_message_t msg;
    result = ring_buffer_read(rb, &msg);
    TEST_ASSERT(result == RING_BUFFER_SUCCESS, "Failed to read message");
    TEST_ASSERT(msg.header.length == test_size, "Message size mismatch");
    TEST_ASSERT(memcmp(msg.data, test_data, test_size) == 0, "Message data mismatch");
    
    /* Test reading from empty buffer */
    result = ring_buffer_read(rb, &msg);
    TEST_ASSERT(result == RING_BUFFER_ERROR_EMPTY, "Should return empty error");
    
    ring_buffer_destroy(rb);
    return true;
}

/* Test multiple messages */
static bool test_multiple_messages(void) {
    ring_buffer_t *rb = ring_buffer_create(TEST_BUFFER_SIZE);
    TEST_ASSERT(rb != NULL, "Failed to create ring buffer");
    
    const int num_messages = 100;
    char data[256];
    
    /* Write multiple messages */
    for (int i = 0; i < num_messages; i++) {
        snprintf(data, sizeof(data), "Message %d", i);
        size_t size = strlen(data);
        
        ring_buffer_error_t result = ring_buffer_write(rb, data, size);
        TEST_ASSERT(result == RING_BUFFER_SUCCESS, "Failed to write message");
    }
    
    /* Read and verify messages */
    for (int i = 0; i < num_messages; i++) {
        ring_buffer_message_t msg;
        ring_buffer_error_t result = ring_buffer_read(rb, &msg);
        TEST_ASSERT(result == RING_BUFFER_SUCCESS, "Failed to read message");
        
        char expected[256];
        snprintf(expected, sizeof(expected), "Message %d", i);
        TEST_ASSERT(msg.header.length == strlen(expected), "Message size mismatch");
        TEST_ASSERT(memcmp(msg.data, expected, strlen(expected)) == 0, "Message data mismatch");
    }
    
    ring_buffer_destroy(rb);
    return true;
}

/* Test buffer wrap-around */
static bool test_buffer_wraparound(void) {
    ring_buffer_t *rb = ring_buffer_create(8192);  /* Small buffer to force wrap */
    TEST_ASSERT(rb != NULL, "Failed to create ring buffer");
    
    char data[512];
    const int num_messages = 50;  /* Will cause wrap-around */
    
    /* Write and read messages to force wrap-around */
    for (int round = 0; round < 5; round++) {
        /* Write messages */
        for (int i = 0; i < num_messages; i++) {
            generate_test_data(data, sizeof(data), i + round * num_messages);
            
            ring_buffer_error_t result = ring_buffer_write(rb, data, sizeof(data));
            if (result == RING_BUFFER_ERROR_FULL) {
                break;  /* Buffer full, normal for wrap-around test */
            }
            TEST_ASSERT(result == RING_BUFFER_SUCCESS, "Failed to write message");
        }
        
        /* Read some messages */
        for (int i = 0; i < num_messages / 2; i++) {
            ring_buffer_message_t msg;
            ring_buffer_error_t result = ring_buffer_read(rb, &msg);
            if (result == RING_BUFFER_ERROR_EMPTY) {
                break;  /* No more messages */
            }
            TEST_ASSERT(result == RING_BUFFER_SUCCESS, "Failed to read message");
        }
    }
    
    ring_buffer_destroy(rb);
    return true;
}

/* Test buffer overflow protection */
static bool test_buffer_overflow(void) {
    ring_buffer_t *rb = ring_buffer_create(4096);  /* Very small buffer */
    TEST_ASSERT(rb != NULL, "Failed to create ring buffer");
    
    char data[1024];
    generate_test_data(data, sizeof(data), 0);
    
    /* Fill buffer until full */
    int messages_written = 0;
    while (true) {
        ring_buffer_error_t result = ring_buffer_write(rb, data, sizeof(data));
        if (result == RING_BUFFER_ERROR_FULL) {
            break;
        }
        TEST_ASSERT(result == RING_BUFFER_SUCCESS, "Unexpected write error");
        messages_written++;
    }
    
    TEST_ASSERT(messages_written > 0, "Should have written at least one message");
    
    /* Verify buffer is full */
    ring_buffer_error_t result = ring_buffer_write(rb, data, sizeof(data));
    TEST_ASSERT(result == RING_BUFFER_ERROR_FULL, "Should reject write to full buffer");
    
    ring_buffer_destroy(rb);
    return true;
}

/* Test backpressure mechanism */
static bool test_backpressure(void) {
    ring_buffer_t *rb = ring_buffer_create(8192);
    TEST_ASSERT(rb != NULL, "Failed to create ring buffer");
    
    char data[512];
    generate_test_data(data, sizeof(data), 0);
    
    /* Fill buffer until backpressure is triggered */
    while (true) {
        ring_buffer_error_t result = ring_buffer_write(rb, data, sizeof(data));
        if (result == RING_BUFFER_ERROR_BACKPRESSURE) {
            TEST_ASSERT(ring_buffer_is_backpressure(rb), "Backpressure flag not set");
            break;
        }
        if (result == RING_BUFFER_ERROR_FULL) {
            break;  /* Buffer full before backpressure */
        }
        TEST_ASSERT(result == RING_BUFFER_SUCCESS, "Unexpected write error");
    }
    
    ring_buffer_destroy(rb);
    return true;
}

/* Test statistics */
static bool test_statistics(void) {
    ring_buffer_t *rb = ring_buffer_create(TEST_BUFFER_SIZE);
    TEST_ASSERT(rb != NULL, "Failed to create ring buffer");
    
    ring_buffer_stats_t stats;
    ring_buffer_get_stats(rb, &stats);
    
    TEST_ASSERT(stats.messages_written == 0, "Initial messages_written should be 0");
    TEST_ASSERT(stats.messages_read == 0, "Initial messages_read should be 0");
    TEST_ASSERT(stats.bytes_written == 0, "Initial bytes_written should be 0");
    TEST_ASSERT(stats.bytes_read == 0, "Initial bytes_read should be 0");
    
    /* Write some messages */
    const int num_messages = 10;
    char data[100];
    for (int i = 0; i < num_messages; i++) {
        generate_test_data(data, sizeof(data), i);
        ring_buffer_error_t result = ring_buffer_write(rb, data, sizeof(data));
        TEST_ASSERT(result == RING_BUFFER_SUCCESS, "Failed to write message");
    }
    
    /* Check write statistics */
    ring_buffer_get_stats(rb, &stats);
    TEST_ASSERT(stats.messages_written == num_messages, "Incorrect messages_written count");
    TEST_ASSERT(stats.bytes_written == num_messages * sizeof(data), "Incorrect bytes_written count");
    
    /* Read some messages */
    for (int i = 0; i < num_messages / 2; i++) {
        ring_buffer_message_t msg;
        ring_buffer_error_t result = ring_buffer_read(rb, &msg);
        TEST_ASSERT(result == RING_BUFFER_SUCCESS, "Failed to read message");
    }
    
    /* Check read statistics */
    ring_buffer_get_stats(rb, &stats);
    TEST_ASSERT(stats.messages_read == num_messages / 2, "Incorrect messages_read count");
    TEST_ASSERT(stats.bytes_read == (num_messages / 2) * sizeof(data), "Incorrect bytes_read count");
    
    ring_buffer_destroy(rb);
    return true;
}

/* Test message checksum validation */
static bool test_checksum_validation(void) {
    ring_buffer_t *rb = ring_buffer_create(TEST_BUFFER_SIZE);
    TEST_ASSERT(rb != NULL, "Failed to create ring buffer");
    
    char data[256];
    generate_test_data(data, sizeof(data), 42);
    
    /* Write message */
    ring_buffer_error_t result = ring_buffer_write(rb, data, sizeof(data));
    TEST_ASSERT(result == RING_BUFFER_SUCCESS, "Failed to write message");
    
    /* Read message and verify checksum */
    ring_buffer_message_t msg;
    result = ring_buffer_read(rb, &msg);
    TEST_ASSERT(result == RING_BUFFER_SUCCESS, "Failed to read message");
    
    /* Verify checksum manually */
    uint32_t expected_checksum = ring_buffer_crc32(data, sizeof(data));
    TEST_ASSERT(msg.header.checksum == expected_checksum, "Checksum mismatch");
    
    ring_buffer_destroy(rb);
    return true;
}

/* Test utility functions */
static bool test_utility_functions(void) {
    /* Test CRC32 function */
    const char *test_data = "Hello, World!";
    uint32_t crc1 = ring_buffer_crc32(test_data, strlen(test_data));
    uint32_t crc2 = ring_buffer_crc32(test_data, strlen(test_data));
    TEST_ASSERT(crc1 == crc2, "CRC32 should be deterministic");
    
    /* Test different data produces different CRC */
    const char *test_data2 = "Hello, World?";
    uint32_t crc3 = ring_buffer_crc32(test_data2, strlen(test_data2));
    TEST_ASSERT(crc1 != crc3, "Different data should produce different CRC");
    
    /* Test power of 2 function */
    TEST_ASSERT(ring_buffer_next_power_of_2(1) == 1, "Next power of 2 for 1 should be 1");
    TEST_ASSERT(ring_buffer_next_power_of_2(2) == 2, "Next power of 2 for 2 should be 2");
    TEST_ASSERT(ring_buffer_next_power_of_2(3) == 4, "Next power of 2 for 3 should be 4");
    TEST_ASSERT(ring_buffer_next_power_of_2(1023) == 1024, "Next power of 2 for 1023 should be 1024");
    
    /* Test timestamp function */
    uint64_t ts1 = ring_buffer_timestamp();
    usleep(1000);  /* Sleep 1ms */
    uint64_t ts2 = ring_buffer_timestamp();
    TEST_ASSERT(ts2 > ts1, "Timestamp should increase");
    
    return true;
}

/* Writer thread function */
static void *writer_thread(void *arg) {
    thread_test_data_t *data = (thread_test_data_t *)arg;
    char message[256];
    
    data->start_time = get_time();
    
    for (int i = 0; i < data->message_count; i++) {
        snprintf(message, sizeof(message), "Thread %d Message %d", data->thread_id, i);
        
        ring_buffer_error_t result = ring_buffer_write(data->rb, message, strlen(message));
        if (result == RING_BUFFER_SUCCESS) {
            data->messages_written++;
        } else {
            data->write_errors++;
            if (result == RING_BUFFER_ERROR_FULL) {
                usleep(100);  /* Brief pause on full buffer */
                i--;  /* Retry this message */
            }
        }
    }
    
    data->end_time = get_time();
    return NULL;
}

/* Reader thread function */
static void *reader_thread(void *arg) {
    thread_test_data_t *data = (thread_test_data_t *)arg;
    
    data->start_time = get_time();
    
    while (data->messages_read < data->message_count) {
        ring_buffer_message_t msg;
        ring_buffer_error_t result = ring_buffer_read(data->rb, &msg);
        
        if (result == RING_BUFFER_SUCCESS) {
            data->messages_read++;
        } else if (result == RING_BUFFER_ERROR_EMPTY) {
            usleep(10);  /* Brief pause on empty buffer */
        } else {
            data->read_errors++;
        }
    }
    
    data->end_time = get_time();
    return NULL;
}

/* Test concurrent access */
static bool test_concurrent_access(void) {
    ring_buffer_t *rb = ring_buffer_create(TEST_BUFFER_SIZE);
    TEST_ASSERT(rb != NULL, "Failed to create ring buffer");
    
    const int num_threads = 4;
    const int messages_per_thread = 1000;
    
    pthread_t writer_threads[num_threads];
    pthread_t reader_threads[num_threads];
    thread_test_data_t writer_data[num_threads];
    thread_test_data_t reader_data[num_threads];
    
    /* Create writer threads */
    for (int i = 0; i < num_threads; i++) {
        writer_data[i].rb = rb;
        writer_data[i].thread_id = i;
        writer_data[i].message_count = messages_per_thread;
        writer_data[i].messages_written = 0;
        writer_data[i].write_errors = 0;
        
        int result = pthread_create(&writer_threads[i], NULL, writer_thread, &writer_data[i]);
        TEST_ASSERT(result == 0, "Failed to create writer thread");
    }
    
    /* Create reader threads */
    for (int i = 0; i < num_threads; i++) {
        reader_data[i].rb = rb;
        reader_data[i].thread_id = i;
        reader_data[i].message_count = messages_per_thread;
        reader_data[i].messages_read = 0;
        reader_data[i].read_errors = 0;
        
        int result = pthread_create(&reader_threads[i], NULL, reader_thread, &reader_data[i]);
        TEST_ASSERT(result == 0, "Failed to create reader thread");
    }
    
    /* Wait for all threads to complete */
    for (int i = 0; i < num_threads; i++) {
        pthread_join(writer_threads[i], NULL);
        pthread_join(reader_threads[i], NULL);
    }
    
    /* Verify results */
    int total_written = 0, total_read = 0;
    for (int i = 0; i < num_threads; i++) {
        total_written += writer_data[i].messages_written;
        total_read += reader_data[i].messages_read;
    }
    
    TEST_ASSERT(total_written > 0, "Should have written some messages");
    TEST_ASSERT(total_read > 0, "Should have read some messages");
    
    /* Check statistics */
    ring_buffer_stats_t stats;
    ring_buffer_get_stats(rb, &stats);
    TEST_ASSERT(stats.messages_written == (uint64_t)total_written, "Statistics mismatch");
    TEST_ASSERT(stats.messages_read == (uint64_t)total_read, "Statistics mismatch");
    
    ring_buffer_destroy(rb);
    return true;
}

/* Test large messages */
static bool test_large_messages(void) {
    ring_buffer_t *rb = ring_buffer_create(TEST_BUFFER_SIZE);
    TEST_ASSERT(rb != NULL, "Failed to create ring buffer");
    
    /* Test message at maximum size */
    char *large_data = malloc(TEST_LARGE_MESSAGE_SIZE);
    TEST_ASSERT(large_data != NULL, "Failed to allocate large message");
    
    generate_test_data(large_data, TEST_LARGE_MESSAGE_SIZE, 0xAB);
    
    ring_buffer_error_t result = ring_buffer_write(rb, large_data, TEST_LARGE_MESSAGE_SIZE);
    TEST_ASSERT(result == RING_BUFFER_SUCCESS, "Failed to write large message");
    
    ring_buffer_message_t msg;
    result = ring_buffer_read(rb, &msg);
    TEST_ASSERT(result == RING_BUFFER_SUCCESS, "Failed to read large message");
    TEST_ASSERT(msg.header.length == TEST_LARGE_MESSAGE_SIZE, "Large message size mismatch");
    TEST_ASSERT(verify_test_data(msg.data, TEST_LARGE_MESSAGE_SIZE, 0xAB), "Large message data corruption");
    
    free(large_data);
    ring_buffer_destroy(rb);
    return true;
}

/* Test error conditions */
static bool test_error_conditions(void) {
    ring_buffer_t *rb = ring_buffer_create(TEST_BUFFER_SIZE);
    TEST_ASSERT(rb != NULL, "Failed to create ring buffer");
    
    /* Test message too large */
    char *huge_data = malloc(RING_BUFFER_MAX_MESSAGE_SIZE + 1);
    TEST_ASSERT(huge_data != NULL, "Failed to allocate huge message");
    
    ring_buffer_error_t result = ring_buffer_write(rb, huge_data, RING_BUFFER_MAX_MESSAGE_SIZE + 1);
    TEST_ASSERT(result == RING_BUFFER_ERROR_TOO_LARGE, "Should reject oversized message");
    
    free(huge_data);
    
    /* Test error strings */
    const char *error_str = ring_buffer_error_string(RING_BUFFER_SUCCESS);
    TEST_ASSERT(error_str != NULL, "Error string should not be NULL");
    TEST_ASSERT(strlen(error_str) > 0, "Error string should not be empty");
    
    ring_buffer_destroy(rb);
    return true;
}

/* Run all tests */
static void run_all_tests(void) {
    printf("=== Chronicle Ring Buffer Unit Tests ===\n");
    printf("Build: %s %s\n", __DATE__, __TIME__);
    printf("Buffer size: %d bytes\n", TEST_BUFFER_SIZE);
    printf("==========================================\n\n");
    
    RUN_TEST(test_create_destroy);
    RUN_TEST(test_invalid_params);
    RUN_TEST(test_basic_read_write);
    RUN_TEST(test_multiple_messages);
    RUN_TEST(test_buffer_wraparound);
    RUN_TEST(test_buffer_overflow);
    RUN_TEST(test_backpressure);
    RUN_TEST(test_statistics);
    RUN_TEST(test_checksum_validation);
    RUN_TEST(test_utility_functions);
    RUN_TEST(test_concurrent_access);
    RUN_TEST(test_large_messages);
    RUN_TEST(test_error_conditions);
    
    printf("\n=== Test Summary ===\n");
    printf("Tests run: %d\n", g_test_stats.tests_run);
    printf("Tests passed: %d\n", g_test_stats.tests_passed);
    printf("Tests failed: %d\n", g_test_stats.tests_failed);
    printf("Success rate: %.1f%%\n", 
           (double)g_test_stats.tests_passed / (double)g_test_stats.tests_run * 100.0);
    
    if (g_test_stats.tests_failed == 0) {
        printf("\nAll tests PASSED! ✓\n");
    } else {
        printf("\nSome tests FAILED! ✗\n");
    }
}

int main(void) {
    run_all_tests();
    return (g_test_stats.tests_failed == 0) ? 0 : 1;
}
