//
//  ConfigManager.swift
//  ChronicleCollectors
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import os.log

/// Configuration manager for Chronicle collectors
public class ConfigManager {
    private let logger = Logger(subsystem: "com.chronicle.collectors", category: "ConfigManager")
    private let fileManager = FileManager.default
    private let configURL: URL
    private let userDefaults = UserDefaults.standard
    
    /// Shared instance
    public static let shared = ConfigManager()
    
    /// Current configuration
    @Published public private(set) var config: ChronicleConfig
    
    private init() {
        // Determine config file location
        let appSupport = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
        let chronicleDir = appSupport.appendingPathComponent("Chronicle")
        
        // Create directory if it doesn't exist
        try? fileManager.createDirectory(at: chronicleDir, withIntermediateDirectories: true)
        
        self.configURL = chronicleDir.appendingPathComponent("config.json")
        
        // Load configuration
        self.config = loadConfiguration()
        
        logger.info("Configuration manager initialized with config at: \(configURL.path)")
    }
    
    /// Load configuration from file
    public func loadConfiguration() -> ChronicleConfig {
        // Try to load from file first
        if let fileConfig = loadConfigurationFromFile() {
            return fileConfig
        }
        
        // Fall back to user defaults
        if let defaultsConfig = loadConfigurationFromDefaults() {
            return defaultsConfig
        }
        
        // Use default configuration
        let defaultConfig = ChronicleConfig.default
        logger.info("Using default configuration")
        return defaultConfig
    }
    
    /// Save configuration to file
    public func saveConfiguration(_ config: ChronicleConfig) throws {
        self.config = config
        
        do {
            let encoder = JSONEncoder()
            encoder.outputFormatting = .prettyPrinted
            encoder.dateEncodingStrategy = .iso8601
            
            let data = try encoder.encode(config)
            try data.write(to: configURL)
            
            // Also save to user defaults as backup
            saveConfigurationToDefaults(config)
            
            logger.info("Configuration saved to: \(configURL.path)")
        } catch {
            logger.error("Failed to save configuration: \(error)")
            throw ChronicleCollectorError.configurationError("Failed to save configuration: \(error)")
        }
    }
    
    /// Get collector configuration
    public func getCollectorConfig(_ collectorId: String) -> CollectorConfiguration {
        return config.collectors[collectorId] ?? CollectorConfiguration.default
    }
    
    /// Update collector configuration
    public func updateCollectorConfig(_ collectorId: String, config: CollectorConfiguration) throws {
        var newConfig = self.config
        newConfig.collectors[collectorId] = config
        try saveConfiguration(newConfig)
    }
    
    /// Reset to default configuration
    public func resetToDefaults() throws {
        try saveConfiguration(ChronicleConfig.default)
    }
    
    /// Reload configuration from file
    public func reloadConfiguration() {
        config = loadConfiguration()
    }
    
    // MARK: - Private Methods
    
    private func loadConfigurationFromFile() -> ChronicleConfig? {
        guard fileManager.fileExists(atPath: configURL.path) else {
            return nil
        }
        
        do {
            let data = try Data(contentsOf: configURL)
            let decoder = JSONDecoder()
            decoder.dateDecodingStrategy = .iso8601
            
            let config = try decoder.decode(ChronicleConfig.self, from: data)
            logger.info("Configuration loaded from file")
            return config
        } catch {
            logger.error("Failed to load configuration from file: \(error)")
            return nil
        }
    }
    
    private func loadConfigurationFromDefaults() -> ChronicleConfig? {
        guard let data = userDefaults.data(forKey: "ChronicleConfig") else {
            return nil
        }
        
        do {
            let decoder = JSONDecoder()
            decoder.dateDecodingStrategy = .iso8601
            
            let config = try decoder.decode(ChronicleConfig.self, from: data)
            logger.info("Configuration loaded from user defaults")
            return config
        } catch {
            logger.error("Failed to load configuration from user defaults: \(error)")
            return nil
        }
    }
    
    private func saveConfigurationToDefaults(_ config: ChronicleConfig) {
        do {
            let encoder = JSONEncoder()
            encoder.dateEncodingStrategy = .iso8601
            
            let data = try encoder.encode(config)
            userDefaults.set(data, forKey: "ChronicleConfig")
            userDefaults.synchronize()
        } catch {
            logger.error("Failed to save configuration to user defaults: \(error)")
        }
    }
}

/// Main Chronicle configuration
public struct ChronicleConfig: Codable {
    public let version: String
    public let createdAt: Date
    public let modifiedAt: Date
    public let general: GeneralConfig
    public let ringBuffer: RingBufferConfig
    public let collectors: [String: CollectorConfiguration]
    public let privacy: PrivacyConfig
    public let performance: PerformanceConfig
    public let logging: LoggingConfig
    
    public init(version: String = "1.0.0",
                createdAt: Date = Date(),
                modifiedAt: Date = Date(),
                general: GeneralConfig = GeneralConfig(),
                ringBuffer: RingBufferConfig = RingBufferConfig(),
                collectors: [String: CollectorConfiguration] = [:],
                privacy: PrivacyConfig = PrivacyConfig(),
                performance: PerformanceConfig = PerformanceConfig(),
                logging: LoggingConfig = LoggingConfig()) {
        self.version = version
        self.createdAt = createdAt
        self.modifiedAt = modifiedAt
        self.general = general
        self.ringBuffer = ringBuffer
        self.collectors = collectors
        self.privacy = privacy
        self.performance = performance
        self.logging = logging
    }
    
    public static let `default` = ChronicleConfig(
        collectors: [
            "key_tap": CollectorConfiguration(
                enabled: true,
                sampleRate: 1.0,
                activeFrameRate: 1.0,
                idleFrameRate: 0.2
            ),
            "screen_tap": CollectorConfiguration(
                enabled: true,
                sampleRate: 1.0,
                activeFrameRate: 1.0,
                idleFrameRate: 0.2
            ),
            "window_mon": CollectorConfiguration(
                enabled: true,
                sampleRate: 1.0,
                activeFrameRate: 2.0,
                idleFrameRate: 0.5
            ),
            "pointer_mon": CollectorConfiguration(
                enabled: true,
                sampleRate: 0.5,
                activeFrameRate: 10.0,
                idleFrameRate: 1.0
            ),
            "clip_mon": CollectorConfiguration(
                enabled: true,
                sampleRate: 1.0,
                activeFrameRate: 1.0,
                idleFrameRate: 0.1
            ),
            "fs_mon": CollectorConfiguration(
                enabled: true,
                sampleRate: 1.0
            ),
            "audio_mon": CollectorConfiguration(
                enabled: true,
                sampleRate: 1.0,
                activeFrameRate: 1.0,
                idleFrameRate: 0.1
            ),
            "net_mon": CollectorConfiguration(
                enabled: true,
                sampleRate: 1.0,
                activeFrameRate: 0.5,
                idleFrameRate: 0.1
            )
        ]
    )
}

/// General configuration
public struct GeneralConfig: Codable {
    public let appName: String
    public let appVersion: String
    public let enableAnalytics: Bool
    public let enableCrashReporting: Bool
    public let autoStartCollectors: Bool
    public let checkForUpdates: Bool
    
    public init(appName: String = "Chronicle",
                appVersion: String = "1.0.0",
                enableAnalytics: Bool = false,
                enableCrashReporting: Bool = true,
                autoStartCollectors: Bool = true,
                checkForUpdates: Bool = true) {
        self.appName = appName
        self.appVersion = appVersion
        self.enableAnalytics = enableAnalytics
        self.enableCrashReporting = enableCrashReporting
        self.autoStartCollectors = autoStartCollectors
        self.checkForUpdates = checkForUpdates
    }
}

/// Privacy configuration
public struct PrivacyConfig: Codable {
    public let enableDataEncryption: Bool
    public let encryptionKey: String?
    public let enableDataAnonymization: Bool
    public let dataRetentionDays: Int
    public let excludeApplications: [String]
    public let excludeWebsites: [String]
    public let excludeKeywords: [String]
    public let enableSensitiveDataFiltering: Bool
    
    public init(enableDataEncryption: Bool = true,
                encryptionKey: String? = nil,
                enableDataAnonymization: Bool = true,
                dataRetentionDays: Int = 30,
                excludeApplications: [String] = [],
                excludeWebsites: [String] = [],
                excludeKeywords: [String] = [],
                enableSensitiveDataFiltering: Bool = true) {
        self.enableDataEncryption = enableDataEncryption
        self.encryptionKey = encryptionKey
        self.enableDataAnonymization = enableDataAnonymization
        self.dataRetentionDays = dataRetentionDays
        self.excludeApplications = excludeApplications
        self.excludeWebsites = excludeWebsites
        self.excludeKeywords = excludeKeywords
        self.enableSensitiveDataFiltering = enableSensitiveDataFiltering
    }
}

/// Performance configuration
public struct PerformanceConfig: Codable {
    public let maxCpuUsage: Double
    public let maxMemoryUsage: Int64
    public let enablePerformanceMonitoring: Bool
    public let enableThrottling: Bool
    public let throttleThreshold: Double
    public let enableBatteryOptimization: Bool
    public let pauseOnLowBattery: Bool
    public let lowBatteryThreshold: Double
    
    public init(maxCpuUsage: Double = 10.0,
                maxMemoryUsage: Int64 = 1024 * 1024 * 500, // 500MB
                enablePerformanceMonitoring: Bool = true,
                enableThrottling: Bool = true,
                throttleThreshold: Double = 80.0,
                enableBatteryOptimization: Bool = true,
                pauseOnLowBattery: Bool = true,
                lowBatteryThreshold: Double = 0.2) {
        self.maxCpuUsage = maxCpuUsage
        self.maxMemoryUsage = maxMemoryUsage
        self.enablePerformanceMonitoring = enablePerformanceMonitoring
        self.enableThrottling = enableThrottling
        self.throttleThreshold = throttleThreshold
        self.enableBatteryOptimization = enableBatteryOptimization
        self.pauseOnLowBattery = pauseOnLowBattery
        self.lowBatteryThreshold = lowBatteryThreshold
    }
}

/// Logging configuration
public struct LoggingConfig: Codable {
    public let enableLogging: Bool
    public let logLevel: String
    public let enableFileLogging: Bool
    public let logFilePath: String?
    public let maxLogFileSize: Int64
    public let maxLogFiles: Int
    public let enableRemoteLogging: Bool
    public let remoteLoggingEndpoint: String?
    
    public init(enableLogging: Bool = true,
                logLevel: String = "info",
                enableFileLogging: Bool = true,
                logFilePath: String? = nil,
                maxLogFileSize: Int64 = 1024 * 1024 * 10, // 10MB
                maxLogFiles: Int = 5,
                enableRemoteLogging: Bool = false,
                remoteLoggingEndpoint: String? = nil) {
        self.enableLogging = enableLogging
        self.logLevel = logLevel
        self.enableFileLogging = enableFileLogging
        self.logFilePath = logFilePath
        self.maxLogFileSize = maxLogFileSize
        self.maxLogFiles = maxLogFiles
        self.enableRemoteLogging = enableRemoteLogging
        self.remoteLoggingEndpoint = remoteLoggingEndpoint
    }
}

// MARK: - Configuration Validation

extension ChronicleConfig {
    /// Validate configuration
    public func validate() throws {
        // Validate ring buffer configuration
        guard ringBuffer.bufferSize > 0 else {
            throw ChronicleCollectorError.configurationError("Ring buffer size must be greater than 0")
        }
        
        guard ringBuffer.maxEventSize > 0 else {
            throw ChronicleCollectorError.configurationError("Max event size must be greater than 0")
        }
        
        guard ringBuffer.maxEventSize <= ringBuffer.bufferSize else {
            throw ChronicleCollectorError.configurationError("Max event size cannot exceed ring buffer size")
        }
        
        // Validate performance configuration
        guard performance.maxCpuUsage > 0 && performance.maxCpuUsage <= 100 else {
            throw ChronicleCollectorError.configurationError("Max CPU usage must be between 0 and 100")
        }
        
        guard performance.maxMemoryUsage > 0 else {
            throw ChronicleCollectorError.configurationError("Max memory usage must be greater than 0")
        }
        
        // Validate privacy configuration
        guard privacy.dataRetentionDays > 0 else {
            throw ChronicleCollectorError.configurationError("Data retention days must be greater than 0")
        }
        
        // Validate collector configurations
        for (collectorId, config) in collectors {
            guard config.sampleRate > 0 && config.sampleRate <= 1 else {
                throw ChronicleCollectorError.configurationError("Sample rate for \(collectorId) must be between 0 and 1")
            }
            
            guard config.activeFrameRate > 0 else {
                throw ChronicleCollectorError.configurationError("Active frame rate for \(collectorId) must be greater than 0")
            }
            
            guard config.idleFrameRate > 0 else {
                throw ChronicleCollectorError.configurationError("Idle frame rate for \(collectorId) must be greater than 0")
            }
        }
    }
}

// MARK: - Configuration Helpers

extension ConfigManager {
    /// Get configuration value with fallback
    public func getValue<T>(_ keyPath: KeyPath<ChronicleConfig, T>, fallback: T) -> T {
        return config[keyPath: keyPath] ?? fallback
    }
    
    /// Update configuration value
    public func updateValue<T>(_ keyPath: WritableKeyPath<ChronicleConfig, T>, value: T) throws {
        var newConfig = config
        newConfig[keyPath: keyPath] = value
        try saveConfiguration(newConfig)
    }
    
    /// Get environment-specific configuration
    public func getEnvironmentConfig() -> [String: Any] {
        let env = ProcessInfo.processInfo.environment
        var envConfig: [String: Any] = [:]
        
        // Override with environment variables
        if let logLevel = env["CHRONICLE_LOG_LEVEL"] {
            envConfig["logLevel"] = logLevel
        }
        
        if let bufferSize = env["CHRONICLE_BUFFER_SIZE"], let size = Int(bufferSize) {
            envConfig["bufferSize"] = size
        }
        
        if let enableAnalytics = env["CHRONICLE_ENABLE_ANALYTICS"] {
            envConfig["enableAnalytics"] = Bool(enableAnalytics) ?? false
        }
        
        return envConfig
    }
}