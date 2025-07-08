//
//  RingBufferWriter.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import os.log

/// C ring buffer interface
private struct RingBuffer {
    var buffer: UnsafeMutableRawPointer
    var size: Int
    var head: Int
    var tail: Int
    var count: Int
    var mutex: pthread_mutex_t
}

/// Ring buffer writer for Chronicle events
public class RingBufferWriter {
    private let logger = Logger(subsystem: "com.chronicle.collectors", category: "RingBufferWriter")
    private var ringBuffer: UnsafeMutablePointer<RingBuffer>?
    private let bufferSize: Int
    private let encoder = JSONEncoder()
    
    /// Initialize ring buffer writer
    /// - Parameter bufferSize: Size of the ring buffer in bytes
    public init(bufferSize: Int = 1024 * 1024 * 100) { // 100MB default
        self.bufferSize = bufferSize
        self.encoder.dateEncodingStrategy = .secondsSince1970
        
        // Initialize ring buffer
        self.ringBuffer = UnsafeMutablePointer<RingBuffer>.allocate(capacity: 1)
        self.ringBuffer?.pointee = RingBuffer(
            buffer: UnsafeMutableRawPointer.allocate(byteCount: bufferSize, alignment: 8),
            size: bufferSize,
            head: 0,
            tail: 0,
            count: 0,
            mutex: pthread_mutex_t()
        )
        
        // Initialize mutex
        var attr = pthread_mutexattr_t()
        pthread_mutexattr_init(&attr)
        pthread_mutexattr_settype(&attr, PTHREAD_MUTEX_RECURSIVE)
        pthread_mutex_init(&ringBuffer!.pointee.mutex, &attr)
        pthread_mutexattr_destroy(&attr)
        
        logger.info("Ring buffer writer initialized with size: \(bufferSize) bytes")
    }
    
    deinit {
        guard let ringBuffer = ringBuffer else { return }
        
        // Clean up
        pthread_mutex_destroy(&ringBuffer.pointee.mutex)
        ringBuffer.pointee.buffer.deallocate()
        ringBuffer.deallocate()
        
        logger.info("Ring buffer writer deinitialized")
    }
    
    /// Write event to ring buffer
    /// - Parameter event: Chronicle event to write
    /// - Throws: ChronicleCollectorError if write fails
    public func write(_ event: ChronicleEvent) throws {
        guard let ringBuffer = ringBuffer else {
            throw ChronicleCollectorError.ringBufferWriteError("Ring buffer not initialized")
        }
        
        // Serialize event to JSON
        let eventData: Data
        do {
            eventData = try encoder.encode(event)
        } catch {
            throw ChronicleCollectorError.serializationError("Failed to encode event: \(error)")
        }
        
        // Create Arrow IPC message
        let arrowData = try createArrowIPCMessage(from: eventData)
        
        // Write to ring buffer
        let result = pthread_mutex_lock(&ringBuffer.pointee.mutex)
        guard result == 0 else {
            throw ChronicleCollectorError.ringBufferWriteError("Failed to lock mutex: \(result)")
        }
        
        defer {
            pthread_mutex_unlock(&ringBuffer.pointee.mutex)
        }
        
        // Check if we have enough space
        let messageSize = arrowData.count + MemoryLayout<UInt32>.size
        if availableSpace(in: ringBuffer) < messageSize {
            // Make space by advancing tail
            try makeSpace(in: ringBuffer, needed: messageSize)
        }
        
        // Write message size first
        let sizeData = withUnsafeBytes(of: UInt32(arrowData.count)) { Data($0) }
        try writeData(sizeData, to: ringBuffer)
        
        // Write message data
        try writeData(arrowData, to: ringBuffer)
        
        logger.debug("Wrote event \(event.id) of type \(event.type) to ring buffer")
    }
    
    /// Get ring buffer statistics
    public func getStatistics() -> RingBufferStatistics {
        guard let ringBuffer = ringBuffer else {
            return RingBufferStatistics(size: 0, used: 0, available: 0, eventCount: 0)
        }
        
        pthread_mutex_lock(&ringBuffer.pointee.mutex)
        defer { pthread_mutex_unlock(&ringBuffer.pointee.mutex) }
        
        let used = usedSpace(in: ringBuffer)
        let available = availableSpace(in: ringBuffer)
        
        return RingBufferStatistics(
            size: bufferSize,
            used: used,
            available: available,
            eventCount: ringBuffer.pointee.count
        )
    }
    
    /// Clear ring buffer
    public func clear() {
        guard let ringBuffer = ringBuffer else { return }
        
        pthread_mutex_lock(&ringBuffer.pointee.mutex)
        defer { pthread_mutex_unlock(&ringBuffer.pointee.mutex) }
        
        ringBuffer.pointee.head = 0
        ringBuffer.pointee.tail = 0
        ringBuffer.pointee.count = 0
        
        logger.info("Ring buffer cleared")
    }
    
    // MARK: - Private Methods
    
    private func createArrowIPCMessage(from data: Data) throws -> Data {
        // Simple Arrow IPC format implementation
        // In a real implementation, this would use proper Arrow IPC format
        let header = ArrowIPCHeader(
            metadataLength: 0,
            bodyLength: UInt64(data.count),
            timestamp: UInt64(Date().timeIntervalSince1970 * 1000)
        )
        
        var result = Data()
        result.append(withUnsafeBytes(of: header) { Data($0) })
        result.append(data)
        
        return result
    }
    
    private func availableSpace(in ringBuffer: UnsafeMutablePointer<RingBuffer>) -> Int {
        let rb = ringBuffer.pointee
        if rb.head == rb.tail && rb.count == 0 {
            return rb.size
        } else if rb.head == rb.tail {
            return 0
        } else if rb.head > rb.tail {
            return rb.size - rb.head + rb.tail
        } else {
            return rb.tail - rb.head
        }
    }
    
    private func usedSpace(in ringBuffer: UnsafeMutablePointer<RingBuffer>) -> Int {
        return bufferSize - availableSpace(in: ringBuffer)
    }
    
    private func makeSpace(in ringBuffer: UnsafeMutablePointer<RingBuffer>, needed: Int) throws {
        // Advance tail to make space
        var freed = 0
        
        while freed < needed && ringBuffer.pointee.count > 0 {
            // Read message size
            let sizeBytes = UnsafeMutablePointer<UInt32>.allocate(capacity: 1)
            defer { sizeBytes.deallocate() }
            
            let sizeData = ringBuffer.pointee.buffer.advanced(by: ringBuffer.pointee.tail)
            sizeData.copyMemory(from: sizeBytes, byteCount: MemoryLayout<UInt32>.size)
            
            let messageSize = Int(sizeBytes.pointee)
            let totalSize = messageSize + MemoryLayout<UInt32>.size
            
            // Advance tail
            ringBuffer.pointee.tail = (ringBuffer.pointee.tail + totalSize) % ringBuffer.pointee.size
            ringBuffer.pointee.count -= 1
            freed += totalSize
        }
    }
    
    private func writeData(_ data: Data, to ringBuffer: UnsafeMutablePointer<RingBuffer>) throws {
        let rb = ringBuffer.pointee
        
        data.withUnsafeBytes { bytes in
            let bytesToWrite = bytes.count
            
            if rb.head + bytesToWrite <= rb.size {
                // Write in one piece
                rb.buffer.advanced(by: rb.head).copyMemory(from: bytes.baseAddress!, byteCount: bytesToWrite)
            } else {
                // Write in two pieces (wrap around)
                let firstPart = rb.size - rb.head
                let secondPart = bytesToWrite - firstPart
                
                rb.buffer.advanced(by: rb.head).copyMemory(from: bytes.baseAddress!, byteCount: firstPart)
                rb.buffer.copyMemory(from: bytes.baseAddress!.advanced(by: firstPart), byteCount: secondPart)
            }
        }
        
        ringBuffer.pointee.head = (rb.head + data.count) % rb.size
        ringBuffer.pointee.count += 1
    }
}

/// Ring buffer statistics
public struct RingBufferStatistics {
    public let size: Int
    public let used: Int
    public let available: Int
    public let eventCount: Int
    
    public var utilizationPercentage: Double {
        return size > 0 ? Double(used) / Double(size) * 100.0 : 0.0
    }
}

/// Arrow IPC header structure
private struct ArrowIPCHeader {
    let metadataLength: UInt32
    let bodyLength: UInt64
    let timestamp: UInt64
}

/// Ring buffer configuration
public struct RingBufferConfig {
    public let bufferSize: Int
    public let maxEventSize: Int
    public let compressionEnabled: Bool
    public let flushInterval: TimeInterval
    
    public init(bufferSize: Int = 1024 * 1024 * 100,
                maxEventSize: Int = 1024 * 1024,
                compressionEnabled: Bool = true,
                flushInterval: TimeInterval = 5.0) {
        self.bufferSize = bufferSize
        self.maxEventSize = maxEventSize
        self.compressionEnabled = compressionEnabled
        self.flushInterval = flushInterval
    }
    
    public static let `default` = RingBufferConfig()
}

/// Thread-safe ring buffer writer with performance optimizations
public class PerformantRingBufferWriter {
    private let writer: RingBufferWriter
    private let queue: DispatchQueue
    private let config: RingBufferConfig
    private var writeCount: Int64 = 0
    private var lastFlushTime: TimeInterval = 0
    private let logger = Logger(subsystem: "com.chronicle.collectors", category: "PerformantRingBufferWriter")
    
    public init(config: RingBufferConfig = .default) {
        self.config = config
        self.writer = RingBufferWriter(bufferSize: config.bufferSize)
        self.queue = DispatchQueue(label: "com.chronicle.ringbuffer", qos: .utility)
        self.lastFlushTime = Date().timeIntervalSince1970
    }
    
    /// Write event asynchronously
    public func writeAsync(_ event: ChronicleEvent) {
        queue.async { [weak self] in
            self?.writeSync(event)
        }
    }
    
    /// Write event synchronously
    public func writeSync(_ event: ChronicleEvent) {
        do {
            try writer.write(event)
            
            OSAtomicIncrement64(&writeCount)
            
            // Check if we should flush
            let now = Date().timeIntervalSince1970
            if now - lastFlushTime >= config.flushInterval {
                flush()
                lastFlushTime = now
            }
        } catch {
            logger.error("Failed to write event: \(error)")
        }
    }
    
    /// Force flush buffer
    public func flush() {
        // In a real implementation, this would flush to disk
        logger.debug("Flushed ring buffer (write count: \(writeCount))")
    }
    
    /// Get statistics
    public func getStatistics() -> RingBufferStatistics {
        return writer.getStatistics()
    }
    
    /// Clear buffer
    public func clear() {
        queue.sync {
            writer.clear()
            OSAtomicAnd64(0, &writeCount)
        }
    }
}