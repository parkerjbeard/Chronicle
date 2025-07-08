/*
 * Distributed Ring Buffer Architecture
 * Future-proof design for scalability and reliability
 */

#ifndef DISTRIBUTED_BUFFER_H
#define DISTRIBUTED_BUFFER_H

#include <stdint.h>
#include <stddef.h>

// Buffer shard configuration
typedef struct {
    uint32_t shard_id;
    size_t capacity;
    char* mmap_path;
    uint64_t partition_key_start;
    uint64_t partition_key_end;
} buffer_shard_config_t;

// Distributed buffer manager
typedef struct {
    buffer_shard_config_t* shards;
    uint32_t num_shards;
    uint32_t replication_factor;
    uint64_t global_sequence;
    void* routing_table;
} distributed_buffer_t;

// Buffer operations
typedef enum {
    BUFFER_OP_WRITE,
    BUFFER_OP_READ,
    BUFFER_OP_REPLICATE,
    BUFFER_OP_COMPACT
} buffer_operation_t;

// Future-proof API
int distributed_buffer_create(distributed_buffer_t** buffer, 
                             const buffer_shard_config_t* config,
                             uint32_t num_shards);

int distributed_buffer_write(distributed_buffer_t* buffer,
                            const void* data,
                            size_t data_size,
                            uint64_t partition_key);

int distributed_buffer_read(distributed_buffer_t* buffer,
                           void** data,
                           size_t* data_size,
                           uint32_t shard_id,
                           uint64_t offset);

// Scaling operations
int distributed_buffer_add_shard(distributed_buffer_t* buffer,
                                const buffer_shard_config_t* new_shard);

int distributed_buffer_rebalance(distributed_buffer_t* buffer);

// Reliability operations
int distributed_buffer_replicate(distributed_buffer_t* buffer,
                                uint32_t source_shard,
                                uint32_t target_shard);

int distributed_buffer_recover(distributed_buffer_t* buffer,
                              uint32_t failed_shard);

void distributed_buffer_destroy(distributed_buffer_t* buffer);

#endif // DISTRIBUTED_BUFFER_H