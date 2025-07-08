/**
 * @file ring_buffer.c
 * @brief Lock-free circular buffer implementation
 * 
 * Implementation of a high-performance, lock-free circular buffer
 * using mmap for memory management and atomic operations for thread safety.
 */

#include "ring_buffer.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <errno.h>
#include <time.h>
#include <assert.h>

/* Magic number for buffer validation */
#define RING_BUFFER_MAGIC 0x52424652  /* "RBFR" */

/* CRC32 polynomial (IEEE 802.3) */
#define CRC32_POLYNOMIAL 0xEDB88320

/* Memory barriers for different architectures */
#ifdef __x86_64__
#define memory_barrier() __asm__ __volatile__("mfence" ::: "memory")
#define read_barrier() __asm__ __volatile__("lfence" ::: "memory")
#define write_barrier() __asm__ __volatile__("sfence" ::: "memory")
#else
#define memory_barrier() __sync_synchronize()
#define read_barrier() __sync_synchronize()
#define write_barrier() __sync_synchronize()
#endif

/* Align size to message alignment */
static inline size_t align_size(size_t size) {
    return (size + MESSAGE_ALIGNMENT - 1) & ~(MESSAGE_ALIGNMENT - 1);
}

/* Safe memory copy with bounds checking */
static inline int safe_memcpy(void *dest, const void *src, size_t n, size_t dest_size) {
    if (dest == NULL || src == NULL || n > dest_size) {
        return -1; /* Invalid parameters or buffer overflow */
    }
    memcpy(dest, src, n);
    return 0;
}

/* Calculate total message size including header */
static inline size_t total_message_size(size_t data_size) {
    return align_size(sizeof(arrow_ipc_header_t) + data_size);
}

/* CRC32 table for fast computation */
static uint32_t crc32_table[256];
static bool crc32_table_initialized = false;

/* Initialize CRC32 lookup table */
static void init_crc32_table(void) {
    if (crc32_table_initialized) return;
    
    for (int i = 0; i < 256; i++) {
        uint32_t crc = i;
        for (int j = 0; j < 8; j++) {
            if (crc & 1) {
                crc = (crc >> 1) ^ CRC32_POLYNOMIAL;
            } else {
                crc >>= 1;
            }
        }
        crc32_table[i] = crc;
    }
    crc32_table_initialized = true;
}

uint32_t ring_buffer_crc32(const void *data, size_t size) {
    if (!crc32_table_initialized) {
        init_crc32_table();
    }
    
    uint32_t crc = 0xFFFFFFFF;
    const uint8_t *bytes = (const uint8_t *)data;
    
    for (size_t i = 0; i < size; i++) {
        crc = crc32_table[(crc ^ bytes[i]) & 0xFF] ^ (crc >> 8);
    }
    
    return crc ^ 0xFFFFFFFF;
}

uint64_t ring_buffer_timestamp(void) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ULL + (uint64_t)ts.tv_nsec;
}

size_t ring_buffer_next_power_of_2(size_t n) {
    if (n == 0) return 1;
    n--;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    n |= n >> 32;
    return n + 1;
}

ring_buffer_t *ring_buffer_create(size_t size) {
    if (size == 0) {
        size = RING_BUFFER_DEFAULT_SIZE;
    }
    
    /* Ensure size is power of 2 for efficient modulo operations */
    size = ring_buffer_next_power_of_2(size);
    
    
    /* Allocate ring buffer structure */
    ring_buffer_t *rb = calloc(1, sizeof(ring_buffer_t));
    if (!rb) {
        return NULL;
    }
    
    /* Allocate buffer memory - fallback to malloc for compatibility */
    rb->fd = -1;
    
    /* Try MAP_ANON first (macOS) */
#if defined(MAP_ANON)
    rb->buffer = mmap(NULL, size, PROT_READ | PROT_WRITE, 
                      MAP_PRIVATE | MAP_ANON, -1, 0);
    if (rb->buffer == MAP_FAILED) {
#elif defined(MAP_ANONYMOUS)
    rb->buffer = mmap(NULL, size, PROT_READ | PROT_WRITE, 
                      MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (rb->buffer == MAP_FAILED) {
#else
    /* Force fallback */
    rb->buffer = MAP_FAILED;
    if (true) {
#endif
        /* Fallback to regular malloc */
        rb->buffer = malloc(size);
        if (!rb->buffer) {
            free(rb);
            return NULL;
        }
        memset(rb->buffer, 0, size);
    }
    
    /* Initialize buffer structure */
    rb->size = size;
    rb->magic = RING_BUFFER_MAGIC;
    rb->backpressure_threshold = RING_BUFFER_BACKPRESSURE_THRESHOLD;
    
    /* Initialize atomic positions */
    atomic_store(&rb->write_pos, 0);
    atomic_store(&rb->read_pos, 0);
    atomic_store(&rb->commit_pos, 0);
    atomic_store(&rb->is_full, false);
    atomic_store(&rb->backpressure, false);
    
    /* Initialize statistics */
    atomic_store(&rb->messages_written, 0);
    atomic_store(&rb->messages_read, 0);
    atomic_store(&rb->bytes_written, 0);
    atomic_store(&rb->bytes_read, 0);
    atomic_store(&rb->write_errors, 0);
    atomic_store(&rb->read_errors, 0);
    atomic_store(&rb->backpressure_events, 0);
    
    /* Initialize CRC32 table */
    init_crc32_table();
    
    return rb;
}

void ring_buffer_destroy(ring_buffer_t *rb) {
    if (!rb) return;
    
    if (rb->buffer) {
        /* Check if it was allocated with mmap or malloc */
        if (rb->fd >= 0) {
            /* mmap allocated, use munmap */
            munmap(rb->buffer, rb->size);
            close(rb->fd);
        } else {
            /* Try munmap first, if it fails, it was malloc allocated */
            if (munmap(rb->buffer, rb->size) != 0) {
                /* munmap failed, it was malloc allocated */
                free(rb->buffer);
            }
        }
    }
    
    free(rb);
}

double ring_buffer_utilization(const ring_buffer_t *rb) {
    if (!rb) return 0.0;
    
    size_t write_pos = atomic_load(&rb->write_pos);
    size_t read_pos = atomic_load(&rb->read_pos);
    
    size_t used;
    if (write_pos >= read_pos) {
        used = write_pos - read_pos;
    } else {
        used = rb->size - (read_pos - write_pos);
    }
    
    return (double)used / (double)rb->size;
}

size_t ring_buffer_available_write(const ring_buffer_t *rb) {
    if (!rb) return 0;
    
    size_t write_pos = atomic_load(&rb->write_pos);
    size_t read_pos = atomic_load(&rb->read_pos);
    
    if (write_pos >= read_pos) {
        return rb->size - (write_pos - read_pos) - 1;
    } else {
        return read_pos - write_pos - 1;
    }
}

size_t ring_buffer_available_read(const ring_buffer_t *rb) {
    if (!rb) return 0;
    
    size_t write_pos = atomic_load(&rb->commit_pos);
    size_t read_pos = atomic_load(&rb->read_pos);
    
    if (write_pos >= read_pos) {
        return write_pos - read_pos;
    } else {
        return rb->size - (read_pos - write_pos);
    }
}

bool ring_buffer_is_backpressure(const ring_buffer_t *rb) {
    if (!rb) return false;
    return atomic_load(&rb->backpressure);
}

void ring_buffer_get_stats(const ring_buffer_t *rb, ring_buffer_stats_t *stats) {
    if (!rb || !stats) return;
    
    stats->messages_written = atomic_load(&rb->messages_written);
    stats->messages_read = atomic_load(&rb->messages_read);
    stats->bytes_written = atomic_load(&rb->bytes_written);
    stats->bytes_read = atomic_load(&rb->bytes_read);
    stats->write_errors = atomic_load(&rb->write_errors);
    stats->read_errors = atomic_load(&rb->read_errors);
    stats->backpressure_events = atomic_load(&rb->backpressure_events);
}

void ring_buffer_reset_stats(ring_buffer_t *rb) {
    if (!rb) return;
    
    atomic_store(&rb->messages_written, 0);
    atomic_store(&rb->messages_read, 0);
    atomic_store(&rb->bytes_written, 0);
    atomic_store(&rb->bytes_read, 0);
    atomic_store(&rb->write_errors, 0);
    atomic_store(&rb->read_errors, 0);
    atomic_store(&rb->backpressure_events, 0);
}

bool ring_buffer_validate(const ring_buffer_t *rb) {
    if (!rb) return false;
    
    /* Check magic number */
    if (rb->magic != RING_BUFFER_MAGIC) {
        return false;
    }
    
    /* Check buffer pointer */
    if (!rb->buffer || rb->buffer == MAP_FAILED) {
        return false;
    }
    
    /* Check size is power of 2 */
    if (rb->size == 0 || (rb->size & (rb->size - 1)) != 0) {
        return false;
    }
    
    /* Check positions are within bounds */
    size_t write_pos = atomic_load(&rb->write_pos);
    size_t read_pos = atomic_load(&rb->read_pos);
    size_t commit_pos = atomic_load(&rb->commit_pos);
    
    if (write_pos >= rb->size || read_pos >= rb->size || commit_pos >= rb->size) {
        return false;
    }
    
    return true;
}

const char *ring_buffer_error_string(ring_buffer_error_t error) {
    switch (error) {
        case RING_BUFFER_SUCCESS: return "Success";
        case RING_BUFFER_ERROR_INVALID_PARAM: return "Invalid parameter";
        case RING_BUFFER_ERROR_MEMORY: return "Memory allocation error";
        case RING_BUFFER_ERROR_FULL: return "Buffer full";
        case RING_BUFFER_ERROR_EMPTY: return "Buffer empty";
        case RING_BUFFER_ERROR_TOO_LARGE: return "Message too large";
        case RING_BUFFER_ERROR_CORRUPTED: return "Buffer corrupted";
        case RING_BUFFER_ERROR_BACKPRESSURE: return "Backpressure active";
        default: return "Unknown error";
    }
}

ring_buffer_error_t ring_buffer_write(ring_buffer_t *rb, const void *data, size_t size) {
    if (!rb || !data || size == 0) {
        return RING_BUFFER_ERROR_INVALID_PARAM;
    }
    
    if (!ring_buffer_validate(rb)) {
        atomic_fetch_add(&rb->write_errors, 1);
        return RING_BUFFER_ERROR_CORRUPTED;
    }
    
    if (size > RING_BUFFER_MAX_MESSAGE_SIZE) {
        atomic_fetch_add(&rb->write_errors, 1);
        return RING_BUFFER_ERROR_TOO_LARGE;
    }
    
    /* Check backpressure */
    if (ring_buffer_utilization(rb) >= rb->backpressure_threshold) {
        atomic_store(&rb->backpressure, true);
        atomic_fetch_add(&rb->backpressure_events, 1);
        return RING_BUFFER_ERROR_BACKPRESSURE;
    } else {
        atomic_store(&rb->backpressure, false);
    }
    
    size_t msg_size = total_message_size(size);
    
    /* Check if message fits */
    if (msg_size > ring_buffer_available_write(rb)) {
        atomic_fetch_add(&rb->write_errors, 1);
        return RING_BUFFER_ERROR_FULL;
    }
    
    /* Reserve space atomically */
    size_t write_pos = atomic_load(&rb->write_pos);
    size_t new_write_pos = (write_pos + msg_size) & (rb->size - 1);
    
    /* Try to reserve space with CAS loop */
    while (!atomic_compare_exchange_weak(&rb->write_pos, &write_pos, new_write_pos)) {
        /* Recalculate after position change */
        new_write_pos = (write_pos + msg_size) & (rb->size - 1);
        
        /* Check if we still have space */
        if (msg_size > ring_buffer_available_write(rb)) {
            atomic_fetch_add(&rb->write_errors, 1);
            return RING_BUFFER_ERROR_FULL;
        }
    }
    
    /* Write message header */
    arrow_ipc_header_t header = {
        .magic = ARROW_IPC_MAGIC,
        .length = (uint32_t)size,
        .timestamp = ring_buffer_timestamp(),
        .checksum = ring_buffer_crc32(data, size),
        .reserved = 0
    };
    
    uint8_t *buffer = (uint8_t *)rb->buffer;
    
    /* Handle wrap-around for header */
    if (write_pos + sizeof(arrow_ipc_header_t) <= rb->size) {
        if (safe_memcpy(buffer + write_pos, &header, sizeof(arrow_ipc_header_t), rb->size - write_pos) != 0) {
            return RING_BUFFER_ERROR;
        }
    } else {
        /* Header spans across buffer boundary */
        size_t first_part = rb->size - write_pos;
        if (safe_memcpy(buffer + write_pos, &header, first_part, first_part) != 0 ||
            safe_memcpy(buffer, (uint8_t *)&header + first_part, sizeof(arrow_ipc_header_t) - first_part, rb->size) != 0) {
            return RING_BUFFER_ERROR;
        }
    }
    
    /* Write message data */
    size_t data_start = (write_pos + sizeof(arrow_ipc_header_t)) & (rb->size - 1);
    
    if (data_start + size <= rb->size) {
        if (safe_memcpy(buffer + data_start, data, size, rb->size - data_start) != 0) {
            return RING_BUFFER_ERROR;
        }
    } else {
        /* Data spans across buffer boundary */
        size_t first_part = rb->size - data_start;
        if (safe_memcpy(buffer + data_start, data, first_part, first_part) != 0 ||
            safe_memcpy(buffer, (uint8_t *)data + first_part, size - first_part, rb->size) != 0) {
            return RING_BUFFER_ERROR;
        }
    }
    
    /* Memory barrier to ensure data is written before commit */
    write_barrier();
    
    /* Commit the write atomically */
    atomic_store(&rb->commit_pos, new_write_pos);
    
    /* Update statistics */
    atomic_fetch_add(&rb->messages_written, 1);
    atomic_fetch_add(&rb->bytes_written, size);
    
    return RING_BUFFER_SUCCESS;
}

ring_buffer_error_t ring_buffer_read(ring_buffer_t *rb, ring_buffer_message_t *msg) {
    if (!rb || !msg) {
        return RING_BUFFER_ERROR_INVALID_PARAM;
    }
    
    if (!ring_buffer_validate(rb)) {
        atomic_fetch_add(&rb->read_errors, 1);
        return RING_BUFFER_ERROR_CORRUPTED;
    }
    
    /* Check if data is available */
    if (ring_buffer_available_read(rb) < sizeof(arrow_ipc_header_t)) {
        return RING_BUFFER_ERROR_EMPTY;
    }
    
    size_t read_pos = atomic_load(&rb->read_pos);
    uint8_t *buffer = (uint8_t *)rb->buffer;
    
    /* Read message header */
    arrow_ipc_header_t header;
    if (read_pos + sizeof(arrow_ipc_header_t) <= rb->size) {
        if (safe_memcpy(&header, buffer + read_pos, sizeof(arrow_ipc_header_t), sizeof(arrow_ipc_header_t)) != 0) {
            return RING_BUFFER_ERROR;
        }
    } else {
        /* Header spans across buffer boundary */
        size_t first_part = rb->size - read_pos;
        if (safe_memcpy(&header, buffer + read_pos, first_part, sizeof(arrow_ipc_header_t)) != 0 ||
            safe_memcpy((uint8_t *)&header + first_part, buffer, sizeof(arrow_ipc_header_t) - first_part, sizeof(arrow_ipc_header_t) - first_part) != 0) {
            return RING_BUFFER_ERROR;
        }
    }
    
    /* Validate header */
    if (header.magic != ARROW_IPC_MAGIC) {
        atomic_fetch_add(&rb->read_errors, 1);
        return RING_BUFFER_ERROR_CORRUPTED;
    }
    
    if (header.length > RING_BUFFER_MAX_MESSAGE_SIZE) {
        atomic_fetch_add(&rb->read_errors, 1);
        return RING_BUFFER_ERROR_CORRUPTED;
    }
    
    size_t msg_size = total_message_size(header.length);
    
    /* Check if complete message is available */
    if (ring_buffer_available_read(rb) < msg_size) {
        return RING_BUFFER_ERROR_EMPTY;
    }
    
    /* Get pointer to message data */
    size_t data_start = (read_pos + sizeof(arrow_ipc_header_t)) & (rb->size - 1);
    
    /* For messages that don't wrap around, we can return direct pointer */
    if (data_start + header.length <= rb->size) {
        msg->data = buffer + data_start;
        msg->data_size = header.length;
        
        /* Validate checksum */
        uint32_t checksum = ring_buffer_crc32(msg->data, header.length);
        if (checksum != header.checksum) {
            atomic_fetch_add(&rb->read_errors, 1);
            return RING_BUFFER_ERROR_CORRUPTED;
        }
    } else {
        /* Message wraps around - this is a limitation of zero-copy approach */
        /* For now, we'll return an error for wrapped messages */
        /* In a production system, you might want to copy to a temporary buffer */
        atomic_fetch_add(&rb->read_errors, 1);
        return RING_BUFFER_ERROR_CORRUPTED;
    }
    
    /* Copy header to output */
    msg->header = header;
    
    /* Update read position */
    size_t new_read_pos = (read_pos + msg_size) & (rb->size - 1);
    atomic_store(&rb->read_pos, new_read_pos);
    
    /* Update statistics */
    atomic_fetch_add(&rb->messages_read, 1);
    atomic_fetch_add(&rb->bytes_read, header.length);
    
    return RING_BUFFER_SUCCESS;
}

