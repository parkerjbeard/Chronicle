/**
 * @file ring_buffer.h
 * @brief Lock-free circular buffer for Arrow IPC messages
 * 
 * This header defines a high-performance, lock-free circular buffer
 * that stores Arrow IPC messages using mmap for memory management.
 * The buffer supports concurrent readers and writers with atomic operations
 * for thread safety.
 */

#ifndef RING_BUFFER_H
#define RING_BUFFER_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdatomic.h>
#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Default buffer size: 64MB */
#define RING_BUFFER_DEFAULT_SIZE (64 * 1024 * 1024)

/* Backpressure threshold: 80% full */
#define RING_BUFFER_BACKPRESSURE_THRESHOLD 0.8

/* Maximum message size: 16MB */
#define RING_BUFFER_MAX_MESSAGE_SIZE (16 * 1024 * 1024)

/* Arrow IPC message magic number */
#define ARROW_IPC_MAGIC 0x41524157  /* "ARAW" */

/* Message alignment in bytes */
#define MESSAGE_ALIGNMENT 8

/**
 * @brief Error codes for ring buffer operations
 */
typedef enum {
    RING_BUFFER_SUCCESS = 0,
    RING_BUFFER_ERROR_INVALID_PARAM = -1,
    RING_BUFFER_ERROR_MEMORY = -2,
    RING_BUFFER_ERROR_FULL = -3,
    RING_BUFFER_ERROR_EMPTY = -4,
    RING_BUFFER_ERROR_TOO_LARGE = -5,
    RING_BUFFER_ERROR_CORRUPTED = -6,
    RING_BUFFER_ERROR_BACKPRESSURE = -7
} ring_buffer_error_t;

/**
 * @brief Arrow IPC message header
 * 
 * Each message in the buffer is prefixed with this header
 * to enable proper serialization/deserialization
 */
typedef struct {
    uint32_t magic;         /* Magic number for validation */
    uint32_t length;        /* Message length in bytes */
    uint64_t timestamp;     /* Message timestamp (nanoseconds since epoch) */
    uint32_t checksum;      /* CRC32 checksum of message data */
    uint32_t reserved;      /* Reserved for future use */
} __attribute__((packed)) arrow_ipc_header_t;

/**
 * @brief Ring buffer statistics
 */
typedef struct {
    uint64_t messages_written;
    uint64_t messages_read;
    uint64_t bytes_written;
    uint64_t bytes_read;
    uint64_t write_errors;
    uint64_t read_errors;
    uint64_t backpressure_events;
} ring_buffer_stats_t;

/**
 * @brief Lock-free ring buffer structure
 * 
 * Uses atomic operations for thread-safe access between multiple
 * readers and writers. Memory is allocated via mmap for efficient
 * virtual memory management.
 */
typedef struct {
    /* Memory mapped buffer */
    void *buffer;
    size_t size;
    int fd;  /* File descriptor for mmap */
    
    /* Lock-free atomic positions */
    atomic_size_t write_pos;    /* Next write position */
    atomic_size_t read_pos;     /* Next read position */
    atomic_size_t commit_pos;   /* Last committed write position */
    
    /* Buffer state flags */
    atomic_bool is_full;
    atomic_bool backpressure;
    
    /* Statistics (atomic for thread safety) */
    atomic_uint_fast64_t messages_written;
    atomic_uint_fast64_t messages_read;
    atomic_uint_fast64_t bytes_written;
    atomic_uint_fast64_t bytes_read;
    atomic_uint_fast64_t write_errors;
    atomic_uint_fast64_t read_errors;
    atomic_uint_fast64_t backpressure_events;
    
    /* Configuration */
    double backpressure_threshold;
    
    /* Validation */
    uint32_t magic;
    
} ring_buffer_t;

/**
 * @brief Message structure for reading from buffer
 */
typedef struct {
    arrow_ipc_header_t header;
    const void *data;
    size_t data_size;
} ring_buffer_message_t;

/* Function declarations */

/**
 * @brief Create a new ring buffer
 * 
 * @param size Buffer size in bytes (must be power of 2)
 * @return Pointer to ring buffer or NULL on error
 */
ring_buffer_t *ring_buffer_create(size_t size);

/**
 * @brief Destroy a ring buffer and free resources
 * 
 * @param rb Ring buffer to destroy
 */
void ring_buffer_destroy(ring_buffer_t *rb);

/**
 * @brief Write an Arrow IPC message to the buffer
 * 
 * This function is lock-free and thread-safe. It will return
 * RING_BUFFER_ERROR_BACKPRESSURE if the buffer is at the
 * backpressure threshold.
 * 
 * @param rb Ring buffer
 * @param data Message data
 * @param size Message size in bytes
 * @return RING_BUFFER_SUCCESS on success, error code on failure
 */
ring_buffer_error_t ring_buffer_write(ring_buffer_t *rb, const void *data, size_t size);

/**
 * @brief Read an Arrow IPC message from the buffer
 * 
 * This function is lock-free and thread-safe. The returned message
 * points to memory within the buffer and is valid until the next
 * read operation or until the buffer wraps around.
 * 
 * @param rb Ring buffer
 * @param msg Output message structure
 * @return RING_BUFFER_SUCCESS on success, error code on failure
 */
ring_buffer_error_t ring_buffer_read(ring_buffer_t *rb, ring_buffer_message_t *msg);

/**
 * @brief Get current buffer utilization percentage
 * 
 * @param rb Ring buffer
 * @return Utilization percentage (0.0 to 1.0)
 */
double ring_buffer_utilization(const ring_buffer_t *rb);

/**
 * @brief Get number of available bytes for writing
 * 
 * @param rb Ring buffer
 * @return Available bytes
 */
size_t ring_buffer_available_write(const ring_buffer_t *rb);

/**
 * @brief Get number of available bytes for reading
 * 
 * @param rb Ring buffer
 * @return Available bytes
 */
size_t ring_buffer_available_read(const ring_buffer_t *rb);

/**
 * @brief Check if buffer is in backpressure state
 * 
 * @param rb Ring buffer
 * @return true if in backpressure, false otherwise
 */
bool ring_buffer_is_backpressure(const ring_buffer_t *rb);

/**
 * @brief Get buffer statistics
 * 
 * @param rb Ring buffer
 * @param stats Output statistics structure
 */
void ring_buffer_get_stats(const ring_buffer_t *rb, ring_buffer_stats_t *stats);

/**
 * @brief Reset buffer statistics
 * 
 * @param rb Ring buffer
 */
void ring_buffer_reset_stats(ring_buffer_t *rb);

/**
 * @brief Validate buffer integrity
 * 
 * @param rb Ring buffer
 * @return true if valid, false if corrupted
 */
bool ring_buffer_validate(const ring_buffer_t *rb);

/**
 * @brief Get error string for error code
 * 
 * @param error Error code
 * @return Human-readable error string
 */
const char *ring_buffer_error_string(ring_buffer_error_t error);

/**
 * @brief Calculate CRC32 checksum
 * 
 * @param data Input data
 * @param size Data size
 * @return CRC32 checksum
 */
uint32_t ring_buffer_crc32(const void *data, size_t size);

/**
 * @brief Get current timestamp in nanoseconds
 * 
 * @return Timestamp in nanoseconds since epoch
 */
uint64_t ring_buffer_timestamp(void);

/**
 * @brief Round up to next power of 2
 * 
 * @param n Input number
 * @return Next power of 2
 */
size_t ring_buffer_next_power_of_2(size_t n);

#ifdef __cplusplus
}
#endif

#endif /* RING_BUFFER_H */

