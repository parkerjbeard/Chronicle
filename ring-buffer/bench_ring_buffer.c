/**
 * @file bench_ring_buffer.c
 * @brief Performance benchmarks for the ring buffer implementation
 * 
 * Comprehensive benchmarks to measure throughput, latency, and
 * concurrent performance of the lock-free ring buffer.
 */

#include "ring_buffer.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <pthread.h>
#include <time.h>
#include <sys/time.h>
#include <getopt.h>
#include <signal.h>
#include <errno.h>

/* macOS doesn't have pthread barriers - implement a simple replacement */
#ifdef __APPLE__
typedef struct {
    pthread_mutex_t mutex;
    pthread_cond_t cond;
    int count;
    int waiting;
} pthread_barrier_t;

typedef int pthread_barrierattr_t;

static int pthread_barrier_init(pthread_barrier_t *barrier, const pthread_barrierattr_t *attr, unsigned int count) {
    (void)attr;
    if (count == 0) {
        errno = EINVAL;
        return -1;
    }
    pthread_mutex_init(&barrier->mutex, NULL);
    pthread_cond_init(&barrier->cond, NULL);
    barrier->count = count;
    barrier->waiting = 0;
    return 0;
}

static int pthread_barrier_wait(pthread_barrier_t *barrier) {
    pthread_mutex_lock(&barrier->mutex);
    barrier->waiting++;
    if (barrier->waiting == barrier->count) {
        barrier->waiting = 0;
        pthread_cond_broadcast(&barrier->cond);
        pthread_mutex_unlock(&barrier->mutex);
        return 1;
    } else {
        pthread_cond_wait(&barrier->cond, &barrier->mutex);
        pthread_mutex_unlock(&barrier->mutex);
        return 0;
    }
}

static int pthread_barrier_destroy(pthread_barrier_t *barrier) {
    pthread_mutex_destroy(&barrier->mutex);
    pthread_cond_destroy(&barrier->cond);
    return 0;
}
#endif

/* Default benchmark parameters */
#define DEFAULT_BUFFER_SIZE (64 * 1024 * 1024)  /* 64MB */
#define DEFAULT_MESSAGE_COUNT 1000000
#define DEFAULT_MESSAGE_SIZE 1024
#define DEFAULT_THREAD_COUNT 4
#define DEFAULT_DURATION 10  /* seconds */

/* Benchmark configuration */
typedef struct {
    size_t buffer_size;
    int message_count;
    size_t message_size;
    int thread_count;
    int duration;
    bool continuous;
    bool verbose;
    int pattern;
} bench_config_t;

/* Benchmark results */
typedef struct {
    double start_time;
    double end_time;
    double duration;
    uint64_t messages_processed;
    uint64_t bytes_processed;
    double throughput_msgs_per_sec;
    double throughput_mbps;
    double avg_latency_us;
    double min_latency_us;
    double max_latency_us;
    uint64_t errors;
} bench_results_t;

/* Thread benchmark data */
typedef struct {
    ring_buffer_t *rb;
    int thread_id;
    bench_config_t *config;
    bench_results_t results;
    volatile bool *stop_flag;
    pthread_barrier_t *start_barrier;
    pthread_barrier_t *end_barrier;
} thread_bench_data_t;

/* Global stop flag for signal handling */
static volatile bool g_stop_benchmark = false;

/* Signal handler for graceful shutdown */
static void signal_handler(int sig) {
    (void)sig;
    g_stop_benchmark = true;
    printf("\nBenchmark interrupted by signal\n");
}

/* Get current time in seconds with high precision */
static double get_time(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec / 1000000000.0;
}

/* Get time in microseconds */
static uint64_t get_time_us(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000000ULL + (uint64_t)ts.tv_nsec / 1000ULL;
}

/* Generate benchmark data */
static void generate_bench_data(void *data, size_t size, int pattern) {
    uint8_t *bytes = (uint8_t *)data;
    for (size_t i = 0; i < size; i++) {
        bytes[i] = (uint8_t)((pattern + i) & 0xFF);
    }
}

/* Print benchmark results */
static void print_results(const char *name, const bench_results_t *results) {
    printf("\n=== %s Results ===\n", name);
    printf("Duration: %.3f seconds\n", results->duration);
    printf("Messages: %llu\n", (unsigned long long)results->messages_processed);
    printf("Bytes: %llu (%.2f MB)\n", (unsigned long long)results->bytes_processed, 
           (double)results->bytes_processed / (1024 * 1024));
    printf("Throughput: %.2f messages/sec\n", results->throughput_msgs_per_sec);
    printf("Throughput: %.2f MB/s\n", results->throughput_mbps);
    if (results->avg_latency_us > 0) {
        printf("Avg Latency: %.2f μs\n", results->avg_latency_us);
        printf("Min Latency: %.2f μs\n", results->min_latency_us);
        printf("Max Latency: %.2f μs\n", results->max_latency_us);
    }
    printf("Errors: %llu\n", (unsigned long long)results->errors);
}

/* Calculate statistics from results */
static void calculate_stats(bench_results_t *results) {
    results->duration = results->end_time - results->start_time;
    if (results->duration > 0) {
        results->throughput_msgs_per_sec = (double)results->messages_processed / results->duration;
        results->throughput_mbps = (double)results->bytes_processed / (1024 * 1024) / results->duration;
    }
}

/* Single-threaded write benchmark */
static void bench_single_write(bench_config_t *config, bench_results_t *results) {
    ring_buffer_t *rb = ring_buffer_create(config->buffer_size);
    if (!rb) {
        printf("Failed to create ring buffer\n");
        return;
    }
    
    char *data = malloc(config->message_size);
    if (!data) {
        printf("Failed to allocate message buffer\n");
        ring_buffer_destroy(rb);
        return;
    }
    
    generate_bench_data(data, config->message_size, config->pattern);
    
    memset(results, 0, sizeof(bench_results_t));
    results->start_time = get_time();
    
    int messages_written = 0;
    while (messages_written < config->message_count && !g_stop_benchmark) {
        ring_buffer_error_t result = ring_buffer_write(rb, data, config->message_size);
        if (result == RING_BUFFER_SUCCESS) {
            messages_written++;
            results->messages_processed++;
            results->bytes_processed += config->message_size;
        } else {
            results->errors++;
            if (result == RING_BUFFER_ERROR_FULL) {
                usleep(1);  /* Brief pause on full buffer */
            }
        }
    }
    
    results->end_time = get_time();
    calculate_stats(results);
    
    free(data);
    ring_buffer_destroy(rb);
}

/* Single-threaded read benchmark */
static void bench_single_read(bench_config_t *config, bench_results_t *results) {
    ring_buffer_t *rb = ring_buffer_create(config->buffer_size);
    if (!rb) {
        printf("Failed to create ring buffer\n");
        return;
    }
    
    char *data = malloc(config->message_size);
    if (!data) {
        printf("Failed to allocate message buffer\n");
        ring_buffer_destroy(rb);
        return;
    }
    
    generate_bench_data(data, config->message_size, config->pattern);
    
    /* Pre-fill buffer with messages */
    int messages_to_fill = config->message_count;
    for (int i = 0; i < messages_to_fill; i++) {
        ring_buffer_error_t result = ring_buffer_write(rb, data, config->message_size);
        if (result != RING_BUFFER_SUCCESS) {
            messages_to_fill = i;
            break;
        }
    }
    
    memset(results, 0, sizeof(bench_results_t));
    results->start_time = get_time();
    
    int messages_read = 0;
    while (messages_read < messages_to_fill && !g_stop_benchmark) {
        ring_buffer_message_t msg;
        ring_buffer_error_t result = ring_buffer_read(rb, &msg);
        if (result == RING_BUFFER_SUCCESS) {
            messages_read++;
            results->messages_processed++;
            results->bytes_processed += msg.header.length;
        } else {
            results->errors++;
            if (result == RING_BUFFER_ERROR_EMPTY) {
                break;
            }
        }
    }
    
    results->end_time = get_time();
    calculate_stats(results);
    
    free(data);
    ring_buffer_destroy(rb);
}

/* Writer thread for concurrent benchmark */
static void *writer_thread_bench(void *arg) {
    thread_bench_data_t *data = (thread_bench_data_t *)arg;
    char *message = malloc(data->config->message_size);
    if (!message) {
        return NULL;
    }
    
    generate_bench_data(message, data->config->message_size, 
                       data->config->pattern + data->thread_id);
    
    /* Wait for all threads to be ready */
    pthread_barrier_wait(data->start_barrier);
    
    memset(&data->results, 0, sizeof(bench_results_t));
    data->results.start_time = get_time();
    
    int messages_written = 0;
    int target_messages = data->config->message_count / data->config->thread_count;
    
    while (messages_written < target_messages && !*data->stop_flag) {
        ring_buffer_error_t result = ring_buffer_write(data->rb, message, data->config->message_size);
        if (result == RING_BUFFER_SUCCESS) {
            messages_written++;
            data->results.messages_processed++;
            data->results.bytes_processed += data->config->message_size;
        } else {
            data->results.errors++;
            if (result == RING_BUFFER_ERROR_FULL || result == RING_BUFFER_ERROR_BACKPRESSURE) {
                usleep(1);  /* Brief pause */
            }
        }
    }
    
    data->results.end_time = get_time();
    calculate_stats(&data->results);
    
    free(message);
    
    /* Wait for all threads to finish */
    pthread_barrier_wait(data->end_barrier);
    
    return NULL;
}


/* Concurrent write benchmark */
static void bench_concurrent_write(bench_config_t *config, bench_results_t *results) {
    ring_buffer_t *rb = ring_buffer_create(config->buffer_size);
    if (!rb) {
        printf("Failed to create ring buffer\n");
        return;
    }
    
    pthread_t *threads = malloc(config->thread_count * sizeof(pthread_t));
    thread_bench_data_t *thread_data = malloc(config->thread_count * sizeof(thread_bench_data_t));
    pthread_barrier_t start_barrier, end_barrier;
    volatile bool stop_flag = false;
    
    if (!threads || !thread_data) {
        printf("Failed to allocate thread data\n");
        ring_buffer_destroy(rb);
        return;
    }
    
    pthread_barrier_init(&start_barrier, NULL, config->thread_count);
    pthread_barrier_init(&end_barrier, NULL, config->thread_count);
    
    /* Create threads */
    for (int i = 0; i < config->thread_count; i++) {
        thread_data[i].rb = rb;
        thread_data[i].thread_id = i;
        thread_data[i].config = config;
        thread_data[i].stop_flag = &stop_flag;
        thread_data[i].start_barrier = &start_barrier;
        thread_data[i].end_barrier = &end_barrier;
        
        int result = pthread_create(&threads[i], NULL, writer_thread_bench, &thread_data[i]);
        if (result != 0) {
            printf("Failed to create thread %d\n", i);
            return;
        }
    }
    
    /* Wait for all threads to complete */
    for (int i = 0; i < config->thread_count; i++) {
        pthread_join(threads[i], NULL);
    }
    
    /* Aggregate results */
    memset(results, 0, sizeof(bench_results_t));
    results->start_time = thread_data[0].results.start_time;
    results->end_time = thread_data[0].results.end_time;
    
    for (int i = 0; i < config->thread_count; i++) {
        results->messages_processed += thread_data[i].results.messages_processed;
        results->bytes_processed += thread_data[i].results.bytes_processed;
        results->errors += thread_data[i].results.errors;
        
        if (thread_data[i].results.start_time < results->start_time) {
            results->start_time = thread_data[i].results.start_time;
        }
        if (thread_data[i].results.end_time > results->end_time) {
            results->end_time = thread_data[i].results.end_time;
        }
    }
    
    calculate_stats(results);
    
    pthread_barrier_destroy(&start_barrier);
    pthread_barrier_destroy(&end_barrier);
    free(threads);
    free(thread_data);
    ring_buffer_destroy(rb);
}

/* Latency benchmark */
static void bench_latency(bench_config_t *config, bench_results_t *results) {
    ring_buffer_t *rb = ring_buffer_create(config->buffer_size);
    if (!rb) {
        printf("Failed to create ring buffer\n");
        return;
    }
    
    char *data = malloc(config->message_size);
    if (!data) {
        printf("Failed to allocate message buffer\n");
        ring_buffer_destroy(rb);
        return;
    }
    
    generate_bench_data(data, config->message_size, config->pattern);
    
    uint64_t *latencies = malloc(config->message_count * sizeof(uint64_t));
    if (!latencies) {
        printf("Failed to allocate latency array\n");
        free(data);
        ring_buffer_destroy(rb);
        return;
    }
    
    memset(results, 0, sizeof(bench_results_t));
    results->start_time = get_time();
    results->min_latency_us = UINT64_MAX;
    results->max_latency_us = 0;
    
    int successful_ops = 0;
    for (int i = 0; i < config->message_count && !g_stop_benchmark; i++) {
        uint64_t start = get_time_us();
        
        ring_buffer_error_t write_result = ring_buffer_write(rb, data, config->message_size);
        if (write_result != RING_BUFFER_SUCCESS) {
            results->errors++;
            continue;
        }
        
        ring_buffer_message_t msg;
        ring_buffer_error_t read_result = ring_buffer_read(rb, &msg);
        if (read_result != RING_BUFFER_SUCCESS) {
            results->errors++;
            continue;
        }
        
        uint64_t end = get_time_us();
        uint64_t latency = end - start;
        
        latencies[successful_ops] = latency;
        results->avg_latency_us += latency;
        
        if (latency < results->min_latency_us) {
            results->min_latency_us = latency;
        }
        if (latency > results->max_latency_us) {
            results->max_latency_us = latency;
        }
        
        successful_ops++;
        results->messages_processed++;
        results->bytes_processed += config->message_size;
    }
    
    results->end_time = get_time();
    
    if (successful_ops > 0) {
        results->avg_latency_us /= successful_ops;
    }
    
    calculate_stats(results);
    
    free(latencies);
    free(data);
    ring_buffer_destroy(rb);
}

/* Memory usage benchmark */
static void bench_memory_usage(bench_config_t *config) {
    printf("\n=== Memory Usage Benchmark ===\n");
    
    ring_buffer_t *rb = ring_buffer_create(config->buffer_size);
    if (!rb) {
        printf("Failed to create ring buffer\n");
        return;
    }
    
    printf("Buffer Size: %zu bytes (%.2f MB)\n", 
           config->buffer_size, (double)config->buffer_size / (1024 * 1024));
    printf("Ring Buffer Struct: %zu bytes\n", sizeof(ring_buffer_t));
    printf("Message Header: %zu bytes\n", sizeof(arrow_ipc_header_t));
    
    /* Calculate utilization at different fill levels */
    char *data = malloc(config->message_size);
    if (!data) {
        printf("Failed to allocate message buffer\n");
        ring_buffer_destroy(rb);
        return;
    }
    
    generate_bench_data(data, config->message_size, config->pattern);
    
    int messages_written = 0;
    while (true) {
        ring_buffer_error_t result = ring_buffer_write(rb, data, config->message_size);
        if (result != RING_BUFFER_SUCCESS) {
            break;
        }
        messages_written++;
        
        if (messages_written % 1000 == 0) {
            double utilization = ring_buffer_utilization(rb);
            printf("Messages: %d, Utilization: %.1f%%\n", 
                   messages_written, utilization * 100.0);
        }
    }
    
    printf("Max messages: %d\n", messages_written);
    printf("Final utilization: %.1f%%\n", ring_buffer_utilization(rb) * 100.0);
    
    free(data);
    ring_buffer_destroy(rb);
}

/* Print usage information */
static void print_usage(const char *program_name) {
    printf("Usage: %s [OPTIONS]\n", program_name);
    printf("Options:\n");
    printf("  -s, --buffer-size SIZE    Ring buffer size in bytes (default: %d)\n", DEFAULT_BUFFER_SIZE);
    printf("  -m, --messages COUNT      Number of messages to process (default: %d)\n", DEFAULT_MESSAGE_COUNT);
    printf("  -z, --message-size SIZE   Message size in bytes (default: %d)\n", DEFAULT_MESSAGE_SIZE);
    printf("  -t, --threads COUNT       Number of threads (default: %d)\n", DEFAULT_THREAD_COUNT);
    printf("  -d, --duration SECONDS    Benchmark duration in seconds (default: %d)\n", DEFAULT_DURATION);
    printf("  -c, --continuous          Run continuous benchmark\n");
    printf("  -v, --verbose             Verbose output\n");
    printf("  -p, --pattern PATTERN     Data pattern (default: 0)\n");
    printf("  -h, --help                Show this help message\n");
    printf("\nBenchmarks:\n");
    printf("  - Single-threaded write throughput\n");
    printf("  - Single-threaded read throughput\n");
    printf("  - Multi-threaded write throughput\n");
    printf("  - Round-trip latency\n");
    printf("  - Memory usage analysis\n");
}

/* Parse command line arguments */
static bool parse_args(int argc, char *argv[], bench_config_t *config) {
    static struct option long_options[] = {
        {"buffer-size", required_argument, 0, 's'},
        {"messages", required_argument, 0, 'm'},
        {"message-size", required_argument, 0, 'z'},
        {"threads", required_argument, 0, 't'},
        {"duration", required_argument, 0, 'd'},
        {"continuous", no_argument, 0, 'c'},
        {"verbose", no_argument, 0, 'v'},
        {"pattern", required_argument, 0, 'p'},
        {"help", no_argument, 0, 'h'},
        {0, 0, 0, 0}
    };
    
    int option_index = 0;
    int c;
    
    while ((c = getopt_long(argc, argv, "s:m:z:t:d:cvp:h", long_options, &option_index)) != -1) {
        switch (c) {
            case 's':
                config->buffer_size = (size_t)atoll(optarg);
                break;
            case 'm':
                config->message_count = atoi(optarg);
                break;
            case 'z':
                config->message_size = (size_t)atoll(optarg);
                break;
            case 't':
                config->thread_count = atoi(optarg);
                break;
            case 'd':
                config->duration = atoi(optarg);
                break;
            case 'c':
                config->continuous = true;
                break;
            case 'v':
                config->verbose = true;
                break;
            case 'p':
                config->pattern = atoi(optarg);
                break;
            case 'h':
                print_usage(argv[0]);
                return false;
            default:
                print_usage(argv[0]);
                return false;
        }
    }
    
    return true;
}

/* Main benchmark function */
int main(int argc, char *argv[]) {
    bench_config_t config = {
        .buffer_size = DEFAULT_BUFFER_SIZE,
        .message_count = DEFAULT_MESSAGE_COUNT,
        .message_size = DEFAULT_MESSAGE_SIZE,
        .thread_count = DEFAULT_THREAD_COUNT,
        .duration = DEFAULT_DURATION,
        .continuous = false,
        .verbose = false,
        .pattern = 0
    };
    
    if (!parse_args(argc, argv, &config)) {
        return 1;
    }
    
    /* Install signal handlers */
    signal(SIGINT, signal_handler);
    signal(SIGTERM, signal_handler);
    
    printf("=== Chronicle Ring Buffer Benchmarks ===\n");
    printf("Build: %s %s\n", __DATE__, __TIME__);
    printf("Buffer Size: %zu bytes (%.2f MB)\n", 
           config.buffer_size, (double)config.buffer_size / (1024 * 1024));
    printf("Message Count: %d\n", config.message_count);
    printf("Message Size: %zu bytes\n", config.message_size);
    printf("Thread Count: %d\n", config.thread_count);
    printf("==========================================\n");
    
    bench_results_t results;
    
    /* Single-threaded write benchmark */
    printf("\nRunning single-threaded write benchmark...\n");
    bench_single_write(&config, &results);
    print_results("Single-threaded Write", &results);
    
    /* Single-threaded read benchmark */
    printf("\nRunning single-threaded read benchmark...\n");
    bench_single_read(&config, &results);
    print_results("Single-threaded Read", &results);
    
    /* Multi-threaded write benchmark */
    printf("\nRunning multi-threaded write benchmark...\n");
    bench_concurrent_write(&config, &results);
    print_results("Multi-threaded Write", &results);
    
    /* Latency benchmark */
    printf("\nRunning latency benchmark...\n");
    bench_latency(&config, &results);
    print_results("Latency", &results);
    
    /* Memory usage benchmark */
    bench_memory_usage(&config);
    
    printf("\n=== Benchmark Complete ===\n");
    
    return 0;
}
